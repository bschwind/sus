use sus_common::simple_game::bevy::{bevy_ecs, Resource};

#[derive(Debug, Resource)]
pub struct InputCounter(pub u16);

#[derive(Debug, Resource)]
pub struct MyName(pub String);
