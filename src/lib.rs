//! Ballgame - A 2v2 ball sport game built with Bevy
//!
//! This crate provides all game components, resources, and systems organized into modules.

// Core modules
pub mod analytics;
pub mod config_watcher;
pub mod constants;
pub mod countdown;
pub mod db_paths;
pub mod db_schema;
pub mod debug_logging;
pub mod generate;
pub mod generated_assets;
pub use debug_logging::DebugLogConfig;
pub mod events;
pub mod helpers;
pub mod replay;
pub mod run_summary;
pub mod settings;
pub mod simulation;
pub mod snapshot;
pub mod testing;
pub mod training;

// Game logic modules
pub mod ai;
pub mod ball;
pub mod input;
pub mod levels;
pub mod palettes;
pub mod player;
pub mod presets;
pub mod scoring;
pub mod shooting;
pub mod steal;
pub mod tuning;
pub mod ui;
pub mod world;

// Re-export commonly used types for convenience
pub use ai::{
    AI_PROFILES_FILE, AiCapabilities, AiGoal, AiNavState, AiProfile, AiProfileDatabase, AiState,
    EdgeType, InputState, NavAction, NavEdge, NavGraph, NavNode, PathResult, find_path,
    find_path_to_shoot,
};
pub use ball::{
    Ball, BallLabel, BallPlayerContact, BallPulse, BallRolling, BallShotGrace, BallSpin, BallState,
    BallStyle, BallTextures, CurrentPalette, DisplayBall, DisplayBallSpin, DisplayBallWave,
    StyleTextures, display_ball_wave,
};
pub use config_watcher::ConfigWatcher;
pub use constants::*;
pub use countdown::{
    CountdownEndTracker, CountdownText, MatchCountdown, apply_jump_ball_velocity, in_countdown,
    not_in_countdown, spawn_countdown_text, trigger_countdown_on_level_change, update_countdown,
};
pub use events::{
    BusEvent, CharacterId, CharacterTickData, ControllerSource, EventBuffer, EventBus, GameConfig,
    GameEvent, LevelChangeTracker, PlayerId, TeamId, emit_level_change_events,
    update_event_bus_time,
};
pub use helpers::*;
pub use input::{
    ControllerMapping, GameMode, GamepadInfo, GamepadRegistry, InputBuffers, InputSource,
    InputSourceId, InputSourceType, PersistentMapping, PlayerInput, RawInput,
    AI_SOURCE_ID_START, CONTROLLER_MAPPING_FILE, GAMEPAD_SOURCE_ID_START, KEYBOARD_SOURCE_ID,
};
pub use levels::{LevelData, LevelDatabase, PlatformDef};
pub use palettes::{PALETTES_FILE, Palette, PaletteDatabase};
pub use player::{
    Character, CharacterSpawnConfig, ControlledBy, CoyoteTimer, Facing, Grounded, HoldingBall,
    HumanControlled, JumpState, Player, TargetBasket, Team, Velocity,
    color_for_character, initial_facing, spawn_character, spawn_charge_gauge,
    spawn_characters_for_mode, spawn_position, target_basket_for_character, team_for_character,
};
pub use presets::{
    BallPreset, CompositePreset, CurrentPresets, MovementPreset, PRESETS_FILE, PresetDatabase,
    ShootingPreset, apply_composite_preset, apply_preset_to_tweaks,
};
pub use replay::{
    MatchInfo, ReplayData, ReplayMode, ReplayState, TickFrame, TimedEvent, not_replay_active,
    replay_active, replay_input_handler, replay_playback, replay_setup, setup_replay_ui,
    update_replay_ui,
};
pub use run_summary::{FileCategory, FileEntry, NextStep, NextStepPriority, RunSummary};
pub use scoring::{CurrentLevel, GoalRecord, Score};
pub use settings::{CurrentSettings, InitSettings, save_settings_system};
pub use shooting::{ChargingShot, LastShotInfo};
pub use snapshot::{
    BallSnapshot, GameSnapshot, PlayerSnapshot, ScoreSnapshot, ShotSnapshot, SnapshotConfig,
    SnapshotTriggerState,
};
pub use steal::{StealContest, StealCooldown, StealTracker};
pub use training::{
    GameResult, GameSummary, SessionSummary, TrainingPhase, TrainingState, Winner,
    ensure_session_dir, print_session_summary, write_session_summary,
};
pub use tuning::{GAMEPLAY_TUNING_FILE, GameplayTuning, PhysicsTweaks};
pub use ui::{
    ChargeGaugeBackground, ChargeGaugeFill, CycleDirection, CycleIndicator, CycleSelection,
    DebugSettings, DebugText, DownOption, RightOption, ScoreFlash, ScoreLevelText,
    StealCooldownIndicator, StealFailFlash, StealOutOfRangeFlash, TweakPanel, TweakPanelState,
    TweakRow, ViewportScale, VulnerableIndicator,
};
pub use world::{Basket, BasketRim, Collider, CornerRamp, LevelPlatform, Platform};

