//! Global gameplay tuning settings (decoupled from UI)

use bevy::log::warn;
use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};

use crate::constants::*;

// Serde default functions for new accuracy/cadence fields
fn default_shot_max_variance() -> f32 {
    SHOT_MAX_VARIANCE
}
fn default_shot_min_variance() -> f32 {
    SHOT_MIN_VARIANCE
}
fn default_shot_air_variance_penalty() -> f32 {
    SHOT_AIR_VARIANCE_PENALTY
}
fn default_shot_move_variance_penalty() -> f32 {
    SHOT_MOVE_VARIANCE_PENALTY
}
fn default_shot_quick_threshold() -> f32 {
    SHOT_QUICK_THRESHOLD
}
fn default_quick_power_multiplier() -> f32 {
    0.7
}
fn default_quick_power_threshold() -> f32 {
    0.25
}
fn default_speed_randomness_min() -> f32 {
    0.9
}
fn default_speed_randomness_max() -> f32 {
    1.1
}
fn default_shot_distance_variance() -> f32 {
    0.00025
}

/// Path to global gameplay tuning config
pub const GAMEPLAY_TUNING_FILE: &str = "config/gameplay_tuning.json";

/// Serializable tuning values stored in config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameplayTuning {
    pub gravity_rise: f32,
    pub gravity_fall: f32,
    pub jump_velocity: f32,
    pub move_speed: f32,
    pub ground_accel: f32,
    pub ground_decel: f32,
    pub air_accel: f32,
    pub air_decel: f32,
    pub ball_gravity: f32,
    pub ball_bounce: f32,
    pub ball_air_friction: f32,
    pub ball_roll_friction: f32,
    pub shot_max_power: f32,
    pub shot_charge_time: f32,
    // Accuracy/cadence tuning fields
    #[serde(default = "default_shot_max_variance")]
    pub shot_max_variance: f32,
    #[serde(default = "default_shot_min_variance")]
    pub shot_min_variance: f32,
    #[serde(default = "default_shot_air_variance_penalty")]
    pub shot_air_variance_penalty: f32,
    #[serde(default = "default_shot_move_variance_penalty")]
    pub shot_move_variance_penalty: f32,
    #[serde(default = "default_shot_quick_threshold")]
    pub shot_quick_threshold: f32,
    #[serde(default = "default_quick_power_multiplier")]
    pub quick_power_multiplier: f32,
    #[serde(default = "default_quick_power_threshold")]
    pub quick_power_threshold: f32,
    #[serde(default = "default_speed_randomness_min")]
    pub speed_randomness_min: f32,
    #[serde(default = "default_speed_randomness_max")]
    pub speed_randomness_max: f32,
    #[serde(default = "default_shot_distance_variance")]
    pub shot_distance_variance: f32,
}

impl Default for GameplayTuning {
    fn default() -> Self {
        Self {
            gravity_rise: GRAVITY_RISE,
            gravity_fall: GRAVITY_FALL,
            jump_velocity: JUMP_VELOCITY,
            move_speed: MOVE_SPEED,
            ground_accel: GROUND_ACCEL,
            ground_decel: GROUND_DECEL,
            air_accel: AIR_ACCEL,
            air_decel: AIR_DECEL,
            ball_gravity: BALL_GRAVITY,
            ball_bounce: BALL_BOUNCE,
            ball_air_friction: BALL_AIR_FRICTION,
            ball_roll_friction: BALL_ROLL_FRICTION,
            shot_max_power: SHOT_MAX_POWER,
            shot_charge_time: SHOT_CHARGE_TIME,
            // Accuracy/cadence defaults
            shot_max_variance: default_shot_max_variance(),
            shot_min_variance: default_shot_min_variance(),
            shot_air_variance_penalty: default_shot_air_variance_penalty(),
            shot_move_variance_penalty: default_shot_move_variance_penalty(),
            shot_quick_threshold: default_shot_quick_threshold(),
            quick_power_multiplier: default_quick_power_multiplier(),
            quick_power_threshold: default_quick_power_threshold(),
            speed_randomness_min: default_speed_randomness_min(),
            speed_randomness_max: default_speed_randomness_max(),
            shot_distance_variance: default_shot_distance_variance(),
        }
    }
}

