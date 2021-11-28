use crate::resources::InputCounter;
use laminar::{Config as NetworkConfig, Packet, Socket, SocketEvent};
use std::{
    net::SocketAddr,
    time::{Duration, Instant},
};
use sus_common::{
    network::{
        ClientToServer, ConnectPacket, FullGameStatePacket, LobbyTickPacket, NewPlayerPacket,
        ServerToClient,
    },
    simple_game::{
        bevy::{
            App, AppBuilder, BevyGame, Commands, CorePlugin, EventReader, EventWriter,
            FixedTimestep, IntoSystem, Res, ResMut, SystemSet,
        },
        glam::vec3,
        graphics::{
            text::{AxisAlign, Color, DefaultFont, StyledText, TextAlignment, TextSystem},
            DebugDrawer, FullscreenQuad, GraphicsDevice,
        },
        wgpu,
        winit::event::{ElementState, KeyboardInput, VirtualKeyCode},
        WindowDimensions,
    },
    PlayerInput,
};

mod components;
mod events;
mod resources;

const SERVER_ADDR: &str = "127.0.0.1:7600";

struct SusGame {
    server_addr: SocketAddr,
    connected: bool,
}

impl BevyGame for SusGame {
    fn window_title() -> &'static str {
        "Simple Game"
    }

    fn window_dimensions() -> WindowDimensions {
        WindowDimensions::Windowed(1280, 720)
    }

    fn desired_fps() -> usize {
        60
    }

    fn init_systems() -> AppBuilder {
        let mut ecs_world_builder = App::build();

        ecs_world_builder
            .add_plugin(CorePlugin)
            .add_startup_system(init.system())
            .add_system(handle_input.system())
            .add_system_set(
                SystemSet::new()
                    .with_run_criteria(
                        FixedTimestep::step(1.0 / Self::desired_fps() as f64)
                            .with_label("game_timestep"),
                    )
                    .with_system(send_input_to_server.system())
                    .with_system(update_network.system())
                    .with_system(update_game.system()),
            )
            .add_system(render.system());

        ecs_world_builder
    }
}

fn init(mut commands: Commands, graphics_device: Res<GraphicsDevice>) {
    let game = SusGame { server_addr: SERVER_ADDR.parse().unwrap(), connected: false };

    let text_system: TextSystem = TextSystem::new(&graphics_device);
    let debug_drawer = DebugDrawer::new(&graphics_device);
    let fullscreen_quad = FullscreenQuad::new(&graphics_device);
    let player_input = PlayerInput::default();

    let socket = initialize_network(&game);

    commands.insert_resource(game);
    commands.insert_resource(text_system);
    commands.insert_resource(debug_drawer);
    commands.insert_resource(fullscreen_quad);
    commands.insert_resource(player_input);
    commands.insert_resource(InputCounter(0));
    commands.insert_resource(socket);
}

fn initialize_network(game: &SusGame) -> Socket {
    let net_config = NetworkConfig {
        idle_connection_timeout: Duration::from_secs(5),
        heartbeat_interval: Some(Duration::from_secs(4)),
        ..NetworkConfig::default()
    };

    let mut socket =
        Socket::bind_with_config("127.0.0.1:0", net_config).expect("Could not connect to server");

    let connect_packet = ClientToServer::Connect(ConnectPacket::new("Brian"));
    socket
        .send(Packet::reliable_ordered(
            game.server_addr,
            bincode::serialize(&connect_packet).unwrap(),
            None,
        ))
        .expect("Could not send packet to server");

    socket
}

fn handle_input(
    mut keyboard_input_events: EventReader<KeyboardInput>,
    mut player_input: ResMut<PlayerInput>,
) {
    for event in keyboard_input_events.iter() {
        if let KeyboardInput { virtual_keycode: Some(key_code), state, .. } = event {
            let pressed = *state == ElementState::Pressed;

            match key_code {
                VirtualKeyCode::W => player_input.up = pressed,
                VirtualKeyCode::A => player_input.left = pressed,
                VirtualKeyCode::S => player_input.down = pressed,
                VirtualKeyCode::D => player_input.right = pressed,
                _ => {},
            }
        }
    }
}

