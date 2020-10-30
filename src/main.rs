use bytemuck::{Pod, Zeroable};
use game::{
    network::{ClientToServer, ConnectPacket, PlayerInputPacket, ServerToClient},
    PlayerInput,
};
use laminar::{Config as NetworkConfig, Packet, Socket, SocketEvent};
use std::time::{Duration, Instant};
use wgpu::{util::DeviceExt, BackendBit, Instance};
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

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct VertexData {
    pos: [f32; 2],
    uv: [f32; 2],
}

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

    // Begin vertex buffer and pipeline creation
    let vertex_data = vec![
        VertexData { pos: [-1.0, -1.0], uv: [0.0, 1.0] },
        VertexData { pos: [-1.0, 1.0], uv: [0.0, 0.0] },
        VertexData { pos: [1.0, 1.0], uv: [1.0, 0.0] },
        VertexData { pos: [1.0, -1.0], uv: [1.0, 1.0] },
    ];

    let index_data = vec![0u16, 1, 3, 2];

    let vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertex_data),
        usage: wgpu::BufferUsage::VERTEX,
    });

    let index_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Index Buffer"),
        contents: bytemuck::cast_slice(&index_data),
        usage: wgpu::BufferUsage::INDEX,
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            // wgpu::BindGroupLayoutEntry {
            //     binding: 0,
            //     visibility: wgpu::ShaderStage::VERTEX,
            //     ty: wgpu::BindingType::UniformBuffer {
            //         dynamic: false,
            //         min_binding_size: wgpu::BufferSize::new(64), // Size of a 4x4 f32 matrix
            //     },
            //     count: None,
            // },
            // wgpu::BindGroupLayoutEntry {
            //     binding: 1,
            //     visibility: wgpu::ShaderStage::FRAGMENT,
            //     ty: wgpu::BindingType::SampledTexture {
            //         multisampled: false,
            //         component_type: wgpu::TextureComponentType::Float,
            //         dimension: wgpu::TextureViewDimension::D2,
            //     },
            //     count: None,
            // },
            // wgpu::BindGroupLayoutEntry {
            //     binding: 2,
            //     visibility: wgpu::ShaderStage::FRAGMENT,
            //     ty: wgpu::BindingType::Sampler { comparison: false },
            //     count: None,
            // },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &bind_group_layout,
        entries: &[
            // wgpu::BindGroupEntry {
            //     binding: 0,
            //     resource: uniform_buf.as_entire_binding(),
            // },
            // wgpu::BindGroupEntry {
            //     binding: 1,
            //     resource: wgpu::BindingResource::TextureView(&texture_view),
            // },
            // wgpu::BindGroupEntry {
            //     binding: 2,
            //     resource: wgpu::BindingResource::Sampler(&sampler),
            // },
        ],
        label: None,
    });

    let vertex_state = wgpu::VertexStateDescriptor {
        index_format: wgpu::IndexFormat::Uint16,
        vertex_buffers: &[wgpu::VertexBufferDescriptor {
            stride: (std::mem::size_of::<VertexData>()) as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[
                // Pos (vec2)
                wgpu::VertexAttributeDescriptor {
                    format: wgpu::VertexFormat::Float2,
                    offset: 0,
                    shader_location: 0,
                },
                // UV (vec2)
                wgpu::VertexAttributeDescriptor {
                    format: wgpu::VertexFormat::Float2,
                    offset: 2 * 4,
                    shader_location: 1,
                },
            ],
        }],
    };

    let vs_module = device.create_shader_module(wgpu::include_spirv!("../shaders/test.vert.spv"));
    let fs_module = device.create_shader_module(wgpu::include_spirv!("../shaders/test.frag.spv"));

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        vertex_stage: wgpu::ProgrammableStageDescriptor { module: &vs_module, entry_point: "main" },
        fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
            module: &fs_module,
            entry_point: "main",
        }),
        rasterization_state: Some(wgpu::RasterizationStateDescriptor {
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: wgpu::CullMode::Front,
            ..Default::default()
        }),
        primitive_topology: wgpu::PrimitiveTopology::TriangleStrip,
        color_states: &[wgpu::ColorStateDescriptor {
            format: swapchain_desc.format,
            color_blend: wgpu::BlendDescriptor::REPLACE,
            alpha_blend: wgpu::BlendDescriptor::REPLACE,
            write_mask: wgpu::ColorWrite::ALL,
        }],
        depth_stencil_state: None,
        vertex_state,
        sample_count: 1,
        sample_mask: !0,
        alpha_to_coverage_enabled: false,
    });

    // End pipeline creation

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
                    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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

                    rpass.set_pipeline(&pipeline);
                    rpass.set_bind_group(0, &bind_group, &[]);
                    rpass.set_index_buffer(index_buf.slice(..));
                    rpass.set_vertex_buffer(0, vertex_buf.slice(..));
                    rpass.draw_indexed(0..4 as u32, 0, 0..1);
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
