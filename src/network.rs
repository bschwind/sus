use serde::{Deserialize, Serialize};

pub const GAME_VERSION: u32 = 0;

#[derive(Debug, Serialize, Deserialize)]
pub enum ServerToClient {
    ConnectAck,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientToServer {
    Connect(ConnectPacket),
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
