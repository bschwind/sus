use crate::systems::{LobbyPlugin, ServerNetworkPlugin};
use std::time::Duration;
use sus_common::{
    simple_game::bevy::{
        App, FixedTime, HeadlessBevyGame, ScheduleRunnerPlugin, ScheduleRunnerSettings,
        SimpleGamePlugin,
    },
    GameState,
};

mod components;
mod events;
mod resources;
mod systems;

pub const TICK_RATE_HZ: usize = 10;
pub const MAX_PLAYERS: usize = 16;

pub struct SusServer;

impl HeadlessBevyGame for SusServer {
    fn desired_fps() -> usize {
        60
    }

    fn init_systems() -> App {
        let mut ecs_world_builder = App::new();

        ecs_world_builder
            .add_plugin(SimpleGamePlugin)
            .insert_resource(FixedTime::new_from_secs(1.0 / Self::desired_fps() as f32))
            .add_state::<GameState>()
            .insert_resource(ScheduleRunnerSettings::run_loop(Duration::from_secs_f64(
                1.0 / TICK_RATE_HZ as f64,
            )))
            .add_plugin(ScheduleRunnerPlugin::default())
            .add_plugin(ServerNetworkPlugin)
            .add_plugin(LobbyPlugin::new(Self::desired_fps()));

        ecs_world_builder
    }
}

fn main() {
    sus_common::simple_game::bevy::run_headless_bevy_game::<SusServer>();
}
