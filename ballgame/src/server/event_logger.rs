//! Server event logging to SQLite
//!
//! Logs game events to a SQLite database for later analysis and replay.

use bevy::prelude::*;
use std::path::Path;

use crate::db_paths::{self, DbType};
use crate::events::SqliteEventLogger;
use crate::levels::LevelDatabase;
use crate::scoring::CurrentLevel;
use crate::Score;

use super::assignment::CharacterAssignments;
use super::tournament::TournamentConfig;
use super::lobby::LobbyState;

/// Tracks whether we were in lobby last frame (for transition detection)
#[derive(Resource, Default)]
pub struct ServerLoggingState {
    /// Was lobby active last frame?
    pub was_in_lobby: bool,
    /// Is a match currently being logged?
    pub match_active: bool,
}

/// Resource that wraps the SQLite logger for server mode
#[derive(Resource)]
pub struct ServerEventLogger {
    /// The underlying SQLite logger
    pub logger: SqliteEventLogger,
    /// Path to the database file
    pub db_path: String,
}

impl ServerEventLogger {
    /// Create a new server event logger
    ///
    /// Returns None if logging fails to initialize (game can continue without logging)
    pub fn new() -> Option<Self> {
        // Ensure db directory exists
        if let Err(e) = db_paths::ensure_dir() {
            warn!("Failed to create db directory: {}", e);
            return None;
        }

        let db_path = db_paths::timestamped(DbType::Server);
        let path = Path::new(&db_path);

        match SqliteEventLogger::new(path, "server") {
            Ok(logger) => {
                info!("Server SQLite logger initialized: {}", db_path);
                Some(Self { logger, db_path })
            }
            Err(e) => {
                warn!("Failed to create server SQLite logger: {}", e);
                None
            }
        }
    }

    /// Start a new match
    pub fn start_match(
        &self,
        level_index: u32,
        level_name: &str,
        assignments: &CharacterAssignments,
    ) -> Option<i64> {
        // Generate a seed from current time
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);

        // Get profile names for left and right teams
        let left_profile = assignments
            .get_profile_name(crate::CharacterId::L0)
            .unwrap_or_else(|| "Human".to_string());
        let right_profile = assignments
            .get_profile_name(crate::CharacterId::R0)
            .unwrap_or_else(|| "Human".to_string());

        let match_id = self.logger.start_match(
            level_index,
            level_name,
            &left_profile,
            &right_profile,
            seed,
        );

        if let Some(id) = match_id {
            // Set game mode to 2v2
            self.logger.set_game_mode("2v2");

            // Record character assignments
            self.record_character_assignments(assignments);

            info!(
                "Started server match {} on level {} ({} vs {})",
                id, level_name, left_profile, right_profile
            );
        }

        match_id
    }

    /// Record character assignments for the current match
    fn record_character_assignments(&self, assignments: &CharacterAssignments) {
        use crate::CharacterId;

        for char_id in CharacterId::all() {
            let assignment = assignments.get(char_id);
            let (source_id, profile_name) = match assignment {
                super::assignment::CharacterAssignment::Local { source_id, .. } => {
                    (Some(*source_id as u32), None)
                }
                super::assignment::CharacterAssignment::Remote { client_id, .. } => {
                    (Some(*client_id as u32), None)
                }
                super::assignment::CharacterAssignment::ServerAi { profile_name } => {
                    (None, Some(profile_name.as_str()))
                }
                super::assignment::CharacterAssignment::Unassigned => {
                    (None, Some("Dummy"))
                }
            };

            self.logger.record_match_character(
                &char_id.to_string(),
                source_id,
                profile_name,
            );
        }
    }

    /// End the current match
    pub fn end_match(&self, score: &Score, duration_secs: f32) {
        self.logger.end_match(score.left, score.right, duration_secs);
        info!(
            "Ended server match (score: {}-{}, duration: {:.1}s)",
            score.left, score.right, duration_secs
        );
    }
}

/// System to detect lobby transitions and start/end match logging
///
/// Runs every frame and detects:
/// - Lobby -> Game: Start a new match in the database
/// - Game -> Lobby: End the current match in the database
pub fn track_match_logging(
    logger: Option<Res<ServerEventLogger>>,
    mut logging_state: ResMut<ServerLoggingState>,
    lobby_state: Option<Res<LobbyState>>,
    assignments: Res<CharacterAssignments>,
    current_level: Res<CurrentLevel>,
    level_db: Res<LevelDatabase>,
    score: Res<Score>,
    config: Res<TournamentConfig>,
) {
    let in_lobby = lobby_state.map(|l| l.active).unwrap_or(false);

    // Detect lobby -> game transition (start match)
    if logging_state.was_in_lobby && !in_lobby && !logging_state.match_active {
        if let Some(ref logger) = logger {
            // Get level info
            let level_data = level_db.get_by_id(&current_level.0);
            let level_name = level_data.map(|l| l.name.as_str()).unwrap_or("Unknown");
            let level_index = level_db
                .all()
                .iter()
                .position(|l| l.id == current_level.0)
                .map(|i| i as u32 + 1)
                .unwrap_or(1);

            logger.start_match(level_index, level_name, &assignments);
            logging_state.match_active = true;
        }
    }

    // Detect game -> lobby transition (end match)
    if !logging_state.was_in_lobby && in_lobby && logging_state.match_active {
        if let Some(ref logger) = logger {
            logger.end_match(&score, config.match_elapsed_secs);
            logging_state.match_active = false;
        }
    }

    logging_state.was_in_lobby = in_lobby;
}

/// System to flush events from EventBus to SQLite
pub fn flush_server_events(
    mut event_bus: ResMut<crate::events::EventBus>,
    logger: Option<Res<ServerEventLogger>>,
) {
    let Some(logger) = logger else {
        // Still drain to prevent buildup
        let _ = event_bus.export_events();
        return;
    };

    if !logger.logger.is_enabled() {
        let _ = event_bus.export_events();
        return;
    }

    let events = event_bus.export_events();
    if !events.is_empty() {
        logger.logger.log_events(&events);
    }
}
