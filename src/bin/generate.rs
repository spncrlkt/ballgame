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

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_help();
        std::process::exit(1);
    }

    match args[1].as_str() {
        "ball" | "balls" => {
            println!("=== Ball Texture Generator ===\n");
            generate::ball::run();
        }
        "showcase" => {
            println!("=== Ball Styles Showcase Generator ===\n");
            generate::showcase::run();
        }
        "levels" | "level" => {
            println!("=== Level Showcase Generator ===\n");
            generate::levels::run();
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
                }
                "baseball" => {
                    println!("=== Baseball GIF Generator ===\n");
                    generate::gif_baseball::run();
                }
                other => {
                    eprintln!("Error: Unknown GIF type '{}'. Use 'wedge' or 'baseball'.", other);
                    std::process::exit(1);
                }
            }
        }
        "--help" | "-h" | "help" => {
            print_help();
        }
        other => {
            eprintln!("Error: Unknown command '{}'\n", other);
            print_help();
            std::process::exit(1);
        }
    }
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
