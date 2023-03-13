use crate::network::PlayerInputPacket;
use simple_game::bevy::{bevy_ecs, Resource, States};

pub mod components;
pub mod math;
pub mod network;
pub mod resources;

pub use laminar;
pub use simple_game;

#[derive(States, Default, Debug, Clone, Eq, PartialEq, Hash)]
pub enum GameState {
    #[default]
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

#[derive(Debug, Default, Resource)]
pub struct PlayerInput {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
}

impl PlayerInput {
    pub fn x(&self) -> i16 {
        match (self.left, self.right) {
            (true, true) => 0,
            (true, false) => i16::MIN,
            (false, true) => i16::MAX,
            (false, false) => 0,
        }
    }

    pub fn y(&self) -> i16 {
        match (self.up, self.down) {
            (true, true) => 0,
            (true, false) => i16::MAX,
            (false, true) => i16::MIN,
            (false, false) => 0,
        }
    }
}

impl PlayerInput {
    pub fn to_player_input_packet(&self, counter: u16) -> PlayerInputPacket {
        PlayerInputPacket { counter, x: self.x(), y: self.y() }
    }
}
