//! Shared event emission logic for simulation and training modes
//!
//! This module consolidates the duplicated event detection and logging code
//! that was previously in `simulation/runner.rs` and `bin/training.rs`.

use bevy::prelude::*;

use crate::{
    AiState, BallState, ChargingShot, HoldingBall, InputState, Score, StealContest, StealCooldown,
    Team, Velocity,
};
use super::{EventBuffer, GameEvent, PlayerId};

/// Configuration for event emission behavior
#[derive(Debug, Clone)]
pub struct EmitterConfig {
    /// Track AI goals for both players (true) or just right player (false)
    pub track_both_ai_goals: bool,
}

impl Default for EmitterConfig {
    fn default() -> Self {
        Self {
            track_both_ai_goals: true,
        }
    }
}

/// Shared state for tracking changes between frames
///
/// This struct holds all the "previous frame" state needed to detect
/// changes and emit the appropriate events.
#[derive(Debug, Clone, Resource)]
pub struct EventEmitterState {
    /// Previous left player score
    pub prev_score_left: u32,
    /// Previous right player score
    pub prev_score_right: u32,
    /// Entity that was holding ball last frame
    pub prev_ball_holder: Option<Entity>,
    /// Whether each player was charging last frame [left, right]
    pub prev_charging: [bool; 2],
    /// Previous AI goal strings [left, right]
    pub prev_ai_goals: [Option<String>; 2],
    /// Previous steal cooldowns [left, right]
    pub prev_steal_cooldowns: [f32; 2],
    /// Time of last tick event
    pub last_tick_time: f32,
    /// Frame counter for tick events
    pub tick_frame_count: u64,
    /// Configuration
    pub config: EmitterConfig,
}

impl Default for EventEmitterState {
    fn default() -> Self {
        Self {
            prev_score_left: 0,
            prev_score_right: 0,
            prev_ball_holder: None,
            prev_charging: [false, false],
            prev_ai_goals: [None, None],
            prev_steal_cooldowns: [0.0, 0.0],
            last_tick_time: 0.0,
            tick_frame_count: 0,
            config: EmitterConfig::default(),
        }
    }
}

impl EventEmitterState {
    /// Create a new emitter state with the given configuration
    pub fn with_config(config: EmitterConfig) -> Self {
        Self {
            config,
            ..Default::default()
        }
    }

    /// Reset state for a new match
    pub fn reset(&mut self) {
        self.prev_score_left = 0;
        self.prev_score_right = 0;
        self.prev_ball_holder = None;
        self.prev_charging = [false, false];
        self.prev_ai_goals = [None, None];
        self.prev_steal_cooldowns = [0.0, 0.0];
        self.last_tick_time = 0.0;
        self.tick_frame_count = 0;
    }
}

/// Player data extracted from queries for event emission
pub struct PlayerSnapshot {
    pub entity: Entity,
    pub team: Team,
    pub position: (f32, f32),
    pub velocity: (f32, f32),
    pub charge_time: f32,
    pub ai_goal: String,
    pub steal_cooldown: f32,
    pub is_holding_ball: bool,
    /// Input state for replay/analysis
    pub input_move_x: f32,
    pub input_jump: bool,
    pub input_throw: bool,
    pub input_pickup: bool,
}

/// Ball data extracted from queries for event emission
pub struct BallSnapshot {
    pub position: (f32, f32),
    pub velocity: (f32, f32),
    pub state: BallState,
}

/// Emit all game events by comparing current state to previous state
///
/// This is the main entry point for event emission. Call this once per frame
/// with the current game state.
pub fn emit_game_events(
    state: &mut EventEmitterState,
    buffer: &mut EventBuffer,
    elapsed: f32,
    score: &Score,
    steal_contest: &StealContest,
    players: &[PlayerSnapshot],
    ball: Option<&BallSnapshot>,
) {
    // === Tick events at 50ms (20 Hz) ===
    emit_tick_events(state, buffer, elapsed, players, ball);

    // === Detect score changes (Goal events) ===
    emit_goal_events(state, buffer, elapsed, score);

    // === AI goal change detection ===
    emit_ai_goal_events(state, buffer, elapsed, players);

    // === Steal event detection ===
    emit_steal_events(state, buffer, elapsed, players, steal_contest);

    // === Track ball possession changes and shot charging ===
    emit_possession_events(state, buffer, elapsed, players);

    // === Detect when ball becomes free (drop or shot release) ===
    if let Some(ball) = ball {
        emit_ball_state_events(state, buffer, elapsed, ball, players);
    }
}

