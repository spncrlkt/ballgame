//! Bevy systems for server integration
//!
//! Systems that bridge the async WebSocket server with Bevy's game loop.

use bevy::prelude::*;

use ballgame_protocol::{AgentInput, LobbySnapshot, ServerPayload, SlotInfo};

use super::bridge::ServerBridge;
use super::lobby::LobbyState;
use super::slots::{Slot, SlotDisplay};
use super::snapshot::create_game_snapshot;
use super::tournament::TournamentConfig;

use crate::{
    countdown::MatchCountdown,
    player::{BlockState, Facing},
    scoring::CurrentLevel,
    AiState, Ball, BallState, BallStyle, Buff, Character, CharacterId, ChargingShot, Grounded,
    HoldingBall, InputState, Player, RemoteControlled, Score, TargetBasket, Team, TurboGauge,
    Velocity,
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
///
/// Also manages the RemoteControlled component: players in Remote slots get the marker
/// (so embedded AI skips them), players not in Remote slots have it removed.
pub fn read_remote_inputs(
    mut commands: Commands,
    bridge: Res<ServerBridge>,
    mut players: Query<(Entity, &Character, &mut InputState), With<Player>>,
) {
    // Block on async call to get snapshot of slots
    let slots = bridge.runtime.block_on(bridge.slots.snapshot());

    for (entity, character, mut input_state) in players.iter_mut() {
        let slot_id = character_to_slot(character.0);
        if let Some(slot) = slots.get(slot_id as usize) {
            if let Slot::Remote { last_input, .. } = slot {
                // Debug: log remote input being applied (only if there's actual input)
                if last_input.has_input() && bridge.tick_count % 60 == 0 {
                    info!(
                        "Remote input for {:?} (slot {}): move_x={:.1}",
                        character.0, slot_id, last_input.move_x
                    );
                }
                // Convert protocol AgentInput to game InputState
                *input_state = agent_input_to_input_state(last_input);
                // Mark as remote-controlled so embedded AI skips this player
                commands.entity(entity).insert(RemoteControlled);
            } else {
                // Not a remote slot - remove marker so embedded AI takes over
                commands.entity(entity).remove::<RemoteControlled>();
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
    bridge
        .runtime
        .block_on(bridge.broadcaster.broadcast_state(tick, snapshot));
}

/// Run condition: check if server mode is active
pub fn server_mode_active(bridge: Option<Res<ServerBridge>>) -> bool {
    bridge.is_some()
}

/// Broadcast lobby state to all connected clients
///
/// Runs every 0.5 seconds while the lobby is active to keep clients in sync.
pub fn broadcast_lobby_state(
    mut lobby_state: ResMut<LobbyState>,
    bridge: Res<ServerBridge>,
    current_level: Res<crate::scoring::CurrentLevel>,
    tournament_config: Res<TournamentConfig>,
    time: Res<Time>,
) {
    // Update broadcast timer
    lobby_state.broadcast_timer += time.delta_secs();

    // Only broadcast every 0.5 seconds
    if lobby_state.broadcast_timer < 0.5 {
        return;
    }
    lobby_state.broadcast_timer = 0.0;

    // Get slot displays
    let displays = bridge
        .runtime
        .block_on(bridge.slots.get_all_slot_displays());

    // Convert to SlotInfo array
    let slots: [SlotInfo; 4] = [
        slot_display_to_info(0, &displays[0]),
        slot_display_to_info(1, &displays[1]),
        slot_display_to_info(2, &displays[2]),
        slot_display_to_info(3, &displays[3]),
    ];

    let snapshot = LobbySnapshot {
        slots,
        level_id: current_level.0.clone(),
        score_limit: tournament_config.score_limit,
        time_limit_secs: tournament_config.time_limit_secs,
    };

    // Broadcast to all clients
    let tick = bridge.current_tick();
    bridge.runtime.block_on(async {
        bridge
            .broadcaster
            .broadcast(tick, ServerPayload::LobbyUpdate(snapshot))
            .await;
    });
}

/// Convert SlotDisplay to SlotInfo for protocol
fn slot_display_to_info(slot_id: u8, display: &SlotDisplay) -> SlotInfo {
    SlotInfo {
        slot_id,
        state: display.to_slot_state(),
        client_name: display.client_name().map(|s| s.to_string()),
        ai_profile: display.ai_profile().map(|s| s.to_string()),
    }
}
