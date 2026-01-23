//! Config file auto-reload system
//!
//! Polls config files every 10 seconds and reloads when modified.
//! Replaces F2 manual hot-reload.

use bevy::prelude::*;
use std::fs;
use std::time::SystemTime;

use crate::ai::{AI_PROFILES_FILE, AiProfileDatabase};
use crate::ball::CurrentPalette;
use crate::constants::LEVELS_FILE;
use crate::levels::{LevelDatabase, reload_level_geometry};
use crate::palettes::{PALETTES_FILE, PaletteDatabase};
use crate::presets::{PRESETS_FILE, PresetDatabase};
use crate::scoring::CurrentLevel;
use crate::world::{Basket, CornerRamp, LevelPlatform};

/// Path to ball options file
const BALL_OPTIONS_FILE: &str = "assets/ball_options.txt";

/// How often to check for config changes (seconds)
const CHECK_INTERVAL: f32 = 10.0;

/// Tracks modification times of config files for hot-reload
#[derive(Resource)]
pub struct ConfigWatcher {
    /// Time since last check
    pub timer: f32,
    /// Last known modification times
    pub levels_mtime: Option<SystemTime>,
    pub palettes_mtime: Option<SystemTime>,
    pub ball_options_mtime: Option<SystemTime>,
    pub ai_profiles_mtime: Option<SystemTime>,
    pub presets_mtime: Option<SystemTime>,
}

impl Default for ConfigWatcher {
    fn default() -> Self {
        Self {
            timer: 0.0,
            levels_mtime: get_mtime(LEVELS_FILE),
            palettes_mtime: get_mtime(PALETTES_FILE),
            ball_options_mtime: get_mtime(BALL_OPTIONS_FILE),
            ai_profiles_mtime: get_mtime(AI_PROFILES_FILE),
            presets_mtime: get_mtime(PRESETS_FILE),
        }
    }
}

/// Get file modification time, or None if file doesn't exist
fn get_mtime(path: &str) -> Option<SystemTime> {
    fs::metadata(path).ok().and_then(|m| m.modified().ok())
}

/// Check for config file changes and reload as needed.
/// Runs every 10 seconds.
#[allow(clippy::too_many_arguments)]
pub fn check_config_changes(
    time: Res<Time>,
    mut watcher: ResMut<ConfigWatcher>,
    mut commands: Commands,
    mut level_db: ResMut<LevelDatabase>,
    mut palette_db: ResMut<PaletteDatabase>,
    mut profile_db: ResMut<AiProfileDatabase>,
    mut preset_db: ResMut<PresetDatabase>,
    current_level: Res<CurrentLevel>,
    current_palette: Res<CurrentPalette>,
    level_platforms: Query<Entity, With<LevelPlatform>>,
    corner_ramps: Query<Entity, With<CornerRamp>>,
    mut baskets: Query<(&mut Transform, &Basket)>,
) {
    watcher.timer += time.delta_secs();

    if watcher.timer < CHECK_INTERVAL {
        return;
    }
    watcher.timer = 0.0;

    let mut levels_changed = false;
    let mut palettes_changed = false;
    let mut ball_options_changed = false;
    let mut ai_profiles_changed = false;
    let mut presets_changed = false;

    // Check levels.txt
    let new_levels_mtime = get_mtime(LEVELS_FILE);
    if new_levels_mtime != watcher.levels_mtime {
        watcher.levels_mtime = new_levels_mtime;
        levels_changed = true;
    }

    // Check palettes.txt
    let new_palettes_mtime = get_mtime(PALETTES_FILE);
    if new_palettes_mtime != watcher.palettes_mtime {
        watcher.palettes_mtime = new_palettes_mtime;
        palettes_changed = true;
    }

    // Check ball_options.txt
    let new_ball_options_mtime = get_mtime(BALL_OPTIONS_FILE);
    if new_ball_options_mtime != watcher.ball_options_mtime {
        watcher.ball_options_mtime = new_ball_options_mtime;
        ball_options_changed = true;
    }

    // Check ai_profiles.txt
    let new_ai_profiles_mtime = get_mtime(AI_PROFILES_FILE);
    if new_ai_profiles_mtime != watcher.ai_profiles_mtime {
        watcher.ai_profiles_mtime = new_ai_profiles_mtime;
        ai_profiles_changed = true;
    }

    // Check game_presets.txt
    let new_presets_mtime = get_mtime(PRESETS_FILE);
    if new_presets_mtime != watcher.presets_mtime {
        watcher.presets_mtime = new_presets_mtime;
        presets_changed = true;
    }

    // Reload levels if changed
    if levels_changed {
        *level_db = LevelDatabase::load_from_file(LEVELS_FILE);
        info!("Auto-reloaded levels from {}", LEVELS_FILE);

        let level_index = (current_level.0 - 1) as usize;
        let palette = palette_db
            .get(current_palette.0)
            .expect("Palette index out of bounds");

        // Reload level geometry (platforms + corner ramps)
        if let Some((left_x, right_x, basket_y)) = reload_level_geometry(
            &mut commands,
            &level_db,
            level_index,
            palette.platforms,
            level_platforms.iter(),
            corner_ramps.iter(),
        ) {
            // Update basket positions
            for (mut basket_transform, basket) in &mut baskets {
                basket_transform.translation.y = basket_y;
                basket_transform.translation.x = match basket {
                    Basket::Left => left_x,
                    Basket::Right => right_x,
                };
            }
        }
    }

    // Reload palettes if changed
    if palettes_changed {
        *palette_db = PaletteDatabase::load_or_create(PALETTES_FILE);
        info!("Auto-reloaded palettes from {}", PALETTES_FILE);
        // Note: Palette colors will be applied on next frame by apply_palette_colors system
    }

    // Ball options reload would require regenerating textures, which is complex
    // For now, just log that it changed - full reload requires restart
    if ball_options_changed {
        info!("ball_options.txt changed - restart game to apply new ball styles");
    }

    // Reload AI profiles if changed
    if ai_profiles_changed {
        *profile_db = AiProfileDatabase::load_from_file(AI_PROFILES_FILE);
        info!("Auto-reloaded AI profiles from {}", AI_PROFILES_FILE);
    }

    // Reload game presets if changed
    if presets_changed {
        *preset_db = PresetDatabase::load_from_file(PRESETS_FILE);
        info!("Auto-reloaded game presets from {}", PRESETS_FILE);
        // Note: Preset values are applied when cycling through presets
    }
}
