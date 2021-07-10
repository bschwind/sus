use crossbeam_channel::{Receiver, Sender};
use laminar::{Config as NetworkConfig, Packet, Socket, SocketEvent};
use simple_game::bevy::{
    App, AppBuilder, Commands, CorePlugin, EventReader, EventWriter, FixedTimestep,
    HeadlessBevyGame, IntoSystem, ResMut, SystemSet,
};
use std::{collections::HashMap, net::SocketAddr, thread::JoinHandle, time::Duration};
use sus_common::{
    network::{
        ClientToServer, ConnectPacket, FullGameStatePacket, NewPlayerPacket, PlayerInputPacket,
        ServerToClient,
    },
    Player,
};

const BIND_ADDR: &str = "0.0.0.0:7600";
const MAX_PLAYERS: usize = 16;

struct SusServer {
    _network_thread: JoinHandle<()>,
    net_tx: Sender<laminar::Packet>,
    net_rx: Receiver<SocketEvent>,
    players: HashMap<SocketAddr, Player>,
    id_counter: u16,
}

struct PlayerInput {
    id: u16,
    input: PlayerInputPacket,
}

struct NewPlayer {
    addr: SocketAddr,
    connect_packet: ConnectPacket,
}

impl HeadlessBevyGame for SusServer {
    fn init_systems() -> AppBuilder {
        let mut ecs_world_builder = App::build();

        ecs_world_builder
            .add_plugin(CorePlugin)
            .add_event::<PlayerInput>()
            .add_event::<NewPlayer>()
            .add_startup_system(init.system())
            .add_system_set(
                SystemSet::new()
                    .with_run_criteria(
                        FixedTimestep::step(1.0 / Self::desired_fps() as f64)
                            .with_label("game_timestep"),
                    )
                    .with_system(update_network.system()) // TOOD(bschwind) - Enforce ordering on network systems
                    .with_system(handle_player_input.system())
                    .with_system(handle_new_player.system()),
            );

        ecs_world_builder
    }
}

fn init(mut commands: Commands) {
    let mut socket = initialize_network();
    let (net_tx, net_rx) = (socket.get_packet_sender(), socket.get_event_receiver());

    let server = SusServer {
        _network_thread: std::thread::spawn(move || socket.start_polling()),
        net_rx,
        net_tx,
        players: HashMap::new(),
        id_counter: 0,
    };

    commands.insert_resource(server);
}

fn initialize_network() -> Socket {
    // TODO(bschwind) - Remove this once we start having a steady flow of packets.
    let net_config = NetworkConfig {
        idle_connection_timeout: Duration::from_secs(5),
        heartbeat_interval: Some(Duration::from_secs(4)),
        ..NetworkConfig::default()
    };

    Socket::bind_with_config(BIND_ADDR, net_config).expect("Couldn't bind to server BIND_ADDR")
}

fn handle_player_input(mut input_rx: EventReader<PlayerInput>) {
    for event in input_rx.iter() {
        println!("Player (id={}) sent input: {:?}", event.id, event.input);
    }
}

fn handle_new_player(mut server: ResMut<SusServer>, mut new_player_rx: EventReader<NewPlayer>) {
    for new_player in new_player_rx.iter() {
        let addr = new_player.addr;
        let connect_packet = &new_player.connect_packet;

        println!(
            "{} (ip = {}) connected with game version {}",
            connect_packet.name, addr, connect_packet.version
        );

        if server.players.len() >= MAX_PLAYERS {
            println!("Max players exceeded, not accepting");
        } else {
            let new_player_id = server.id_counter;
            let new_player_pos = (0, 0);
            server.players.insert(addr, Player::new(&connect_packet.name, new_player_id));
            server.id_counter += 1;

            let reply = ServerToClient::ConnectAck;
            server
                .net_tx
                .send(Packet::reliable_ordered(addr, bincode::serialize(&reply).unwrap(), None))
                .expect("Failed to send ConnectAck");
            // Send all existing state to new client
            let players_vec = server
                .players
                .values()
                .map(|p| NewPlayerPacket::new(p.name.clone(), p.id, p.pos))
                .collect();

            let full_state_packet =
                ServerToClient::FullGameState(FullGameStatePacket::new(players_vec));
            server
                .net_tx
                .send(Packet::reliable_ordered(
                    addr,
                    bincode::serialize(&full_state_packet).unwrap(),
                    None,
                ))
                .expect("Failed to send ConnectAck");

            // Tell all other players this one has connected
            let new_player_packet = ServerToClient::NewPlayer(NewPlayerPacket::new(
                connect_packet.name.clone(), // TODO(bschwind) - can we get an EventReader iterator which gives owned values?
                new_player_id,
                new_player_pos,
            ));
            for player_addr in server.players.keys() {
                if *player_addr != addr {
                    server
                        .net_tx
                        .send(Packet::reliable_ordered(
                            *player_addr,
                            bincode::serialize(&new_player_packet).unwrap(),
                            None,
                        ))
                        .expect("Failed to send ConnectAck");
                }
            }
        }
    }
}

fn update_network(
    mut server: ResMut<SusServer>,
    mut new_player_tx: EventWriter<NewPlayer>,
    mut input_tx: EventWriter<PlayerInput>,
) {
    if let Ok(event) = server.net_rx.recv() {
        match event {
            SocketEvent::Packet(packet) => {
                let msg = packet.payload();

                if let Ok(decoded) = bincode::deserialize::<ClientToServer>(msg) {
                    match decoded {
                        ClientToServer::Connect(connect_packet) => {
                            new_player_tx.send(NewPlayer { addr: packet.addr(), connect_packet });
                        },
                        ClientToServer::PlayerInput(input) => {
                            if let Some(player) = server.players.get(&packet.addr()) {
                                input_tx.send(PlayerInput { id: player.id, input });
                            }
                        },
                    }
                } else {
                    println!("Received an invalid packet");
                }
            },
            SocketEvent::Timeout(addr) => {
                if let Some(player) = server.players.get(&addr) {
                    println!("{} ({}) timed out", player.name, addr);
                } else {
                    println!("Unknown player timed out: {}", addr);
                }
            },
            SocketEvent::Connect(addr) => {
                println!("Client connected: {}", addr);
            },
            SocketEvent::Disconnect(addr) => {
                if let Some(player) = server.players.remove(&addr) {
                    println!("Player {} disconnected ({})", player.name, addr);
                } else {
                    println!("Unknown player disconnected: {}", addr);
                }
            },
        }
    }
}

fn main() {
    simple_game::bevy::run_headless_bevy_game::<SusServer>();
}
