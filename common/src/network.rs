use serde::{Deserialize, Serialize};

pub const GAME_VERSION: u32 = 0;
pub const INPUT_STREAM: u8 = 0;
pub const CHAT_STREAM: u8 = 1;
pub const VOICE_STREAM: u8 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerToClient {
    ConnectAck,
    NewPlayer(NewPlayerPacket),
    FullGameState(FullGameStatePacket),
    PlayerMovement,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewPlayerPacket {
    pub name: String,
    pub id: u16,
    pub pos: (i32, i32),
}

impl NewPlayerPacket {
    pub fn new(name: String, id: u16, pos: (i32, i32)) -> Self {
        Self { name, id, pos }
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

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientToServer {
    Connect(ConnectPacket),
    PlayerInput(PlayerInputPacket),
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

#[derive(Debug, Serialize, Deserialize)]
pub struct PlayerInputPacket {
    pub x: i16,
    pub y: i16,
}

impl PlayerInputPacket {
    pub fn new(x: i16, y: i16) -> Self {
        Self { x, y }
    }
}
