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

use ballgame::analytics::{
    AggregateMetrics, Leaderboard, generate_suggestions, format_update_report,
    load_targets, default_targets, update_default_profiles, parse_all_matches_from_db,
    format_suggestions, TuningTargets, ParameterSuggestion,
};

fn main() {
    let config = AnalyzeConfig::from_args();

    if config.show_help {
        print_help();
        return;
    }

    // Parse all event logs from SQLite
    println!("Parsing SQLite events from {}...", config.db_path.display());
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
                    println!("{}", format_update_report(&old_left, &old_right, best, second));
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
    update_defaults: bool,
    show_help: bool,
}

impl Default for AnalyzeConfig {
    fn default() -> Self {
        Self {
            db_path: PathBuf::from("training.db"),
            targets_file: None,
            output_file: None,
            update_defaults: false,
            show_help: false,
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
                "--update-defaults" => {
                    config.update_defaults = true;
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
    --update-defaults   Update default profiles in src/constants.rs
    --help, -h          Show this help

EXAMPLES:
    # Analyze logs with default targets
    cargo run --bin analyze -- training.db

    # Use custom tuning targets
    cargo run --bin analyze -- training.db --targets assets/tuning_targets.toml

    # Update default profiles based on leaderboard
    cargo run --bin analyze -- training.db --update-defaults

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
