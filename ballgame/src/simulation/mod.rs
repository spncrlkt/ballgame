//! AI Simulation module - headless game simulation for AI testing
//!
//! Provides tools to run the game without rendering, collecting metrics
//! on AI behavior, performance, and decision-making.
//!
//! ## Participant Types
//!
//! Simulations can run with two types of AI participants:
//!
//! - **Profiles**: Embedded AI using profiles from `AiProfileDatabase`. Fast path,
//!   no network overhead. Configured via `--left`/`--right` flags.
//!
//! - **Clients**: External AI clients that connect via WebSocket. Supports testing
//!   different AI architectures (v1, v2, etc.) against each other. Configured via
//!   `--left-team`/`--right-team` flags.
//!
//! ## 2v2 Format
//!
//! All matches are 2v2 with 4 participant slots:
//! - L0, L1: Left team (slots 0, 1)
//! - R0, R1: Right team (slots 2, 3)
//!
//! When using profile-based configuration (`--left Balanced --right Aggressive`),
//! each profile fills both slots on its team.

pub mod app_builder;
pub mod bracket;
pub mod client_db;
pub mod config;
pub mod control;
pub mod db;
pub mod ghost;
pub mod metrics;
pub mod multihop_test;
pub mod orchestrator;
pub mod parallel;
pub mod participant;
pub mod reachability_test;
pub mod runner;
pub mod setup;
pub mod shot_test;

pub use app_builder::HeadlessAppBuilder;
pub use bracket::{
    BracketEntry, BracketExecutor, BracketMatch, BracketMatchResult, BracketSeedingConfig,
    BracketSide, BracketState, GameResult, LossCount, MatchFormat, Placement, SeedingMethod,
    WarmupResult, format_standings, pad_to_power_of_2, seed_entries, select_profiles,
    warmup_seeding,
};
pub use config::{SimConfig, SimMode};
pub use control::{SimControl, SimEventBuffer};
pub use db::{
    ClosestMoment,
    // Analysis types
    ClientWinRate,
    DistanceAnalysis,
    EventRecord,
    GoalTransition,
    InputAnalysis,
    MatchEventStats,
    MatchFilter,
    MatchSummary,
    ParticipantData,
    ProfileStats,
    SessionSummary,
    SimDatabase,
    TeamCompositionStats,
};
pub use ghost::{
    GhostOutcome, GhostPlaybackState, GhostTrial, GhostTrialResult, InputSample,
    ghost_check_end_conditions, ghost_input_system, load_ghost_trial, max_tick,
};
pub use metrics::{MatchResult, PlayerStats, SimMetrics, TournamentResult};
pub use runner::{requires_orchestrator, run_match, run_simulation};
pub use setup::{level_geometry_setup, sim_setup, spawn_corner_steps};
pub use shot_test::{ShotOutcome, run_shot_test};

// Participant types for AI client support
pub use client_db::{AiClientDatabase, AiClientInfo, AI_CLIENTS_FILE};
pub use orchestrator::{
    MatchOrchestrator, MatchSetup, OrchError, OrchestratorConfig, resolve_participants,
    teardown_match,
};
pub use participant::{AiParticipant, MatchParticipants, TeamParticipants};
