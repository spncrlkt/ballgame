//! Scenario test runner CLI
//!
//! Usage:
//!   cargo run --bin test-scenarios              # Run all tests
//!   cargo run --bin test-scenarios -- movement/ # Run category
//!   cargo run --bin test-scenarios -- shooting/shoot_basic  # Run single test
//!   cargo run --bin test-scenarios -- --verbose # Show details on failure

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use ballgame::testing::{SCENARIOS_DIR, TestResult, parser::parse_test_file, runner::run_test};

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut verbose = false;
    let mut filter: Option<String> = None;

    for arg in &args[1..] {
        if arg == "--verbose" || arg == "-v" {
            verbose = true;
        } else if !arg.starts_with('-') {
            filter = Some(arg.clone());
        }
    }

    println!("Scenario Tests");
    println!("==============\n");

    let scenarios_path = Path::new(SCENARIOS_DIR);
    if !scenarios_path.exists() {
        println!("No scenarios directory found at {}", SCENARIOS_DIR);
        println!("Create test files in tests/scenarios/");
        std::process::exit(1);
    }

    let tests = discover_tests(scenarios_path, filter.as_deref());

    if tests.is_empty() {
        println!("No test files found.");
        if let Some(f) = filter {
            println!("Filter: {}", f);
        }
        std::process::exit(1);
    }

    let mut passed = 0;
    let mut failed = 0;
    let mut errors = 0;
    let mut current_category = String::new();

    for test_path in &tests {
        let rel_path = test_path.strip_prefix(scenarios_path).unwrap_or(test_path);

        // Print category header
        if let Some(parent) = rel_path.parent() {
            let category = parent.to_string_lossy().to_string();
            if category != current_category && !category.is_empty() {
                if !current_category.is_empty() {
                    println!();
                }
                println!("{}/", category);
                current_category = category;
            }
        }

        let test_name = rel_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        // Parse test file
        let test_def = match parse_test_file(test_path) {
            Ok(def) => def,
            Err(e) => {
                print_result(&test_name, &TestResult::Error { message: e }, verbose);
                errors += 1;
                continue;
            }
        };

        // Run test
        let result = run_test(&test_def);

        match &result {
            TestResult::Pass { .. } => passed += 1,
            TestResult::Fail { .. } => failed += 1,
            TestResult::Error { .. } => errors += 1,
        }

        print_result(&test_name, &result, verbose);
    }

    println!("\n==============");
    println!(
        "Results: {} passed, {} failed, {} errors",
        passed, failed, errors
    );

    if failed > 0 || errors > 0 {
        std::process::exit(1);
    }
}

fn discover_tests(base: &Path, filter: Option<&str>) -> Vec<PathBuf> {
    let mut tests = Vec::new();
    discover_tests_recursive(base, base, filter, &mut tests);
    tests.sort();
    tests
}

fn discover_tests_recursive(
    base: &Path,
    current: &Path,
    filter: Option<&str>,
    tests: &mut Vec<PathBuf>,
) {
    let entries = match fs::read_dir(current) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_dir() {
            discover_tests_recursive(base, &path, filter, tests);
        } else if path.extension().map(|e| e == "toml").unwrap_or(false) {
            // Check filter
            if let Some(f) = filter {
                let rel = path.strip_prefix(base).unwrap_or(&path).to_string_lossy();

                if !rel.contains(f) {
                    continue;
                }
            }

            tests.push(path);
        }
    }
}

fn print_result(name: &str, result: &TestResult, verbose: bool) {
    let dots = ".".repeat(40 - name.len().min(39));

    match result {
        TestResult::Pass { frames } => {
            println!("  {} {} PASS ({} frames)", name, dots, frames);
        }
        TestResult::Fail { error } => {
            println!("  {} {} FAIL", name, dots);
            if verbose {
                println!("    {}", error);
            } else {
                println!("    {}", error.message);
            }
        }
        TestResult::Error { message } => {
            println!("  {} {} ERROR", name, dots);
            println!("    {}", message);
        }
    }
}
