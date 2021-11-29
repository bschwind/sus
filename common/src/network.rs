use laminar::Packet;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

pub const GAME_VERSION: u32 = 0;
pub const INPUT_STREAM: u8 = 0;
pub const GAME_STATE_STREAM: u8 = 1;
pub const CHAT_STREAM: u8 = 2;
pub const VOICE_STREAM: u8 = 3;

#[allow(unused)]
#[derive(Debug, Copy, Clone)]
pub enum DeliveryType {
    ReliableOrdered,
    ReliableSequenced,
    ReliableUnordered,
    Unreliable,
    UnreliableSequenced,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerToClient {
    ConnectAck(ConnectAckPacket),
    NewPlayer(NewPlayerPacket),
    FullGameState(FullGameStatePacket),
    LobbyTick(LobbyTickPacket),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientToServer {
    Connect(ConnectPacket),
    PlayerInput(PlayerInputPacket),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectAckPacket {
    pub id: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewPlayerPacket {
    pub name: String,
    pub id: u16,
}

impl NewPlayerPacket {
    pub fn new(name: String, id: u16) -> Self {
        Self { name, id }
    }
}

// Only needed for players joining the lobby, because currently
// players can't join a session in progress.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullGameStatePacket {
    pub players: Vec<NewPlayerPacket>,
}

impl FullGameStatePacket {
    pub fn new(players: Vec<NewPlayerPacket>) -> Self {
        Self { players }
    }
}

// Sent from the server to every player after every lobby network tick.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LobbyTickPacket {
    pub last_input_counter: u16,
    pub players: Vec<LobbyPlayer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LobbyPlayer {
    pub id: u16,
    pub pos: (f32, f32),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectPacket {
    pub version: u32,
    pub name: String,
}

impl ConnectPacket {
    pub fn new(name: &str) -> Self {
        Self { version: GAME_VERSION, name: name.to_string() }
    }
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct PlayerInputPacket {
    pub counter: u16,
    pub x: i16,
    pub y: i16,
}

impl PlayerInputPacket {
    pub fn new(counter: u16, x: i16, y: i16) -> Self {
        Self { counter, x, y }
    }
}

pub fn make_packet(
    delivery_type: DeliveryType,
    data: Vec<u8>,
    addr: SocketAddr,
    stream: Option<u8>,
) -> laminar::Packet {
    match delivery_type {
        DeliveryType::ReliableOrdered => Packet::reliable_ordered(addr, data, stream),
        DeliveryType::ReliableSequenced => Packet::reliable_sequenced(addr, data, stream),
        DeliveryType::ReliableUnordered => Packet::reliable_unordered(addr, data),
        DeliveryType::Unreliable => Packet::unreliable(addr, data),
        DeliveryType::UnreliableSequenced => Packet::unreliable_sequenced(addr, data, stream),
    }
}

// Helper trait for handling u16 wraparound.
pub trait SequenceCmp {
    fn sequentially_greater_than(&self, other: u16) -> bool;
    fn sequentially_greater_than_or_equal_to(&self, other: u16) -> bool;
    fn sequentially_less_than(&self, other: u16) -> bool;
}

impl SequenceCmp for u16 {
    fn sequentially_greater_than(&self, other: u16) -> bool {
        let max_half = (u16::max_value() / 2) - 1;
        ((*self > other) && *self - other <= max_half)
            || ((other > *self) && other - *self > max_half)
    }

    fn sequentially_greater_than_or_equal_to(&self, other: u16) -> bool {
        let max_half = (u16::max_value() / 2) - 1;
        (*self == other)
            || ((*self > other) && *self - other <= max_half)
            || ((other > *self) && other - *self > max_half)
    }

    fn sequentially_less_than(&self, other: u16) -> bool {
        let max_half = (u16::max_value() / 2) - 1;
        ((*self < other) && other - *self <= max_half)
            || ((*self > other) && *self - other > max_half)
    }
}
