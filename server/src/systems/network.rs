use crate::{systems::labels, MAX_PLAYERS};
use crossbeam_channel::{Receiver, Sender};
use laminar::{Config as NetworkConfig, Packet, Socket, SocketEvent};
use simple_game::bevy::{
    AppBuilder, Commands, EventReader, EventWriter, IntoSystem, ParallelSystemDescriptorCoercion,
    Plugin, Res, ResMut, SystemSet,
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

pub struct NetworkPlugin;

struct NetworkThread(JoinHandle<()>);
struct NetTx(Sender<laminar::Packet>);
struct NetRx(Receiver<SocketEvent>);
struct AddrToPlayer(HashMap<SocketAddr, Player>);
struct PlayerIdCounter(u16);

struct NewPlayer {
    addr: SocketAddr,
    connect_packet: ConnectPacket,
}

struct PlayerInput {
    id: u16,
    input: PlayerInputPacket,
}

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup.system())
            .add_event::<PlayerInput>()
            .add_event::<NewPlayer>()
            .add_system_set(
                SystemSet::new()
                    .label(labels::Network)
                    .with_system(network_receive.system().label(labels::NetworkSystem::Receive))
                    .with_system(
                        handle_player_input
                            .system()
                            .label(labels::NetworkSystem::PlayerInput)
                            .after(labels::NetworkSystem::Receive),
                    )
                    .with_system(
                        handle_new_player
                            .system()
                            .label(labels::NetworkSystem::NewPlayer)
                            .after(labels::NetworkSystem::Receive),
                    ),
            );
    }
}

fn setup(mut commands: Commands) {
    let mut socket = initialize_network();
    let (net_tx, net_rx) = (socket.get_packet_sender(), socket.get_event_receiver());

    let network_thread = std::thread::spawn(move || socket.start_polling());

    commands.insert_resource(NetworkThread(network_thread));
    commands.insert_resource(NetTx(net_tx));
    commands.insert_resource(NetRx(net_rx));
    commands.insert_resource(AddrToPlayer(HashMap::new()));
    commands.insert_resource(PlayerIdCounter(0));
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

fn handle_new_player(
    mut players: ResMut<AddrToPlayer>,
    mut player_id_counter: ResMut<PlayerIdCounter>,
    net_tx: ResMut<NetTx>,
    mut new_player_rx: EventReader<NewPlayer>,
) {
    let players = &mut players.0;
    let player_id_counter = &mut player_id_counter.0;
    let net_tx = &net_tx.0;

    for new_player in new_player_rx.iter() {
        let addr = new_player.addr;
        let connect_packet = &new_player.connect_packet;

        println!(
            "{} (ip = {}) connected with game version {}",
            connect_packet.name, addr, connect_packet.version
        );

        if players.len() >= MAX_PLAYERS {
            println!("Max players exceeded, not accepting");
        } else {
            let new_player_id = *player_id_counter;
            let new_player_pos = (0, 0);
            players.insert(addr, Player::new(&connect_packet.name, new_player_id));
            *player_id_counter += 1;

            let reply = ServerToClient::ConnectAck;
            net_tx
                .send(Packet::reliable_ordered(addr, bincode::serialize(&reply).unwrap(), None))
                .expect("Failed to send ConnectAck");

            // Send all existing state to new client
            let players_vec = players
                .values()
                .map(|p| NewPlayerPacket::new(p.name.clone(), p.id, p.pos))
                .collect();

            let full_state_packet =
                ServerToClient::FullGameState(FullGameStatePacket::new(players_vec));
            net_tx
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

            for player_addr in players.keys() {
                if *player_addr != addr {
                    net_tx
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

fn network_receive(
    mut players: ResMut<AddrToPlayer>,
    net_rx: Res<NetRx>,
    mut new_player_tx: EventWriter<NewPlayer>,
    mut input_tx: EventWriter<PlayerInput>,
) {
    let players = &mut players.0;
    let net_rx = &net_rx.0;

    while let Ok(event) = net_rx.try_recv() {
        match event {
            SocketEvent::Packet(packet) => {
                let msg = packet.payload();

                if let Ok(decoded) = bincode::deserialize::<ClientToServer>(msg) {
                    match decoded {
                        ClientToServer::Connect(connect_packet) => {
                            new_player_tx.send(NewPlayer { addr: packet.addr(), connect_packet });
                        },
                        ClientToServer::PlayerInput(input) => {
                            if let Some(player) = players.get(&packet.addr()) {
                                input_tx.send(PlayerInput { id: player.id, input });
                            }
                        },
                    }
                } else {
                    println!("Received an invalid packet");
                }
            },
            SocketEvent::Timeout(addr) => {
                if let Some(player) = players.get(&addr) {
                    println!("{} ({}) timed out", player.name, addr);
                } else {
                    println!("Unknown player timed out: {}", addr);
                }
            },
            SocketEvent::Connect(addr) => {
                println!("Client connected: {}", addr);
            },
            SocketEvent::Disconnect(addr) => {
                if let Some(player) = players.remove(&addr) {
                    println!("Player {} disconnected ({})", player.name, addr);
                } else {
                    println!("Unknown player disconnected: {}", addr);
                }
            },
        }
    }
}
