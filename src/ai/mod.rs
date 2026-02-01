//! AI module - AI decision making and input generation

pub mod capabilities;
pub mod decision;
pub mod heatmaps;
pub mod navigation;
pub mod pathfinding;
mod profiles;
pub mod shot_quality;
pub mod world_model;

pub use capabilities::AiCapabilities;
pub use decision::*;
pub use heatmaps::{HeatmapBundle, load_heatmaps_on_level_change};
pub use navigation::{
    AiNavState, EdgeType, LevelGeometry, NavAction, NavEdge, NavGraph, NavNode, PlatformSource,
    mark_nav_dirty_on_level_change, rebuild_nav_graph,
};
pub use pathfinding::{PathResult, find_path, find_path_to_shoot};
pub use profiles::*;
pub use shot_quality::{SHOT_QUALITY_ACCEPTABLE, SHOT_QUALITY_GOOD, evaluate_shot_quality};
pub use world_model::{PlatformBounds, extract_platform_data, extract_platforms_from_nav};

use bevy::prelude::*;

use crate::events::{CharacterId, EventBus, GameEvent};
use crate::input::PlayerInput;
use crate::player::{Character, HumanControlled, Player, Team};

/// Per-entity input buffer used by physics systems.
/// All players have this component - human input is copied here, AI writes directly.
/// This unifies input handling so physics systems read from one source.
#[derive(Component, Default)]
pub struct InputState {
    pub move_x: f32,
    pub jump_buffer_timer: f32,
    pub jump_held: bool,
    pub pickup_pressed: bool,
    pub throw_held: bool,
    pub throw_released: bool,
    /// Pass button pressed (for 2v2 mode - pass to teammate)
    pub pass_pressed: bool,
    /// Block button pressed (RB when not holding ball)
    pub block_pressed: bool,
    /// Turbo button held (West button)
    pub turbo_held: bool,
}

/// AI state machine tracking current goal and parameters
#[derive(Component, Default)]
pub struct AiState {
    pub current_goal: AiGoal,
    pub shot_charge_target: f32,
    /// UUID of the AI profile for this AI's personality
    pub profile_id: String,
    /// Target position for navigation (set by goal system, consumed by nav system)
    pub nav_target: Option<bevy::prelude::Vec2>,
    /// Whether AI is performing a jump shot
    pub jump_shot_active: bool,
    /// Timer for jump shot (tracks jump phase)
    pub jump_shot_timer: f32,
    /// Last position for stuck detection
    pub last_position: Option<bevy::prelude::Vec2>,
    /// Timer for how long AI has been stuck (not moving while trying to)
    pub stuck_timer: f32,
    /// Elapsed time at last defensive goal switch (for hysteresis)
    pub last_defense_switch: f32,
    /// Timer for steal reaction delay (simulates human reaction time)
    pub steal_reaction_timer: f32,
    /// Whether AI was in steal range last frame (for reset detection)
    pub was_in_steal_range: bool,
    /// Cooldown timer for button presses (simulates human mashing speed)
    pub button_press_cooldown: f32,
    /// Commitment timer for steal attempts - prevents premature exit from AttemptSteal
    pub steal_commit_timer: f32,
    /// Time in seconds the AI has been holding the ball (for desperation shots)
    pub ball_hold_time: f32,
    /// Position at start of stuck detection window (for cumulative movement check)
    pub stuck_window_start: Option<bevy::prelude::Vec2>,
    /// Timer for stuck detection window (resets when movement detected)
    pub stuck_window_timer: f32,
    /// Timer for stuck reversal - when > 0, override movement direction
    pub stuck_reverse_timer: f32,
    /// The reversed direction to use when stuck_reverse_timer > 0
    pub stuck_reverse_direction: f32,

    // === CatchPartner mode state ===
    /// Timer for HoldAndPass goal - passes after this reaches 3 seconds
    pub hold_and_pass_timer: f32,
    /// Whether this AI is in CatchPartner mode (debug teammate behavior)
    pub catch_partner_mode: bool,

