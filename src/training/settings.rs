//! Training mode settings
//!
//! Loads from training_settings.json (local, gitignored) or falls back to
//! training_settings.template.json (tracked). CLI args override file settings.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Path to local settings file (gitignored)
pub const SETTINGS_FILE: &str = "assets/training_settings.json";
/// Path to template file (tracked in git)
pub const TEMPLATE_FILE: &str = "assets/training_settings.template.json";

/// Training mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TrainingMode {
    /// Full games to win_score points
    #[default]
    Game,
    /// Single goals with reset after each
    Goal,
}

/// Level selector - accepts number or name
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LevelSelector {
    Number(u32),
    Name(String),
}

impl std::fmt::Display for LevelSelector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LevelSelector::Number(n) => write!(f, "{}", n),
            LevelSelector::Name(s) => write!(f, "{}", s),
        }
    }
}

/// Training session settings
#[derive(Debug, Clone, Resource, Serialize, Deserialize)]
pub struct TrainingSettings {
    /// Training mode (game or goal-by-goal)
    pub mode: TrainingMode,
    /// Number of iterations (games in Game mode, goals in Goal mode)
    pub iterations: u32,
    /// Points needed to win (Game mode only)
    pub win_score: u32,
    /// AI opponent profile name
    pub ai_profile: String,
    /// Specific level to use (null = randomize, number or name)
    pub level: Option<LevelSelector>,
    /// Levels to exclude from randomization
    pub exclude_levels: Vec<String>,

    /// RNG seed for determinism (null = random)
    pub seed: Option<u64>,
    /// Time limit per iteration in seconds (null = no limit)
    pub time_limit_secs: Option<f32>,
    /// Timeout if no score within this many seconds (null = no timeout)
    pub first_point_timeout_secs: Option<f32>,

    /// Viewport preset index
    pub viewport_index: usize,
    /// Color palette index
    pub palette_index: usize,
    /// Ball visual style (None = random)
    pub ball_style: Option<String>,
}

impl Default for TrainingSettings {
    fn default() -> Self {
        Self {
            mode: TrainingMode::Goal,
            iterations: 5,
            win_score: 5,
            ai_profile: "Balanced".to_string(),
            level: None,
            exclude_levels: vec!["Pit".to_string()],
            seed: None,
            time_limit_secs: None,
            first_point_timeout_secs: None,
            viewport_index: 2,
            palette_index: 0,
            ball_style: None,
        }
    }
}

impl TrainingSettings {
    /// Load settings with priority: CLI args > local file > template > defaults
    pub fn load() -> Self {
        // Try local file first
        let local_path = Path::new(SETTINGS_FILE);
        if local_path.exists() {
            if let Ok(content) = fs::read_to_string(local_path) {
                if let Ok(settings) = serde_json::from_str(&content) {
                    info!("Loaded training settings from {}", SETTINGS_FILE);
                    return settings;
                } else {
                    warn!("Failed to parse {}, trying template", SETTINGS_FILE);
                }
            }
        }

        // Try template file
        let template_path = Path::new(TEMPLATE_FILE);
        if template_path.exists() {
            if let Ok(content) = fs::read_to_string(template_path) {
                if let Ok(settings) = serde_json::from_str(&content) {
                    info!("Loaded training settings from {}", TEMPLATE_FILE);
                    return settings;
                } else {
                    warn!("Failed to parse {}, using defaults", TEMPLATE_FILE);
                }
            }
        }

        info!("No training settings found, using defaults");
        Self::default()
    }

