use crate::{
    components::ServerPlayerBundle,
    events::{NewPlayer, OutgoingPacket, PlayerInput},
    resources::AddrToPlayer,
    systems::{network::PlayerIdCounter, sets, PacketDestination},
};
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};
use sus_common::{
    components::player::{
        LastInputCounter, PlayerId, PlayerName, PlayerNetworkAddr, PositionHistory,
        UnprocessedInputs,
    },
    math::NormalizedInt,
    network::{
        ConnectAckPacket, DeliveryType, FullGameStatePacket, LobbyPlayer, LobbyTickPacket,
        NewPlayerPacket, SequenceCmp, ServerToClient, GAME_STATE_STREAM,
    },
    resources::PlayerToEntity,
    simple_game::{
        bevy::{
            bevy_ecs, bevy_ecs::event::Events, schedule::State, App, Commands, Component,
            CoreSchedule, EventReader, EventWriter, IntoSystemAppConfig, IntoSystemAppConfigs,
            IntoSystemConfig, IntoSystemConfigs, NextState, OnEnter, OnExit, OnUpdate, Plugin,
            Query, Res, ResMut, Transform,
        },
        glam::{vec3, Vec3},
    },
    state_active, GameState,
};

#[allow(unused)]
pub struct LobbyPlugin {
    fixed_timestep: f64,
}

impl LobbyPlugin {
    pub fn new(update_fps: usize) -> Self {
        Self { fixed_timestep: 1.0 / update_fps as f64 }
    }
}

impl Plugin for LobbyPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup)
            .add_system(setup_lobby.in_schedule(OnEnter(GameState::Lobby)))
            .add_systems(
                (
                    handle_player_input
                        .in_set(sets::NetworkSystem::PlayerInput)
                        .after(sets::NetworkSystem::Receive),
                    update_lobby.after(sets::NetworkSystem::PlayerInput),
                    new_player_joined,
                )
                    .in_set(sets::Lobby)
                    .after(sets::Network)
                    .distributive_run_if(state_active(GameState::Lobby))
                    .in_schedule(CoreSchedule::FixedUpdate),
            )
            .add_system(update_lobby_timer.after(sets::Lobby).in_set(OnUpdate(GameState::Lobby)))
            .add_system(send_new_state.in_set(sets::NetworkSystem::SendPackets))
            .add_system(close_lobby.in_schedule(OnExit(GameState::Lobby)));
    }
}

#[derive(Component)]
struct LobbyTimer(Instant);
const LOBBY_COUNTDOWN_TIME: Duration = Duration::from_secs(50);

fn setup(mut commands: Commands) {
    commands.spawn(LobbyTimer(Instant::now()));
}

fn setup_lobby() {
    println!("Lobby started");
}

fn update_lobby(
    mut players: Query<(
        &PlayerId,
        &mut Transform,
        &mut UnprocessedInputs,
        &mut PositionHistory,
        &mut LastInputCounter,
    )>,
) {
    // println!("Lobby tick");

    for (
        _player_id,
        mut transform,
        mut unprocessed_inputs,
        mut position_history,
        mut last_input_counter,
    ) in players.iter_mut()
    {
        if let Some(input) = unprocessed_inputs.0.pop_front() {
            if input.counter.sequentially_greater_than(last_input_counter.0) {
                last_input_counter.0 = input.counter;
                // println!("Moving player ID {} with input {:?}", player_id.0, input);

                let velocity = vec3(input.x.normalized(), input.y.normalized(), 0.0);

                position_history.0.push((transform.translation.x, transform.translation.y));
                transform.translation += velocity * 0.1;
            }
        }
    }
}

fn update_lobby_timer(
    game_state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
    lobby_timer: Query<&LobbyTimer>,
) {
    let lobby_timer = lobby_timer.single().0;

    if lobby_timer.elapsed() > LOBBY_COUNTDOWN_TIME {
        println!("Leaving lobby!");
        if game_state.0 == GameState::Lobby {
            next_state.set(GameState::IntroScreen);
        }
    }
}