    // === Distance drill state ===
    /// Distance drill: current target distance from teammate (starts at 1200)
    pub distance_drill_target: f32,
    /// Distance drill: whether AI needs to reposition after pass
    pub distance_drill_reposition: bool,
    /// Distance drill: timer to wait after passing before chasing loose balls (1s delay)
    pub post_pass_wait_timer: f32,
    /// Distance drill: true = shrinking toward 100, false = growing toward 1200
    pub distance_drill_shrinking: bool,

    // === KeepAway adversary mode state ===
    /// Whether this AI is in KeepAwayAdversary mode (aggressive ball chase + intercepts)
    pub keep_away_adversary_mode: bool,
    /// Last known position of the teammate for pass prediction
    pub last_teammate_pos: Option<bevy::prelude::Vec2>,
    /// Timer for pass prediction (how long we've been tracking pass trajectory)
    pub pass_prediction_timer: f32,
}

/// Goals the AI can pursue
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum AiGoal {
    /// Debug mode - stand still, do nothing
    Idle,
    /// Move toward free ball and pick it up
    #[default]
    ChaseBall,
    /// Move toward basket with ball
    AttackWithBall,
    /// Charging a shot at the basket
    ChargeShot,
    /// Pass the ball to teammate in better position
    PassToTeammate,
    /// Attempting to steal from opponent
    AttemptSteal,
    /// Chase ball carrier, position on shot line (uses navigation for platforms)
    InterceptDefense,
    /// Close-range: stay on opponent, attempt steals
    PressureDefense,

    // === CatchPartner goals (debug teammate AI) ===
    /// Position in open spot to receive passes from teammate
    MoveToOpenSpot,
    /// Track incoming pass and prepare to catch
    ReceivePass,
    /// Chase ball after missed pass, retrieve it
    ChaseMissedBall,
    /// Holding ball for 3 seconds before passing back to teammate
    HoldAndPass,
    /// Reposition to target distance from teammate (distance drill)
    RepositionToDistance,

    // === KeepAway goals (adversary vs teammates) ===
    /// Teammate: evade pressure from adversary, get open, pass when safe
    EvadeAndPass,
    /// Adversary: aggressive direct chase of ball carrier
    PressureBallCarrier,
    /// Adversary: position between ball carrier and their pass target
    PredictIntercept,
}

/// Copy human PlayerInput into the human-controlled player's InputState.
/// This unifies input handling - all systems just read from InputState.
/// Consumable flags (pickup_pressed, throw_released) are moved, not copied.
/// Runs early in Update, after capture_input.
pub fn copy_human_input(
    mut human_input: ResMut<PlayerInput>,
    mut human_query: Query<&mut InputState, (With<Player>, With<HumanControlled>)>,
) {
    let Ok(mut input_state) = human_query.single_mut() else {
        return;
    };

    // Continuous inputs (overwrite each frame)
    input_state.move_x = human_input.move_x;
    input_state.jump_held = human_input.jump_held;
    input_state.throw_held = human_input.throw_held;

    // Jump buffer timer: copy from PlayerInput to InputState
    // The timer decrements in capture_input (Update) and gets consumed in apply_input (FixedUpdate)
    // We always copy the latest value - if FixedUpdate consumed it, input_state.timer will be 0
    // and won't trigger another jump until a new press sets human_input.timer again
    input_state.jump_buffer_timer = human_input.jump_buffer_timer;

    // Consumable flags (move to InputState, clear from PlayerInput)
    if human_input.pickup_pressed {
        input_state.pickup_pressed = true;
        human_input.pickup_pressed = false;
    }
    if human_input.throw_released {
        input_state.throw_released = true;
        human_input.throw_released = false;
    }
    if human_input.pass_pressed {
        input_state.pass_pressed = true;
        human_input.pass_pressed = false;
    }
    if human_input.block_pressed {
        input_state.block_pressed = true;
        human_input.block_pressed = false;
    }

    // Continuous turbo held state
    input_state.turbo_held = human_input.turbo_held;
}

