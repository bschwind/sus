use std::{collections::HashMap, net::SocketAddr};
use sus_common::simple_game::bevy::{bevy_ecs, Resource};

#[derive(Debug, Resource)]
pub struct AddrToPlayer(pub HashMap<SocketAddr, u16>);
