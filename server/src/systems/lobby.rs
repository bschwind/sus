use crate::systems::{fixed_timestep_with_state, labels};
use simple_game::bevy::{
    schedule::{ShouldRun, State},
    AppBuilder, Commands, FixedTimestep, In, IntoChainSystem, IntoSystem, Plugin, Query, Res,
    ResMut, SystemSet,
};
use std::time::{Duration, Instant};
use sus_common::GameState;

#[allow(unused)]
pub struct LobbyPlugin {
    fixed_timestep: f64,
}

impl LobbyPlugin {
    pub fn new(update_fps: usize) -> Self {
        Self { fixed_timestep: 1.0 / update_fps as f64 }
    }
}

impl Plugin for LobbyPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup.system())
            .add_system_set(SystemSet::on_enter(GameState::Lobby).with_system(setup_lobby.system()))
            .add_system_set(
                SystemSet::new()
                    .with_run_criteria(fixed_timestep_with_state!(
                        self.fixed_timestep,
                        GameState::Lobby,
                    ))
                    .label(labels::Lobby)
                    .after(labels::Network)
                    .with_system(update_lobby.system()),
            )
            .add_system_set(SystemSet::on_exit(GameState::Lobby).with_system(close_lobby.system()));
    }
}

struct LobbyTimer(Instant);
const LOBBY_COUNTDOWN_TIME: Duration = Duration::from_secs(3);

fn setup(mut commands: Commands) {
    commands.spawn().insert(LobbyTimer(Instant::now()));
}

fn setup_lobby() {
    println!("Lobby started");
}

fn update_lobby(mut game_state: ResMut<State<GameState>>, lobby_timer: Query<&LobbyTimer>) {
    let lobby_timer = lobby_timer.single().unwrap().0;

    if lobby_timer.elapsed() > LOBBY_COUNTDOWN_TIME {
        println!("Leaving lobby!");
        if game_state.current() == &GameState::Lobby {
            game_state.set(GameState::IntroScreen).unwrap();
        }
    }
}

fn close_lobby() {}
