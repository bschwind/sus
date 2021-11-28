use sus_common::{
    components::player::{PlayerId, PlayerName},
    simple_game::bevy::{bevy_ecs, Bundle, Transform},
};

#[derive(Debug, Bundle)]
pub struct ClientPlayerBundle {
    pub id: PlayerId,
    pub name: PlayerName,
    pub transform: Transform,
}
