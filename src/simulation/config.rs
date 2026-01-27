//! Simulation configuration

use serde::{Deserialize, Serialize};

/// Simulation mode
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum SimMode {
    /// Run a single match
    #[default]
    Single,
    /// Run multiple matches with same config
    MultiMatch { count: u32 },
    /// Run all profile combinations
    Tournament { matches_per_pair: u32 },
    /// Test one profile across all levels
    LevelSweep { matches_per_level: u32 },
    /// Compare to baseline metrics
    Regression,
    /// Shot accuracy test - fire shots from fixed positions
    ShotTest { shots_per_position: u32 },
    /// Ghost trial - play back recorded inputs against AI
    GhostTrial {
        /// Path to ghost trial file or directory
        path: String,
    },
}

/// Configuration for a simulation run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimConfig {
    /// Simulation mode
    pub mode: SimMode,
    /// Level index (1-based), None = random per match (excludes debug levels and Pit)
    pub level: Option<u32>,
    /// Left player AI profile name
    pub left_profile: String,
    /// Right player AI profile name
    pub right_profile: String,
    /// Match duration limit in seconds
    pub duration_limit: f32,
    /// Score limit (first to reach wins, 0 = no limit)
    pub score_limit: u32,
    /// RNG seed for reproducibility (None = random)
    pub seed: Option<u64>,
    /// Stalemate timeout - end match if no score for this many seconds
    pub stalemate_timeout: f32,
    /// Output file path (None = stdout)
    pub output_file: Option<String>,
    /// Suppress progress output
    pub quiet: bool,
    /// Number of parallel threads (0 = sequential, N = N threads)
    pub parallel: usize,
    /// Path to SQLite database for storing results
    pub db_path: Option<String>,
    /// Profiles to include in tournament (empty = all profiles)
    pub profiles: Vec<String>,
    /// Levels to use for matches (empty = all non-debug levels)
    pub levels: Vec<u32>,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            mode: SimMode::Single,
            level: None, // Random per match, excludes debug levels and Pit
            left_profile: "Balanced".to_string(),
            right_profile: "Balanced".to_string(),
            duration_limit: 60.0,
            score_limit: 0,
            seed: None,
            stalemate_timeout: 30.0,
            output_file: None,
            quiet: false,
            parallel: 0, // Sequential by default
            db_path: None,
            profiles: Vec::new(), // Empty = all profiles
            levels: Vec::new(),   // Empty = all non-debug levels
        }
    }
}

/// Template simulation settings (checked into git)
pub const SIM_SETTINGS_TEMPLATE: &str = "config/simulation_settings.template.json";
/// Local simulation settings (gitignored, user's custom settings)
pub const SIM_SETTINGS_FILE: &str = "config/simulation_settings.json";

impl SimConfig {
    /// Load configuration from a JSON settings file
    pub fn from_file(path: &str) -> Result<Self, String> {
        let contents =
            std::fs::read_to_string(path).map_err(|e| format!("Failed to read {}: {}", path, e))?;
        serde_json::from_str(&contents).map_err(|e| format!("Failed to parse {}: {}", path, e))
    }

    /// Load configuration from default config files
    /// Priority: local settings > template settings > built-in defaults
    pub fn from_config_files() -> Self {
        // Try local settings first
        if let Ok(config) = Self::from_file(SIM_SETTINGS_FILE) {
            return config;
        }
        // Fall back to template settings
        if let Ok(config) = Self::from_file(SIM_SETTINGS_TEMPLATE) {
            return config;
        }
        // Fall back to built-in defaults
        Self::default()
    }

