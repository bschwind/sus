use simple_game::bevy::{bevy_ecs, SystemLabel};

#[derive(Clone, Hash, Debug, PartialEq, Eq, SystemLabel)]
pub struct Network;

#[derive(Clone, Hash, Debug, PartialEq, Eq, SystemLabel)]
pub struct Lobby;

#[derive(Clone, Hash, Debug, PartialEq, Eq, SystemLabel)]
pub enum NetworkSystem {
    Receive,
    PlayerInput,
    SendPackets,
}