fn emit_tick_events(
    state: &mut EventEmitterState,
    buffer: &mut EventBuffer,
    elapsed: f32,
    players: &[PlayerSnapshot],
    ball: Option<&BallSnapshot>,
) {
    if elapsed - state.last_tick_time < 0.05 {
        return;
    }

    state.last_tick_time = elapsed;
    state.tick_frame_count += 1;
    let frame = state.tick_frame_count;

    // Collect player data
    let mut left_pos = (0.0, 0.0);
    let mut left_vel = (0.0, 0.0);
    let mut right_pos = (0.0, 0.0);
    let mut right_vel = (0.0, 0.0);

    for player in players {
        match player.team {
            Team::Left => {
                left_pos = player.position;
                left_vel = player.velocity;
            }
            Team::Right => {
                right_pos = player.position;
                right_vel = player.velocity;
            }
        }
    }

    // Collect ball data
    let (ball_pos, ball_vel, ball_state_char) = ball
        .map(|b| {
            let state_char = match &b.state {
                BallState::Free => 'F',
                BallState::Held(_) => 'H',
                BallState::InFlight { .. } => 'I',
            };
            (b.position, b.velocity, state_char)
        })
        .unwrap_or(((0.0, 0.0), (0.0, 0.0), 'F'));

    buffer.log(
        elapsed,
        GameEvent::Tick {
            frame,
            left_pos,
            left_vel,
            right_pos,
            right_vel,
            ball_pos,
            ball_vel,
            ball_state: ball_state_char,
        },
    );

    // Log input state for each player at the same rate as ticks
    for player in players {
        let player_id = match player.team {
            Team::Left => PlayerId::L,
            Team::Right => PlayerId::R,
        };
        buffer.log(
            elapsed,
            GameEvent::Input {
                player: player_id,
                move_x: player.input_move_x,
                jump: player.input_jump,
                throw: player.input_throw,
                pickup: player.input_pickup,
            },
        );
    }
}

fn emit_goal_events(
    state: &mut EventEmitterState,
    buffer: &mut EventBuffer,
    elapsed: f32,
    score: &Score,
) {
    if score.left > state.prev_score_left {
        buffer.log(
            elapsed,
            GameEvent::Goal {
                player: PlayerId::L,
                score_left: score.left,
                score_right: score.right,
            },
        );
        state.prev_score_left = score.left;
    }
    if score.right > state.prev_score_right {
        buffer.log(
            elapsed,
            GameEvent::Goal {
                player: PlayerId::R,
                score_left: score.left,
                score_right: score.right,
            },
        );
        state.prev_score_right = score.right;
    }
}

fn emit_ai_goal_events(
    state: &mut EventEmitterState,
    buffer: &mut EventBuffer,
    elapsed: f32,
    players: &[PlayerSnapshot],
) {
    for player in players {
        let (idx, player_id) = match player.team {
            Team::Left => (0, PlayerId::L),
            Team::Right => (1, PlayerId::R),
        };

        // Skip left player if not tracking both
        if idx == 0 && !state.config.track_both_ai_goals {
            continue;
        }

        let goal_str = &player.ai_goal;
        if state.prev_ai_goals[idx].as_ref() != Some(goal_str) {
            state.prev_ai_goals[idx] = Some(goal_str.clone());
            buffer.log(
                elapsed,
                GameEvent::AiGoal {
                    player: player_id,
                    goal: goal_str.clone(),
                },
            );
        }
    }
}

fn emit_steal_events(
    state: &mut EventEmitterState,
    buffer: &mut EventBuffer,
    elapsed: f32,
    players: &[PlayerSnapshot],
    steal_contest: &StealContest,
) {
    for player in players {
        let (idx, player_id) = match player.team {
            Team::Left => (0, PlayerId::L),
            Team::Right => (1, PlayerId::R),
        };

        let current_cooldown = player.steal_cooldown;
        let prev_cooldown = state.prev_steal_cooldowns[idx];

        // Detect if cooldown just jumped up (steal was attempted)
        // Attacker gets STEAL_COOLDOWN (0.3s) on success, STEAL_FAIL_COOLDOWN (0.5s) on fail
        // Victim gets STEAL_VICTIM_COOLDOWN (1.0s) - we skip these (not an attempt)
        // Detect attacker cooldowns only (< 0.6s to exclude victim's 1.0s)
        let is_attacker_cooldown = current_cooldown > 0.25 && current_cooldown < 0.6;
        let cooldown_just_set = prev_cooldown < 0.1;

        if is_attacker_cooldown && cooldown_just_set {
            buffer.log(elapsed, GameEvent::StealAttempt { attacker: player_id });
            // Check StealContest for success/fail (fail_flash_timer > 0 means fail)
            if steal_contest.fail_flash_timer > 0.0 {
                buffer.log(elapsed, GameEvent::StealFail { attacker: player_id });
            } else {
                buffer.log(elapsed, GameEvent::StealSuccess { attacker: player_id });
            }
        }
        state.prev_steal_cooldowns[idx] = current_cooldown;
    }
}

