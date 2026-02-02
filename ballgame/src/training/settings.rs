//! Training mode settings
//!
//! Loads from training_settings.json (local, gitignored) or falls back to
//! training_settings.template.json (tracked). CLI args override file settings.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use super::protocol::TrainingProtocol;

/// Path to local settings file (gitignored)
pub const SETTINGS_FILE: &str = "config/training_settings.json";
/// Path to template file (tracked in git)
pub const TEMPLATE_FILE: &str = "config/training_settings.template.json";

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

/// Game mode for training
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TrainingGameMode {
    /// 1v1 mode (2 players)
    #[default]
    #[serde(rename = "1v1")]
    OneVsOne,
    /// 2v2 mode (4 players)
    #[serde(rename = "2v2")]
    TwoVsTwo,
}

impl TrainingGameMode {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "1v1" | "onevone" => Some(Self::OneVsOne),
            "2v2" | "twovtwo" => Some(Self::TwoVsTwo),
            _ => None,
        }
    }

    pub fn character_count(&self) -> usize {
        match self {
            Self::OneVsOne => 2,
            Self::TwoVsTwo => 4,
        }
    }
}

impl std::fmt::Display for TrainingGameMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OneVsOne => write!(f, "1v1"),
            Self::TwoVsTwo => write!(f, "2v2"),
        }
    }
}

/// Training session settings
#[derive(Debug, Clone, Resource, Serialize, Deserialize)]
pub struct TrainingSettings {
    /// Training protocol (advanced-platform, pursuit, etc.)
    #[serde(default)]
    pub protocol: TrainingProtocol,
    /// Game mode (1v1 or 2v2)
    #[serde(default)]
    pub game_mode: TrainingGameMode,
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
    /// Optional allowlist file for offline training levels
    pub offline_levels_file: Option<String>,
    /// Custom AI profiles file path (default: config/ai_profiles.txt)
    pub profiles_file: Option<String>,
    /// Profile list file for multi-profile training (one profile per iteration)
    pub profile_list: Option<String>,

    /// RNG seed for determinism (null = random)
    pub seed: Option<u64>,
    /// Time limit per iteration in seconds (null = no limit, protocol may set default)
    pub time_limit_secs: Option<f32>,
    /// Timeout if no score within this many seconds (null = no timeout)
    pub first_point_timeout_secs: Option<f32>,

    /// Viewport preset index
    pub viewport_index: usize,
    /// Color palette index
    pub palette_index: usize,
    /// Ball visual style (None = random)
    pub ball_style: Option<String>,
    /// Drive mode (start with ball, regain on loss, first point wins)
    #[serde(default)]
    pub drive_mode: bool,
    /// Headless mode (no window, for automated simulation)
    #[serde(default)]
    pub headless: bool,

    // AI Client Support
    /// AI client ID to use instead of embedded profile (e.g., "ai-v1", "ai-v2")
    /// When set, the training will spawn an external AI client process.
    #[serde(default)]
    pub ai_client: Option<String>,
    /// Path to AI clients registry file (default: config/ai_clients.txt)
    #[serde(default)]
    pub clients_file: Option<String>,
    /// Connection timeout for AI clients in seconds
    #[serde(default = "default_client_timeout")]
    pub client_timeout_secs: u64,
}

fn default_client_timeout() -> u64 {
    30
}