fn send_input_to_server(
    game: Res<SusGame>,
    player_input: Res<PlayerInput>,
    mut input_counter: ResMut<InputCounter>,
    mut socket: ResMut<Socket>,
) {
    // TODO(bschwind) - Store unacked inputs in a list
    let input_packet = player_input.to_player_input_packet(input_counter.0);
    input_counter.0 = input_counter.0.wrapping_add(1);

    let msg = ClientToServer::PlayerInput(input_packet);

    socket
        .send(Packet::unreliable_sequenced(
            game.server_addr,
            bincode::serialize(&msg).unwrap(),
            Some(sus_common::network::INPUT_STREAM),
        ))
        .expect("Could not send packet to server");
}

fn update_network(
    mut game: ResMut<SusGame>,
    mut socket: ResMut<Socket>,
    mut new_player_tx: EventWriter<NewPlayerPacket>,
    mut full_game_state_tx: EventWriter<FullGameStatePacket>,
    mut lobby_tick_tx: EventWriter<LobbyTickPacket>,
) {
    let now = Instant::now();
    socket.manual_poll(now);

    match socket.recv() {
        Some(SocketEvent::Packet(packet)) => {
            let msg = packet.payload();

            if packet.addr() == game.server_addr {
                if let Ok(decoded) = bincode::deserialize::<ServerToClient>(msg) {
                    match decoded {
                        ServerToClient::ConnectAck => {
                            println!("Server accepted us, yay!");
                            game.connected = true;
                        },
                        ServerToClient::NewPlayer(new_player_packet) => {
                            println!("New player: {:?}", new_player_packet);
                            new_player_tx.send(new_player_packet);
                        },
                        ServerToClient::FullGameState(full_game_state) => {
                            println!("Full game state: {:?}", full_game_state);
                            full_game_state_tx.send(full_game_state);
                        },
                        ServerToClient::LobbyTick(lobby_tick_packet) => {
                            println!("Lobby tick - {:?}", lobby_tick_packet);
                            lobby_tick_tx.send(lobby_tick_packet);
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
}

fn update_game(_player_input: Res<PlayerInput>) {
    // println!("player_input: {:?}", *player_input);
}

fn render(
    game: Res<SusGame>,
    mut graphics_device: ResMut<GraphicsDevice>,
    fullscreen_quad: ResMut<FullscreenQuad>,
    mut text_system: ResMut<TextSystem>,
    mut debug_drawer: ResMut<DebugDrawer>,
) {
    let mut frame_encoder = graphics_device.begin_frame();

    {
        let encoder = &mut frame_encoder.encoder;

        let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Screen Clear"),
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view: &frame_encoder.backbuffer_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        });
    }

    fullscreen_quad.render(&mut frame_encoder);

    let mut shape_recorder = debug_drawer.begin();
    shape_recorder.draw_circle(vec3(0.0, 0.0, 0.0), 2.0, 0.0);
    shape_recorder.end(&mut frame_encoder);

    text_system.render_horizontal(
        TextAlignment {
            x: AxisAlign::Start(10),
            y: AxisAlign::WindowCenter,
            max_width: None,
            max_height: None,
        },
        &[
            StyledText::default_styling("This is a test."),
            StyledText {
                text: "Another test, blue this time",
                font: DefaultFont::SpaceMono400(40),
                color: Color::new(0, 0, 255, 255),
            },
            StyledText {
                text: "\nTest with a line break, green.",
                font: DefaultFont::SpaceMono400(40),
                color: Color::new(0, 255, 0, 255),
            },
            StyledText {
                text: "Red test\nHere are some numbers:\n0123456789!@#$%^&*(){}[].",
                font: DefaultFont::SpaceMono400(40),
                color: Color::new(255, 0, 0, 255),
            },
            StyledText {
                text: "\nOpacity test, this should be half-faded white",
                font: DefaultFont::SpaceMono400(40),
                color: Color::new(255, 255, 255, 128),
            },
            StyledText {
                text: &format!(
                    "\nServer addr: {}\nConnected: {}",
                    game.server_addr, game.connected
                ),
                font: DefaultFont::SpaceMono400(40),
                color: Color::new(255, 255, 255, 255),
            },
        ],
        &mut frame_encoder,
    );

    frame_encoder.finish();
}

fn main() {
    sus_common::simple_game::bevy::run_bevy_game::<SusGame>();
}
