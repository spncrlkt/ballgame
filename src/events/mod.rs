//! Game event logging system for analytics
//!
//! Provides a compact text format for logging all game events and inputs.
//! Used by AI simulation, gameplay sessions, and analytics pipelines.
//!
//! The EventBus enables decoupled cross-module communication where all events
//! are logged to SQLite for full auditability.

mod bus;
mod emitter;
pub mod evlog_parser;
mod format;
mod logger;
mod types;

pub use bus::{BusEvent, EventBus, LevelChangeTracker, emit_level_change_events, update_event_bus_time};
pub use emitter::{
    emit_game_events, snapshot_ball, snapshot_player, BallSnapshot, EmitterConfig,
    EventEmitterState, PlayerSnapshot,
};
pub use evlog_parser::{
    parse_evlog, parse_evlog_content, parse_all_evlogs, ParsedEvlog, MatchMetadata,
    TickData, GoalData, ShotData, PickupData, DropData, StealAttemptData,
    StealSuccessData, StealFailData, AiGoalData, TimestampedEvent,
};
pub use format::{parse_event, serialize_event};
pub use logger::{EventBuffer, EventLogConfig, EventLogger};
pub use types::{ControllerSource, GameConfig, GameEvent, PlayerId};
