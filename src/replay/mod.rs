//! Replay system for playing back recorded game sessions.
//!
//! The replay system loads recorded sessions from SQLite and plays them back
//! with interpolated positions, variable speed control, and behavior observation overlays.

mod data;
mod sqlite_loader;
mod state;
mod systems;
mod ui;

pub use data::{ReplayData, TickFrame, TimedEvent};
pub use sqlite_loader::load_replay_from_db;
pub use state::ReplayState;
pub use systems::{replay_input_handler, replay_playback, replay_setup};
pub use ui::{
    PlayerGoalLabel, ReplayEventMarker, ReplaySpeedDisplay, ReplayTimeDisplay, ReplayTimeline,
    setup_replay_ui, update_replay_ui,
};

use bevy::prelude::*;

/// Resource to control replay mode activation
#[derive(Resource, Default)]
pub struct ReplayMode {
    /// Whether replay mode is active
    pub active: bool,
    /// Match ID for SQLite replay
    pub match_id: Option<i64>,
}

impl ReplayMode {
    pub fn new_db(match_id: i64) -> Self {
        Self {
            active: true,
            match_id: Some(match_id),
        }
    }
}

/// Run condition: returns true when replay mode is active
pub fn replay_active(replay_mode: Res<ReplayMode>) -> bool {
    replay_mode.active
}

/// Run condition: returns true when replay mode is NOT active (normal game)
pub fn not_replay_active(replay_mode: Res<ReplayMode>) -> bool {
    !replay_mode.active
}

/// Match info extracted from MatchStart event
#[derive(Debug, Clone, Default)]
pub struct MatchInfo {
    pub level: u32,
    pub level_name: String,
    pub left_profile: String,
    pub right_profile: String,
    pub seed: u64,
}
