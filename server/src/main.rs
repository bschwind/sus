use crate::systems::{LobbyPlugin, NetworkPlugin};
use std::time::Duration;
use sus_common::{
    simple_game::bevy::{
        App, AppBuilder, CorePlugin, HeadlessBevyGame, ScheduleRunnerPlugin, ScheduleRunnerSettings,
    },
    GameState,
};

mod components;
mod systems;

pub const TICK_RATE_HZ: usize = 10;
pub const MAX_PLAYERS: usize = 16;

pub struct SusServer;

impl HeadlessBevyGame for SusServer {
    fn desired_fps() -> usize {
        60
    }

    fn init_systems() -> AppBuilder {
        let mut ecs_world_builder = App::build();

        ecs_world_builder
            .add_plugin(CorePlugin)
            .insert_resource(ScheduleRunnerSettings::run_loop(Duration::from_secs_f64(
                1.0 / TICK_RATE_HZ as f64,
            )))
            .add_plugin(ScheduleRunnerPlugin::default())
            .add_plugin(NetworkPlugin)
            .add_plugin(LobbyPlugin::new(Self::desired_fps()))
            .add_state(GameState::Lobby);

        ecs_world_builder
    }
}

fn main() {
    sus_common::simple_game::bevy::run_headless_bevy_game::<SusServer>();
}
