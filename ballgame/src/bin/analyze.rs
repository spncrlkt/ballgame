//! Analytics Tool - Analyze simulation results and generate reports
//!
//! Reads SQLite event logs from simulation runs, computes aggregate metrics,
//! generates profile leaderboards, and suggests parameter changes.
//!
//! Usage:
//!   cargo run --bin analyze -- training.db
//!   cargo run --bin analyze -- training.db --targets assets/tuning_targets.toml
//!   cargo run --bin analyze -- training.db --update-defaults

use std::path::PathBuf;

use rusqlite::Connection;

use ballgame::analytics::{
    AggregateMetrics, AnalysisQuery, AnalysisRequest, AnalysisRequestFile, Leaderboard,
    ParameterSuggestion, TrainingDebugReport, TuningTargets, default_targets, format_suggestions,
    format_update_report, generate_suggestions, load_targets, parse_all_matches_from_db,
    run_bracket_analysis, run_event_audit, run_focused_analysis, run_request,
    run_team_interaction_analysis, run_training_debug_analysis, update_default_profiles,
};

fn main() {
    let config = AnalyzeConfig::from_args();

    if config.show_help {
        print_help();
        return;
    }

    if config.request_list {
        let requests =
            AnalysisRequestFile::load(&config.requests_file).unwrap_or(AnalysisRequestFile {
                requests: Vec::new(),
            });
        if requests.requests.is_empty() {
            println!(
                "No analysis requests found in {}",
                config.requests_file.display()
            );
        } else {
            println!("Analysis requests in {}:", config.requests_file.display());
            for req in requests.requests {
                if let Some(desc) = &req.description {
                    println!("- {}: {}", req.name, desc);
                } else {
                    println!("- {}", req.name);
                }
            }
        }
        return;
    }

    if let Some(name) = &config.request_add {
        let sql = match &config.request_sql {
            Some(sql) => sql.clone(),
            None => {
                eprintln!("--request-add requires --request-sql");
                std::process::exit(1);
            }
        };
        let mut requests =
            AnalysisRequestFile::load(&config.requests_file).unwrap_or(AnalysisRequestFile {
                requests: Vec::new(),
            });
        let query_name = config
            .request_query_name
            .clone()
            .unwrap_or_else(|| "query".to_string());
        let request = AnalysisRequest {
            name: name.clone(),
            description: config.request_desc.clone(),
            db_path: config.request_db.as_ref().map(|p| p.display().to_string()),
            db_label: config.request_db_label.clone(),
            queries: vec![AnalysisQuery {
                name: query_name,
                sql,
                notes: None,
            }],
        };
        requests.add_request(request);
        if let Some(parent) = config.requests_file.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        if let Err(e) = requests.save(&config.requests_file) {
            eprintln!("Failed to save requests file: {}", e);
            std::process::exit(1);
        }
        println!(
            "Saved analysis request '{}' to {}",
            name,
            config.requests_file.display()
        );
        return;
    }

    if let Some(name) = &config.request_name {
        let requests =
            AnalysisRequestFile::load(&config.requests_file).unwrap_or(AnalysisRequestFile {
                requests: Vec::new(),
            });
        let request = requests
            .requests
            .iter()
            .find(|req| req.name == *name)
            .unwrap_or_else(|| {
                eprintln!(
                    "Request '{}' not found in {}",
                    name,
                    config.requests_file.display()
                );
                std::process::exit(1);
            });
        let report = run_request(request, config.request_db.as_deref())
            .unwrap_or_else(|e| {
                eprintln!("Failed to run request '{}': {}", name, e);
                std::process::exit(1);
            })
            .to_markdown();
        let output_path = config
            .request_output
            .clone()
            .unwrap_or_else(|| default_request_output_path(name));
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        if let Err(e) = std::fs::write(&output_path, &report) {
            eprintln!("Failed to write request report: {}", e);
            std::process::exit(1);
        }
        println!("Request report written to {}", output_path.display());
        return;
    }

    if let Some(db_path) = &config.training_db {
        let output_dir = config
            .training_output
            .clone()
            .unwrap_or_else(|| default_training_output_dir(db_path));
        if let Err(e) = std::fs::create_dir_all(&output_dir) {
            eprintln!(
                "Failed to create training output directory {}: {}",
                output_dir.display(),
                e
            );
            std::process::exit(1);
        }
        let report = run_training_debug_analysis(db_path, &output_dir)
            .map_err(|e| format!("{e}"))
            .unwrap_or_else(|e| {
                eprintln!("Failed to run training debug analysis: {}", e);
                std::process::exit(1);
            });
        let report_path = output_dir.join(default_training_report_name(&report));
        if let Err(e) = std::fs::write(&report_path, report.to_markdown()) {
            eprintln!("Failed to write training report: {}", e);
            std::process::exit(1);
        }
        println!(
            "Training debug analysis written to {}",
            report_path.display()
        );
        return;
    }

    // Event audit mode (base vs current)
    if let Some((base_db, current_db)) = &config.event_audit {
        let report = run_event_audit(base_db, current_db)
            .unwrap_or_else(|e| {
                eprintln!("Failed to run event audit: {}", e);
                std::process::exit(1);
            })
            .to_markdown();

        let output_path = config
            .audit_output
            .clone()
            .unwrap_or_else(default_audit_output_path);
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        if let Err(e) = std::fs::write(&output_path, &report) {
            eprintln!("Failed to write audit report: {}", e);
            std::process::exit(1);
        }
        println!("Event audit written to {}", output_path.display());
        return;
    }

    // Focused analysis (single DB)
    if let Some(db_path) = &config.focused_db {
        let report = run_focused_analysis(db_path)
            .unwrap_or_else(|e| {
                eprintln!("Failed to run focused analysis: {}", e);
                std::process::exit(1);
            })
            .to_markdown();
        let output_path = config
            .focused_output
            .clone()
            .unwrap_or_else(default_focused_output_path);
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        if let Err(e) = std::fs::write(&output_path, &report) {
            eprintln!("Failed to write focused report: {}", e);
            std::process::exit(1);
        }
        println!("Focused analysis written to {}", output_path.display());
        return;
    }

    // Team interaction analysis
    if let Some(db_path) = &config.team_interaction_db {
        let report = run_team_interaction_analysis(db_path)
            .unwrap_or_else(|e| {
                eprintln!("Failed to run team interaction analysis: {}", e);
                std::process::exit(1);
            });
        let output_path = config
            .team_interaction_output
            .clone()
            .unwrap_or_else(default_team_interaction_output_path);
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        if let Err(e) = std::fs::write(&output_path, report.to_markdown()) {
            eprintln!("Failed to write team interaction report: {}", e);
            std::process::exit(1);
        }
        println!("Team interaction analysis written to {}", output_path.display());
        return;
    }

    // AI Client win rates analysis
    if config.client_winrates {
        let db_path = config
            .client_winrates_db
            .as_ref()
            .unwrap_or(&config.db_path);

        match run_client_winrates_analysis(db_path) {
            Ok(report) => {
                println!("{}", report);
            }
            Err(e) => {
                eprintln!("Failed to run client win rates analysis: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    // AI Client comparison
    if let Some((client_a, client_b)) = &config.client_comparison {
        let db_path = config
            .client_winrates_db
            .as_ref()
            .unwrap_or(&config.db_path);

        match run_client_comparison(db_path, client_a, client_b) {
            Ok(report) => {
                println!("{}", report);
            }
            Err(e) => {
                eprintln!("Failed to run client comparison: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    // Bracket tournament analysis
    if config.bracket_analysis {
        let db_path = &config.bracket_db.clone().unwrap_or(config.db_path.clone());
        let output_dir = config
            .bracket_output
            .clone()
            .unwrap_or_else(default_bracket_output_dir);

        // Auto-generate rankings path from db path if export requested
        let rankings_path = if config.export_bracket_rankings {
            let db_str = db_path.to_string_lossy();
            let rankings_str = db_str.replace(".db", "_rankings.txt");
            Some(PathBuf::from(rankings_str))
        } else {
            None
        };
        let rankings_file = rankings_path.as_deref();

        match run_bracket_analysis(db_path, &output_dir, rankings_file) {
            Ok(report) => {
                // Print summary to console
                println!("\n=== Bracket Tournament Analysis ===\n");
                println!("Tournament ID: {}", report.tournament.id);
                println!(
                    "Format: Best of {} (First to {})",
                    report.tournament.format_best_of, report.tournament.format_score_limit
                );
                println!("Entrants: {}", report.tournament.entrant_count);
                if let Some(ref champion) = report.tournament.champion_profile {
                    println!("Champion: {}", champion);
                }
                println!(
                    "Total Games: {} ({} events)",
                    report.total_games, report.total_events
                );
                println!("\nTop Standings:");
                println!(
                    "{:<4} {:<24} {:>8} {:>10}",
                    "Rank", "Profile", "Match", "Game"
                );
                println!("{:-<4} {:-<24} {:->8} {:->10}", "", "", "", "");
                for (i, standing) in report.standings.iter().enumerate().take(10) {
                    println!(
                        "{:<4} {:<24} {:>3}-{:<3} {:>4}-{:<4}",
                        i + 1,
                        &standing.profile_name[..standing.profile_name.len().min(24)],
                        standing.match_wins,
                        standing.match_losses,
                        standing.game_wins,
                        standing.game_losses
                    );
                }

                // Print next steps with correct profiles file from db
                println!("\n=== Next Steps ===\n");
                println!("Generate evolved profiles from bracket results:\n");

                // Extract version info from profiles_file path
                let (input_profiles, output_profiles, next_version) =
                    if let Some(ref pf) = report.tournament.profiles_file {
                        // Parse version number from path like "config/ai_profiles_v13.txt"
                        let version_num = pf
                            .find("_v")
                            .and_then(|i| {
                                let start = i + 2;
                                let end = pf[start..].find(|c: char| !c.is_ascii_digit())
                                    .map(|j| start + j)
                                    .unwrap_or(pf.len());
                                pf[start..end].parse::<u32>().ok()
                            });

                        if let Some(v) = version_num {
                            let next_v = v + 1;
                            let output = pf.replace(&format!("_v{}", v), &format!("_v{}", next_v));
                            (pf.clone(), output, format!("v{}", next_v))
                        } else {
                            // Couldn't parse version, use the file as-is with generic output
                            (pf.clone(), format!("{}.next", pf), "vnext".to_string())
                        }
                    } else {
                        // No profiles_file in db, fall back to defaults
                        ("config/ai_profiles_v13.txt".to_string(),
                         "config/ai_profiles_v14.txt".to_string(),
                         "v14".to_string())
                    };

                println!("  python3 scripts/generate_bracket_profiles.py \\");
                println!("    --db {} \\", db_path.display());
                println!("    --profiles {} \\", input_profiles);
                println!("    --output {} \\", output_profiles);
                println!("    --version {}", next_version);
                println!();
                println!("Note: Rankings file was auto-generated alongside the db file.");
            }
            Err(e) => {
                eprintln!("Failed to run bracket analysis: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    // Check that db_path was provided for main analysis
    if config.db_path.as_os_str().is_empty() {
        eprintln!("Error: Database path required for analysis");
        eprintln!("Usage: cargo run --bin analyze -- <db_path>");
        eprintln!("       cargo run --bin analyze -- --db <db_path>");
        std::process::exit(1);
    }

    // Parse all event logs from SQLite
    println!("Using database: {}", config.db_path.display());
    let matches = parse_all_matches_from_db(&config.db_path);

    if matches.is_empty() {
        println!("No valid matches found in {}", config.db_path.display());
        println!("\nTo generate logs, run simulations with --db:");
        println!("  cargo run --bin simulate -- --tournament 5 --db training.db");
        return;
    }

    println!("Parsed {} matches.\n", matches.len());

    // Compute aggregate metrics
    let metrics = AggregateMetrics::from_matches(&matches);

    // Print header
    println!("============================================================");
    println!("{}", metrics.format_summary());

    // Load targets and compare
    let targets = if let Some(path) = &config.targets_file {
        load_targets(path).unwrap_or_else(|| {
            println!("Warning: Could not parse targets file, using defaults");
            default_targets()
        })
    } else {
        default_targets()
    };

    println!("{}", targets.format_report(&metrics));

    // Generate leaderboard
    let profiles: Vec<_> = metrics.by_profile.values().cloned().collect();
    let leaderboard = Leaderboard::from_metrics(&profiles);
    println!("{}", leaderboard.format_table());

    // Generate suggestions
    let deltas = targets.compare(&metrics);
    let suggestions = generate_suggestions(&deltas);
    println!("{}", format_suggestions(&suggestions));

    // Update defaults if requested
    if config.update_defaults {
        if let (Some(best), Some(second)) = (
            leaderboard.best_profile(),
            leaderboard.second_best_profile(),
        ) {
            let constants_path = PathBuf::from("src/constants.rs");

            match update_default_profiles(&constants_path, best, second) {
                Ok((old_left, old_right)) => {
                    println!(
                        "{}",
                        format_update_report(&old_left, &old_right, best, second)
                    );
                }
                Err(e) => {
                    println!("\nFailed to update defaults: {}", e);
                }
            }
        } else {
            println!("\nNot enough profiles in leaderboard to update defaults.");
        }
    }

    // Save report if requested
    if let Some(output_path) = &config.output_file {
        let report = generate_full_report(&metrics, &leaderboard, &targets, &suggestions);
        if let Err(e) = std::fs::write(output_path, &report) {
            eprintln!("Failed to write report: {}", e);
        } else {
            println!("\nReport written to {}", output_path.display());
        }
    }
}

/// Configuration for the analyze tool
struct AnalyzeConfig {
    db_path: PathBuf,
    targets_file: Option<PathBuf>,
    output_file: Option<PathBuf>,
    event_audit: Option<(PathBuf, PathBuf)>,
    audit_output: Option<PathBuf>,
    focused_db: Option<PathBuf>,
    focused_output: Option<PathBuf>,
    training_db: Option<PathBuf>,
    training_output: Option<PathBuf>,
    request_name: Option<String>,
    request_output: Option<PathBuf>,
    request_db: Option<PathBuf>,
    request_list: bool,
    requests_file: PathBuf,
    request_add: Option<String>,
    request_sql: Option<String>,
    request_desc: Option<String>,
    request_query_name: Option<String>,
    request_db_label: Option<String>,
    bracket_analysis: bool,
    bracket_db: Option<PathBuf>,
    bracket_output: Option<PathBuf>,
    export_bracket_rankings: bool,
    team_interaction_db: Option<PathBuf>,
    team_interaction_output: Option<PathBuf>,
    update_defaults: bool,
    show_help: bool,
    // AI Client analysis
    client_winrates: bool,
    client_winrates_db: Option<PathBuf>,
    client_comparison: Option<(String, String)>,
}

impl Default for AnalyzeConfig {
    fn default() -> Self {
        Self {
            db_path: PathBuf::new(), // Must be set explicitly via --db or positional arg
            targets_file: None,
            output_file: None,
            event_audit: None,
            audit_output: None,
            focused_db: None,
            focused_output: None,
            training_db: None,
            training_output: None,
            request_name: None,
            request_output: None,
            request_db: None,
            request_list: false,
            requests_file: PathBuf::from("config/analysis_requests.json"),
            request_add: None,
            request_sql: None,
            request_desc: None,
            request_query_name: None,
            request_db_label: None,
            bracket_analysis: false,
            bracket_db: None,
            bracket_output: None,
            export_bracket_rankings: false,
            team_interaction_db: None,
            team_interaction_output: None,
            update_defaults: false,
            show_help: false,
            // AI Client analysis
            client_winrates: false,
            client_winrates_db: None,
            client_comparison: None,
        }
    }
}

impl AnalyzeConfig {
    fn from_args() -> Self {
        let args: Vec<String> = std::env::args().collect();
        let mut config = Self::default();

        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "--targets" => {
                    if i + 1 < args.len() {
                        config.targets_file = Some(PathBuf::from(&args[i + 1]));
                        i += 1;
                    }
                }
                "--output" | "-o" => {
                    if i + 1 < args.len() {
                        config.output_file = Some(PathBuf::from(&args[i + 1]));
                        i += 1;
                    }
                }
                "--event-audit" => {
                    if i + 2 < args.len() {
                        config.event_audit =
                            Some((PathBuf::from(&args[i + 1]), PathBuf::from(&args[i + 2])));
                        i += 2;
                    }
                }
                "--audit-output" => {
                    if i + 1 < args.len() {
                        config.audit_output = Some(PathBuf::from(&args[i + 1]));
                        i += 1;
                    }
                }
                "--focused" => {
                    if i + 1 < args.len() {
                        config.focused_db = Some(PathBuf::from(&args[i + 1]));
                        i += 1;
                    }
                }
                "--focused-output" => {
                    if i + 1 < args.len() {
                        config.focused_output = Some(PathBuf::from(&args[i + 1]));
                        i += 1;
                    }
                }
                "--training-db" => {
                    if i + 1 < args.len() {
                        config.training_db = Some(PathBuf::from(&args[i + 1]));
                        i += 1;
                    }
                }
                "--training-output" => {
                    if i + 1 < args.len() {
                        config.training_output = Some(PathBuf::from(&args[i + 1]));
                        i += 1;
                    }
                }
                "--request" => {
                    if i + 1 < args.len() {
                        config.request_name = Some(args[i + 1].clone());
                        i += 1;
                    }
                }
                "--request-output" => {
                    if i + 1 < args.len() {
                        config.request_output = Some(PathBuf::from(&args[i + 1]));
                        i += 1;
                    }
                }
                "--request-db" => {
                    if i + 1 < args.len() {
                        config.request_db = Some(PathBuf::from(&args[i + 1]));
                        i += 1;
                    }
                }
                "--request-list" => {
                    config.request_list = true;
                }
                "--requests-file" => {
                    if i + 1 < args.len() {
                        config.requests_file = PathBuf::from(&args[i + 1]);
                        i += 1;
                    }
                }
                "--request-add" => {
                    if i + 1 < args.len() {
                        config.request_add = Some(args[i + 1].clone());
                        i += 1;
                    }
                }
                "--request-sql" => {
                    if i + 1 < args.len() {
                        config.request_sql = Some(args[i + 1].clone());
                        i += 1;
                    }
                }
                "--request-desc" => {
                    if i + 1 < args.len() {
                        config.request_desc = Some(args[i + 1].clone());
                        i += 1;
                    }
                }
                "--request-query-name" => {
                    if i + 1 < args.len() {
                        config.request_query_name = Some(args[i + 1].clone());
                        i += 1;
                    }
                }
                "--request-db-label" => {
                    if i + 1 < args.len() {
                        config.request_db_label = Some(args[i + 1].clone());
                        i += 1;
                    }
                }
                "--bracket" => {
                    config.bracket_analysis = true;
                }
                "--bracket-db" => {
                    if i + 1 < args.len() {
                        config.bracket_db = Some(PathBuf::from(&args[i + 1]));
                        i += 1;
                    }
                }
                "--bracket-output" => {
                    if i + 1 < args.len() {
                        config.bracket_output = Some(PathBuf::from(&args[i + 1]));
                        i += 1;
                    }
                }
                "--bracket-rankings" => {
                    config.export_bracket_rankings = true;
                }
                "--team-interaction" => {
                    if i + 1 < args.len() {
                        config.team_interaction_db = Some(PathBuf::from(&args[i + 1]));
                        i += 1;
                    }
                }
                "--team-interaction-output" => {
                    if i + 1 < args.len() {
                        config.team_interaction_output = Some(PathBuf::from(&args[i + 1]));
                        i += 1;
                    }
                }
                "--update-defaults" => {
                    config.update_defaults = true;
                }
                // AI Client analysis
                "--client-winrates" => {
                    config.client_winrates = true;
                }
                "--client-winrates-db" => {
                    if i + 1 < args.len() {
                        config.client_winrates_db = Some(PathBuf::from(&args[i + 1]));
                        i += 1;
                    }
                }
                "--client-comparison" => {
                    if i + 2 < args.len() {
                        config.client_comparison =
                            Some((args[i + 1].clone(), args[i + 2].clone()));
                        i += 2;
                    }
                }
                "--help" | "-h" => {
                    config.show_help = true;
                }
                arg if !arg.starts_with('-') => {
                    // Positional argument: db path
                    config.db_path = PathBuf::from(arg);
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
        r#"Analytics Tool - Analyze simulation results

USAGE:
    cargo run --bin analyze -- [LOG_DIR] [OPTIONS]

ARGUMENTS:
    DB_PATH             SQLite database path (default: training.db)

OPTIONS:
    --targets <FILE>    Load tuning targets from TOML file
    --output, -o <FILE> Write full report to file
    --event-audit <BASE_DB> <CURRENT_DB>  Compare two DBs via event audit queries
    --audit-output <FILE> Write event audit report to file (default: notes/analysis_runs/...)
    --focused <DB>       Run focused analysis on a single DB
    --focused-output <FILE> Write focused report to file (default: notes/analysis_runs/...)
    --training-db <DB>   Run training debug analysis on a training DB
    --training-output <DIR> Output directory for training analysis (default: training_logs/session_x/analysis)
    --request <NAME>     Run a stored SQL analysis request
    --request-output <FILE> Write request report to file (default: notes/analysis_runs/...)
    --request-db <DB>    Override DB path for a request
    --request-list       List available analysis requests
    --requests-file <FILE> Use an alternate analysis requests file
    --request-add <NAME> Add a new analysis request (requires --request-sql)
    --request-sql <SQL>  SQL for --request-add
    --request-desc <TEXT> Description for --request-add
    --request-query-name <NAME> Query name for --request-add (default: query)
    --request-db-label <LABEL> Label stored with request DB
    --bracket           Analyze most recent bracket tournament
    --bracket-db <DB>   Override DB path for bracket analysis
    --bracket-output <DIR> Output directory for bracket reports
    --bracket-rankings  Export standings to auto-generated rankings file
    --team-interaction <DB> Analyze team interactions (passes, blocks) from training DB
    --team-interaction-output <FILE> Output file for team interaction report
    --update-defaults   Update default profiles in src/constants.rs
    --help, -h          Show this help

AI CLIENT ANALYSIS:
    --client-winrates   Show win rates for AI clients (external + embedded)
    --client-winrates-db <DB>  Database for client analysis (default: uses DB_PATH)
    --client-comparison <A> <B>  Head-to-head comparison of two clients/profiles

EXAMPLES:
    # Analyze logs with default targets
    cargo run --bin analyze -- training.db

    # Use custom tuning targets
    cargo run --bin analyze -- training.db --targets assets/tuning_targets.toml

    # Update default profiles based on leaderboard
    cargo run --bin analyze -- training.db --update-defaults

    # Event audit: compare baseline vs current tournament DBs
    cargo run --bin analyze -- --event-audit db/baseline.db db/current.db

    # Focused analysis: deep dive on a single DB
    cargo run --bin analyze -- --focused db/current.db

    # Training debug analysis
    cargo run --bin analyze -- --training-db db/training_YYYYMMDD_HHMMSS.db

    # Run a stored analysis request
    cargo run --bin analyze -- --request focused_core --request-db db/current.db

    # Add a new stored request
    cargo run --bin analyze -- --request-add my_query --request-sql "SELECT COUNT(*) FROM matches"

    # Bracket tournament analysis (uses most recent bracket DB in db/)
    cargo run --bin analyze -- --bracket

    # Bracket analysis with specific DB and rankings export
    cargo run --bin analyze -- --bracket --bracket-db db/bracket_20260129_143022.db --bracket-rankings

    # Team interaction analysis (passes, blocks, coordination)
    cargo run --bin analyze -- --team-interaction db/training_YYYYMMDD_HHMMSS.db

    # AI Client win rates (requires participant tracking data)
    cargo run --bin analyze -- --client-winrates --client-winrates-db db/simulation.db

    # Head-to-head comparison between two AI clients
    cargo run --bin analyze -- --client-comparison ai-v1 ai-v2 --client-winrates-db db/simulation.db

TARGETS FILE FORMAT (TOML):
    [targets]
    avg_score = {{ target = 14.0, tolerance = 1.0 }}
    score_differential = {{ target = 2.0, tolerance = 1.0 }}
    match_duration_secs = {{ target = 180.0, tolerance = 15.0 }}
    turnovers_per_match = {{ target = 20.0, tolerance = 5.0 }}
    missed_shots_per_match = {{ target = 20.0, tolerance = 5.0 }}
"#
    );
}

fn generate_full_report(
    metrics: &AggregateMetrics,
    leaderboard: &Leaderboard,
    targets: &TuningTargets,
    suggestions: &[ParameterSuggestion],
) -> String {
    let mut report = String::new();

    report.push_str("============================================================\n");
    report.push_str(&metrics.format_summary());
    report.push_str(&targets.format_report(metrics));
    report.push_str(&leaderboard.format_table());
    report.push_str(&format_suggestions(suggestions));

    report
}

fn default_audit_output_path() -> PathBuf {
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    PathBuf::from(format!("notes/analysis_runs/event_audit_{}.md", timestamp))
}

fn default_focused_output_path() -> PathBuf {
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    PathBuf::from(format!("notes/analysis_runs/focused_{}.md", timestamp))
}

fn default_team_interaction_output_path() -> PathBuf {
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    PathBuf::from(format!(
        "notes/analysis_runs/team_interaction_{}.md",
        timestamp
    ))
}

fn default_request_output_path(name: &str) -> PathBuf {
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    PathBuf::from(format!(
        "notes/analysis_runs/request_{}_{}.md",
        name, timestamp
    ))
}

fn default_training_output_dir(db_path: &PathBuf) -> PathBuf {
    if is_combined_training_db(db_path) {
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        return PathBuf::from(format!("training_logs/combined_{}/analysis", timestamp));
    }
    let session_dir = infer_training_session_dir(db_path)
        .unwrap_or_else(|| PathBuf::from("training_logs").join("analysis_unknown"));
    session_dir.join("analysis")
}

fn is_combined_training_db(db_path: &PathBuf) -> bool {
    if db_path
        .file_name()
        .and_then(|s| s.to_str())
        .map(|name| name.contains("combined"))
        .unwrap_or(false)
    {
        return true;
    }
    training_session_count(db_path).unwrap_or(1) > 1
}

fn training_session_count(db_path: &PathBuf) -> Option<usize> {
    let conn = Connection::open(db_path).ok()?;
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))
        .ok()?;
    Some(count as usize)
}

fn infer_training_session_dir(db_path: &PathBuf) -> Option<PathBuf> {
    let mut resolved = db_path.clone();
    if db_path.file_name().and_then(|n| n.to_str()) == Some("training.db") {
        if let Ok(target) = std::fs::read_link(db_path) {
            resolved = target;
        }
    }
    let file_name = resolved.file_stem()?.to_string_lossy();
    let timestamp = file_name.strip_prefix("training_")?;
    Some(PathBuf::from("training_logs").join(format!("session_{}", timestamp)))
}

fn default_training_report_name(report: &TrainingDebugReport) -> String {
    if let Some(session_id) = &report.session_id {
        format!("training_debug_{}.md", session_id)
    } else {
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        format!("training_debug_{}.md", timestamp)
    }
}

fn default_bracket_output_dir() -> PathBuf {
    PathBuf::from("notes/analysis_runs/bracket")
}

/// Run client win rates analysis
fn run_client_winrates_analysis(db_path: &PathBuf) -> Result<String, String> {
    use ballgame::simulation::SimDatabase;

    let db = SimDatabase::open(db_path).map_err(|e| format!("Failed to open database: {}", e))?;

    // Check if there's any participant data
    if !db.has_participant_data().map_err(|e| e.to_string())? {
        return Err("No participant data found in database. Run simulations with --left-team/--right-team to track AI clients.".to_string());
    }

    let client_rates = db.get_client_win_rates().map_err(|e| e.to_string())?;
    let all_rates = db.get_participant_win_rates(None).map_err(|e| e.to_string())?;

    let mut report = String::new();
    report.push_str("# AI Client Win Rates Analysis\n\n");

    // Client-specific rates
    if client_rates.is_empty() {
        report.push_str("## External AI Clients\n\n");
        report.push_str("No external AI client data found.\n\n");
    } else {
        let total_matches: i64 = client_rates.iter().map(|r| r.matches).sum();
        report.push_str(&format!(
            "## External AI Clients (from {} matches)\n\n",
            total_matches / 2
        )); // Divide by 2 since each match has 2 participants

        report.push_str("| Client | Matches | Wins | Losses | Draws | Win Rate |\n");
        report.push_str("|--------|---------|------|--------|-------|----------|\n");

        for rate in &client_rates {
            report.push_str(&format!(
                "| {} | {} | {} | {} | {} | {:.1}% |\n",
                rate.client_id,
                rate.matches,
                rate.wins,
                rate.losses,
                rate.draws,
                rate.win_rate * 100.0
            ));
        }
        report.push('\n');
    }

    // All participants (profiles and clients)
    if !all_rates.is_empty() {
        report.push_str("## All Participants (Profiles + Clients)\n\n");
        report.push_str("| Participant | Matches | Wins | Losses | Draws | Win Rate |\n");
        report.push_str("|-------------|---------|------|--------|-------|----------|\n");

        for rate in &all_rates {
            report.push_str(&format!(
                "| {} | {} | {} | {} | {} | {:.1}% |\n",
                rate.client_id,
                rate.matches,
                rate.wins,
                rate.losses,
                rate.draws,
                rate.win_rate * 100.0
            ));
        }
    }

    Ok(report)
}

/// Run head-to-head client comparison
fn run_client_comparison(
    db_path: &PathBuf,
    client_a: &str,
    client_b: &str,
) -> Result<String, String> {
    let conn =
        Connection::open(db_path).map_err(|e| format!("Failed to open database: {}", e))?;

    // Find matches where both clients participated on opposite teams
    let sql = r#"
        SELECT
            m.id,
            m.left_score,
            m.right_score,
            m.winner,
            m.level_name,
            mp_a.team as team_a,
            mp_b.team as team_b
        FROM matches m
        JOIN match_participants mp_a ON mp_a.match_id = m.id AND mp_a.participant_id = ?1
        JOIN match_participants mp_b ON mp_b.match_id = m.id AND mp_b.participant_id = ?2
        WHERE mp_a.team != mp_b.team
        ORDER BY m.id
    "#;

    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([client_a, client_b], |row| {
            Ok((
                row.get::<_, i64>(0)?,      // match_id
                row.get::<_, i64>(1)?,      // left_score
                row.get::<_, i64>(2)?,      // right_score
                row.get::<_, String>(3)?,   // winner
                row.get::<_, String>(4)?,   // level_name
                row.get::<_, String>(5)?,   // team_a
                row.get::<_, String>(6)?,   // team_b
            ))
        })
        .map_err(|e| e.to_string())?;

    let matches: Vec<_> = rows.filter_map(|r| r.ok()).collect();

    let mut report = String::new();
    report.push_str(&format!(
        "# Head-to-Head: {} vs {}\n\n",
        client_a, client_b
    ));

    if matches.is_empty() {
        report.push_str("No head-to-head matches found between these clients.\n");
        return Ok(report);
    }

    // Calculate stats
    let mut a_wins = 0;
    let mut b_wins = 0;
    let mut ties = 0;
    let mut a_total_score = 0i64;
    let mut b_total_score = 0i64;

    for (_, left_score, right_score, winner, _, team_a, _) in &matches {
        let (a_score, b_score) = if team_a == "left" {
            (*left_score, *right_score)
        } else {
            (*right_score, *left_score)
        };

        a_total_score += a_score;
        b_total_score += b_score;

        if winner == "tie" {
            ties += 1;
        } else if (winner == "left" && team_a == "left") || (winner == "right" && team_a == "right")
        {
            a_wins += 1;
        } else {
            b_wins += 1;
        }
    }

    let total = matches.len();
    let a_win_rate = a_wins as f64 / total as f64 * 100.0;
    let b_win_rate = b_wins as f64 / total as f64 * 100.0;

    report.push_str(&format!("## Summary ({} matches)\n\n", total));
    report.push_str("| Client | Wins | Losses | Ties | Win Rate | Avg Score |\n");
    report.push_str("|--------|------|--------|------|----------|----------|\n");
    report.push_str(&format!(
        "| {} | {} | {} | {} | {:.1}% | {:.1} |\n",
        client_a,
        a_wins,
        b_wins,
        ties,
        a_win_rate,
        a_total_score as f64 / total as f64
    ));
    report.push_str(&format!(
        "| {} | {} | {} | {} | {:.1}% | {:.1} |\n",
        client_b,
        b_wins,
        a_wins,
        ties,
        b_win_rate,
        b_total_score as f64 / total as f64
    ));

    report.push_str("\n## Match History\n\n");
    report.push_str("| Match | Level | Score | Winner |\n");
    report.push_str("|-------|-------|-------|--------|\n");

    for (match_id, left_score, right_score, winner, level_name, team_a, _) in matches.iter().take(20)
    {
        let (a_score, b_score) = if team_a == "left" {
            (*left_score, *right_score)
        } else {
            (*right_score, *left_score)
        };

        let winner_name = if winner == "tie" {
            "Tie".to_string()
        } else if (winner == "left" && team_a == "left") || (winner == "right" && team_a == "right")
        {
            client_a.to_string()
        } else {
            client_b.to_string()
        };

        report.push_str(&format!(
            "| {} | {} | {}-{} | {} |\n",
            match_id, level_name, a_score, b_score, winner_name
        ));
    }

    if matches.len() > 20 {
        report.push_str(&format!("\n_(showing 20 of {} matches)_\n", matches.len()));
    }

    Ok(report)
}