// Re-export database path utilities
pub use db_paths::{DbType, ensure_dir as ensure_db_dir, timestamped as timestamped_db_path};

// =============================================================================
// TRAJECTORY CALCULATION (shared with tools like heatmap generator)
// =============================================================================

/// Shot trajectory result containing angle, required speed, and distance variance
#[derive(Debug, Clone, Copy)]
pub struct ShotTrajectory {
    /// Absolute angle in radians (0=right, π/2=up, π=left)
    pub angle: f32,
    /// Exact speed needed to hit target at this angle
    pub required_speed: f32,
    /// Variance penalty from distance
    pub distance_variance: f32,
}

/// Default variance per unit distance for trajectory calculation
pub const SHOT_DISTANCE_VARIANCE: f32 = 0.00025;

/// Calculate shot trajectory to hit target.
/// Returns the angle and exact speed needed to hit the target.
/// Uses a fixed elevation angle (60°) and calculates the required speed.
///
/// `distance_variance_factor` controls how much distance affects accuracy.
/// Use `SHOT_DISTANCE_VARIANCE` (0.00025) for the default value.
pub fn calculate_shot_trajectory(
    shooter_x: f32,
    shooter_y: f32,
    target_x: f32,
    target_y: f32,
    gravity: f32,
    distance_variance_factor: f32,
) -> Option<ShotTrajectory> {
    let tx = target_x - shooter_x; // Positive = target is right, negative = left
    let ty = target_y - shooter_y; // Positive = target is above, negative = below
    let dx = tx.abs(); // Horizontal distance (always positive)
    let distance = (tx * tx + ty * ty).sqrt();

    // Variance penalty based on distance (longer shots are less accurate)
    let distance_variance = distance * distance_variance_factor;

    // Directly under/over target
    if dx < 1.0 {
        let required_speed = if ty > 0.0 {
            // Need enough speed to reach height ty against gravity
            // v² = 2*g*h → v = sqrt(2*g*h)
            (2.0 * gravity * ty).sqrt()
        } else {
            constants::SHOT_MAX_SPEED * 0.3 // Minimal speed for dropping down
        };
        return Some(ShotTrajectory {
            angle: if ty > 0.0 {
                std::f32::consts::FRAC_PI_2
            } else {
                -std::f32::consts::FRAC_PI_2
            },
            required_speed,
            distance_variance,
        });
    }

    // Calculate optimal angle for minimum energy trajectory
    // θ = atan2(dy + sqrt(dx² + dy²), dx)
    let distance_to_target = (dx * dx + ty * ty).sqrt();
    let optimal_elevation = (ty + distance_to_target).atan2(dx);

    // Clamp to reasonable range (don't go below ~30° or above ~85°)
    let min_angle = 30.0_f32.to_radians();
    let max_angle = 85.0_f32.to_radians();
    let final_elevation = optimal_elevation.clamp(min_angle, max_angle);

    // Calculate required speed: v² = g*dx² / (2*cos²(θ)*(dx*tan(θ) - dy))
    let cos_e = final_elevation.cos();
    let tan_e = final_elevation.tan();
    let denominator = 2.0 * cos_e * cos_e * (dx * tan_e - ty);

    let required_speed = if denominator > 0.0 {
        (gravity * dx * dx / denominator).sqrt()
    } else {
        // Fallback for edge cases (nearly vertical)
        (2.0 * gravity * ty.abs()).sqrt()
    };

    // Convert elevation to absolute angle based on target direction
    let angle = if tx >= 0.0 {
        final_elevation
    } else {
        std::f32::consts::PI - final_elevation
    };

    Some(ShotTrajectory {
        angle,
        required_speed,
        distance_variance,
    })
}
