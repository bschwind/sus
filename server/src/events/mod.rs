use crate::systems::network::PacketDestination;
use std::net::SocketAddr;
use sus_common::network::{ConnectPacket, DeliveryType, PlayerInputPacket, ServerToClient};

pub struct NewPlayer {
    pub addr: SocketAddr,
    pub connect_packet: ConnectPacket,
}

#[derive(Debug)]
pub struct PlayerInput {
    pub id: u16,
    pub input: PlayerInputPacket,
}

pub struct OutgoingPacket {
    pub destination: PacketDestination,
    pub packet: ServerToClient,
    pub delivery_type: DeliveryType,
    pub stream_id: Option<u8>,
}

impl OutgoingPacket {
    pub fn new(
        destination: PacketDestination,
        packet: ServerToClient,
        delivery_type: DeliveryType,
        stream_id: Option<u8>,
    ) -> Self {
        Self { destination, packet, delivery_type, stream_id }
    }
}
