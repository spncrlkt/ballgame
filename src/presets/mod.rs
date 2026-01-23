//! Presets module - game tuning preset system
//!
//! Provides hierarchical presets for movement, ball physics, and shooting.
//! Presets can be loaded from assets/game_presets.txt and hot-reloaded.

mod apply;
mod database;
mod types;

pub use apply::{CurrentPresets, apply_composite_preset, apply_preset_to_tweaks};
pub use database::{PRESETS_FILE, PresetDatabase};
pub use types::{BallPreset, CompositePreset, MovementPreset, ShootingPreset};
