use crate::{
    components::{PlayerBundle, PlayerId, PlayerName, PlayerNetworkAddr},
    systems::{
        labels,
        network::{DeliveryType, NewPlayer, OutgoingPacket, PlayerIdCounter},
        PacketDestination,
    },
};
use simple_game::bevy::{
    schedule::State, AppBuilder, Commands, EventReader, EventWriter, IntoSystem, Plugin, Query,
    ResMut, SystemSet,
};
use std::time::{Duration, Instant};
use sus_common::{
    network::{FullGameStatePacket, NewPlayerPacket, ServerToClient},
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
                SystemSet::on_update(GameState::Lobby)
                // SystemSet::new()
                    // .with_run_criteria(fixed_timestep_with_state!(
                    //     self.fixed_timestep,
                    //     GameState::Lobby,
                    // ))
                    .label(labels::Lobby)
                    .after(labels::Network)
                    .with_system(update_lobby.system())
                    .with_system(new_player_joined.system()),
            )
            .add_system_set(SystemSet::on_exit(GameState::Lobby).with_system(close_lobby.system()));
    }
}

struct LobbyTimer(Instant);
const LOBBY_COUNTDOWN_TIME: Duration = Duration::from_secs(5);

fn setup(mut commands: Commands) {
    commands.spawn().insert(LobbyTimer(Instant::now()));
}

fn setup_lobby() {
    println!("Lobby started");
}

fn update_lobby(mut game_state: ResMut<State<GameState>>, lobby_timer: Query<&LobbyTimer>) {
    let lobby_timer = lobby_timer.single().unwrap().0;

    if lobby_timer.elapsed() > LOBBY_COUNTDOWN_TIME {
        println!("Leaving lobby!");
        if game_state.current() == &GameState::Lobby {
            game_state.set(GameState::IntroScreen).unwrap();
        }
    }
}

fn new_player_joined(
    mut commands: Commands,
    mut new_player_rx: EventReader<NewPlayer>,
    mut player_id_counter: ResMut<PlayerIdCounter>,
    mut outgoing_packets: EventWriter<OutgoingPacket>,
    existing_players: Query<(&PlayerName, &PlayerId)>,
) {
    let player_id_counter = &mut player_id_counter.0;

    for new_player in new_player_rx.iter() {
        let new_player_id = *player_id_counter;
        *player_id_counter += 1;

        println!("Spawning new player with id {}", new_player_id);

        commands.spawn().insert_bundle(PlayerBundle {
            id: PlayerId(new_player_id),
            name: PlayerName(new_player.connect_packet.name.clone()),
            network_addr: PlayerNetworkAddr(new_player.addr),
        });

        let reply = ServerToClient::ConnectAck;
        outgoing_packets.send(OutgoingPacket::new(
            PacketDestination::Single(new_player.addr),
            reply,
            DeliveryType::ReliableOrdered,
        ));

        // Send all existing state to new client
        let players_vec = existing_players
            .iter()
            .map(|(PlayerName(name), PlayerId(id))| NewPlayerPacket::new(name.clone(), *id, (0, 0)))
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
            (0, 0),
        ));

        outgoing_packets.send(OutgoingPacket::new(
            PacketDestination::BroadcastToAllExcept(new_player.addr),
            new_player_packet,
            DeliveryType::ReliableOrdered,
        ));
    }
}

fn close_lobby() {}
