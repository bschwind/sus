use crate::systems::{LobbyPlugin, NetworkPlugin};
use simple_game::bevy::{App, AppBuilder, CorePlugin, HeadlessBevyGame};
use sus_common::GameState;

mod systems;

pub const MAX_PLAYERS: usize = 16;

pub struct SusServer;

impl HeadlessBevyGame for SusServer {
    fn init_systems() -> AppBuilder {
        let mut ecs_world_builder = App::build();

        ecs_world_builder
            .add_plugin(CorePlugin)
            .add_plugin(NetworkPlugin)
            .add_plugin(LobbyPlugin::new(Self::desired_fps()))
            .add_state(GameState::Lobby);

        ecs_world_builder
    }
}

fn main() {
    simple_game::bevy::run_headless_bevy_game::<SusServer>();
}
