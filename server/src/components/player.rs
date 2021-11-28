use std::{collections::HashMap, net::SocketAddr};
use sus_common::{
    components::player::{
        LastInputCounter, PlayerId, PlayerName, PlayerNetworkAddr, UnprocessedInputs,
    },
    simple_game::bevy::{bevy_ecs, Bundle, Entity, Transform},
};

#[derive(Debug, Bundle)]
pub struct ServerPlayerBundle {
    pub id: PlayerId,
    pub name: PlayerName,
    pub network_addr: PlayerNetworkAddr,
    pub unprocessed_inputs: UnprocessedInputs,
    pub last_input_counter: LastInputCounter,
    pub transform: Transform,
}

pub struct AddrToPlayer(pub HashMap<SocketAddr, u16>);
pub struct PlayerToEntity(pub HashMap<u16, Entity>);
