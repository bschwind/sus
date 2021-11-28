use crate::PlayerInputPacket;
use std::{collections::VecDeque, net::SocketAddr};

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
