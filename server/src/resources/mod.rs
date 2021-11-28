use std::{collections::HashMap, net::SocketAddr};
use sus_common::simple_game::bevy::Entity;

pub struct AddrToPlayer(pub HashMap<SocketAddr, u16>);
pub struct PlayerToEntity(pub HashMap<u16, Entity>);
