//! Simulation configuration

use serde::{Deserialize, Serialize};

use super::bracket::BracketSeedingConfig;
use super::participant::{AiParticipant, MatchParticipants};
use crate::db_paths;

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
    /// Multi-hop platform reachability test
    /// Tests that NavGraph correctly chains edges for platforms only reachable
    /// via intermediate hops from the floor
    MultihopTest,
    /// Random point reachability validation
    /// Samples random test points from exploration data and verifies that
    /// NavGraph can path to positions that players have actually reached
    ReachabilityTest {
        /// Number of random samples to test per level
        samples: u32,
        /// Path to SQLite database with exploration data
        db_path: String,
    },
    /// Double elimination bracket tournament
    Bracket {
        /// Number of entrants (must be power of 2: 8, 16, 32, 64, 128)
        entrants: u32,
        /// Games per match (e.g., 3 for best-of-3)
        best_of: u32,
        /// Points to win a game (first to N)
        game_score_limit: u32,
        /// Seeding method
        seeding: BracketSeedingConfig,
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
    /// Estimate runtime based on prior sessions and exit
    #[serde(default)]
    pub est_run_time: bool,
    /// Wall-clock timeout in seconds for a tournament run (None = no limit)
    #[serde(default)]
    pub run_timeout_secs: Option<f32>,
    /// Path to SQLite database for storing results
    pub db_path: Option<String>,
    /// Profiles to include in tournament (empty = all profiles)
    pub profiles: Vec<String>,
    /// Levels to use for matches (empty = all non-debug levels)
    pub levels: Vec<u32>,
    /// Enable debug sample logging
    #[serde(default)]
    pub debug_log: bool,
    /// Path to custom AI profiles file (None = use default config/ai_profiles.txt)
    #[serde(default)]
    pub profiles_file: Option<String>,

    // ============================================================
    // AI Client Support (2v2 format)
    // ============================================================

    /// All 4 participants [L0, L1, R0, R1] - overrides profiles and teams when set
    #[serde(default)]
    pub participants: Option<MatchParticipants>,

    /// Left team participants [L0, L1] - overrides left_profile when set
    #[serde(default)]
    pub left_team: Option<[AiParticipant; 2]>,

    /// Right team participants [R0, R1] - overrides right_profile when set
    #[serde(default)]
    pub right_team: Option<[AiParticipant; 2]>,

    /// Client IDs to include in tournament (for client tournaments)
    #[serde(default)]
    pub clients: Vec<String>,

    /// Path to AI clients registry file (None = use default config/ai_clients.txt)
    #[serde(default)]
    pub clients_file: Option<String>,

    /// Connection timeout for AI clients (seconds)
    #[serde(default = "default_client_timeout")]
    pub client_timeout_secs: u64,
}

/// Default client connection timeout
fn default_client_timeout() -> u64 {
    30
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
            est_run_time: false,
            run_timeout_secs: None,
            db_path: None,
            profiles: Vec::new(), // Empty = all profiles
            levels: Vec::new(),   // Empty = all non-debug levels
            debug_log: false,
            profiles_file: None, // Use default config/ai_profiles.txt
            // AI Client support
            participants: None,
            left_team: None,
            right_team: None,
            clients: Vec::new(),
            clients_file: None,
            client_timeout_secs: default_client_timeout(),
        }
    }
}

/// Template simulation settings (checked into git)
pub const SIM_SETTINGS_TEMPLATE: &str = "config/simulation_settings.template.json";
/// Local simulation settings (gitignored, user's custom settings)
pub const SIM_SETTINGS_FILE: &str = "config/simulation_settings.json";

/// Parse a team argument like "ai-v1,ai-v1" or "Balanced,Aggressive"
///
/// Participants starting with "ai-" are treated as clients, others as profiles.
fn parse_team_arg(arg: &str) -> [AiParticipant; 2] {
    let parts: Vec<&str> = arg.split(',').map(|s| s.trim()).collect();

    let parse_participant = |s: &str| -> AiParticipant {
        if s.starts_with("ai-") || s.contains('/') {
            // Treat as client ID (ai-v1, ai-v2, or path-like identifiers)
            AiParticipant::client(s)
        } else {
            // Treat as profile name
            AiParticipant::profile(s)
        }
    };

    match parts.len() {
        0 => [
            AiParticipant::profile("Balanced"),
            AiParticipant::profile("Balanced"),
        ],
        1 => {
            // Single value: use for both slots
            let p = parse_participant(parts[0]);
            [p.clone(), p]
        }
        _ => {
            // Two values: use for primary and secondary
            [parse_participant(parts[0]), parse_participant(parts[1])]
        }
    }
}

