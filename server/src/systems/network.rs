use crate::{
    events::{NewPlayer, OutgoingPacket, PlayerInput},
    resources::AddrToPlayer,
    systems::labels,
    TICK_RATE_HZ,
};
use std::{collections::HashMap, net::SocketAddr, time::Duration};
use sus_common::{
    components::player::PlayerNetworkAddr,
    laminar::{Config as NetworkConfig, Socket, SocketEvent},
    network::{make_packet, ClientToServer},
    resources::{
        network::{NetRx, NetTx, NetworkThread},
        PlayerToEntity,
    },
    simple_game::bevy::{
        bevy_ecs, bevy_ecs::event::Events, App, Commands, EventWriter, IntoSystemConfig, Plugin,
        Query, Res, ResMut, Resource,
    },
};

const BIND_ADDR: &str = "0.0.0.0:7600";

pub struct ServerNetworkPlugin;

#[derive(Debug, Resource)]
pub struct PlayerIdCounter(pub u16);

impl Plugin for ServerNetworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup)
            .add_event::<PlayerInput>()
            .init_resource::<Events<NewPlayer>>()
            .init_resource::<Events<OutgoingPacket>>()
            .add_system(
                network_receive.in_set(labels::NetworkSystem::Receive).in_set(labels::Network),
            )
            .add_system(
                network_send
                    .in_set(labels::NetworkSystem::SendPackets)
                    .in_set(labels::Network)
                    .after(labels::Lobby), // TODO - Use better ordering here.
            );
    }
}

fn setup(mut commands: Commands) {
    let mut socket = initialize_network();
    let (net_tx, net_rx) = (socket.get_packet_sender(), socket.get_event_receiver());

    let network_thread = std::thread::spawn(move || {
        socket.start_polling_with_duration(Some(std::time::Duration::from_millis(
            (1000 / TICK_RATE_HZ / 2) as u64,
        )))
    });

    commands.insert_resource(NetworkThread(network_thread));
    commands.insert_resource(NetTx(net_tx));
    commands.insert_resource(NetRx(net_rx));
    commands.insert_resource(AddrToPlayer(HashMap::new()));
    commands.insert_resource(PlayerToEntity(HashMap::new()));
    commands.insert_resource(PlayerIdCounter(0));
}

fn initialize_network() -> Socket {
    // TODO(bschwind) - Remove this once we start having a steady flow of packets.
    let net_config = NetworkConfig {
        idle_connection_timeout: Duration::from_secs(5),
        heartbeat_interval: Some(Duration::from_secs(4)),
        ..NetworkConfig::default()
    };

    let socket =
        Socket::bind_with_config(BIND_ADDR, net_config).expect("Couldn't bind to server BIND_ADDR");

    println!("Listening on {:?}", BIND_ADDR);

    socket
}

fn network_receive(
    mut players: ResMut<AddrToPlayer>,
    net_rx: Res<NetRx>,
    mut new_player_tx: EventWriter<NewPlayer>,
    mut input_tx: EventWriter<PlayerInput>,
) {
    let players = &mut players.0;
    let net_rx = &net_rx.0;

    // println!("Network tick");

    for event in net_rx.try_iter() {
        match event {
            SocketEvent::Packet(packet) => {
                let msg = packet.payload();

                if let Ok(decoded) = bincode::deserialize::<ClientToServer>(msg) {
                    match decoded {
                        ClientToServer::Connect(connect_packet) => {
                            new_player_tx.send(NewPlayer { addr: packet.addr(), connect_packet });
                        },
                        ClientToServer::PlayerInput(input) => {
                            if let Some(player_id) = players.get(&packet.addr()) {
                                input_tx.send(PlayerInput { id: *player_id, input });
                            }
                        },
                    }
                } else {
                    println!("Received an invalid packet");
                }
            },
            SocketEvent::Timeout(addr) => {
                if let Some(player_id) = players.get(&addr) {
                    println!("{} ({}) timed out", player_id, addr);
                } else {
                    println!("Unknown player timed out: {}", addr);
                }
            },
            SocketEvent::Connect(addr) => {
                println!("Client connected: {}", addr);
            },
            SocketEvent::Disconnect(addr) => {
                if let Some(player_id) = players.remove(&addr) {
                    println!("Player {} disconnected ({})", player_id, addr);
                } else {
                    println!("Unknown player disconnected: {}", addr);
                }
            },
        }
    }
}

#[allow(unused)]
pub enum PacketDestination {
    Single(SocketAddr),
    BroadcastToAll,
    BroadcastToAllExcept(SocketAddr),
    BroadcastToSet(Vec<SocketAddr>),
}

fn network_send(
    net_tx: Res<NetTx>,
    mut outgoing_packets: ResMut<Events<OutgoingPacket>>,
    player_addrs: Query<&PlayerNetworkAddr>,
) {
    let net_tx = &net_tx.0;

    for outgoing in outgoing_packets.drain() {
        let data = bincode::serialize(&outgoing.packet).unwrap();

        match &outgoing.destination {
            PacketDestination::Single(addr) => {
                let packet = make_packet(outgoing.delivery_type, data, *addr, outgoing.stream_id);

                if let Err(e) = net_tx.send(packet) {
                    println!("Failed to send packet: {:?}", e);
                }
            },
            PacketDestination::BroadcastToAll => {
                player_addrs.iter().for_each(|PlayerNetworkAddr(addr)| {
                    // TODO(bschwind) - Ideally we wouldn't clone this Vec here, but laminar
                    // packets take a Vec<u8> instead of a slice.
                    let packet = make_packet(
                        outgoing.delivery_type,
                        data.clone(),
                        *addr,
                        outgoing.stream_id,
                    );

                    if let Err(e) = net_tx.send(packet) {
                        println!("Failed to send packet: {:?}", e);
                    }
                });
            },
            PacketDestination::BroadcastToAllExcept(exclude_addr) => {
                player_addrs
                    .iter()
                    .filter(|PlayerNetworkAddr(addr)| *addr != *exclude_addr)
                    .for_each(|PlayerNetworkAddr(addr)| {
                        let packet = make_packet(
                            outgoing.delivery_type,
                            data.clone(),
                            *addr,
                            outgoing.stream_id,
                        );

                        if let Err(e) = net_tx.send(packet) {
                            println!("Failed to send packet: {:?}", e);
                        }
                    });
            },
            PacketDestination::BroadcastToSet(addrs) => {
                addrs.iter().for_each(|addr| {
                    let packet = make_packet(
                        outgoing.delivery_type,
                        data.clone(),
                        *addr,
                        outgoing.stream_id,
                    );

                    if let Err(e) = net_tx.send(packet) {
                        println!("Failed to send packet: {:?}", e);
                    }
                });
            },
        }
    }
}
