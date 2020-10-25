pub mod network;

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
    player_type: PlayerType,
    state: PlayerState,
}

pub struct Game {
    players: Vec<Player>,
    game_state: GameState,
}

impl Game {
    pub fn new() -> Self {
        Self { players: Vec::new(), game_state: GameState::Lobby }
    }
}
