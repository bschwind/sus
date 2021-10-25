pub mod labels;
mod lobby;
mod network;

pub use lobby::*;
pub use network::*;

// A hacky way to work around having run criteria for both a fixed
// timestep and a game state.
// https://github.com/bevyengine/bevy/issues/1839#issuecomment-835807108
#[allow(unused)]
macro_rules! fixed_timestep_with_state {
    ($timestep:expr, $state_condition:expr$(,)*) => {
        FixedTimestep::step($timestep).chain(
            (|In(input): In<ShouldRun>, state: Res<State<GameState>>| {
                if state.current() == &($state_condition) {
                    input
                } else {
                    ShouldRun::No
                }
            })
            .system(),
        )
    };
}

#[allow(unused)]
pub(crate) use fixed_timestep_with_state;
