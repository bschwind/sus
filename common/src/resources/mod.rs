use simple_game::bevy::Entity;
use std::collections::HashMap;

#[derive(Debug)]
pub struct PlayerToEntity(pub HashMap<u16, Entity>);

pub mod network {
    use crossbeam_channel::{Receiver, Sender};
    use laminar::SocketEvent;
    use std::thread::JoinHandle;

    pub struct NetworkThread(pub JoinHandle<()>);
    pub struct NetTx(pub Sender<laminar::Packet>);
    pub struct NetRx(pub Receiver<SocketEvent>);
}
