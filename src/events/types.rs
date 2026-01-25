//! Event type definitions for the logging system

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Player identifier (Left or Right)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayerId {
    L,
    R,
}

impl std::fmt::Display for PlayerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlayerId::L => write!(f, "L"),
            PlayerId::R => write!(f, "R"),
        }
    }
}

/// Game configuration snapshot for analytics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GameConfig {
    // Physics
    pub gravity_rise: f32,
    pub gravity_fall: f32,
    pub jump_velocity: f32,
    pub move_speed: f32,
    pub ground_accel: f32,
    pub air_accel: f32,
    // Ball physics
    pub ball_gravity: f32,
    pub ball_bounce: f32,
    pub ball_air_friction: f32,
    pub ball_ground_friction: f32,
    // Shooting
    pub shot_max_power: f32,
    pub shot_max_speed: f32,
    pub shot_charge_time: f32,
    pub shot_max_variance: f32,
    pub shot_min_variance: f32,
    // Steal
    pub steal_range: f32,
    pub steal_success_chance: f32,
    pub steal_cooldown: f32,
    // Active presets (if using preset system)
    pub preset_movement: Option<String>,
    pub preset_ball: Option<String>,
    pub preset_shooting: Option<String>,
    pub preset_composite: Option<String>,
}

/// All game events that can be logged
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameEvent {
    // === Session Events ===
    /// Session started (generated once per game launch)
    SessionStart {
        session_id: String, // UUID v4
        timestamp: String,  // ISO 8601
    },
    /// Game configuration snapshot (logged after session start)
    Config(GameConfig),

    // === Match Events ===
    /// Match started
    MatchStart {
        level: u32,
        level_name: String,
        left_profile: String,
        right_profile: String,
        seed: u64,
    },
    /// Match ended
    MatchEnd {
        score_left: u32,
        score_right: u32,
        duration: f32,
    },

    // === Scoring Events ===
    /// Goal scored
    Goal {
        player: PlayerId,
        score_left: u32,
        score_right: u32,
    },

    // === Ball Events ===
    /// Ball picked up
    Pickup { player: PlayerId },
    /// Ball dropped/lost without shot
    Drop { player: PlayerId },
    /// Shot started (charge began)
    ShotStart {
        player: PlayerId,
        pos: (f32, f32),
        quality: f32,
    },
    /// Shot released
    ShotRelease {
        player: PlayerId,
        charge: f32,
        angle: f32,
        power: f32,
    },

    // === Steal Events ===
    /// Steal attempted
    StealAttempt { attacker: PlayerId },
    /// Steal succeeded
    StealSuccess { attacker: PlayerId },
    /// Steal failed
    StealFail { attacker: PlayerId },
    /// Steal attempted but out of range
    StealOutOfRange { attacker: PlayerId },

    // === Movement Events ===
    /// Player jumped
    Jump { player: PlayerId },
    /// Player landed
    Land { player: PlayerId },

    // === AI State Events ===
    /// AI goal changed
    AiGoal {
        player: PlayerId,
        goal: String,
    },
    /// AI navigation path started
    NavStart {
        player: PlayerId,
        target: (f32, f32),
    },
    /// AI navigation completed
    NavComplete { player: PlayerId },

    // === Input Events (for replay/analysis) ===
    /// Input state snapshot (periodic, every N frames)
    Input {
        player: PlayerId,
        move_x: f32,
        jump: bool,
        throw: bool,
        pickup: bool,
    },

    // === Debug/Tick Events ===
    /// Frame tick with positions and velocities (sampled at 50ms / 20 Hz)
    Tick {
        frame: u64,
        left_pos: (f32, f32),
        left_vel: (f32, f32),
        right_pos: (f32, f32),
        right_vel: (f32, f32),
        ball_pos: (f32, f32),
        ball_vel: (f32, f32),
        ball_state: char, // F=Free, H=Held, I=InFlight
    },
}

impl GameEvent {
    /// Get the event type code for compact serialization
    pub fn type_code(&self) -> &'static str {
        match self {
            GameEvent::SessionStart { .. } => "SE",
            GameEvent::Config(_) => "CF",
            GameEvent::MatchStart { .. } => "MS",
            GameEvent::MatchEnd { .. } => "ME",
            GameEvent::Goal { .. } => "G",
            GameEvent::Pickup { .. } => "PU",
            GameEvent::Drop { .. } => "DR",
            GameEvent::ShotStart { .. } => "SS",
            GameEvent::ShotRelease { .. } => "SR",
            GameEvent::StealAttempt { .. } => "SA",
            GameEvent::StealSuccess { .. } => "S+",
            GameEvent::StealFail { .. } => "S-",
            GameEvent::StealOutOfRange { .. } => "SO",
            GameEvent::Jump { .. } => "J",
            GameEvent::Land { .. } => "LD",
            GameEvent::AiGoal { .. } => "AG",
            GameEvent::NavStart { .. } => "NS",
            GameEvent::NavComplete { .. } => "NC",
            GameEvent::Input { .. } => "I",
            GameEvent::Tick { .. } => "T",
        }
    }
}
