//! AI Simulation module - headless game simulation for AI testing
//!
//! Provides tools to run the game without rendering, collecting metrics
//! on AI behavior, performance, and decision-making.

pub mod config;
pub mod metrics;
pub mod runner;

pub use config::{SimConfig, SimMode};
pub use metrics::{MatchResult, PlayerStats, SimMetrics, TournamentResult};
pub use runner::{run_match, run_simulation};
