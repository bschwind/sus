use simple_game::bevy::{bevy_ecs, Entity, Resource};
use std::collections::HashMap;

#[derive(Debug, Resource)]
pub struct PlayerToEntity(pub HashMap<u16, Entity>);

pub mod network {
    use super::*;
    use crossbeam_channel::{Receiver, Sender};
    use laminar::SocketEvent;
    use std::thread::JoinHandle;

    #[derive(Debug, Resource)]
    pub struct NetworkThread(pub JoinHandle<()>);

    #[derive(Debug, Resource)]
    pub struct NetTx(pub Sender<laminar::Packet>);

    #[derive(Debug, Resource)]
    pub struct NetRx(pub Receiver<SocketEvent>);
}
