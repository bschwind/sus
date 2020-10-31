use crate::graphics::{GraphicsDevice, TexturedQuad};
use game::{
    network::{ClientToServer, ConnectPacket, PlayerInputPacket, ServerToClient},
    PlayerInput,
};
use laminar::{Config as NetworkConfig, Packet, Socket, SocketEvent};
use std::time::{Duration, Instant};
use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

mod graphics;

const TARGET_FPS: usize = 60;
const FRAME_DT: Duration = Duration::from_micros((1000000.0 / TARGET_FPS as f64) as u64);
const SERVER_ADDR: &str = "127.0.0.1:7600";

async fn run() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().with_title("sus").build(&event_loop).unwrap();

    let mut graphics_device = GraphicsDevice::new(&window).await;
    let textured_quad = TexturedQuad::new(&graphics_device);

    let mut last_frame_time = Instant::now();

    // Connect to the server
    let net_config = NetworkConfig {
        idle_connection_timeout: Duration::from_secs(5),
        heartbeat_interval: Some(Duration::from_secs(4)),
        ..NetworkConfig::default()
    };
    let mut socket =
        Socket::bind_with_config("127.0.0.1:0", net_config).expect("Could not connect to server");
    let server_addr = SERVER_ADDR.parse().unwrap();
    let connect_packet = ClientToServer::Connect(ConnectPacket::new("Brian"));
    socket
        .send(Packet::reliable_ordered(
            server_addr,
            bincode::serialize(&connect_packet).unwrap(),
            None,
        ))
        .expect("Could not send packet to server");

    // Game state
    let mut player_input = PlayerInput::new();

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::MainEventsCleared => {
                if last_frame_time.elapsed() >= FRAME_DT {
                    let now = Instant::now();
                    last_frame_time = now;

                    // Game logic here
                    // Consider using this: https://github.com/tuzz/game-loop
                    let input_packet: PlayerInputPacket = (&player_input).into();
                    let msg = ClientToServer::PlayerInput(input_packet);
                    socket
                        .send(Packet::unreliable_sequenced(
                            server_addr,
                            bincode::serialize(&msg).unwrap(),
                            Some(game::network::INPUT_STREAM),
                        ))
                        .expect("Could not send packet to server");

                    socket.manual_poll(now);
                    match socket.recv() {
                        Some(SocketEvent::Packet(packet)) => {
                            let msg = packet.payload();

                            if packet.addr() == server_addr {
                                if let Ok(decoded) = bincode::deserialize::<ServerToClient>(msg) {
                                    match decoded {
                                        ServerToClient::ConnectAck => {
                                            println!("Server accepted us, yay!");
                                        },
                                        ServerToClient::NewPlayer(new_player_packet) => {
                                            println!("New player: {:?}", new_player_packet);
                                        },
                                        ServerToClient::FullGameState(full_game_state) => {
                                            println!("Full game state: {:?}", full_game_state);
                                        },
                                        ServerToClient::PlayerMovement => {
                                            println!("Player moved!");
                                        },
                                    }
                                }
                            } else {
                                println!("Unknown sender.");
                            }
                        },
                        Some(SocketEvent::Timeout(addr)) => {
                            println!("Server timed out: {}", addr);
                        },
                        Some(SocketEvent::Connect(addr)) => {
                            println!("Server connected: {}", addr);
                        },
                        Some(SocketEvent::Disconnect(addr)) => {
                            println!("Server disconnected: {}", addr);
                        },
                        None => {},
                    }

                    window.request_redraw();
                }
            },
            Event::WindowEvent { event: WindowEvent::Resized(new_size), .. } => {
                println!("Resizing to {}x{}", new_size.width, new_size.height);
                graphics_device.resize(new_size);

                window.request_redraw();
            },
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                },
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(virtual_code),
                            state: ElementState::Pressed,
                            ..
                        },
                    ..
                } => {
                    if let VirtualKeyCode::Escape = virtual_code {
                        *control_flow = ControlFlow::Exit;
                    }

                    match virtual_code {
                        VirtualKeyCode::Escape => *control_flow = ControlFlow::Exit,
                        VirtualKeyCode::W => player_input.up = true,
                        VirtualKeyCode::A => player_input.left = true,
                        VirtualKeyCode::S => player_input.down = true,
                        VirtualKeyCode::D => player_input.right = true,
                        _ => {},
                    }
                },
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(virtual_code),
                            state: ElementState::Released,
                            ..
                        },
                    ..
                } => {
                    if let VirtualKeyCode::Escape = virtual_code {
                        *control_flow = ControlFlow::Exit;
                    }

                    match virtual_code {
                        VirtualKeyCode::W => player_input.up = false,
                        VirtualKeyCode::A => player_input.left = false,
                        VirtualKeyCode::S => player_input.down = false,
                        VirtualKeyCode::D => player_input.right = false,
                        _ => {},
                    }
                },
                _ => (),
            },
            Event::RedrawRequested(_window_id) => {
                // Draw the scene
                let mut frame_encoder = graphics_device.begin_frame();
                textured_quad.render(&mut frame_encoder);
                frame_encoder.finish();
            },
            _ => (),
        }
    });
}

fn main() {
    futures::executor::block_on(run());
}