impl GameplayTuning {
    pub fn apply_to(&self, tweaks: &mut PhysicsTweaks) {
        tweaks.gravity_rise = self.gravity_rise;
        tweaks.gravity_fall = self.gravity_fall;
        tweaks.jump_velocity = self.jump_velocity;
        tweaks.move_speed = self.move_speed;
        tweaks.ground_accel = self.ground_accel;
        tweaks.ground_decel = self.ground_decel;
        tweaks.air_accel = self.air_accel;
        tweaks.air_decel = self.air_decel;
        tweaks.ball_gravity = self.ball_gravity;
        tweaks.ball_bounce = self.ball_bounce;
        tweaks.ball_air_friction = self.ball_air_friction;
        tweaks.ball_roll_friction = self.ball_roll_friction;
        tweaks.shot_max_power = self.shot_max_power;
        tweaks.shot_charge_time = self.shot_charge_time;
        // Accuracy/cadence fields
        tweaks.shot_max_variance = self.shot_max_variance;
        tweaks.shot_min_variance = self.shot_min_variance;
        tweaks.shot_air_variance_penalty = self.shot_air_variance_penalty;
        tweaks.shot_move_variance_penalty = self.shot_move_variance_penalty;
        tweaks.shot_quick_threshold = self.shot_quick_threshold;
        tweaks.quick_power_multiplier = self.quick_power_multiplier;
        tweaks.quick_power_threshold = self.quick_power_threshold;
        tweaks.speed_randomness_min = self.speed_randomness_min;
        tweaks.speed_randomness_max = self.speed_randomness_max;
        tweaks.shot_distance_variance = self.shot_distance_variance;
    }
}

/// Runtime-adjustable physics values for tweaking gameplay feel
#[derive(Resource, Debug, Clone)]
pub struct PhysicsTweaks {
    pub gravity_rise: f32,
    pub gravity_fall: f32,
    pub jump_velocity: f32,
    pub move_speed: f32,
    pub ground_accel: f32,
    pub ground_decel: f32,
    pub air_accel: f32,
    pub air_decel: f32,
    pub ball_gravity: f32,
    pub ball_bounce: f32,
    pub ball_air_friction: f32,
    pub ball_roll_friction: f32,
    pub shot_max_power: f32,
    pub shot_charge_time: f32,
    // Accuracy/cadence tuning fields
    pub shot_max_variance: f32,
    pub shot_min_variance: f32,
    pub shot_air_variance_penalty: f32,
    pub shot_move_variance_penalty: f32,
    pub shot_quick_threshold: f32,
    pub quick_power_multiplier: f32,
    pub quick_power_threshold: f32,
    pub speed_randomness_min: f32,
    pub speed_randomness_max: f32,
    pub shot_distance_variance: f32,
}

impl Default for PhysicsTweaks {
    fn default() -> Self {
        let defaults = GameplayTuning::default();
        Self {
            gravity_rise: defaults.gravity_rise,
            gravity_fall: defaults.gravity_fall,
            jump_velocity: defaults.jump_velocity,
            move_speed: defaults.move_speed,
            ground_accel: defaults.ground_accel,
            ground_decel: defaults.ground_decel,
            air_accel: defaults.air_accel,
            air_decel: defaults.air_decel,
            ball_gravity: defaults.ball_gravity,
            ball_bounce: defaults.ball_bounce,
            ball_air_friction: defaults.ball_air_friction,
            ball_roll_friction: defaults.ball_roll_friction,
            shot_max_power: defaults.shot_max_power,
            shot_charge_time: defaults.shot_charge_time,
            // Accuracy/cadence fields
            shot_max_variance: defaults.shot_max_variance,
            shot_min_variance: defaults.shot_min_variance,
            shot_air_variance_penalty: defaults.shot_air_variance_penalty,
            shot_move_variance_penalty: defaults.shot_move_variance_penalty,
            shot_quick_threshold: defaults.shot_quick_threshold,
            quick_power_multiplier: defaults.quick_power_multiplier,
            quick_power_threshold: defaults.quick_power_threshold,
            speed_randomness_min: defaults.speed_randomness_min,
            speed_randomness_max: defaults.speed_randomness_max,
            shot_distance_variance: defaults.shot_distance_variance,
        }
    }
}

impl PhysicsTweaks {
    pub const LABELS: [&'static str; 24] = [
        "Gravity Rise",
        "Gravity Fall",
        "Jump Velocity",
        "Move Speed",
        "Ground Accel",
        "Ground Decel",
        "Air Accel",
        "Air Decel",
        "Ball Gravity",
        "Ball Bounce",
        "Ball Air Friction",
        "Ball Roll Friction",
        "Shot Max Power",
        "Shot Charge Time",
        // Accuracy/cadence labels
        "Shot Max Variance",
        "Shot Min Variance",
        "Shot Air Variance",
        "Shot Move Variance",
        "Quick Shot Threshold",
        "Quick Power Mult",
        "Quick Power Thresh",
        "Speed Random Min",
        "Speed Random Max",
        "Shot Dist Variance",
    ];

