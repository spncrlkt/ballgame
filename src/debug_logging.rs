//! Debug logging configuration shared across binaries.

use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub const DEBUG_LOG_SETTINGS_FILE: &str = "config/debug_logging.json";

#[derive(Debug, Clone, Serialize, Deserialize, Resource)]
pub struct DebugLogConfig {
    pub enabled: bool,
    pub skip_reachability_heatmaps: bool,
}

impl Default for DebugLogConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            skip_reachability_heatmaps: false,
        }
    }
}

impl DebugLogConfig {
    pub fn load() -> Self {
        let path = Path::new(DEBUG_LOG_SETTINGS_FILE);
        if !path.exists() {
            return Self::default();
        }
        match fs::read_to_string(path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn enabled_from_args(args: &[String]) -> bool {
        args.iter().any(|arg| arg == "--debug-log")
    }

    pub fn load_with_args(args: &[String]) -> Self {
        let mut config = Self::load();
        if Self::enabled_from_args(args) {
            config.enabled = true;
        }
        config
    }

    pub fn apply_env(&self) {
        if self.skip_reachability_heatmaps {
            unsafe {
                std::env::set_var("BALLGAME_SKIP_REACHABILITY_HEATMAPS", "1");
            }
        }
    }
}
