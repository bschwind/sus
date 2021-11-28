use crate::{
    components::{AddrToPlayer, PlayerToEntity},
    systems::labels,
};
use crossbeam_channel::{Receiver, Sender};
use laminar::{Config as NetworkConfig, Packet, Socket, SocketEvent};
use std::{collections::HashMap, net::SocketAddr, thread::JoinHandle, time::Duration};
use sus_common::{
    components::player::PlayerNetworkAddr,
    network::{ClientToServer, ConnectPacket, PlayerInputPacket, ServerToClient},
    simple_game::bevy::{
        AppBuilder, Commands, EventReader, EventWriter, IntoSystem,
        ParallelSystemDescriptorCoercion, Plugin, Query, Res, ResMut, SystemSet,
    },
};

const BIND_ADDR: &str = "0.0.0.0:7600";

pub struct NetworkPlugin;

struct NetworkThread(JoinHandle<()>);
struct NetTx(Sender<laminar::Packet>);
struct NetRx(Receiver<SocketEvent>);
pub struct PlayerIdCounter(pub u16);

pub struct NewPlayer {
    pub addr: SocketAddr,
    pub connect_packet: ConnectPacket,
}

#[derive(Debug)]
pub struct PlayerInput {
    pub id: u16,
    pub input: PlayerInputPacket,
}

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup.system())
            .add_event::<PlayerInput>()
            .add_event::<NewPlayer>()
            .add_event::<OutgoingPacket>()
            .add_system_set(
                SystemSet::new()
                    .label(labels::Network)
                    .with_system(network_receive.system().label(labels::NetworkSystem::Receive)),
            )
            .add_system(
                network_send
                    .system()
                    .label(labels::NetworkSystem::SendPackets)
                    .after(labels::Lobby), // TODO - Use better ordering here.
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

    Socket::bind_with_config(BIND_ADDR, net_config).expect("Couldn't bind to server BIND_ADDR")
}

fn network_receive(
    mut players: ResMut<AddrToPlayer>,
    net_rx: Res<NetRx>,
    mut new_player_tx: EventWriter<NewPlayer>,
    mut input_tx: EventWriter<PlayerInput>,
) {
    let players = &mut players.0;
    let net_rx = &net_rx.0;

    println!("Network tick");

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

pub struct OutgoingPacket {
    destination: PacketDestination,
    packet: ServerToClient,
    delivery_type: DeliveryType,
}

impl OutgoingPacket {
    pub fn new(
        destination: PacketDestination,
        packet: ServerToClient,
        delivery_type: DeliveryType,
    ) -> Self {
        Self { destination, packet, delivery_type }
    }
}

#[allow(unused)]
#[derive(Debug, Copy, Clone)]
pub enum DeliveryType {
    ReliableOrdered,
    ReliableSequenced,
    ReliableUnordered,
    Unreliable,
    UnreliableSequenced,
}

fn network_send(
    net_tx: Res<NetTx>,
    mut outgoing_packets: EventReader<OutgoingPacket>,
    player_addrs: Query<&PlayerNetworkAddr>,
) {
    let net_tx = &net_tx.0;

    for outgoing in outgoing_packets.iter() {
        let data = bincode::serialize(&outgoing.packet).unwrap();

        match &outgoing.destination {
            PacketDestination::Single(addr) => {
                let packet = make_packet(outgoing.delivery_type, data, *addr);

                if let Err(e) = net_tx.send(packet) {
                    println!("Failed to send packet: {:?}", e);
                }
            },
            PacketDestination::BroadcastToAll => {
                player_addrs.iter().for_each(|PlayerNetworkAddr(addr)| {
                    // TODO(bschwind) - Ideally we wouldn't clone this Vec here, but laminar
                    // packets take a Vec<u8> instead of a slice.
                    let packet = make_packet(outgoing.delivery_type, data.clone(), *addr);

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
                        let packet = make_packet(outgoing.delivery_type, data.clone(), *addr);

                        if let Err(e) = net_tx.send(packet) {
                            println!("Failed to send packet: {:?}", e);
                        }
                    });
            },
            PacketDestination::BroadcastToSet(addrs) => {
                addrs.iter().for_each(|addr| {
                    let packet = make_packet(outgoing.delivery_type, data.clone(), *addr);

                    if let Err(e) = net_tx.send(packet) {
                        println!("Failed to send packet: {:?}", e);
                    }
                });
            },
        }
    }
}

fn make_packet(delivery_type: DeliveryType, data: Vec<u8>, addr: SocketAddr) -> laminar::Packet {
    match delivery_type {
        DeliveryType::ReliableOrdered => Packet::reliable_ordered(addr, data, None),
        DeliveryType::ReliableSequenced => Packet::reliable_sequenced(addr, data, None),
        DeliveryType::ReliableUnordered => Packet::reliable_unordered(addr, data),
        DeliveryType::Unreliable => Packet::unreliable(addr, data),
        DeliveryType::UnreliableSequenced => Packet::unreliable_sequenced(addr, data, None),
    }
}
