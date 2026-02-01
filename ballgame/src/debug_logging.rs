//! Debug logging configuration shared across binaries.

use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};

use crate::settings::InitSettings;

#[derive(Debug, Clone, Serialize, Deserialize, Resource)]
pub struct DebugLogConfig {
    pub enabled: bool,
}

impl Default for DebugLogConfig {
    fn default() -> Self {
        Self { enabled: false }
    }
}

impl DebugLogConfig {
    pub fn load() -> Self {
        let settings = InitSettings::load();
        Self {
            enabled: settings.debug_log_enabled,
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
}