impl Default for TrainingSettings {
    fn default() -> Self {
        Self {
            protocol: TrainingProtocol::default(),
            game_mode: TrainingGameMode::default(),
            mode: TrainingMode::Goal,
            iterations: 3,
            win_score: 1,
            ai_profile: "Balanced".to_string(),
            level: None,
            exclude_levels: vec!["Pit".to_string()],
            offline_levels_file: None,
            profiles_file: None,
            profile_list: None,
            seed: None,
            time_limit_secs: None,
            first_point_timeout_secs: None,
            viewport_index: 2,
            palette_index: 0,
            ball_style: None,
            drive_mode: false,
            headless: false,
            // AI Client support
            ai_client: None,
            clients_file: None,
            client_timeout_secs: default_client_timeout(),
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
                "--protocol" => {
                    if let Some(val) = args.get(i + 1) {
                        if let Some(protocol) = TrainingProtocol::from_str(val) {
                            self.protocol = protocol;
                            // Apply protocol defaults - fixed level always overrides
                            if let Some(level_name) = protocol.fixed_level() {
                                self.level = Some(LevelSelector::Name(level_name.to_string()));
                            }
                            if self.time_limit_secs.is_none() {
                                self.time_limit_secs = protocol.default_time_limit();
                            }
                        } else {
                            eprintln!(
                                "Warning: Unknown protocol '{}', using default (advanced-platform, pursuit, pursuit2)",
                                val
                            );
                        }
                        i += 1;
                    }
                }
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
                "--game-mode" => {
                    if let Some(val) = args.get(i + 1) {
                        if let Some(game_mode) = TrainingGameMode::from_str(val) {
                            self.game_mode = game_mode;
                        } else {
                            eprintln!("Warning: Unknown game mode '{}', using default (1v1, 2v2)", val);
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
                "--profiles-file" => {
                    if let Some(val) = args.get(i + 1) {
                        self.profiles_file = Some(val.clone());
                        i += 1;
                    }
                }
                "--profile-list" => {
                    if let Some(val) = args.get(i + 1) {
                        self.profile_list = Some(val.clone());
                        i += 1;
                    }
                }
                "--drive-mode" => {
                    self.drive_mode = true;
                    self.mode = TrainingMode::Goal;
                    self.iterations = 1;
                    self.win_score = 1;
                }
                "--headless" => {
                    self.headless = true;
                }
                // AI Client arguments
                "--ai-client" => {
                    if let Some(val) = args.get(i + 1) {
                        self.ai_client = Some(val.clone());
                        i += 1;
                    }
                }
                "--clients-file" => {
                    if let Some(val) = args.get(i + 1) {
                        self.clients_file = Some(val.clone());
                        i += 1;
                    }
                }
                "--client-timeout" => {
                    if let Some(val) = args.get(i + 1) {
                        if let Ok(n) = val.parse() {
                            self.client_timeout_secs = n;
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

PROTOCOLS:
    advanced-platform (default) - Full 1v1 games on random levels
    team-interaction            - Cooperative pass practice (aliases: team, catch)
    pursuit                     - Flat level chase test (verifies AI pursues player)
    pursuit2                    - Platform chase test (pursuit with center obstacle)
    reachability                - Solo level exploration for coverage mapping (LB to advance)
    auto-reachability           - Automated random walk/hop exploration (headless compatible)

MODES:
    goal  (default) - Each iteration ends after one goal, then reset
    game            - Each iteration is a full game to win_score points

OPTIONS:
    --protocol NAME            Training protocol (default: advanced-platform)
    --game-mode MODE           Game mode: 1v1 or 2v2 (default: 1v1)
    -m, --mode MODE            Training mode: goal or game (default: goal)
    -n, --iterations N         Number of iterations (default: 5)
    -w, --win-score N          Points to win in game mode (default: 5)
    -p, --profile NAME         AI opponent profile (default: Balanced)
    -l, --level N              Force specific level (default: random or protocol default)
    -s, --seed N               RNG seed for determinism (default: random)
    -t, --time-limit SECS      Time limit per iteration (default: none or protocol default)
    --first-point-timeout SECS End if no score within SECS (default: none)
    --viewport N               Viewport preset index (default: 2)
    --palette N                Color palette index (default: 0)
    --ball-style NAME          Ball visual style (default: random)
    --profiles-file PATH       AI profiles file (default: config/ai_profiles.txt)
    --profile-list PATH        File with profile names (one per line) for multi-profile training
    --debug-log                Enable debug sample logging to SQLite
    --headless                 Run without window (for auto-reachability)
    --ai-client ID             Use external AI client instead of embedded profile
    --clients-file PATH        AI clients registry file (default: config/ai_clients.txt)
    --client-timeout SECS      Connection timeout for AI clients (default: 30)
    -h, --help                 Show this help

AI CLIENTS:
    External AI clients connect via WebSocket. Use --ai-client to spawn an AI client
    process instead of using the embedded AI profile. The client must be registered
    in config/ai_clients.txt (or custom file via --clients-file).

    Example: cargo run --bin training -- --ai-client ai-v1

SETTINGS FILES:
    config/training_settings.json          Local settings (gitignored)
    config/training_settings.template.json Template with defaults (tracked)

    CLI arguments override file settings.

AI PROFILES:
    Balanced, Aggressive, Defensive, Sniper, Rusher,
    Turtle, Chaotic, Patient, Hunter, Goalie

EXAMPLES:
    cargo run --bin training -- --protocol pursuit
    cargo run --bin training -- --protocol pursuit --time-limit 60
    cargo run --bin training -- --protocol advanced-platform --iterations 3
    cargo run --bin training -- --profiles-file config/ai_profiles_champions.txt --profile-list tools/offline/champions_profiles.txt
"#
    );
}
