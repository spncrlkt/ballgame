//! AI Simulation module - headless game simulation for AI testing
//!
//! Provides tools to run the game without rendering, collecting metrics
//! on AI behavior, performance, and decision-making.

pub mod app_builder;
pub mod config;
pub mod control;
pub mod db;
pub mod metrics;
pub mod parallel;
pub mod runner;
pub mod setup;
pub mod shot_test;

pub use app_builder::HeadlessAppBuilder;
pub use config::{SimConfig, SimMode};
pub use control::{SimControl, SimEventBuffer};
pub use metrics::{MatchResult, PlayerStats, SimMetrics, TournamentResult};
pub use runner::{run_match, run_simulation};
pub use setup::{sim_setup, spawn_corner_steps};
pub use shot_test::{run_shot_test, ShotOutcome};
pub use db::{SimDatabase, ProfileStats, MatchFilter, MatchSummary};
