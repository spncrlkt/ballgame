//! Unified asset generator
//!
//! Consolidates all asset generation into one binary with subcommands.
//!
//! Usage:
//!   cargo run --bin generate ball       # Generate ball textures
//!   cargo run --bin generate showcase   # Generate ball styles showcase
//!   cargo run --bin generate levels     # Generate level showcase grid
//!   cargo run --bin generate gif wedge  # Generate wedge rotation GIF
//!   cargo run --bin generate gif baseball  # Generate baseball rotation GIF
//!   cargo run --bin generate --help     # Show help

use ballgame::generate;
use ballgame::run_summary::{FileCategory, FileEntry, NextStep, RunSummary};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_help();
        std::process::exit(1);
    }

    let start_time = std::time::Instant::now();

    let (command, files, next) = match args[1].as_str() {
        "ball" | "balls" => {
            println!("=== Ball Texture Generator ===\n");
            generate::ball::run();
            (
                "ball",
                vec![FileEntry::new(
                    "assets/textures/balls/",
                    FileCategory::Image,
                )],
                NextStep::primary(
                    "cargo run --bin generate showcase",
                    "Generate ball styles showcase image",
                ),
            )
        }
        "showcase" => {
            println!("=== Ball Styles Showcase Generator ===\n");
            generate::showcase::run();
            (
                "showcase",
                vec![FileEntry::new(
                    "showcase/ball_styles_showcase.png",
                    FileCategory::Image,
                )],
                NextStep::secondary("cargo run --bin generate ball", "Regenerate ball textures"),
            )
        }
        "levels" | "level" => {
            println!("=== Level Showcase Generator ===\n");
            generate::levels::run();
            (
                "levels",
                vec![FileEntry::new(
                    "showcase/level_showcase.png",
                    FileCategory::Image,
                )],
                NextStep::secondary(
                    "./scripts/generate_level_showcase.sh",
                    "Recapture level screenshots",
                ),
            )
        }
        "gif" => {
            if args.len() < 3 {
                eprintln!("Error: 'gif' requires a type: wedge or baseball");
                eprintln!("  cargo run --bin generate gif wedge");
                eprintln!("  cargo run --bin generate gif baseball");
                std::process::exit(1);
            }
            match args[2].as_str() {
                "wedge" => {
                    println!("=== Wedge GIF Generator ===\n");
                    generate::gif_wedge::run();
                    (
                        "gif wedge",
                        vec![FileEntry::new(
                            "assets/wedge_rotation.gif",
                            FileCategory::Image,
                        )],
                        NextStep::secondary(
                            "cargo run --bin generate gif baseball",
                            "Generate baseball rotation GIF",
                        ),
                    )
                }
                "baseball" => {
                    println!("=== Baseball GIF Generator ===\n");
                    generate::gif_baseball::run();
                    (
                        "gif baseball",
                        vec![FileEntry::new(
                            "assets/baseball_rotation.gif",
                            FileCategory::Image,
                        )],
                        NextStep::secondary(
                            "cargo run --bin generate gif wedge",
                            "Generate wedge rotation GIF",
                        ),
                    )
                }
                other => {
                    eprintln!(
                        "Error: Unknown GIF type '{}'. Use 'wedge' or 'baseball'.",
                        other
                    );
                    std::process::exit(1);
                }
            }
        }
        "--help" | "-h" | "help" => {
            print_help();
            return;
        }
        other => {
            eprintln!("Error: Unknown command '{}'\n", other);
            print_help();
            std::process::exit(1);
        }
    };

    let mut summary = RunSummary::new(format!("Asset Generation Complete: {}", command))
        .duration(start_time.elapsed());

    for file in files {
        summary = summary.file(file);
    }

    summary = summary.next_step(next);

    summary.print();
}

fn print_help() {
    println!(
        r#"Asset Generator - Generate game assets

USAGE:
    cargo run --bin generate <COMMAND>

COMMANDS:
    ball        Generate ball textures for all styles × palettes
                Output: assets/textures/balls/ball_<style>_<palette>.png

    showcase    Generate ball styles showcase image
                Output: showcase/ball_styles_showcase.png

    levels      Generate level showcase grid (requires level_screenshots/)
                Output: showcase/level_showcase.png

    gif wedge      Generate wedge ball rotation GIF
                   Output: assets/wedge_frames/ + wedge.gif

    gif baseball   Generate baseball rotation GIF
                   Output: assets/baseball_frames/ + baseball.gif

    help        Show this help message

EXAMPLES:
    cargo run --bin generate ball
    cargo run --bin generate showcase
    cargo run --bin generate gif wedge
"#
    );
}
