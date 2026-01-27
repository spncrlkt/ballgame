//! Game event logging system for analytics
//!
//! Provides a compact text format for logging all game events and inputs.
//! Used by AI simulation, gameplay sessions, and analytics pipelines.
//!
//! The EventBus enables decoupled cross-module communication where all events
//! are logged to SQLite for full auditability.
//!
//! ## Architecture
//!
//! ```text
//! EventBus (in-memory) --> SqliteEventLogger --> SQLite database
//!                                                    |
//!                                                    v
//!                                            SQL analysis views
//! ```

mod buffer;
mod bus;
mod emitter;
mod format;
mod sqlite_logger;
mod types;

pub use buffer::EventBuffer;
pub use bus::{
    BusEvent, EventBus, LevelChangeTracker, emit_level_change_events, update_event_bus_time,
};
pub use emitter::{
    BallSnapshot, EmitterConfig, EventEmitterState, PlayerSnapshot, emit_game_events,
    snapshot_ball, snapshot_player,
};
pub use format::{parse_event, serialize_event};
pub use sqlite_logger::{SqliteEventLogger, flush_events_to_sqlite};
pub use types::{ControllerSource, GameConfig, GameEvent, PlayerId};
