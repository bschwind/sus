use game::{
    network::{ClientToServer, ServerToClient},
    Game,
};
use laminar::{ErrorKind, Packet, Socket, SocketEvent};

const BIND_ADDR: &str = "0.0.0.0:7600";

fn main() -> Result<(), ErrorKind> {
    let mut game = Game::new();

    let mut socket = Socket::bind(BIND_ADDR)?;
    let (sender, receiver) = (socket.get_packet_sender(), socket.get_event_receiver());
    let _thread = std::thread::spawn(move || socket.start_polling());

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

                                let reply = ServerToClient::ConnectAck;
                                sender
                                    .send(Packet::reliable_ordered(
                                        packet.addr(),
                                        bincode::serialize(&reply).unwrap(),
                                        None,
                                    ))
                                    .expect("This should send");
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
                    println!("Client disconnected: {}", addr);
                },
            }
        }
    }
}
