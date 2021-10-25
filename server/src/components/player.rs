use simple_game::bevy::{bevy_ecs, Bundle};
use std::net::SocketAddr;

#[derive(Debug)]
pub struct PlayerId(pub u16);

#[derive(Debug)]
pub struct PlayerNetworkAddr(pub SocketAddr);

#[derive(Debug)]
pub struct PlayerName(pub String);

#[derive(Debug, Bundle)]
pub struct PlayerBundle {
    pub id: PlayerId,
    pub name: PlayerName,
    pub network_addr: PlayerNetworkAddr,
    // transform
}
