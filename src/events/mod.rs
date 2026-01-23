//! Game event logging system for analytics
//!
//! Provides a compact text format for logging all game events and inputs.
//! Used by AI simulation, gameplay sessions, and analytics pipelines.

mod format;
mod logger;
mod types;

pub use format::{parse_event, serialize_event};
pub use logger::{EventBuffer, EventLogConfig, EventLogger};
pub use types::{GameConfig, GameEvent, PlayerId};
