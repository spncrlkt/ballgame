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
    AggregateMetrics, AnalysisQuery, AnalysisRequest, AnalysisRequestFile, Leaderboard,
    ParameterSuggestion, TrainingDebugReport, TuningTargets, default_targets, format_suggestions,
    format_update_report, generate_suggestions, load_targets, parse_all_matches_from_db,
    run_event_audit, run_focused_analysis, run_request, run_training_debug_analysis,
    update_default_profiles,
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
    update_defaults: bool,
    show_help: bool,
}

impl Default for AnalyzeConfig {
    fn default() -> Self {
        Self {
            db_path: PathBuf::from("db/training.db"),
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
    --update-defaults   Update default profiles in src/constants.rs
    --help, -h          Show this help

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

fn default_request_output_path(name: &str) -> PathBuf {
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    PathBuf::from(format!(
        "notes/analysis_runs/request_{}_{}.md",
        name, timestamp
    ))
}

fn default_training_output_dir(db_path: &PathBuf) -> PathBuf {
    let session_dir = infer_training_session_dir(db_path)
        .unwrap_or_else(|| PathBuf::from("training_logs").join("analysis_unknown"));
    session_dir.join("analysis")
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
