//! AI Simulation Tool - Headless game simulation for AI testing
//!
//! Run AI vs AI matches without rendering to collect performance metrics.
//!
//! Usage:
//!   cargo run --bin simulate -- --help
//!   cargo run --bin simulate -- --level 3 --left Balanced --right Aggressive
//!   cargo run --bin simulate -- --tournament 10
//!   cargo run --bin simulate -- --level-sweep 5 --left Sniper

use ballgame::simulation::{SimConfig, run_simulation};

fn main() {
    let config = SimConfig::from_args();
    run_simulation(config);
}
