//! AI profiles - configurable AI personality parameters
//!
//! Each profile defines numeric values that affect AI behavior.
//! Loaded from assets/ai_profiles.txt and hot-reloaded every 10 seconds.

use bevy::prelude::*;
use std::fs;

/// Path to AI profiles file
pub const AI_PROFILES_FILE: &str = "assets/ai_profiles.txt";

/// AI behavior parameters loaded from config file
#[derive(Debug, Clone)]
pub struct AiProfile {
    /// Profile name for display
    pub name: String,
    /// How close AI needs to be to target before stopping (pixels)
    pub position_tolerance: f32,
    /// Distance from basket at which AI will shoot (pixels)
    pub shoot_range: f32,
    /// Minimum charge time before releasing shot (seconds)
    pub charge_min: f32,
    /// Maximum charge time for shots (seconds)
    pub charge_max: f32,
    /// Distance at which AI will attempt steal (pixels)
    pub steal_range: f32,
    /// How far from own basket AI defends (pixels)
    pub defense_offset: f32,
}

impl Default for AiProfile {
    fn default() -> Self {
        Self {
            name: "Balanced".to_string(),
            position_tolerance: 30.0,
            shoot_range: 400.0,
            charge_min: 0.5,
            charge_max: 1.2,
            steal_range: 80.0,
            defense_offset: 400.0,
        }
    }
}

/// Database of AI profiles loaded from file
#[derive(Resource)]
pub struct AiProfileDatabase {
    profiles: Vec<AiProfile>,
}

impl Default for AiProfileDatabase {
    fn default() -> Self {
        Self::load_from_file(AI_PROFILES_FILE)
    }
}

impl AiProfileDatabase {
    /// Load profiles from file, or return default if file doesn't exist
    pub fn load_from_file(path: &str) -> Self {
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                warn!("Could not read AI profiles file: {}, using defaults", e);
                return Self {
                    profiles: vec![AiProfile::default()],
                };
            }
        };

        let profiles = parse_profiles(&content);
        if profiles.is_empty() {
            warn!("No profiles parsed from {}, using defaults", path);
            return Self {
                profiles: vec![AiProfile::default()],
            };
        }

        info!("Loaded {} AI profiles from {}", profiles.len(), path);
        Self { profiles }
    }

    /// Get profile by index, wrapping around if out of bounds
    pub fn get(&self, index: usize) -> &AiProfile {
        &self.profiles[index % self.profiles.len()]
    }

    /// Get number of profiles
    pub fn len(&self) -> usize {
        self.profiles.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.profiles.is_empty()
    }
}

/// Parse profiles from file content
fn parse_profiles(content: &str) -> Vec<AiProfile> {
    let mut profiles = Vec::new();
    let mut current: Option<AiProfile> = None;

    for line in content.lines() {
        let line = line.trim();

        // Skip comments and empty lines
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // New profile starts
        if let Some(name) = line.strip_prefix("profile:") {
            // Save previous profile if any
            if let Some(p) = current.take() {
                profiles.push(p);
            }
            current = Some(AiProfile {
                name: name.trim().to_string(),
                ..default()
            });
            continue;
        }

        // Parse key: value pairs
        let Some(profile) = current.as_mut() else {
            continue;
        };

        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim();

            match key {
                "position_tolerance" => {
                    if let Ok(v) = value.parse() {
                        profile.position_tolerance = v;
                    }
                }
                "shoot_range" => {
                    if let Ok(v) = value.parse() {
                        profile.shoot_range = v;
                    }
                }
                "charge_min" => {
                    if let Ok(v) = value.parse() {
                        profile.charge_min = v;
                    }
                }
                "charge_max" => {
                    if let Ok(v) = value.parse() {
                        profile.charge_max = v;
                    }
                }
                "steal_range" => {
                    if let Ok(v) = value.parse() {
                        profile.steal_range = v;
                    }
                }
                "defense_offset" => {
                    if let Ok(v) = value.parse() {
                        profile.defense_offset = v;
                    }
                }
                _ => {}
            }
        }
    }

    // Don't forget the last profile
    if let Some(p) = current {
        profiles.push(p);
    }

    profiles
}