    /// Save current settings to local file
    pub fn save(&self) -> Result<(), std::io::Error> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        if let Some(parent) = Path::new(SETTINGS_FILE).parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(SETTINGS_FILE, json)?;
        info!("Saved training settings to {}", SETTINGS_FILE);
        Ok(())
    }

    /// Apply CLI argument overrides
    pub fn apply_cli_overrides(&mut self, args: &[String]) {
        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "--mode" | "-m" => {
                    if let Some(val) = args.get(i + 1) {
                        match val.to_lowercase().as_str() {
                            "game" | "games" => self.mode = TrainingMode::Game,
                            "goal" | "goals" => self.mode = TrainingMode::Goal,
                            _ => {}
                        }
                        i += 1;
                    }
                }
                "--iterations" | "-n" => {
                    if let Some(val) = args.get(i + 1) {
                        if let Ok(n) = val.parse() {
                            self.iterations = n;
                        }
                        i += 1;
                    }
                }
                "--win-score" | "-w" => {
                    if let Some(val) = args.get(i + 1) {
                        if let Ok(n) = val.parse() {
                            self.win_score = n;
                        }
                        i += 1;
                    }
                }
                "--profile" | "-p" => {
                    if let Some(val) = args.get(i + 1) {
                        self.ai_profile = val.clone();
                        i += 1;
                    }
                }
                "--level" | "-l" => {
                    if let Some(val) = args.get(i + 1) {
                        if let Ok(n) = val.parse::<u32>() {
                            self.level = Some(LevelSelector::Number(n));
                        } else {
                            self.level = Some(LevelSelector::Name(val.clone()));
                        }
                        i += 1;
                    }
                }
                "--seed" | "-s" => {
                    if let Some(val) = args.get(i + 1) {
                        if let Ok(n) = val.parse() {
                            self.seed = Some(n);
                        }
                        i += 1;
                    }
                }
                "--time-limit" | "-t" => {
                    if let Some(val) = args.get(i + 1) {
                        if let Ok(n) = val.parse() {
                            self.time_limit_secs = Some(n);
                        }
                        i += 1;
                    }
                }
                "--first-point-timeout" => {
                    if let Some(val) = args.get(i + 1) {
                        if let Ok(n) = val.parse() {
                            self.first_point_timeout_secs = Some(n);
                        }
                        i += 1;
                    }
                }
                "--viewport" => {
                    if let Some(val) = args.get(i + 1) {
                        if let Ok(n) = val.parse() {
                            self.viewport_index = n;
                        }
                        i += 1;
                    }
                }
                "--palette" => {
                    if let Some(val) = args.get(i + 1) {
                        if let Ok(n) = val.parse() {
                            self.palette_index = n;
                        }
                        i += 1;
                    }
                }
                "--ball-style" => {
                    if let Some(val) = args.get(i + 1) {
                        if val.to_lowercase() == "random" {
                            self.ball_style = None;
                        } else {
                            self.ball_style = Some(val.clone());
                        }
                        i += 1;
                    }
                }
                "--help" | "-h" => {
                    print_help();
                    std::process::exit(0);
                }
                _ => {}
            }
            i += 1;
        }
    }

    /// Load settings and apply CLI overrides
    pub fn from_args() -> Self {
        let args: Vec<String> = std::env::args().collect();
        let mut settings = Self::load();
        settings.apply_cli_overrides(&args);
        settings
    }
}

fn print_help() {
    println!(
        r#"Training Mode - Play against AI and collect analysis data

USAGE:
    cargo run --bin training [OPTIONS]

MODES:
    goal  (default) - Each iteration ends after one goal, then reset
    game            - Each iteration is a full game to win_score points

OPTIONS:
    -m, --mode MODE            Training mode: goal or game (default: goal)
    -n, --iterations N         Number of iterations (default: 5)
    -w, --win-score N          Points to win in game mode (default: 5)
    -p, --profile NAME         AI opponent profile (default: Balanced)
    -l, --level N              Force specific level (default: random)
    -s, --seed N               RNG seed for determinism (default: random)
    -t, --time-limit SECS      Time limit per iteration (default: none)
    --first-point-timeout SECS End if no score within SECS (default: none)
    --viewport N               Viewport preset index (default: 2)
    --palette N                Color palette index (default: 0)
    --ball-style NAME          Ball visual style (default: random)
    -h, --help                 Show this help

SETTINGS FILES:
    assets/training_settings.json          Local settings (gitignored)
    assets/training_settings.template.json Template with defaults (tracked)

    CLI arguments override file settings.

AI PROFILES:
    Balanced, Aggressive, Defensive, Sniper, Rusher,
    Turtle, Chaotic, Patient, Hunter, Goalie
"#
    );
}
