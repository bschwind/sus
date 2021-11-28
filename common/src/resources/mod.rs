use simple_game::bevy::Entity;
use std::collections::HashMap;

#[derive(Debug)]
pub struct PlayerToEntity(pub HashMap<u16, Entity>);
