//! Level database - parsing and storage

use bevy::prelude::*;
use std::fs;

use crate::constants::*;

/// Platform definition in level data
#[derive(Clone, Debug)]
pub enum PlatformDef {
    Mirror { x: f32, y: f32, width: f32 }, // Spawns at -x and +x
    Center { y: f32, width: f32 },         // Spawns at x=0
}

/// Single level definition
#[derive(Clone, Debug)]
pub struct LevelData {
    pub name: String,
    pub basket_height: f32,
    pub basket_push_in: f32, // Distance from wall inner edge to basket center
    pub platforms: Vec<PlatformDef>,
    pub step_count: usize, // 0 = no steps, otherwise number of steps per corner
    pub corner_height: f32, // Total height of corner ramp
    pub corner_width: f32, // Total width of corner ramp
    pub step_push_in: f32, // Distance from wall to where stairs start (top step extends to wall)
    pub debug: bool,       // Debug mode: spawn all ball styles, AI idle
}

/// Database of all loaded levels
#[derive(Resource)]
pub struct LevelDatabase {
    pub levels: Vec<LevelData>,
}

impl Default for LevelDatabase {
    fn default() -> Self {
        Self { levels: Vec::new() }
    }
}

impl LevelDatabase {
    /// Load levels from file, returns default hardcoded levels on error
    pub fn load_from_file(path: &str) -> Self {
        match fs::read_to_string(path) {
            Ok(content) => Self::parse(&content),
            Err(e) => {
                warn!("Failed to load levels from {}: {}, using defaults", path, e);
                Self::default_levels()
            }
        }
    }

    /// Parse level data from string
    pub fn parse(content: &str) -> Self {
        let mut levels = Vec::new();
        let mut current_level: Option<LevelData> = None;

        for line in content.lines() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some(name) = line.strip_prefix("level:") {
                // Save previous level if exists
                if let Some(level) = current_level.take() {
                    levels.push(level);
                }
                // Start new level
                current_level = Some(LevelData {
                    name: name.trim().to_string(),
                    basket_height: 400.0,           // default
                    basket_push_in: BASKET_PUSH_IN, // default
                    platforms: Vec::new(),
                    step_count: CORNER_STEP_COUNT,           // default
                    corner_height: CORNER_STEP_TOTAL_HEIGHT, // default
                    corner_width: CORNER_STEP_TOTAL_WIDTH,   // default
                    step_push_in: STEP_PUSH_IN,              // default
                    debug: false,                            // default
                });
            } else if let Some(height_str) = line.strip_prefix("basket_height:") {
                if let Some(level) = &mut current_level {
                    if let Ok(height) = height_str.trim().parse::<f32>() {
                        level.basket_height = height;
                    }
                }
            } else if let Some(params) = line.strip_prefix("mirror:") {
                if let Some(level) = &mut current_level {
                    let parts: Vec<&str> = params.trim().split_whitespace().collect();
                    if parts.len() >= 3 {
                        if let (Ok(x), Ok(y), Ok(w)) = (
                            parts[0].parse::<f32>(),
                            parts[1].parse::<f32>(),
                            parts[2].parse::<f32>(),
                        ) {
                            level.platforms.push(PlatformDef::Mirror { x, y, width: w });
                        }
                    }
                }
            } else if let Some(params) = line.strip_prefix("center:") {
                if let Some(level) = &mut current_level {
                    let parts: Vec<&str> = params.split_whitespace().collect();
                    if parts.len() >= 2 {
                        if let (Ok(y), Ok(w)) = (parts[0].parse::<f32>(), parts[1].parse::<f32>()) {
                            level.platforms.push(PlatformDef::Center { y, width: w });
                        }
                    }
                }
            } else if let Some(count_str) = line.strip_prefix("steps:") {
                if let Some(level) = &mut current_level {
                    if let Ok(count) = count_str.trim().parse::<usize>() {
                        level.step_count = count;
                    }
                }
            } else if let Some(height_str) = line.strip_prefix("corner_height:") {
                if let Some(level) = &mut current_level {
                    if let Ok(height) = height_str.trim().parse::<f32>() {
                        level.corner_height = height;
                    }
                }
            } else if let Some(width_str) = line.strip_prefix("corner_width:") {
                if let Some(level) = &mut current_level {
                    if let Ok(width) = width_str.trim().parse::<f32>() {
                        level.corner_width = width;
                    }
                }
            } else if let Some(offset_str) = line.strip_prefix("basket_push_in:") {
                if let Some(level) = &mut current_level {
                    if let Ok(offset) = offset_str.trim().parse::<f32>() {
                        level.basket_push_in = offset;
                    }
                }
            } else if let Some(offset_str) = line.strip_prefix("step_push_in:") {
                if let Some(level) = &mut current_level {
                    if let Ok(offset) = offset_str.trim().parse::<f32>() {
                        level.step_push_in = offset;
                    }
                }
            } else if let Some(val) = line.strip_prefix("debug:") {
                if let Some(level) = &mut current_level {
                    level.debug = val.trim() == "true";
                }
            }
        }

        // Don't forget the last level
        if let Some(level) = current_level {
            levels.push(level);
        }

        if levels.is_empty() {
            warn!("No levels parsed, using defaults");
            return Self::default_levels();
        }

        info!("Loaded {} levels from file", levels.len());
        Self { levels }
    }

    /// Hardcoded fallback levels
    pub fn default_levels() -> Self {
        Self {
            levels: vec![
                LevelData {
                    name: "Simple".to_string(),
                    basket_height: 350.0,
                    basket_push_in: BASKET_PUSH_IN,
                    platforms: vec![PlatformDef::Mirror {
                        x: 400.0,
                        y: 150.0,
                        width: 200.0,
                    }],
                    step_count: CORNER_STEP_COUNT,
                    corner_height: CORNER_STEP_TOTAL_HEIGHT,
                    corner_width: CORNER_STEP_TOTAL_WIDTH,
                    step_push_in: STEP_PUSH_IN,
                    debug: false,
                },
                LevelData {
                    name: "Default".to_string(),
                    basket_height: 400.0,
                    basket_push_in: BASKET_PUSH_IN,
                    platforms: vec![
                        PlatformDef::Mirror {
                            x: 400.0,
                            y: 150.0,
                            width: 180.0,
                        },
                        PlatformDef::Center {
                            y: 280.0,
                            width: 200.0,
                        },
                    ],
                    step_count: CORNER_STEP_COUNT,
                    corner_height: CORNER_STEP_TOTAL_HEIGHT,
                    corner_width: CORNER_STEP_TOTAL_WIDTH,
                    step_push_in: STEP_PUSH_IN,
                    debug: false,
                },
            ],
        }
    }

    /// Get level by index
    pub fn get(&self, index: usize) -> Option<&LevelData> {
        self.levels.get(index)
    }

    /// Get number of levels
    pub fn len(&self) -> usize {
        self.levels.len()
    }

    /// Check if database is empty
    pub fn is_empty(&self) -> bool {
        self.levels.is_empty()
    }
}
