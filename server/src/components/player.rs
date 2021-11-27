use simple_game::bevy::{bevy_ecs, Bundle, Entity, Transform};
use std::{
    collections::{HashMap, VecDeque},
    net::SocketAddr,
};
use sus_common::network::PlayerInputPacket;

#[derive(Debug)]
pub struct PlayerId(pub u16);

#[derive(Debug)]
pub struct PlayerNetworkAddr(pub SocketAddr);

#[derive(Debug)]
pub struct PlayerName(pub String);

#[derive(Debug)]
pub struct LastInputCounter(pub u16);

#[derive(Debug)]
pub struct UnprocessedInputs(pub VecDeque<PlayerInputPacket>);

#[derive(Debug, Bundle)]
pub struct PlayerBundle {
    pub id: PlayerId,
    pub name: PlayerName,
    pub network_addr: PlayerNetworkAddr,
    pub unprocessed_inputs: UnprocessedInputs,
    pub last_input_counter: LastInputCounter,
    pub transform: Transform,
}

pub struct AddrToPlayer(pub HashMap<SocketAddr, u16>);
pub struct PlayerToEntity(pub HashMap<u16, Entity>);
