use crate::systems::labels;
use simple_game::bevy::{AppBuilder, FixedTimestep, IntoSystem, Plugin, SystemSet};
use sus_common::GameState;

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
                SystemSet::on_update(GameState::Lobby)
                    .with_run_criteria(
                        FixedTimestep::step(self.fixed_timestep).with_label("lobby_timestep"),
                    )
                    .label(labels::Lobby)
                    .after(labels::Network)
                    .with_system(update_lobby.system()),
            )
            .add_system_set(SystemSet::on_exit(GameState::Lobby).with_system(close_lobby.system()));
    }
}

fn setup() {}

fn setup_lobby() {
    println!("Lobby started");
}

fn update_lobby() {}

fn close_lobby() {}
