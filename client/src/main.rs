use crate::{
    components::{ClientPlayerBundle, MyPlayer},
    events::OutgoingPacket,
    resources::{InputCounter, MyName},
    systems::ClientNetworkPlugin,
};
use std::{collections::HashMap, net::SocketAddr};
use sus_common::{
    components::player::{PlayerId, PlayerName},
    network::{
        ClientToServer, ConnectAckPacket, DeliveryType, FullGameStatePacket, LobbyTickPacket,
        NewPlayerPacket,
    },
    resources::PlayerToEntity,
    simple_game::{
        bevy::{
            App, AppBuilder, BevyGame, Commands, CorePlugin, EventReader, EventWriter,
            FixedTimestep, IntoSystem, Query, Res, ResMut, SystemSet, Transform,
        },
        glam::{vec3, Vec3},
        graphics::{
            text::{AxisAlign, Color, DefaultFont, StyledText, TextAlignment, TextSystem},
            DebugDrawer, FullscreenQuad, GraphicsDevice,
        },
        wgpu,
        winit::event::{ElementState, KeyboardInput, VirtualKeyCode},
        WindowDimensions,
    },
    systems::labels,
    PlayerInput,
};

mod components;
mod events;
mod resources;
mod systems;

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

        let game = SusGame { server_addr: SERVER_ADDR.parse().unwrap(), connected: false };
        let my_name = "Brian".to_string();

        ecs_world_builder
            .add_plugin(CorePlugin)
            .insert_resource(game)
            .insert_resource(MyName(my_name))
            .add_startup_system(init.system())
            .add_plugin(ClientNetworkPlugin)
            .add_system(handle_input.system())
            .add_system_set(
                SystemSet::new()
                    .after(labels::NetworkSystem::Receive)
                    .with_run_criteria(
                        FixedTimestep::step(1.0 / Self::desired_fps() as f64)
                            .with_label("game_timestep"),
                    )
                    .with_system(send_input_to_server.system())
                    .with_system(handle_connect_ack.system())
                    .with_system(handle_full_game_state.system())
                    .with_system(new_player_joined.system())
                    .with_system(handle_lobby_tick.system())
                    .with_system(update_game.system()),
            )
            .add_system(render.system());

        ecs_world_builder
    }
}

fn init(mut commands: Commands, graphics_device: Res<GraphicsDevice>) {
    let text_system: TextSystem = TextSystem::new(&graphics_device);
    let debug_drawer = DebugDrawer::new(&graphics_device);
    let fullscreen_quad = FullscreenQuad::new(&graphics_device);
    let player_input = PlayerInput::default();

    commands.insert_resource(text_system);
    commands.insert_resource(debug_drawer);
    commands.insert_resource(fullscreen_quad);
    commands.insert_resource(player_input);
    commands.insert_resource(InputCounter(0));
    commands.insert_resource(PlayerToEntity(HashMap::new()));
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
    player_input: Res<PlayerInput>,
    mut input_counter: ResMut<InputCounter>,
    mut outgoing_packets: EventWriter<OutgoingPacket>,
) {
    // TODO(bschwind) - Store unacked inputs in a list
    let input_packet = player_input.to_player_input_packet(input_counter.0);
    input_counter.0 = input_counter.0.wrapping_add(1);

    let msg = ClientToServer::PlayerInput(input_packet);

    outgoing_packets.send(OutgoingPacket::new(
        msg,
        DeliveryType::UnreliableSequenced,
        Some(sus_common::network::INPUT_STREAM),
    ));
}

fn handle_connect_ack(
    mut commands: Commands,
    mut connect_ack_rx: EventReader<ConnectAckPacket>,
    my_name: Res<MyName>,
    mut player_to_entity: ResMut<PlayerToEntity>,
) {
    for connect_ack in connect_ack_rx.iter() {
        println!("Got a connect ack");

        let entity_id = commands
            .spawn()
            .insert_bundle(ClientPlayerBundle {
                id: PlayerId(connect_ack.id),
                name: PlayerName(my_name.0.clone()),
                transform: Transform::from_translation(Vec3::ZERO),
            })
            .insert(MyPlayer)
            .id();

        player_to_entity.0.insert(connect_ack.id, entity_id);
    }
}

fn new_player_joined(
    mut commands: Commands,
    mut new_player_rx: EventReader<NewPlayerPacket>,
    mut player_to_entity: ResMut<PlayerToEntity>,
) {
    for new_player in new_player_rx.iter() {
        let entity_id = commands
            .spawn()
            .insert_bundle(ClientPlayerBundle {
                id: PlayerId(new_player.id),
                name: PlayerName(new_player.name.clone()),
                transform: Transform::from_translation(Vec3::ZERO),
            })
            .insert(MyPlayer)
            .id();

        player_to_entity.0.insert(new_player.id, entity_id);
    }
}

fn handle_full_game_state(
    mut commands: Commands,
    mut full_game_state_rx: EventReader<FullGameStatePacket>,
    mut player_to_entity: ResMut<PlayerToEntity>,
) {
    for full_game_state in full_game_state_rx.iter() {
        for player in &full_game_state.players {
            let entity_id = commands
                .spawn()
                .insert_bundle(ClientPlayerBundle {
                    id: PlayerId(player.id),
                    name: PlayerName(player.name.clone()),
                    transform: Transform::from_translation(Vec3::ZERO),
                })
                .id();

            player_to_entity.0.insert(player.id, entity_id);
        }
    }
}

fn handle_lobby_tick(
    player_to_entity: Res<PlayerToEntity>,
    mut lobby_tick_rx: EventReader<LobbyTickPacket>,
    mut players: Query<(&PlayerId, &mut Transform)>,
) {
    for lobby_tick in lobby_tick_rx.iter() {
        for player in &lobby_tick.players {
            if let Some(player_entity) = player_to_entity.0.get(&player.id) {
                if let Ok((_, mut transform)) = players.get_mut(*player_entity) {
                    transform.translation = vec3(player.pos.0, player.pos.1, 0.0);
                }
            }
        }
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
    players: Query<(&PlayerId, &Transform)>,
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

    for (_player_id, transform) in players.iter() {
        shape_recorder.draw_circle(transform.translation, 2.0, 0.0);
    }
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