    pub fn get_value(&self, index: usize) -> f32 {
        match index {
            0 => self.gravity_rise,
            1 => self.gravity_fall,
            2 => self.jump_velocity,
            3 => self.move_speed,
            4 => self.ground_accel,
            5 => self.ground_decel,
            6 => self.air_accel,
            7 => self.air_decel,
            8 => self.ball_gravity,
            9 => self.ball_bounce,
            10 => self.ball_air_friction,
            11 => self.ball_roll_friction,
            12 => self.shot_max_power,
            13 => self.shot_charge_time,
            14 => self.shot_max_variance,
            15 => self.shot_min_variance,
            16 => self.shot_air_variance_penalty,
            17 => self.shot_move_variance_penalty,
            18 => self.shot_quick_threshold,
            19 => self.quick_power_multiplier,
            20 => self.quick_power_threshold,
            21 => self.speed_randomness_min,
            22 => self.speed_randomness_max,
            23 => self.shot_distance_variance,
            _ => 0.0,
        }
    }

    pub fn get_default_value(index: usize) -> f32 {
        match index {
            0 => GRAVITY_RISE,
            1 => GRAVITY_FALL,
            2 => JUMP_VELOCITY,
            3 => MOVE_SPEED,
            4 => GROUND_ACCEL,
            5 => GROUND_DECEL,
            6 => AIR_ACCEL,
            7 => AIR_DECEL,
            8 => BALL_GRAVITY,
            9 => BALL_BOUNCE,
            10 => BALL_AIR_FRICTION,
            11 => BALL_ROLL_FRICTION,
            12 => SHOT_MAX_POWER,
            13 => SHOT_CHARGE_TIME,
            14 => SHOT_MAX_VARIANCE,
            15 => SHOT_MIN_VARIANCE,
            16 => SHOT_AIR_VARIANCE_PENALTY,
            17 => SHOT_MOVE_VARIANCE_PENALTY,
            18 => SHOT_QUICK_THRESHOLD,
            19 => 0.7,  // quick_power_multiplier default
            20 => 0.25, // quick_power_threshold default
            21 => 0.9,  // speed_randomness_min default
            22 => 1.1,  // speed_randomness_max default
            23 => 0.00025, // shot_distance_variance default
            _ => 0.0,
        }
    }

    pub fn set_value(&mut self, index: usize, value: f32) {
        match index {
            0 => self.gravity_rise = value,
            1 => self.gravity_fall = value,
            2 => self.jump_velocity = value,
            3 => self.move_speed = value,
            4 => self.ground_accel = value,
            5 => self.ground_decel = value,
            6 => self.air_accel = value,
            7 => self.air_decel = value,
            8 => self.ball_gravity = value,
            9 => self.ball_bounce = value,
            10 => self.ball_air_friction = value,
            11 => self.ball_roll_friction = value,
            12 => self.shot_max_power = value,
            13 => self.shot_charge_time = value,
            14 => self.shot_max_variance = value,
            15 => self.shot_min_variance = value,
            16 => self.shot_air_variance_penalty = value,
            17 => self.shot_move_variance_penalty = value,
            18 => self.shot_quick_threshold = value,
            19 => self.quick_power_multiplier = value,
            20 => self.quick_power_threshold = value,
            21 => self.speed_randomness_min = value,
            22 => self.speed_randomness_max = value,
            23 => self.shot_distance_variance = value,
            _ => {}
        }
    }

    pub fn is_modified(&self, index: usize) -> bool {
        let current = self.get_value(index);
        let default = Self::get_default_value(index);
        (current - default).abs() > 0.001
    }

    pub fn reset_value(&mut self, index: usize) {
        self.set_value(index, Self::get_default_value(index));
    }

    pub fn reset_all(&mut self) {
        for i in 0..Self::LABELS.len() {
            self.reset_value(i);
        }
    }

    pub fn get_step(&self, index: usize) -> f32 {
        let default = Self::get_default_value(index);
        (default * 0.1).max(0.01)
    }
}

pub fn load_gameplay_tuning_from_file(path: &str) -> Result<GameplayTuning, String> {
    let contents =
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read {}: {}", path, e))?;
    serde_json::from_str(&contents).map_err(|e| format!("Failed to parse {}: {}", path, e))
}

pub fn apply_global_tuning(tweaks: &mut PhysicsTweaks) -> Result<(), String> {
    match load_gameplay_tuning_from_file(GAMEPLAY_TUNING_FILE) {
        Ok(tuning) => {
            tuning.apply_to(tweaks);
            Ok(())
        }
        Err(err) => {
            GameplayTuning::default().apply_to(tweaks);
            Err(err)
        }
    }
}

pub fn load_global_tuning_system(mut tweaks: bevy::prelude::ResMut<PhysicsTweaks>) {
    if let Err(err) = apply_global_tuning(&mut tweaks) {
        warn!("{}", err);
    }
}
