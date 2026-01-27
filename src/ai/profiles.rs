//! AI profiles - configurable AI personality parameters
//!
//! Each profile defines numeric values that affect AI behavior.
//! Loaded from config/ai_profiles.txt and hot-reloaded every 10 seconds.

use bevy::prelude::*;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};

/// Path to AI profiles file
pub const AI_PROFILES_FILE: &str = "config/ai_profiles.txt";

/// Generate a deterministic 16-char hex UUID from a name.
/// Used for backward compatibility when config files lack explicit IDs.
fn generate_uuid_from_name(name: &str) -> String {
    let mut hasher = DefaultHasher::new();
    name.hash(&mut hasher);
    let hash = hasher.finish();
    format!("{:016x}", hash)
}

/// AI behavior parameters loaded from config file
#[derive(Debug, Clone)]
pub struct AiProfile {
    /// 16-char hex UUID for stable identification
    pub id: String,
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
    /// Minimum shot quality (0.0-1.0) before AI will shoot (based on position heatmap)
    pub min_shot_quality: f32,
    /// How close AI tries to stay to ball carrier (pixels)
    /// Lower = more aggressive pressure, higher = zone defense
    pub pressure_distance: f32,
    /// How aggressively AI pursues ball carrier (0.0-1.0)
    /// 0.0 = passive zone, 1.0 = relentless chase
    pub aggression: f32,
    /// How well AI positions on shot line (0.0-1.0)
    /// Higher = better interception angles
    pub defensive_iq: f32,
    /// Reaction delay before attempting steal (seconds)
    /// Simulates human reaction time - lower = faster reflexes
    pub steal_reaction_time: f32,
    /// Maximum button presses per second
    /// Simulates human mashing speed - higher = faster mashing (typical human: 8-15)
    pub button_presses_per_sec: f32,
    /// Multiplier for seek utility calculation (0.3-2.0)
    /// Higher = more willing to seek better positions before shooting
    pub position_patience: f32,
    /// Minimum utility required to seek better position (0.05-0.20)
    /// Higher = shoots more quickly from current position
    pub seek_threshold: f32,
}

impl Default for AiProfile {
    fn default() -> Self {
        Self {
            id: generate_uuid_from_name("Balanced"),
            name: "Balanced".to_string(),
            position_tolerance: 30.0,
            shoot_range: 400.0,
            charge_min: 0.5,
            charge_max: 1.2,
            steal_range: 80.0,
            defense_offset: 400.0,
            min_shot_quality: 0.4, // Only shoot from acceptable positions
            pressure_distance: 120.0,
            aggression: 0.5,
            defensive_iq: 0.5,
            steal_reaction_time: 0.2, // ~200ms like typical human reaction
            button_presses_per_sec: 12.0, // ~12 presses/sec (typical human mashing)
            position_patience: 1.0,   // Moderate willingness to seek better positions
            seek_threshold: 0.10,     // Moderate threshold for seeking
        }
    }
}

/// Database of AI profiles loaded from file
#[derive(Resource, Clone)]
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

    /// Get profile by UUID
    pub fn get_by_id(&self, id: &str) -> Option<&AiProfile> {
        self.profiles.iter().find(|p| p.id == id)
    }

    /// Get number of profiles
    pub fn len(&self) -> usize {
        self.profiles.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.profiles.is_empty()
    }

    /// Get profiles as a slice
    pub fn profiles(&self) -> &[AiProfile] {
        &self.profiles
    }

    /// Get first profile as a fallback default
    pub fn default_profile(&self) -> &AiProfile {
        &self.profiles[0]
    }

    /// Find index of a profile by name (case-insensitive)
    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.profiles
            .iter()
            .position(|p| p.name.eq_ignore_ascii_case(name))
    }

    /// Get profile by name (case-insensitive)
    pub fn get_by_name(&self, name: &str) -> Option<&AiProfile> {
        self.profiles
            .iter()
            .find(|p| p.name.eq_ignore_ascii_case(name))
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
            let trimmed_name = name.trim();
            current = Some(AiProfile {
                id: generate_uuid_from_name(trimmed_name), // Auto-generate, may be overwritten
                name: trimmed_name.to_string(),
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
                "id" => {
                    profile.id = value.to_string();
                }
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
                "min_shot_quality" => {
                    if let Ok(v) = value.parse() {
                        profile.min_shot_quality = v;
                    }
                }
                "pressure_distance" => {
                    if let Ok(v) = value.parse() {
                        profile.pressure_distance = v;
                    }
                }
                "aggression" => {
                    if let Ok(v) = value.parse() {
                        profile.aggression = v;
                    }
                }
                "defensive_iq" => {
                    if let Ok(v) = value.parse() {
                        profile.defensive_iq = v;
                    }
                }
                "steal_reaction_time" => {
                    if let Ok(v) = value.parse() {
                        profile.steal_reaction_time = v;
                    }
                }
                "button_presses_per_sec" => {
                    if let Ok(v) = value.parse() {
                        profile.button_presses_per_sec = v;
                    }
                }
                "position_patience" => {
                    if let Ok(v) = value.parse() {
                        profile.position_patience = v;
                    }
                }
                "seek_threshold" => {
                    if let Ok(v) = value.parse() {
                        profile.seek_threshold = v;
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
