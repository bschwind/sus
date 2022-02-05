use crate::{events::OutgoingPacket, MyName, SusGame, GAME_TIMESTEP_LABEL};
use std::time::Duration;
use sus_common::{
    laminar::{Config as NetworkConfig, Socket, SocketEvent},
    network::{
        make_packet, ClientToServer, ConnectAckPacket, ConnectPacket, DeliveryType,
        FullGameStatePacket, LobbyTickPacket, NewPlayerPacket, ServerToClient,
    },
    resources::network::{NetRx, NetTx, NetworkThread},
    simple_game::bevy::{
        App, Commands, EventWriter, Events, FixedTimestep, ParallelSystemDescriptorCoercion,
        Plugin, Res, ResMut, SystemSet,
    },
    systems::labels,
};

pub struct ClientNetworkPlugin {
    fixed_timestep: f64,
}

impl ClientNetworkPlugin {
    pub fn new(update_fps: usize) -> Self {
        Self { fixed_timestep: 1.0 / update_fps as f64 }
    }
}

impl Plugin for ClientNetworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup)
            .add_event::<ConnectAckPacket>()
            .add_event::<NewPlayerPacket>()
            .add_event::<FullGameStatePacket>()
            .add_event::<LobbyTickPacket>()
            .init_resource::<Events<OutgoingPacket>>()
            .add_system_set(
                SystemSet::new()
                    .with_run_criteria(
                        FixedTimestep::step(self.fixed_timestep).with_label(GAME_TIMESTEP_LABEL),
                    )
                    .label(labels::Network)
                    .with_system(network_receive.label(labels::NetworkSystem::Receive)),
            )
            .add_system_set(
                SystemSet::new()
                    .with_run_criteria(
                        FixedTimestep::step(self.fixed_timestep).with_label(GAME_TIMESTEP_LABEL),
                    )
                    .with_system(
                        network_send.label(labels::NetworkSystem::SendPackets).after(labels::Lobby), // TODO - Use better ordering here.
                    ),
            );
    }
}

fn setup(
    mut commands: Commands,
    my_name: Res<MyName>,
    mut outgoing_packets: EventWriter<OutgoingPacket>,
) {
    let mut socket = initialize_network();
    let (net_tx, net_rx) = (socket.get_packet_sender(), socket.get_event_receiver());

    let network_thread = std::thread::spawn(move || socket.start_polling());

    let connect_packet = ClientToServer::Connect(ConnectPacket::new(&my_name.0));

    outgoing_packets.send(OutgoingPacket::new(connect_packet, DeliveryType::ReliableOrdered, None));

    commands.insert_resource(NetworkThread(network_thread));
    commands.insert_resource(NetTx(net_tx));
    commands.insert_resource(NetRx(net_rx));
}

fn initialize_network() -> Socket {
    let net_config = NetworkConfig {
        idle_connection_timeout: Duration::from_secs(5),
        heartbeat_interval: Some(Duration::from_secs(4)),
        ..NetworkConfig::default()
    };

    let socket =
        Socket::bind_with_config("0.0.0.0:0", net_config).expect("Could not connect to server");

    socket
}

fn network_receive(
    mut game: ResMut<SusGame>,
    net_rx: Res<NetRx>,
    mut connect_ack_tx: EventWriter<ConnectAckPacket>,
    mut new_player_tx: EventWriter<NewPlayerPacket>,
    mut full_game_state_tx: EventWriter<FullGameStatePacket>,
    mut lobby_tick_tx: EventWriter<LobbyTickPacket>,
) {
    let net_rx = &net_rx.0;

    for event in net_rx.try_iter() {
        match event {
            SocketEvent::Packet(packet) => {
                let msg = packet.payload();

                if packet.addr() == game.server_addr {
                    if let Ok(decoded) = bincode::deserialize::<ServerToClient>(msg) {
                        match decoded {
                            ServerToClient::ConnectAck(connect_ack_packet) => {
                                println!(
                                    "Server accepted us, yay! Our id is {}",
                                    connect_ack_packet.id
                                );
                                game.connected = true;

                                connect_ack_tx.send(connect_ack_packet);
                            },
                            ServerToClient::NewPlayer(new_player_packet) => {
                                println!("New player: {:?}", new_player_packet);
                                new_player_tx.send(new_player_packet);
                            },
                            ServerToClient::FullGameState(full_game_state) => {
                                println!("Full game state: {:?}", full_game_state);
                                full_game_state_tx.send(full_game_state);
                            },
                            ServerToClient::LobbyTick(lobby_tick_packet) => {
                                // println!("Lobby tick - {:?}", lobby_tick_packet);
                                lobby_tick_tx.send(lobby_tick_packet);
                            },
                        }
                    }
                } else {
                    println!("Unknown sender.");
                }
            },
            SocketEvent::Timeout(addr) => {
                println!("Server timed out: {}", addr);
            },
            SocketEvent::Connect(addr) => {
                println!("Server connected: {}", addr);
            },
            SocketEvent::Disconnect(addr) => {
                println!("Server disconnected: {}", addr);
            },
        }
    }
}

fn network_send(
    game: Res<SusGame>,
    net_tx: Res<NetTx>,
    mut outgoing_packets: ResMut<Events<OutgoingPacket>>, // Manual event cleanup
) {
    let net_tx = &net_tx.0;

    // Event cleanup happens here with .drain()
    for outgoing in outgoing_packets.drain() {
        if !game.connected && matches!(outgoing.packet, ClientToServer::PlayerInput(_)) {
            // Don't send input packets until we're connected.
            continue;
        }

        let data = bincode::serialize(&outgoing.packet).unwrap();

        let packet =
            make_packet(outgoing.delivery_type, data, game.server_addr, outgoing.stream_id);

        if let Err(e) = net_tx.send(packet) {
            println!("Failed to send packet: {:?}", e);
        }
    }
}
