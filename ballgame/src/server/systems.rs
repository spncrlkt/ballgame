//! Bevy systems for server integration
//!
//! Systems that bridge the async WebSocket server with Bevy's game loop.

use bevy::prelude::*;

use ballgame_protocol::AgentInput;

use super::bridge::ServerBridge;
use super::slots::Slot;
use super::snapshot::create_game_snapshot;

use crate::{
    AiState, Ball, BallState, BallStyle, Buff, Character, CharacterId, ChargingShot, Grounded,
    HoldingBall, InputState, Player, Score, TargetBasket, Team, TurboGauge, Velocity,
    countdown::MatchCountdown,
    player::{BlockState, Facing},
    scoring::CurrentLevel,
};

/// Convert game CharacterId to slot index (0-3)
fn character_to_slot(character: CharacterId) -> u8 {
    match character {
        CharacterId::L0 => 0,
        CharacterId::L1 => 1,
        CharacterId::R0 => 2,
        CharacterId::R1 => 3,
    }
}

/// Convert protocol AgentInput to game InputState
fn agent_input_to_input_state(input: &AgentInput) -> InputState {
    InputState {
        move_x: input.move_x,
        jump_buffer_timer: if input.jump_pressed { 0.1 } else { 0.0 },
        jump_held: input.jump_held,
        pickup_pressed: input.action_pressed,
        throw_held: input.shoot_held,
        throw_released: input.shoot_released,
        pass_pressed: input.pass_pressed,
        block_pressed: input.block_pressed,
        turbo_held: input.turbo_held,
    }
}

/// Read remote inputs from SlotManager and apply to player InputState
///
/// This system runs in Update, before AI decision making, to ensure
/// remote inputs are available for the current frame.
pub fn read_remote_inputs(
    bridge: Res<ServerBridge>,
    mut players: Query<(&Character, &mut InputState), With<Player>>,
) {
    // Block on async call to get snapshot of slots
    let slots = bridge.runtime.block_on(bridge.slots.snapshot());

    for (character, mut input_state) in players.iter_mut() {
        let slot_id = character_to_slot(character.0);
        if let Some(slot) = slots.get(slot_id as usize) {
            if let Slot::Remote { last_input, .. } = slot {
                // Convert protocol AgentInput to game InputState
                *input_state = agent_input_to_input_state(last_input);
            }
        }
    }
}

/// Create game state snapshot and broadcast to all clients
///
/// This system runs in FixedUpdate after scoring to ensure clients
/// receive the latest game state each physics tick.
pub fn broadcast_state_system(
    mut bridge: ResMut<ServerBridge>,
    players: Query<
        (
            Entity,
            &Character,
            &Transform,
            &Velocity,
            &Team,
            &Grounded,
            &Facing,
            &TargetBasket,
            &TurboGauge,
            Option<&HoldingBall>,
            Option<&ChargingShot>,
            Option<&AiState>,
            Option<&BlockState>,
            &Buff,
        ),
        With<Player>,
    >,
    ball_query: Query<(&Transform, &Velocity, &BallState, &BallStyle), With<Ball>>,
    score: Res<Score>,
    current_level: Res<CurrentLevel>,
    countdown: Res<MatchCountdown>,
    player_entities: Query<(Entity, &Character), With<Player>>,
    time: Res<Time>,
) {
    let tick = bridge.next_tick();
    let elapsed = time.elapsed_secs();

    let snapshot = create_game_snapshot(
        tick,
        elapsed,
        players,
        ball_query,
        &score,
        &current_level,
        &countdown,
        &player_entities,
    );

    // Broadcast to all connected clients
    bridge.runtime.block_on(
        bridge.broadcaster.broadcast_state(tick, snapshot)
    );
}

/// Run condition: check if server mode is active
pub fn server_mode_active(bridge: Option<Res<ServerBridge>>) -> bool {
    bridge.is_some()
}
