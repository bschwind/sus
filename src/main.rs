use game::{
    network::{ClientToServer, ConnectPacket, PlayerInputPacket, ServerToClient},
    PlayerInput,
};
use laminar::{Config as NetworkConfig, Packet, Socket, SocketEvent};
use std::time::{Duration, Instant};
use wgpu::{BackendBit, Instance};
use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

const TARGET_FPS: usize = 60;
const FRAME_DT: Duration = Duration::from_micros((1000000.0 / TARGET_FPS as f64) as u64);
const SERVER_ADDR: &str = "127.0.0.1:7600";

const CORNFLOWER_BLUE: wgpu::Color =
    wgpu::Color { r: 100.0 / 255.0, g: 149.0 / 255.0, b: 237.0 / 255.0, a: 1.0 };

async fn run() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().with_title("sus").build(&event_loop).unwrap();

    let size = window.inner_size();

    // All the apis that wgpu offers first tier of support for (Vulkan + Metal + DX12 + Browser WebGPU).
    let instance = Instance::new(BackendBit::PRIMARY);
    let surface = unsafe { instance.create_surface(&window) };
    let swapchain_format = wgpu::TextureFormat::Bgra8Unorm;

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            // Prefer low power when on battery, high performance when on mains.
            power_preference: wgpu::PowerPreference::default(),
            // Request an adapter which can render to our surface
            compatible_surface: Some(&surface),
        })
        .await
        .expect("Failed to find an appropiate adapter");

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
                shader_validation: true,
            },
            None,
        )
        .await
        .expect("Failed to create device");

    let mut swapchain_desc = wgpu::SwapChainDescriptor {
        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        format: swapchain_format,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Mailbox,
    };

    let mut swap_chain = device.create_swap_chain(&surface, &swapchain_desc);
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
                // Recreate the swap chain with the new size
                println!("Resizing to {}x{}", new_size.width, new_size.height);
                swapchain_desc.width = new_size.width;
                swapchain_desc.height = new_size.height;
                swap_chain = device.create_swap_chain(&surface, &swapchain_desc);

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
                let frame = swap_chain
                    .get_current_frame()
                    .expect("Failed to acquire next swap chain texture")
                    .output;
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                {
                    let _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                            attachment: &frame.view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(CORNFLOWER_BLUE),
                                store: true,
                            },
                        }],
                        depth_stencil_attachment: None,
                    });
                }

                queue.submit(Some(encoder.finish()));
            },
            _ => (),
        }
    });
}

fn main() {
    futures::executor::block_on(run());
}