    /// Parse configuration from command line arguments
    pub fn from_args() -> Self {
        let args: Vec<String> = std::env::args().collect();

        // Start with config files as base
        let mut config = Self::from_config_files();

        // Check for explicit settings file override
        let mut i = 1;
        while i < args.len() {
            if args[i] == "--settings" && i + 1 < args.len() {
                match Self::from_file(&args[i + 1]) {
                    Ok(loaded) => config = loaded,
                    Err(e) => {
                        eprintln!("Warning: {}", e);
                    }
                }
                break;
            }
            i += 1;
        }

        // Then apply command line overrides
        i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "--settings" => {
                    // Already handled above
                    i += 1;
                }
                "--level" => {
                    if i + 1 < args.len() {
                        config.level = args[i + 1].parse().ok();
                        i += 1;
                    }
                }
                "--levels" => {
                    if i + 1 < args.len() {
                        // Parse comma-separated list of levels
                        config.levels = args[i + 1]
                            .split(',')
                            .filter_map(|s| s.trim().parse().ok())
                            .collect();
                        i += 1;
                    }
                }
                "--profiles" => {
                    if i + 1 < args.len() {
                        // Parse comma-separated list of profiles
                        config.profiles = args[i + 1]
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                        i += 1;
                    }
                }
                "--left" => {
                    if i + 1 < args.len() {
                        config.left_profile = args[i + 1].clone();
                        i += 1;
                    }
                }
                "--right" => {
                    if i + 1 < args.len() {
                        config.right_profile = args[i + 1].clone();
                        i += 1;
                    }
                }
                "--duration" => {
                    if i + 1 < args.len() {
                        config.duration_limit = args[i + 1].parse().unwrap_or(60.0);
                        i += 1;
                    }
                }
                "--score-limit" => {
                    if i + 1 < args.len() {
                        config.score_limit = args[i + 1].parse().unwrap_or(0);
                        i += 1;
                    }
                }
                "--matches" => {
                    if i + 1 < args.len() {
                        let count = args[i + 1].parse().unwrap_or(1);
                        config.mode = SimMode::MultiMatch { count };
                        i += 1;
                    }
                }
                "--tournament" => {
                    let matches = if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                        i += 1;
                        args[i].parse().unwrap_or(5)
                    } else {
                        5
                    };
                    config.mode = SimMode::Tournament {
                        matches_per_pair: matches,
                    };
                }
                "--level-sweep" => {
                    let matches = if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                        i += 1;
                        args[i].parse().unwrap_or(3)
                    } else {
                        3
                    };
                    config.mode = SimMode::LevelSweep {
                        matches_per_level: matches,
                    };
                }
                "--regression" => {
                    config.mode = SimMode::Regression;
                }
                "--shot-test" => {
                    let shots = if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                        i += 1;
                        args[i].parse().unwrap_or(30)
                    } else {
                        30
                    };
                    config.mode = SimMode::ShotTest {
                        shots_per_position: shots,
                    };
                }
                "--ghost" => {
                    if i + 1 < args.len() {
                        config.mode = SimMode::GhostTrial {
                            path: args[i + 1].clone(),
                        };
                        i += 1;
                    }
                }
                "--seed" => {
                    if i + 1 < args.len() {
                        config.seed = args[i + 1].parse().ok();
                        i += 1;
                    }
                }
                "--output" => {
                    if i + 1 < args.len() {
                        config.output_file = Some(args[i + 1].clone());
                        i += 1;
                    }
                }
                "--quiet" | "-q" => {
                    config.quiet = true;
                }
                "--parallel" => {
                    if i + 1 < args.len() {
                        config.parallel = args[i + 1].parse().unwrap_or(0);
                        i += 1;
                    }
                }
                "--db" => {
                    if i + 1 < args.len() {
                        config.db_path = Some(args[i + 1].clone());
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

        config
    }
}

fn print_help() {
    println!(
        r#"AI Simulation Tool - Headless game simulation for AI testing

USAGE:
    cargo run --bin simulate -- [OPTIONS]

OPTIONS:
    --settings <FILE>   Load settings from JSON file (CLI args override file settings)
    --level <N>         Level number (1-12, default: random per match)
    --levels <LIST>     Comma-separated level numbers to use (e.g., "3,4,7,11")
    --profiles <LIST>   Comma-separated profile names for tournament (e.g., "v4_RP_Gamma,v4_Elite_A")
    --left <PROFILE>    Left player AI profile (default: Balanced)
    --right <PROFILE>   Right player AI profile (default: Balanced)
    --duration <SECS>   Match duration limit in seconds (default: 60)
    --score-limit <N>   End match when a player reaches N points (default: no limit)
    --matches <N>       Run N matches with same config
    --tournament [N]    Run all profile combinations (N matches each, default: 5)
    --level-sweep [N]   Test profile across all levels (N matches each, default: 3)
    --regression        Compare to baseline metrics
    --shot-test [N]     Shot accuracy test (N shots per position, default: 30)
    --ghost <PATH>      Run ghost trials from file or directory
    --seed <N>          RNG seed for reproducibility
    --output <FILE>     Output JSON to file (default: stdout)
    --quiet, -q         Suppress progress output
    --parallel <N>      Run simulations in parallel with N threads
    --db <FILE>         Store results in SQLite database
    --help, -h          Show this help

EXAMPLES:
    # Single match on level 3
    cargo run --bin simulate -- --level 3 --left Balanced --right Aggressive

    # Tournament with specific profiles and levels
    cargo run --bin simulate -- --tournament 5 --profiles "v4_RP_Gamma,v4_Elite_A,v4_RA_Core" --levels "3,4,7,11" --db results.db

    # Load settings from file
    cargo run --bin simulate -- --settings sim_settings.json --tournament 3

    # Test Sniper profile across all levels
    cargo run --bin simulate -- --level-sweep 5 --left Sniper

    # Run ghost trials against AI
    cargo run --bin simulate -- --ghost training_logs/session_xxx/ghost_trials/ --right Aggressive

    # Run matches with SQLite logging
    cargo run --bin simulate -- --tournament 5 --db training.db

PROFILES:
    Balanced, Aggressive, Defensive, Sniper, Rusher, Turtle, Chaotic, Patient, Hunter, Goalie
    (Use --profiles to filter which profiles participate in tournament)

SETTINGS FILE FORMAT (JSON):
    {{
      "profiles": ["v4_RP_Gamma", "v4_Elite_A", "v4_RA_Core"],
      "levels": [3, 4, 5, 6, 7, 8, 11, 14, 15],
      "parallel": 8,
      "duration_limit": 60.0
    }}
"#
    );
}
