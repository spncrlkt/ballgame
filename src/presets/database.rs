//! Preset database - loading and storage for game presets

use bevy::prelude::*;
use std::collections::HashMap;
use std::fs;

use crate::constants::*;
use crate::presets::types::{BallPreset, CompositePreset, MovementPreset, ShootingPreset};

/// Path to game presets file
pub const PRESETS_FILE: &str = "assets/game_presets.txt";

/// Database of all loaded presets
#[derive(Resource, Default)]
pub struct PresetDatabase {
    pub movement: Vec<MovementPreset>,
    pub ball: Vec<BallPreset>,
    pub shooting: Vec<ShootingPreset>,
    pub composite: Vec<CompositePreset>,
}

impl PresetDatabase {
    /// Load presets from file, or return default presets if file doesn't exist
    pub fn load_from_file(path: &str) -> Self {
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                warn!("Could not read presets file: {}, using defaults", e);
                return Self::default_presets();
            }
        };

        Self::parse(&content)
    }

    /// Parse preset file content
    fn parse(content: &str) -> Self {
        let mut db = Self::default();
        let mut current_section = String::new();

        for line in content.lines() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Section headers
            if line.starts_with('[') && line.ends_with(']') {
                current_section = line[1..line.len() - 1].to_string();
                continue;
            }

            // Parse preset definition: Name: key=value, key=value, ...
            if let Some(colon_idx) = line.find(':') {
                let name = line[..colon_idx].trim().to_string();
                let values_str = &line[colon_idx + 1..];
                let values = Self::parse_values(values_str);

                match current_section.as_str() {
                    "Movement" => {
                        if let Some(preset) = Self::parse_movement(&name, &values) {
                            db.movement.push(preset);
                        }
                    }
                    "Ball" => {
                        if let Some(preset) = Self::parse_ball(&name, &values) {
                            db.ball.push(preset);
                        }
                    }
                    "Shooting" => {
                        if let Some(preset) = Self::parse_shooting(&name, &values) {
                            db.shooting.push(preset);
                        }
                    }
                    "Composite" => {
                        if let Some(preset) = Self::parse_composite(&name, &values) {
                            db.composite.push(preset);
                        }
                    }
                    _ => {}
                }
            }
        }

        // Ensure we have at least one of each type
        if db.movement.is_empty() {
            db.movement.push(Self::default_movement());
        }
        if db.ball.is_empty() {
            db.ball.push(Self::default_ball());
        }
        if db.shooting.is_empty() {
            db.shooting.push(Self::default_shooting());
        }
        if db.composite.is_empty() {
            db.composite.push(Self::default_composite());
        }

        info!(
            "Loaded presets: {} movement, {} ball, {} shooting, {} composite",
            db.movement.len(),
            db.ball.len(),
            db.shooting.len(),
            db.composite.len()
        );

        db
    }

    /// Parse comma-separated key=value pairs into a map
    fn parse_values(values_str: &str) -> HashMap<String, String> {
        let mut values = HashMap::new();
        for part in values_str.split(',') {
            let part = part.trim();
            if let Some(eq_idx) = part.find('=') {
                let key = part[..eq_idx].trim().to_string();
                let value = part[eq_idx + 1..].trim().to_string();
                values.insert(key, value);
            }
        }
        values
    }

    fn parse_movement(name: &str, values: &HashMap<String, String>) -> Option<MovementPreset> {
        Some(MovementPreset {
            name: name.to_string(),
            move_speed: values.get("move_speed")?.parse().ok()?,
            ground_accel: values.get("ground_accel")?.parse().ok()?,
            ground_decel: values
                .get("ground_decel")
                .and_then(|v| v.parse().ok())
                .unwrap_or(GROUND_DECEL),
            air_accel: values.get("air_accel")?.parse().ok()?,
            air_decel: values
                .get("air_decel")
                .and_then(|v| v.parse().ok())
                .unwrap_or(AIR_DECEL),
            jump_velocity: values.get("jump_velocity")?.parse().ok()?,
            gravity_rise: values.get("gravity_rise")?.parse().ok()?,
            gravity_fall: values.get("gravity_fall")?.parse().ok()?,
        })
    }

    fn parse_ball(name: &str, values: &HashMap<String, String>) -> Option<BallPreset> {
        Some(BallPreset {
            name: name.to_string(),
            ball_gravity: values.get("ball_gravity")?.parse().ok()?,
            ball_bounce: values.get("ball_bounce")?.parse().ok()?,
            ball_air_friction: values
                .get("ball_air_friction")
                .and_then(|v| v.parse().ok())
                .unwrap_or(BALL_AIR_FRICTION),
            ball_roll_friction: values
                .get("ball_roll_friction")
                .and_then(|v| v.parse().ok())
                .unwrap_or(BALL_ROLL_FRICTION),
        })
    }

    fn parse_shooting(name: &str, values: &HashMap<String, String>) -> Option<ShootingPreset> {
        Some(ShootingPreset {
            name: name.to_string(),
            shot_charge_time: values.get("shot_charge_time")?.parse().ok()?,
            shot_max_power: values.get("shot_max_power")?.parse().ok()?,
        })
    }

    fn parse_composite(name: &str, values: &HashMap<String, String>) -> Option<CompositePreset> {
        Some(CompositePreset {
            name: name.to_string(),
            level: values.get("level").and_then(|v| v.parse().ok()),
            palette: values.get("palette").and_then(|v| v.parse().ok()),
            ball_style: values.get("ball_style").cloned(),
            movement: values.get("movement")?.clone(),
            ball: values.get("ball")?.clone(),
            shooting: values.get("shooting")?.clone(),
        })
    }

    /// Create default presets using game constants
    pub fn default_presets() -> Self {
        Self {
            movement: vec![Self::default_movement()],
            ball: vec![Self::default_ball()],
            shooting: vec![Self::default_shooting()],
            composite: vec![Self::default_composite()],
        }
    }

    fn default_movement() -> MovementPreset {
        MovementPreset {
            name: "Default".to_string(),
            move_speed: MOVE_SPEED,
            ground_accel: GROUND_ACCEL,
            ground_decel: GROUND_DECEL,
            air_accel: AIR_ACCEL,
            air_decel: AIR_DECEL,
            jump_velocity: JUMP_VELOCITY,
            gravity_rise: GRAVITY_RISE,
            gravity_fall: GRAVITY_FALL,
        }
    }

    fn default_ball() -> BallPreset {
        BallPreset {
            name: "Default".to_string(),
            ball_gravity: BALL_GRAVITY,
            ball_bounce: BALL_BOUNCE,
            ball_air_friction: BALL_AIR_FRICTION,
            ball_roll_friction: BALL_ROLL_FRICTION,
        }
    }

    fn default_shooting() -> ShootingPreset {
        ShootingPreset {
            name: "Default".to_string(),
            shot_charge_time: SHOT_CHARGE_TIME,
            shot_max_power: SHOT_MAX_POWER,
        }
    }

    fn default_composite() -> CompositePreset {
        CompositePreset {
            name: "Default".to_string(),
            level: None,      // Don't change level
            palette: None,    // Don't change palette
            ball_style: None, // Don't change ball style
            movement: "Default".to_string(),
            ball: "Default".to_string(),
            shooting: "Default".to_string(),
        }
    }

    /// Get movement preset by index
    pub fn get_movement(&self, index: usize) -> Option<&MovementPreset> {
        self.movement.get(index)
    }

    /// Get ball preset by index
    pub fn get_ball(&self, index: usize) -> Option<&BallPreset> {
        self.ball.get(index)
    }

    /// Get shooting preset by index
    pub fn get_shooting(&self, index: usize) -> Option<&ShootingPreset> {
        self.shooting.get(index)
    }

    /// Get composite preset by index
    pub fn get_composite(&self, index: usize) -> Option<&CompositePreset> {
        self.composite.get(index)
    }

    /// Get movement preset by name
    pub fn get_movement_by_name(&self, name: &str) -> Option<&MovementPreset> {
        self.movement.iter().find(|p| p.name == name)
    }

    /// Get ball preset by name
    pub fn get_ball_by_name(&self, name: &str) -> Option<&BallPreset> {
        self.ball.iter().find(|p| p.name == name)
    }

    /// Get shooting preset by name
    pub fn get_shooting_by_name(&self, name: &str) -> Option<&ShootingPreset> {
        self.shooting.iter().find(|p| p.name == name)
    }

    /// Number of movement presets
    pub fn movement_len(&self) -> usize {
        self.movement.len()
    }

    /// Number of ball presets
    pub fn ball_len(&self) -> usize {
        self.ball.len()
    }

    /// Number of shooting presets
    pub fn shooting_len(&self) -> usize {
        self.shooting.len()
    }

    /// Number of composite presets
    pub fn composite_len(&self) -> usize {
        self.composite.len()
    }
}