impl SimConfig {
    /// Resolve participant configuration to concrete MatchParticipants
    ///
    /// Priority:
    /// 1. `participants` field (explicit all-4 config)
    /// 2. `left_team`/`right_team` fields (team-based config)
    /// 3. `left_profile`/`right_profile` fields (backward compat, duplicated for 2v2)
    pub fn resolve_participants(&self) -> MatchParticipants {
        use super::orchestrator::resolve_participants;
        resolve_participants(
            self.participants.as_ref(),
            self.left_team.as_ref(),
            self.right_team.as_ref(),
            &self.left_profile,
            &self.right_profile,
        )
    }

    /// Check if this configuration uses any AI clients (requires orchestrator)
    pub fn uses_clients(&self) -> bool {
        self.resolve_participants().has_clients()
    }

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
                "--est-run-time" => {
                    config.est_run_time = true;
                }
                "--run-timeout" => {
                    if i + 1 < args.len() {
                        config.run_timeout_secs = args[i + 1].parse().ok();
                        i += 1;
                    }
                }
                "--debug-log" => {
                    config.debug_log = true;
                }
                "--score-limit" => {
                    if i + 1 < args.len() {
                        let score_limit = args[i + 1].parse().unwrap_or(0);
                        config.score_limit = score_limit;
                        // Also update bracket mode game_score_limit if active
                        if let SimMode::Bracket {
                            entrants,
                            best_of,
                            seeding,
                            ..
                        } = &config.mode
                        {
                            config.mode = SimMode::Bracket {
                                entrants: *entrants,
                                best_of: *best_of,
                                game_score_limit: score_limit,
                                seeding: seeding.clone(),
                            };
                        }
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
                "--multihop-test" => {
                    config.mode = SimMode::MultihopTest;
                }
                "--reachability-test" => {
                    // Default samples and db_path, can be overridden with --samples and --db
                    let samples = 50; // Default, will be overridden if --samples specified
                    let db_path = config.db_path.clone().unwrap_or_else(|| {
                        // Try to find the most recent training database
                        db_paths::find_latest(db_paths::DbType::Training)
                            .unwrap_or_else(|| db_paths::default_path(db_paths::DbType::Training))
                    });
                    config.mode = SimMode::ReachabilityTest { samples, db_path };
                }
                "--bracket" => {
                    let entrants = if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                        i += 1;
                        args[i].parse().unwrap_or(64)
                    } else {
                        64
                    };
                    // Use existing score_limit if set, otherwise default to 5
                    let game_score_limit = if config.score_limit > 0 {
                        config.score_limit
                    } else {
                        5
                    };
                    config.mode = SimMode::Bracket {
                        entrants,
                        best_of: 3,
                        game_score_limit,
                        seeding: BracketSeedingConfig::Random,
                    };
                }
                "--best-of" => {
                    if i + 1 < args.len() {
                        let best_of = args[i + 1].parse().unwrap_or(3);
                        if let SimMode::Bracket {
                            entrants,
                            game_score_limit,
                            seeding,
                            ..
                        } = &config.mode
                        {
                            config.mode = SimMode::Bracket {
                                entrants: *entrants,
                                best_of,
                                game_score_limit: *game_score_limit,
                                seeding: seeding.clone(),
                            };
                        }
                        i += 1;
                    }
                }
                "--warmup-seeding" => {
                    // Parse: --warmup-seeding [PROFILE] [GAMES]
                    let baseline = if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                        i += 1;
                        args[i].clone()
                    } else {
                        "Balanced".to_string()
                    };
                    let games = if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                        i += 1;
                        args[i].parse().unwrap_or(5)
                    } else {
                        5
                    };
                    if let SimMode::Bracket {
                        entrants,
                        best_of,
                        game_score_limit,
                        ..
                    } = &config.mode
                    {
                        config.mode = SimMode::Bracket {
                            entrants: *entrants,
                            best_of: *best_of,
                            game_score_limit: *game_score_limit,
                            seeding: BracketSeedingConfig::Warmup {
                                baseline_profile: baseline,
                                games_per_entrant: games,
                            },
                        };
                    }
                }
                "--samples" => {
                    if i + 1 < args.len() {
                        let samples = args[i + 1].parse().unwrap_or(50);
                        // Update samples if we're in ReachabilityTest mode
                        if let SimMode::ReachabilityTest { db_path, .. } = &config.mode {
                            config.mode = SimMode::ReachabilityTest {
                                samples,
                                db_path: db_path.clone(),
                            };
                        }
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
                "--profiles-file" => {
                    if i + 1 < args.len() {
                        config.profiles_file = Some(args[i + 1].clone());
                        i += 1;
                    }
                }
                "--db" => {
                    if i + 1 < args.len() {
                        let new_db_path = args[i + 1].clone();
                        config.db_path = Some(new_db_path.clone());
                        // Also update ReachabilityTest mode if active
                        if let SimMode::ReachabilityTest { samples, .. } = &config.mode {
                            config.mode = SimMode::ReachabilityTest {
                                samples: *samples,
                                db_path: new_db_path,
                            };
                        }
                        i += 1;
                    }
                }
                // AI Client arguments
                "--left-team" => {
                    if i + 1 < args.len() {
                        let team = parse_team_arg(&args[i + 1]);
                        config.left_team = Some(team);
                        i += 1;
                    }
                }
                "--right-team" => {
                    if i + 1 < args.len() {
                        let team = parse_team_arg(&args[i + 1]);
                        config.right_team = Some(team);
                        i += 1;
                    }
                }
                "--clients" => {
                    if i + 1 < args.len() {
                        config.clients = args[i + 1]
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                        i += 1;
                    }
                }
                "--clients-file" => {
                    if i + 1 < args.len() {
                        config.clients_file = Some(args[i + 1].clone());
                        i += 1;
                    }
                }
                "--client-timeout" => {
                    if i + 1 < args.len() {
                        config.client_timeout_secs = args[i + 1].parse().unwrap_or(30);
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
    --profiles-file <FILE>  Load profiles from custom file (default: config/ai_profiles.txt)
    --left <PROFILE>    Left player AI profile (default: Balanced)
    --right <PROFILE>   Right player AI profile (default: Balanced)
    --duration <SECS>   Match duration limit in seconds (default: 60)
    --est-run-time      Estimate runtime from prior sessions and exit
    --run-timeout <SECS> Wall-clock timeout for tournament run (default: 600)
    --score-limit <N>   End match when a player reaches N points (default: no limit)
    --matches <N>       Run N matches with same config
    --tournament [N]    Run all profile combinations (N matches each, default: 5)
    --level-sweep [N]   Test profile across all levels (N matches each, default: 3)
    --regression        Compare to baseline metrics
    --shot-test [N]     Shot accuracy test (N shots per position, default: 30)
    --ghost <PATH>      Run ghost trials from file or directory
    --multihop-test     Test NavGraph multi-hop platform reachability
    --reachability-test Validate NavGraph against exploration data
    --samples <N>       Number of samples for reachability test (default: 50)
    --bracket [N]       Run double elimination bracket with N entrants (default: 64)
    --best-of <N>       Games per bracket match (default: 3, use with --bracket)
    --warmup-seeding [PROFILE] [GAMES]  Seed bracket by win rate vs baseline
    --seed <N>          RNG seed for reproducibility
    --output <FILE>     Output JSON to file (default: stdout)
    --quiet, -q         Suppress progress output
    --parallel <N>      Run simulations in parallel with N threads
    --db <FILE>         Store results in SQLite database
    --debug-log         Enable debug sample logging (if supported)
    --help, -h          Show this help

AI CLIENT OPTIONS (2v2 format):
    --left-team <P,P>   Two participants for left team (e.g., "ai-v1,ai-v1" or "Balanced,Balanced")
    --right-team <P,P>  Two participants for right team (e.g., "ai-v2,ai-v2")
    --clients <LIST>    Comma-separated client IDs for tournament (e.g., "ai-v1,ai-v2")
    --clients-file <FILE>  Custom AI clients registry (default: config/ai_clients.txt)
    --client-timeout <SECS>  Connection timeout for AI clients (default: 30)

    Participant format: IDs starting with "ai-" are external clients, others are profiles.

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

    # 64-player double elimination bracket with warmup seeding
    cargo run --bin simulate -- --bracket 64 --parallel 16 --warmup-seeding Balanced 5

    # BO5 bracket with specific profiles
    cargo run --bin simulate -- --bracket 32 --best-of 5 --profiles "Profile1,Profile2,..."

    # 2v2 match: AI v1 team vs AI v2 team
    cargo run --bin simulate -- --left-team ai-v1,ai-v1 --right-team ai-v2,ai-v2

    # Mixed team match: profiles on left, clients on right
    cargo run --bin simulate -- --left-team Balanced,Aggressive --right-team ai-v1,ai-v2

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
