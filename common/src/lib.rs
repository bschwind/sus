use crate::network::PlayerInputPacket;
use simple_game::bevy::{bevy_ecs, Res, Resource, State, States};

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

// Similar to the `in_state()` function in bevy, but with a Clone bound
// on the returned closure. Named `state_active` instead of `in_state`
// to avoid confusion with the official bevy function.
// https://github.com/bevyengine/bevy/issues/8059#issuecomment-1466389318
pub fn state_active<S: States>(state: S) -> impl Clone + FnMut(Res<State<S>>) -> bool {
    move |current_state: Res<State<S>>| current_state.0 == state
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
