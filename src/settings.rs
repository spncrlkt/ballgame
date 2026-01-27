//! Persistent settings for game initialization
//!
//! Saves and loads user preferences (viewport size, palette, level, etc.)
//! to/from an init_settings.json file in the config directory.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Path to the settings file
pub const SETTINGS_FILE: &str = "config/init_settings.json";

/// Persistent settings that survive between sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitSettings {
    /// Viewport preset index (0-4)
    pub viewport_index: usize,
    /// Palette index (0-29)
    pub palette_index: usize,
    /// Starting level ID (16-char hex UUID)
    /// For backward compatibility, also accepts level number as string
    pub level: String,
    /// Ball style name
    pub ball_style: String,
    /// Left player AI profile name (empty = human)
    pub left_ai_profile: Option<String>,
    /// Right player AI profile name
    pub right_ai_profile: String,
    /// Active D-pad direction
    pub active_direction: String,
    /// Down menu sub-option
    pub down_option: String,
    /// Right menu sub-option
    pub right_option: String,
}

impl Default for InitSettings {
    fn default() -> Self {
        Self {
            viewport_index: 2,    // 1440p default
            palette_index: 0,     // Aurora (first palette)
            level: String::new(), // Empty = use first level
            ball_style: "wedges".to_string(),
            left_ai_profile: None, // Human controlled
            right_ai_profile: "Balanced".to_string(),
            active_direction: "Down".to_string(),
            down_option: "Composite".to_string(),
            right_option: "Level".to_string(),
        }
    }
}

impl InitSettings {
    /// Load settings from file, or return defaults if file doesn't exist
    pub fn load() -> Self {
        let path = Path::new(SETTINGS_FILE);
        if !path.exists() {
            info!("No init_settings.json found, using defaults");
            return Self::default();
        }

        match fs::read_to_string(path) {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(settings) => {
                    info!("Loaded settings from {}", SETTINGS_FILE);
                    settings
                }
                Err(e) => {
                    warn!("Failed to parse init_settings.json: {}, using defaults", e);
                    Self::default()
                }
            },
            Err(e) => {
                warn!("Failed to read init_settings.json: {}, using defaults", e);
                Self::default()
            }
        }
    }

    /// Save settings to file
    pub fn save(&self) -> Result<(), std::io::Error> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        // Ensure assets directory exists
        if let Some(parent) = Path::new(SETTINGS_FILE).parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(SETTINGS_FILE, json)?;
        info!("Saved settings to {}", SETTINGS_FILE);
        Ok(())
    }
}

/// Resource tracking the current init settings (for change detection)
#[derive(Resource)]
pub struct CurrentSettings {
    pub settings: InitSettings,
    pub dirty: bool,
    /// Tracks last Start button press time for double-click detection
    pub last_start_press: Option<f64>,
    /// Whether settings were just reset (for feedback)
    pub just_reset: bool,
}

/// Double-click window in seconds
pub const DOUBLE_CLICK_WINDOW: f64 = 0.5;

impl Default for CurrentSettings {
    fn default() -> Self {
        Self {
            settings: InitSettings::load(),
            dirty: false,
            last_start_press: None,
            just_reset: false,
        }
    }
}

impl CurrentSettings {
    /// Mark settings as changed (will be saved on next update)
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Save if dirty
    pub fn save_if_dirty(&mut self) {
        if self.dirty {
            if let Err(e) = self.settings.save() {
                warn!("Failed to save settings: {}", e);
            }
            self.dirty = false;
        }
    }

    /// Reset settings to defaults and save
    pub fn reset_to_defaults(&mut self) {
        self.settings = InitSettings::default();
        self.dirty = true;
        self.just_reset = true;
        info!("Settings reset to defaults");
    }

    /// Check for double-click and return true if detected
    pub fn check_double_click(&mut self, current_time: f64) -> bool {
        if let Some(last_time) = self.last_start_press
            && current_time - last_time < DOUBLE_CLICK_WINDOW
        {
            self.last_start_press = None;
            return true;
        }
        self.last_start_press = Some(current_time);
        false
    }
}

/// System to save settings periodically when changed
pub fn save_settings_system(mut settings: ResMut<CurrentSettings>) {
    settings.save_if_dirty();
}
