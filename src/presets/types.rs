//! Preset data structures for game tuning categories

/// Movement preset - player physics parameters
#[derive(Debug, Clone)]
pub struct MovementPreset {
    pub name: String,
    pub move_speed: f32,
    pub ground_accel: f32,
    pub ground_decel: f32,
    pub air_accel: f32,
    pub air_decel: f32,
    pub jump_velocity: f32,
    pub gravity_rise: f32,
    pub gravity_fall: f32,
}

/// Ball preset - ball physics parameters
#[derive(Debug, Clone)]
pub struct BallPreset {
    pub name: String,
    pub ball_gravity: f32,
    pub ball_bounce: f32,
    pub ball_air_friction: f32,
    pub ball_roll_friction: f32,
}

/// Shooting preset - shot parameters
#[derive(Debug, Clone)]
pub struct ShootingPreset {
    pub name: String,
    pub shot_charge_time: f32,
    pub shot_max_power: f32,
}

/// Global preset - combines all settings into one preset
#[derive(Debug, Clone)]
pub struct CompositePreset {
    pub name: String,
    pub level: Option<u32>,       // Level number (1-indexed)
    pub palette: Option<usize>,   // Palette index (0-indexed)
    pub ball_style: Option<String>, // Ball style name
    pub movement: String,         // Name of MovementPreset
    pub ball: String,             // Name of BallPreset
    pub shooting: String,         // Name of ShootingPreset
}
