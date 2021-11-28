use crate::{
    components::ServerPlayerBundle,
    events::{NewPlayer, OutgoingPacket, PlayerInput},
    resources::{AddrToPlayer, PlayerToEntity},
    systems::{
        fixed_timestep_with_state, labels,
        network::{DeliveryType, PlayerIdCounter},
        PacketDestination,
    },
};
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};
use sus_common::{
    components::player::{
        LastInputCounter, PlayerId, PlayerName, PlayerNetworkAddr, UnprocessedInputs,
    },
    math::NormalizedInt,
    network::{
        ConnectAckPacket, FullGameStatePacket, LobbyPlayer, LobbyTickPacket, NewPlayerPacket,
        SequenceCmp, ServerToClient,
    },
    simple_game::{
        bevy::{
            schedule::{ShouldRun, State},
            AppBuilder, Commands, EventReader, EventWriter, FixedTimestep, In, IntoChainSystem,
            IntoSystem, ParallelSystemDescriptorCoercion, Plugin, Query, Res, ResMut, SystemSet,
            Transform,
        },
        glam::{vec3, Vec3},
    },
    GameState,
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
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup.system())
            .add_system_set(SystemSet::on_enter(GameState::Lobby).with_system(setup_lobby.system()))
            .add_system_set(
                SystemSet::new()
                    .with_run_criteria(fixed_timestep_with_state!(
                        self.fixed_timestep,
                        GameState::Lobby,
                    ))
                    .label(labels::Lobby)
                    .after(labels::Network)
                    .with_system(
                        handle_player_input
                            .system()
                            .label(labels::NetworkSystem::PlayerInput)
                            .after(labels::NetworkSystem::Receive),
                    )
                    .with_system(update_lobby.system().after(labels::NetworkSystem::PlayerInput))
                    .with_system(send_new_state.system().label(labels::NetworkSystem::SendPackets))
                    .with_system(new_player_joined.system()),
            )
            .add_system_set(SystemSet::on_exit(GameState::Lobby).with_system(close_lobby.system()));
    }
}

struct LobbyTimer(Instant);
const LOBBY_COUNTDOWN_TIME: Duration = Duration::from_secs(50);

fn setup(mut commands: Commands) {
    commands.spawn().insert(LobbyTimer(Instant::now()));
}

fn setup_lobby() {
    println!("Lobby started");
}

fn update_lobby(
    mut game_state: ResMut<State<GameState>>,
    lobby_timer: Query<&LobbyTimer>,
    mut players: Query<(&PlayerId, &mut Transform, &mut UnprocessedInputs, &mut LastInputCounter)>,
) {
    println!("Lobby tick");
    let lobby_timer = lobby_timer.single().unwrap().0;

    if lobby_timer.elapsed() > LOBBY_COUNTDOWN_TIME {
        println!("Leaving lobby!");
        if game_state.current() == &GameState::Lobby {
            game_state.set(GameState::IntroScreen).unwrap();
        }
    }

    for (player_id, mut transform, mut unprocessed_inputs, mut last_input_counter) in
        players.iter_mut()
    {
        if let Some(input) = unprocessed_inputs.0.pop_front() {
            if input.counter.sequentially_greater_than(last_input_counter.0) {
                last_input_counter.0 = input.counter;
                println!("Moving player ID {} with input {:?}", player_id.0, input);

                let velocity = vec3(input.x.normalized(), input.y.normalized(), 0.0);
                transform.translation += velocity;
            }
        }
    }
}

fn send_new_state(
    players: Query<(&PlayerId, &Transform, &PlayerNetworkAddr, &LastInputCounter)>,
    mut outgoing_packets: EventWriter<OutgoingPacket>,
) {
    let players_vec: Vec<_> = players
        .iter()
        .map(|(id, transform, _, _)| LobbyPlayer {
            id: id.0,
            pos: (transform.translation.x, transform.translation.y),
        })
        .collect();

    for (_player_id, _transform, network_addr, last_input_counter) in players.iter() {
        let packet = ServerToClient::LobbyTick(LobbyTickPacket {
            last_input_counter: last_input_counter.0,
            players: players_vec.clone(),
        });

        outgoing_packets.send(OutgoingPacket::new(
            PacketDestination::Single(network_addr.0),
            packet,
            DeliveryType::UnreliableSequenced,
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
            println!("Player (id={}) sent input: {:?}", event.id, event.input);

            if let Ok(mut unprocessed_input) = unprocessed_inputs.get_mut(*player_entity) {
                unprocessed_input.0.push_back(event.input);
            }
        }
    }
}

fn new_player_joined(
    mut commands: Commands,
    mut new_player_rx: EventReader<NewPlayer>,
    mut players: ResMut<AddrToPlayer>,
    mut player_to_entity: ResMut<PlayerToEntity>,
    mut player_id_counter: ResMut<PlayerIdCounter>,
    mut outgoing_packets: EventWriter<OutgoingPacket>,
    existing_players: Query<(&PlayerName, &PlayerId)>,
) {
    let player_id_counter = &mut player_id_counter.0;

    for new_player in new_player_rx.iter() {
        let new_player_id = *player_id_counter;
        *player_id_counter += 1;

        println!("Spawning new player with id {}", new_player_id);

        let entity_id = commands
            .spawn()
            .insert_bundle(ServerPlayerBundle {
                id: PlayerId(new_player_id),
                name: PlayerName(new_player.connect_packet.name.clone()),
                network_addr: PlayerNetworkAddr(new_player.addr),
                unprocessed_inputs: UnprocessedInputs(VecDeque::new()),
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
        ));
    }
}

fn close_lobby() {}
