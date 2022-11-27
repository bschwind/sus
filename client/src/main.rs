use crate::{
    components::{ClientPlayerBundle, MyPlayer},
    events::OutgoingPacket,
    resources::{InputCounter, MyName},
    systems::{ClientNetworkPlugin, RenderPlugin},
};
use std::{
    collections::{HashMap, VecDeque},
    net::SocketAddr,
};
use sus_common::{
    components::player::{MyPlayerId, PlayerId, PlayerName, UnprocessedInputs},
    math::NormalizedInt,
    network::{
        ClientToServer, ConnectAckPacket, DeliveryType, FullGameStatePacket, LobbyTickPacket,
        NewPlayerPacket,
    },
    resources::PlayerToEntity,
    simple_game::{
        bevy::{
            bevy_ecs, App, BevyGame, Commands, EventReader, EventWriter, FixedTimestep, Query, Res,
            ResMut, Resource, SimpleGamePlugin, SystemSet, Transform, With,
        },
        glam::{vec3, Vec3},
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
const GAME_TIMESTEP_LABEL: &str = "game_timestep";

#[derive(Debug, Resource)]
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

    fn init_systems() -> App {
        let mut ecs_world_builder = App::new();

        let game = SusGame { server_addr: SERVER_ADDR.parse().unwrap(), connected: false };
        let my_name = "Brian".to_string();

        ecs_world_builder
            .add_plugin(SimpleGamePlugin)
            .insert_resource(game)
            .insert_resource(MyName(my_name))
            .add_startup_system(init)
            .add_plugin(ClientNetworkPlugin::new(Self::desired_fps()))
            .add_plugin(RenderPlugin)
            .add_system(handle_input)
            .add_system_set(
                SystemSet::new()
                    .after(labels::NetworkSystem::Receive)
                    .with_run_criteria(
                        FixedTimestep::step(1.0 / Self::desired_fps() as f64)
                            .with_label(GAME_TIMESTEP_LABEL),
                    )
                    .with_system(send_input_to_server)
                    .with_system(handle_connect_ack)
                    .with_system(handle_full_game_state)
                    .with_system(new_player_joined)
                    .with_system(handle_lobby_tick)
                    .with_system(update_game),
            );

        ecs_world_builder
    }
}

fn init(mut commands: Commands) {
    commands.insert_resource(PlayerInput::default());
    commands.insert_resource(InputCounter(0));
    commands.insert_resource(MyPlayerId(None));
    commands.insert_resource(PlayerToEntity(HashMap::new()));
    commands.insert_resource(UnprocessedInputs(VecDeque::new()));
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
    mut unprocessed_inputs: ResMut<UnprocessedInputs>,
    mut outgoing_packets: EventWriter<OutgoingPacket>,
) {
    let input_packet = player_input.to_player_input_packet(input_counter.0);
    unprocessed_inputs.0.push_back(input_packet);

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
    mut my_player_id: ResMut<MyPlayerId>,
) {
    for connect_ack in connect_ack_rx.iter() {
        println!("Got a connect ack");

        let entity_id = commands
            .spawn((
                ClientPlayerBundle {
                    id: PlayerId(connect_ack.id),
                    name: PlayerName(my_name.0.clone()),
                    transform: Transform::from_translation(Vec3::ZERO),
                },
                MyPlayer,
            ))
            .id();

        my_player_id.0 = Some(connect_ack.id);

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
            .spawn(ClientPlayerBundle {
                id: PlayerId(new_player.id),
                name: PlayerName(new_player.name.clone()),
                transform: Transform::from_translation(Vec3::ZERO),
            })
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
                .spawn(ClientPlayerBundle {
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
    mut unprocessed_inputs: ResMut<UnprocessedInputs>,
    my_player_id: Res<MyPlayerId>,
    mut players: Query<(&PlayerId, &mut Transform)>,
) {
    for lobby_tick in lobby_tick_rx.iter() {
        unprocessed_inputs.clear_acknowledged_inputs(lobby_tick.last_input_counter);

        for player in &lobby_tick.players {
            if let Some(player_entity) = player_to_entity.0.get(&player.id) {
                if let Ok((_, mut transform)) = players.get_mut(*player_entity) {
                    if let Some(my_player_id) = my_player_id.0 {
                        if my_player_id == player.id {
                            // Update my player
                            transform.translation = vec3(player.pos.0, player.pos.1, 0.0);
                        } else {
                            // Handle updating the other players
                            transform.translation = vec3(player.pos.0, player.pos.1, 0.0);
                        }
                    }
                }
            }
        }

        // Apply all unacknowledged inputs
        if let Some(my_player_id) = my_player_id.0 {
            if let Some(my_player_entity) = player_to_entity.0.get(&my_player_id) {
                if let Ok((_, mut transform)) = players.get_mut(*my_player_entity) {
                    for input in &unprocessed_inputs.0 {
                        let velocity = vec3(input.x.normalized(), input.y.normalized(), 0.0);
                        transform.translation += velocity * 0.1;
                    }
                }
            }
        }
    }
}

fn update_game(
    player_input: Res<PlayerInput>,
    mut players: Query<(&PlayerId, &mut Transform), With<MyPlayer>>,
) {
    if let Ok((_my_player_id, mut transform)) = players.get_single_mut() {
        let velocity = vec3(player_input.x().normalized(), player_input.y().normalized(), 0.0);
        transform.translation += velocity * 0.1;
    }
    // println!("player_input: {:?}", *player_input);
}

fn main() {
    sus_common::simple_game::bevy::run_bevy_game::<SusGame>();
}