fn send_new_state(
    mut players: Query<(
        &PlayerId,
        &Transform,
        &PlayerNetworkAddr,
        &mut PositionHistory,
        &LastInputCounter,
    )>,
    mut outgoing_packets: EventWriter<OutgoingPacket>,
) {
    let players_vec: Vec<_> = players
        .iter()
        .map(|(id, transform, _, position_history, _)| LobbyPlayer {
            id: id.0,
            pos: (transform.translation.x, transform.translation.y),
            pos_history: position_history.0.clone(),
        })
        .collect();

    for (_player_id, _transform, network_addr, mut position_history, last_input_counter) in
        players.iter_mut()
    {
        position_history.0.clear();

        let packet = ServerToClient::LobbyTick(LobbyTickPacket {
            last_input_counter: last_input_counter.0,
            players: players_vec.clone(),
        });

        outgoing_packets.send(OutgoingPacket::new(
            PacketDestination::Single(network_addr.0),
            packet,
            DeliveryType::UnreliableSequenced,
            Some(GAME_STATE_STREAM),
        ));
    }
}

fn handle_player_input(
    mut input_rx: EventReader<PlayerInput>,
    player_to_entity: Res<PlayerToEntity>,
    mut unprocessed_inputs: Query<&mut UnprocessedInputs>,
) {
    for event in input_rx.iter() {
        if let Some(player_entity) = player_to_entity.0.get(&event.id) {
            // println!("Player (id={}) sent input: {:?}", event.id, event.input);

            if let Ok(mut unprocessed_input) = unprocessed_inputs.get_mut(*player_entity) {
                unprocessed_input.0.push_back(event.input);
            }
        }
    }
}

fn new_player_joined(
    mut commands: Commands,
    mut new_player_rx: ResMut<Events<NewPlayer>>,
    mut players: ResMut<AddrToPlayer>,
    mut player_to_entity: ResMut<PlayerToEntity>,
    mut player_id_counter: ResMut<PlayerIdCounter>,
    mut outgoing_packets: EventWriter<OutgoingPacket>,
    existing_players: Query<(&PlayerName, &PlayerId)>,
) {
    let player_id_counter = &mut player_id_counter.0;

    for new_player in new_player_rx.drain() {
        let new_player_id = *player_id_counter;
        *player_id_counter += 1;

        println!("Spawning new player with id {}", new_player_id);

        let entity_id = commands
            .spawn(ServerPlayerBundle {
                id: PlayerId(new_player_id),
                name: PlayerName(new_player.connect_packet.name.clone()),
                network_addr: PlayerNetworkAddr(new_player.addr),
                unprocessed_inputs: UnprocessedInputs(VecDeque::new()),
                position_history: PositionHistory(Vec::new()),
                last_input_counter: LastInputCounter(0),
                transform: Transform::from_translation(Vec3::ZERO),
            })
            .id();

        // TODO - handle this HashMap cleanup on player disconnect.
        players.0.insert(new_player.addr, new_player_id);
        player_to_entity.0.insert(new_player_id, entity_id);

        let reply = ServerToClient::ConnectAck(ConnectAckPacket { id: new_player_id });

        outgoing_packets.send(OutgoingPacket::new(
            PacketDestination::Single(new_player.addr),
            reply,
            DeliveryType::ReliableOrdered,
            Some(GAME_STATE_STREAM),
        ));

        // Send all existing state to new client
        let players_vec = existing_players
            .iter()
            .map(|(PlayerName(name), PlayerId(id))| NewPlayerPacket::new(name.clone(), *id))
            .collect();

        let full_state_packet =
            ServerToClient::FullGameState(FullGameStatePacket::new(players_vec));

        outgoing_packets.send(OutgoingPacket::new(
            PacketDestination::Single(new_player.addr),
            full_state_packet,
            DeliveryType::ReliableOrdered,
            Some(GAME_STATE_STREAM),
        ));

        // Tell all other players this one has connected
        let new_player_packet = ServerToClient::NewPlayer(NewPlayerPacket::new(
            new_player.connect_packet.name.clone(),
            new_player_id,
        ));

        outgoing_packets.send(OutgoingPacket::new(
            PacketDestination::BroadcastToAllExcept(new_player.addr),
            new_player_packet,
            DeliveryType::ReliableOrdered,
            Some(GAME_STATE_STREAM),
        ));
    }
}

fn close_lobby() {
    println!("lobby is closed");
}
