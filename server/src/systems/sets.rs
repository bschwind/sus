use sus_common::simple_game::bevy::{bevy_ecs, SystemSet};

#[derive(Clone, Hash, Debug, PartialEq, Eq, SystemSet)]
pub struct Network;

#[derive(Clone, Hash, Debug, PartialEq, Eq, SystemSet)]
pub struct Lobby;

#[derive(Clone, Hash, Debug, PartialEq, Eq, SystemSet)]
pub enum NetworkSystem {
    Receive,
    PlayerInput,
    SendPackets,
}
