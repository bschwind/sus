use sus_common::{
    components::player::{PlayerId, PlayerName},
    simple_game::bevy::{bevy_ecs, Bundle, Component, Transform},
};

// Marker component to designate which player is "us".
#[derive(Component)]
pub struct MyPlayer;

#[derive(Debug, Bundle)]
pub struct ClientPlayerBundle {
    pub id: PlayerId,
    pub name: PlayerName,
    pub transform: Transform,
}