fn emit_possession_events(
    state: &mut EventEmitterState,
    buffer: &mut EventBuffer,
    elapsed: f32,
    players: &[PlayerSnapshot],
) {
    for player in players {
        let (idx, player_id) = match player.team {
            Team::Left => (0, PlayerId::L),
            Team::Right => (1, PlayerId::R),
        };

        // Track pickup
        let is_holding = player.is_holding_ball;
        let was_holding = state.prev_ball_holder == Some(player.entity);

        if is_holding && !was_holding {
            buffer.log(elapsed, GameEvent::Pickup { player: player_id });
            state.prev_ball_holder = Some(player.entity);
        }

        // Detect shot charging start
        let is_charging = player.charge_time > 0.0;
        if is_charging && !state.prev_charging[idx] {
            buffer.log(
                elapsed,
                GameEvent::ShotStart {
                    player: player_id,
                    pos: player.position,
                    quality: 0.5, // Could calculate based on position
                },
            );
        }
        state.prev_charging[idx] = is_charging;
    }
}

fn emit_ball_state_events(
    state: &mut EventEmitterState,
    buffer: &mut EventBuffer,
    elapsed: f32,
    ball: &BallSnapshot,
    players: &[PlayerSnapshot],
) {
    match &ball.state {
        BallState::InFlight { shooter, power } => {
            // If ball just became InFlight, log shot release
            if state.prev_ball_holder.is_some() {
                let player_id = players
                    .iter()
                    .find(|p| p.entity == *shooter)
                    .map(|p| match p.team {
                        Team::Left => PlayerId::L,
                        Team::Right => PlayerId::R,
                    });

                if let Some(pid) = player_id {
                    buffer.log(
                        elapsed,
                        GameEvent::ShotRelease {
                            player: pid,
                            charge: 0.5,
                            angle: 60.0,
                            power: *power,
                        },
                    );
                }
                state.prev_ball_holder = None;
            }
        }
        BallState::Free => {
            // If ball just became Free after being Held, it was a drop
            if state.prev_ball_holder.is_some() {
                if let Some(player) = players
                    .iter()
                    .find(|p| Some(p.entity) == state.prev_ball_holder)
                {
                    let player_id = match player.team {
                        Team::Left => PlayerId::L,
                        Team::Right => PlayerId::R,
                    };
                    buffer.log(elapsed, GameEvent::Drop { player: player_id });
                }
                state.prev_ball_holder = None;
            }
        }
        BallState::Held(_) => {
            // Ball is held - already tracked in possession events
        }
    }
}

/// Helper function to create PlayerSnapshot from query results
///
/// Use this in your systems to extract the data needed for emit_game_events
pub fn snapshot_player(
    entity: Entity,
    team: &Team,
    transform: &Transform,
    velocity: &Velocity,
    charging: &ChargingShot,
    ai_state: &AiState,
    steal_cooldown: &StealCooldown,
    holding: Option<&HoldingBall>,
    input_state: &InputState,
) -> PlayerSnapshot {
    PlayerSnapshot {
        entity,
        team: *team,
        position: (transform.translation.x, transform.translation.y),
        velocity: (velocity.0.x, velocity.0.y),
        charge_time: charging.charge_time,
        ai_goal: format!("{:?}", ai_state.current_goal),
        steal_cooldown: steal_cooldown.0,
        is_holding_ball: holding.is_some(),
        input_move_x: input_state.move_x,
        input_jump: input_state.jump_buffer_timer > 0.0,
        input_throw: input_state.throw_held,
        input_pickup: input_state.pickup_pressed,
    }
}

/// Helper function to create BallSnapshot from query results
pub fn snapshot_ball(
    transform: &Transform,
    velocity: &Velocity,
    state: &BallState,
) -> BallSnapshot {
    BallSnapshot {
        position: (transform.translation.x, transform.translation.y),
        velocity: (velocity.0.x, velocity.0.y),
        state: state.clone(),
    }
}
