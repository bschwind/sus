use game::{
    network::{ClientToServer, FullGameStatePacket, NewPlayerPacket, ServerToClient},
    Game, Player,
};
use laminar::{Config as NetworkConfig, ErrorKind, Packet, Socket, SocketEvent};
use std::{collections::HashMap, net::SocketAddr, time::Duration};

const BIND_ADDR: &str = "0.0.0.0:7600";
const MAX_PLAYERS: usize = 16;

fn main() -> Result<(), ErrorKind> {
    let mut game = Game::new();

    // TODO(bschwind) - Remove this once we start having a steady flow of packets.
    let net_config = NetworkConfig {
        idle_connection_timeout: Duration::from_secs(5),
        heartbeat_interval: Some(Duration::from_secs(4)),
        ..NetworkConfig::default()
    };
    let mut socket = Socket::bind_with_config(BIND_ADDR, net_config)?;
    let (sender, receiver) = (socket.get_packet_sender(), socket.get_event_receiver());
    let mut players: HashMap<SocketAddr, Player> = HashMap::new();
    let _thread = std::thread::spawn(move || socket.start_polling());

    // Game state
    let mut id_counter: u16 = 0;

    loop {
        if let Ok(event) = receiver.recv() {
            match event {
                SocketEvent::Packet(packet) => {
                    let msg = packet.payload();

                    if let Ok(decoded) = bincode::deserialize::<ClientToServer>(msg) {
                        match decoded {
                            ClientToServer::Connect(connect_packet) => {
                                println!(
                                    "{} (ip = {}) connected with game version {}",
                                    connect_packet.name,
                                    packet.addr(),
                                    connect_packet.version
                                );

                                if players.len() >= MAX_PLAYERS {
                                    println!("Max players exceeded, not accepting");
                                } else {
                                    let new_player_id = id_counter;
                                    let new_player_pos = (0, 0);
                                    players.insert(
                                        packet.addr(),
                                        Player::new(&connect_packet.name, new_player_id),
                                    );
                                    id_counter += 1;

                                    let reply = ServerToClient::ConnectAck;
                                    sender
                                        .send(Packet::reliable_ordered(
                                            packet.addr(),
                                            bincode::serialize(&reply).unwrap(),
                                            None,
                                        ))
                                        .expect("Failed to send ConnectAck");
                                    // Send all existing state to new client
                                    let players_vec = players
                                        .values()
                                        .map(|p| NewPlayerPacket::new(p.name.clone(), p.id, p.pos))
                                        .collect();

                                    let full_state_packet = ServerToClient::FullGameState(
                                        FullGameStatePacket::new(players_vec),
                                    );
                                    sender
                                        .send(Packet::reliable_ordered(
                                            packet.addr(),
                                            bincode::serialize(&full_state_packet).unwrap(),
                                            None,
                                        ))
                                        .expect("Failed to send ConnectAck");

                                    // Tell all other players this one has connected
                                    let new_player_packet =
                                        ServerToClient::NewPlayer(NewPlayerPacket::new(
                                            connect_packet.name,
                                            new_player_id,
                                            new_player_pos,
                                        ));
                                    for player_addr in players.keys() {
                                        if *player_addr != packet.addr() {
                                            sender
                                                .send(Packet::reliable_ordered(
                                                    *player_addr,
                                                    bincode::serialize(&new_player_packet).unwrap(),
                                                    None,
                                                ))
                                                .expect("Failed to send ConnectAck");
                                        }
                                    }
                                }
                            },
                            ClientToServer::PlayerInput(input) => {
                                if let Some(player) = players.get(&packet.addr()) {
                                    // println!("Player {} (id={}) sent input: {:?}", player.name, player.id, input);
                                }
                            },
                        }
                    } else {
                        println!("Received an invalid packet");
                    }
                },
                SocketEvent::Timeout(addr) => {
                    println!("Client timed out: {}", addr);
                },
                SocketEvent::Connect(addr) => {
                    println!("Client connected: {}", addr);
                },
                SocketEvent::Disconnect(addr) => {
                    if let Some(player) = players.remove(&addr) {
                        println!("Player {} disconnected ({})", player.name, addr);
                    } else {
                        println!("Unknown player disconnected: {}", addr);
                    }
                },
            }
        }
    }
}