/// Swap which player the human controls (Q key / L bumper).
/// For 1v1: Cycles through L0 → R0 → Observer → L0
/// For 2v2: Cycles through L0 → L1 → R0 → R1 → Observer → L0
/// Emits ControllerSwap event to EventBus for auditability.
pub fn swap_control(
    mut commands: Commands,
    mut input: ResMut<PlayerInput>,
    players: Query<(Entity, &Team, Option<&Character>), With<Player>>,
    human_query: Query<(Entity, &Team, Option<&Character>), (With<Player>, With<HumanControlled>)>,
    mut input_states: Query<&mut InputState>,
    mut event_bus: ResMut<EventBus>,
) {
    if !input.swap_pressed {
        return;
    }
    input.swap_pressed = false;

    // Collect all players by character ID (or team if no Character component)
    let mut player_entities: std::collections::HashMap<CharacterId, Entity> =
        std::collections::HashMap::new();

    for (entity, team, character) in &players {
        if let Some(char) = character {
            player_entities.insert(char.0, entity);
        } else {
            // Legacy: no Character component, use Team to determine
            match team {
                Team::Left => {
                    player_entities.insert(CharacterId::L0, entity);
                }
                Team::Right => {
                    player_entities.insert(CharacterId::R0, entity);
                }
            }
        }
    }

    // Build cycle order based on which characters exist
    // Order: L0 → L1 → R0 → R1 → Observer (None)
    let all_chars = [
        CharacterId::L0,
        CharacterId::L1,
        CharacterId::R0,
        CharacterId::R1,
    ];
    let available_chars: Vec<CharacterId> = all_chars
        .iter()
        .filter(|c| player_entities.contains_key(c))
        .copied()
        .collect();

    if available_chars.is_empty() {
        return;
    }

    // Find current controlled character
    let current_char: Option<CharacterId> = human_query.iter().next().and_then(|(_, team, character)| {
        if let Some(char) = character {
            Some(char.0)
        } else {
            // Legacy: derive from team
            match team {
                Team::Left => Some(CharacterId::L0),
                Team::Right => Some(CharacterId::R0),
            }
        }
    });

    // Determine next character in cycle
    // cycle: [L0, L1, R0, R1] → None → [L0, ...]
    let next_char: Option<CharacterId> = match current_char {
        Some(current) => {
            // Find index of current in available_chars
            if let Some(idx) = available_chars.iter().position(|c| *c == current) {
                if idx + 1 < available_chars.len() {
                    // Move to next character
                    Some(available_chars[idx + 1])
                } else {
                    // End of list, go to observer mode
                    None
                }
            } else {
                // Current not found, go to first
                Some(available_chars[0])
            }
        }
        None => {
            // Observer mode, go to first character
            Some(available_chars[0])
        }
    };

    // Remove HumanControlled from current
    if let Some((entity, _, _)) = human_query.iter().next() {
        commands.entity(entity).remove::<HumanControlled>();
    }

    // Add HumanControlled to next (if not observer mode)
    if let Some(next) = next_char {
        if let Some(&entity) = player_entities.get(&next) {
            commands.entity(entity).insert(HumanControlled);
            info!("Control: {} player", next);

            // Emit ControllerSwap for the character gaining control
            event_bus.emit(GameEvent::ControllerSwap {
                character: next,
                old_source: crate::input::AI_SOURCE_ID_START, // Was AI
                new_source: crate::input::KEYBOARD_SOURCE_ID, // Now human
            });
        }
    } else {
        info!("Control: Observer (all AI)");
    }

    // If we had a previous character, emit swap event for it losing control
    if let Some(prev) = current_char {
        event_bus.emit(GameEvent::ControllerSwap {
            character: prev,
            old_source: crate::input::KEYBOARD_SOURCE_ID, // Was human
            new_source: crate::input::AI_SOURCE_ID_START, // Now AI
        });
    }

    // Reset all players' InputState to prevent stale input
    for mut input_state in &mut input_states {
        *input_state = InputState::default();
    }
}
