use crate::{network::SequenceCmp, PlayerInputPacket};
use simple_game::bevy::{bevy_ecs, Component};
use std::{collections::VecDeque, net::SocketAddr};

#[derive(Debug, Component)]
pub struct PlayerId(pub u16);

#[derive(Debug, Component)]
pub struct MyPlayerId(pub Option<u16>);

#[derive(Debug, Component)]
pub struct PlayerNetworkAddr(pub SocketAddr);

#[derive(Debug, Component)]
pub struct PlayerName(pub String);

#[derive(Debug, Component)]
pub struct LastInputCounter(pub u16);

#[derive(Debug, Component)]
pub struct UnprocessedInputs(pub VecDeque<PlayerInputPacket>);

impl Default for UnprocessedInputs {
    fn default() -> Self {
        UnprocessedInputs(VecDeque::new())
    }
}

impl UnprocessedInputs {
    pub fn clear_acknowledged_inputs(&mut self, input_ack: u16) {
        // Clear items from the front of the queue until we reach a value which is
        // greater than `input_ack`.
        loop {
            match self.0.front() {
                Some(front) if input_ack.sequentially_greater_than_or_equal_to(front.counter) => {
                    self.0.pop_front();
                },
                _ => break,
            }
        }
    }
}

#[test]
fn test_clear() {
    let mut inputs = VecDeque::new();

    for i in 0..30 {
        inputs.push_back(PlayerInputPacket { counter: i, x: 0, y: 0 });
    }

    let mut unprocessed_inputs = UnprocessedInputs(inputs);
    unprocessed_inputs.clear_acknowledged_inputs(u16::MAX);
    assert_eq!(unprocessed_inputs.0.len(), 30);

    unprocessed_inputs.clear_acknowledged_inputs(0);
    assert_eq!(unprocessed_inputs.0.len(), 29);

    unprocessed_inputs.clear_acknowledged_inputs(10);
    assert_eq!(unprocessed_inputs.0.len(), 19);

    unprocessed_inputs.clear_acknowledged_inputs(40);
    assert_eq!(unprocessed_inputs.0.len(), 0);

    unprocessed_inputs.clear_acknowledged_inputs(200);
    assert_eq!(unprocessed_inputs.0.len(), 0);
}
