use std::{collections::HashMap, net::SocketAddr};

pub struct AddrToPlayer(pub HashMap<SocketAddr, u16>);
