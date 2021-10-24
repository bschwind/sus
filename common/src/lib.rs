use crate::network::PlayerInputPacket;

pub mod network;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum GameState {
    Lobby,
    IntroScreen,
    Main,
    End,
}

pub enum PlayerType {
    Crew,
    Impostor,
}

pub enum PlayerState {
    Alive,
    Dead,
}

pub struct Player {
    pub id: u16,
    pub name: String,
    pub pos: (i32, i32),
    pub player_type: PlayerType,
    pub state: PlayerState,
}

impl Player {
    pub fn new(name: &str, id: u16) -> Self {
        Self {
            id,
            name: name.to_string(),
            pos: (0, 0),
            player_type: PlayerType::Crew,
            state: PlayerState::Alive,
        }
    }
}

#[derive(Debug)]
pub struct PlayerInput {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
}

impl Default for PlayerInput {
    fn default() -> Self {
        Self { up: false, down: false, left: false, right: false }
    }
}

impl From<&PlayerInput> for PlayerInputPacket {
    fn from(player_input: &PlayerInput) -> Self {
        Self {
            x: match (player_input.left, player_input.right) {
                (true, true) => 0,
                (true, false) => i16::MIN,
                (false, true) => i16::MAX,
                (false, false) => 0,
            },
            y: match (player_input.up, player_input.down) {
                (true, true) => 0,
                (true, false) => i16::MAX,
                (false, true) => i16::MIN,
                (false, false) => 0,
            },
        }
    }
}
