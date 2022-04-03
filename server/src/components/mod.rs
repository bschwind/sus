use sus_common::{
    components::player::{
        LastInputCounter, PlayerId, PlayerName, PlayerNetworkAddr, PositionHistory,
        UnprocessedInputs,
    },
    simple_game::bevy::{bevy_ecs, Bundle, Transform},
};

#[derive(Debug, Bundle)]
pub struct ServerPlayerBundle {
    pub id: PlayerId,
    pub name: PlayerName,
    pub network_addr: PlayerNetworkAddr,
    pub unprocessed_inputs: UnprocessedInputs,
    pub position_history: PositionHistory,
    pub last_input_counter: LastInputCounter,
    pub transform: Transform,
}
