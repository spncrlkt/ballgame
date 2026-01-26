//! SQLite Event Logger - central hub for event storage
//!
//! All game events flow through this logger to SQLite.
//! This enables SQL-based analysis without file parsing.

use bevy::prelude::*;
use rusqlite::{Connection, params};
use std::path::Path;
use std::sync::Mutex;

use super::format::serialize_event;
use super::types::GameEvent;

/// Resource for logging events to SQLite
///
/// All binaries (main, training, simulate, test_scenarios) use this resource
/// to write events to a central SQLite database.
///
/// The database connection is wrapped in a Mutex for thread safety.
#[derive(Resource)]
pub struct SqliteEventLogger {
    /// The database connection wrapped in a Mutex for thread safety
    conn: Mutex<Connection>,
    session_id: String,
    current_match_id: Mutex<Option<i64>>,
    current_point_id: Mutex<Option<i64>>,
    current_point_index: Mutex<u32>,
    /// Whether logging is enabled
    enabled: bool,
}

impl SqliteEventLogger {
    /// Create a new SQLite event logger
    ///
    /// # Arguments
    /// * `db_path` - Path to the SQLite database file
    /// * `session_type` - Type of session (e.g., "training", "game", "simulation")
    ///
    /// # Returns
    /// Result with the logger or a database error
    pub fn new(db_path: &Path, session_type: &str) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(db_path)?;

        // Enable WAL mode for concurrent reads during writes
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;

        // Initialize schema
        init_schema(&conn)?;

        // Create session
        let session_id = create_session(&conn, session_type)?;

        Ok(Self {
            conn: Mutex::new(conn),
            session_id,
            current_match_id: Mutex::new(None),
            current_point_id: Mutex::new(None),
            current_point_index: Mutex::new(0),
            enabled: true,
        })
    }

    /// Create a disabled logger (no-op, for testing)
    pub fn disabled() -> Self {
        // Use in-memory database that won't be accessed
        let conn = Connection::open_in_memory().expect("Failed to create in-memory database");
        Self {
            conn: Mutex::new(conn),
            session_id: String::new(),
            current_match_id: Mutex::new(None),
            current_point_id: Mutex::new(None),
            current_point_index: Mutex::new(0),
            enabled: false,
        }
    }

    /// Start a new match and return its ID
    ///
    /// # Arguments
    /// * `level` - Level number
    /// * `level_name` - Level name
    /// * `left_profile` - Left player's AI profile name
    /// * `right_profile` - Right player's AI profile name
    /// * `seed` - Random seed for the match
    pub fn start_match(
        &self,
        level: u32,
        level_name: &str,
        left_profile: &str,
        right_profile: &str,
        seed: u64,
    ) -> Option<i64> {
        if !self.enabled {
            return None;
        }

        let conn = self.conn.lock().ok()?;
        let display_name = short_uuid();

        // Insert match with placeholder score/duration (will be updated at end)
        let result = conn.execute(
            r#"INSERT INTO matches
               (session_id, display_name, seed, level, level_name, left_profile, right_profile,
                score_left, score_right, duration_secs, winner)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, 0, 0.0, '')"#,
            params![
                self.session_id,
                display_name,
                seed as i64,
                level,
                level_name,
                left_profile,
                right_profile,
            ],
        );

        match result {
            Ok(_) => {
                let match_id = conn.last_insert_rowid();
                *self.current_match_id.lock().ok()? = Some(match_id);
                if let Some(point_id) = insert_point(&conn, match_id, 1, 0).ok() {
                    *self.current_point_id.lock().ok()? = Some(point_id);
                    *self.current_point_index.lock().ok()? = 1;
                }
                info!("Started match {} (level: {}, profiles: {} vs {})",
                    match_id, level_name, left_profile, right_profile);
                Some(match_id)
            }
            Err(e) => {
                warn!("Failed to start match: {}", e);
                None
            }
        }
    }

    /// Log a single event
    pub fn log_event(&self, time_ms: u32, event: &GameEvent) {
        if !self.enabled {
            return;
        }

        let match_id = match self.current_match_id.lock() {
            Ok(guard) => match *guard {
                Some(id) => id,
                None => return,
            },
            Err(_) => return,
        };
        let point_id = self.current_point_id.lock().ok().and_then(|g| *g);

        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(_) => return,
        };

        // Serialize event to the compact text format
        let data = serialize_event(time_ms, event);
        let event_type = event.type_code();

        if let Err(e) = conn.execute(
            "INSERT INTO events (match_id, point_id, time_ms, event_type, data) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![match_id, point_id, time_ms, event_type, data],
        ) {
            warn!("Failed to log event: {}", e);
            return;
        }

        if let GameEvent::Goal { player, .. } = event {
            if let Err(e) = end_point_for_goal(&conn, self, match_id, time_ms, *player) {
                warn!("Failed to finalize point on goal: {}", e);
            }
        }
    }

    /// Log multiple events at once (more efficient for batch logging)
    pub fn log_events(&self, events: &[(u32, GameEvent)]) {
        if !self.enabled || events.is_empty() {
            return;
        }

        let match_id = match self.current_match_id.lock() {
            Ok(guard) => match *guard {
                Some(id) => id,
                None => return,
            },
            Err(_) => return,
        };
        let mut point_id = match self.current_point_id.lock() {
            Ok(guard) => *guard,
            Err(_) => None,
        };

        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(_) => return,
        };

        // Use a transaction for batch insert
        if conn.execute("BEGIN TRANSACTION", []).is_err() {
            return;
        }

        for (time_ms, event) in events {
            let data = serialize_event(*time_ms, event);
            let event_type = event.type_code();

            if conn.execute(
                "INSERT INTO events (match_id, point_id, time_ms, event_type, data) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![match_id, point_id, time_ms, event_type, data],
            ).is_err() {
                let _ = conn.execute("ROLLBACK", []);
                return;
            }

            if let GameEvent::Goal { player, .. } = event {
                if let Err(e) = end_point_for_goal(&conn, self, match_id, *time_ms, *player) {
                    warn!("Failed to finalize point on goal: {}", e);
                    let _ = conn.execute("ROLLBACK", []);
                    return;
                }
                point_id = self.current_point_id.lock().ok().and_then(|g| *g);
            }
        }

        let _ = conn.execute("COMMIT", []);
    }

    /// End the current match and record final scores
    pub fn end_match(&self, score_left: u32, score_right: u32, duration_secs: f32) {
        if !self.enabled {
            return;
        }

        let match_id = match self.current_match_id.lock() {
            Ok(guard) => match *guard {
                Some(id) => id,
                None => return,
            },
            Err(_) => return,
        };

        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(_) => return,
        };

        let winner = if score_left > score_right {
            "left"
        } else if score_right > score_left {
            "right"
        } else {
            "tie"
        };

        let result = conn.execute(
            "UPDATE matches SET score_left = ?1, score_right = ?2, duration_secs = ?3, winner = ?4 WHERE id = ?5",
            params![score_left, score_right, duration_secs, winner, match_id],
        );

        if let Err(e) = result {
            warn!("Failed to end match: {}", e);
        } else {
            info!("Ended match {} (score: {}-{}, duration: {:.1}s)",
                match_id, score_left, score_right, duration_secs);
        }

        if let Some(point_id) = self.current_point_id.lock().ok().and_then(|g| *g) {
            let end_time_ms = (duration_secs * 1000.0).round() as u32;
            if let Err(e) = end_point(&conn, point_id, end_time_ms, "none") {
                warn!("Failed to end final point: {}", e);
            }
        }

        // Clear current match
        if let Ok(mut guard) = self.current_match_id.lock() {
            *guard = None;
        }
        if let Ok(mut guard) = self.current_point_id.lock() {
            *guard = None;
        }
        if let Ok(mut guard) = self.current_point_index.lock() {
            *guard = 0;
        }
    }

    /// Get the current match ID (if a match is in progress)
    pub fn current_match_id(&self) -> Option<i64> {
        self.current_match_id.lock().ok().and_then(|g| *g)
    }

    /// Get the session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Check if logging is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Enable or disable logging
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Get event count for the current match
    pub fn event_count(&self) -> Option<u64> {
        let match_id = self.current_match_id()?;
        let conn = self.conn.lock().ok()?;
        conn.query_row(
            "SELECT COUNT(*) FROM events WHERE match_id = ?1",
            params![match_id],
            |row| row.get(0),
        ).ok()
    }
}

/// Initialize the database schema
fn init_schema(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            created_at TEXT NOT NULL,
            session_type TEXT NOT NULL,
            config_json TEXT,
            display_name TEXT
        );

        CREATE TABLE IF NOT EXISTS matches (
            id INTEGER PRIMARY KEY,
            session_id TEXT REFERENCES sessions(id),
            display_name TEXT,
            seed INTEGER NOT NULL,
            level INTEGER NOT NULL,
            level_name TEXT NOT NULL,
            left_profile TEXT NOT NULL,
            right_profile TEXT NOT NULL,
            score_left INTEGER NOT NULL,
            score_right INTEGER NOT NULL,
            duration_secs REAL NOT NULL,
            winner TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS points (
            id INTEGER PRIMARY KEY,
            match_id INTEGER REFERENCES matches(id),
            point_index INTEGER NOT NULL,
            start_time_ms INTEGER NOT NULL,
            end_time_ms INTEGER,
            winner TEXT
        );

        CREATE TABLE IF NOT EXISTS player_stats (
            id INTEGER PRIMARY KEY,
            match_id INTEGER REFERENCES matches(id),
            side TEXT NOT NULL,
            goals INTEGER NOT NULL,
            shots_attempted INTEGER NOT NULL,
            shots_made INTEGER NOT NULL,
            steals_attempted INTEGER NOT NULL,
            steals_successful INTEGER NOT NULL,
            possession_time REAL NOT NULL,
            distance_traveled REAL NOT NULL,
            jumps INTEGER NOT NULL,
            nav_paths_completed INTEGER NOT NULL,
            nav_paths_failed INTEGER NOT NULL,
            avg_shot_x REAL NOT NULL DEFAULT 0.0,
            avg_shot_y REAL NOT NULL DEFAULT 0.0,
            avg_shot_quality REAL NOT NULL DEFAULT 0.0
        );

        CREATE INDEX IF NOT EXISTS idx_matches_session ON matches(session_id);
        CREATE INDEX IF NOT EXISTS idx_matches_profiles ON matches(left_profile, right_profile);
        CREATE INDEX IF NOT EXISTS idx_matches_level ON matches(level);
        CREATE INDEX IF NOT EXISTS idx_player_stats_match ON player_stats(match_id);

        -- Event bus events table for full auditability
        CREATE TABLE IF NOT EXISTS events (
            id INTEGER PRIMARY KEY,
            match_id INTEGER REFERENCES matches(id),
            point_id INTEGER REFERENCES points(id),
            time_ms INTEGER NOT NULL,
            event_type TEXT NOT NULL,
            data TEXT NOT NULL,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP
        );

        CREATE INDEX IF NOT EXISTS idx_events_match ON events(match_id);
        CREATE INDEX IF NOT EXISTS idx_events_point ON events(point_id);
        CREATE INDEX IF NOT EXISTS idx_events_type ON events(event_type);
        CREATE INDEX IF NOT EXISTS idx_events_time ON events(match_id, time_ms);
        CREATE INDEX IF NOT EXISTS idx_points_match ON points(match_id);
        "#,
    )?;

    let _ = conn.execute("ALTER TABLE sessions ADD COLUMN display_name TEXT", []);
    let _ = conn.execute("ALTER TABLE matches ADD COLUMN display_name TEXT", []);
    let _ = conn.execute("ALTER TABLE events ADD COLUMN point_id INTEGER", []);
    Ok(())
}

/// Create a new session and return its ID
fn create_session(conn: &Connection, session_type: &str) -> Result<String, rusqlite::Error> {
    let id = uuid::Uuid::new_v4();
    let id_str = id.to_string();
    let display_name = id.simple().to_string()[..16].to_string();
    let created_at = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO sessions (id, created_at, session_type, config_json, display_name) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id_str, created_at, session_type, Option::<String>::None, display_name],
    )?;

    Ok(id_str)
}

fn short_uuid() -> String {
    let full = uuid::Uuid::new_v4().simple().to_string();
    full[..16].to_string()
}

fn insert_point(conn: &Connection, match_id: i64, point_index: u32, start_time_ms: u32) -> Result<i64, rusqlite::Error> {
    conn.execute(
        r#"INSERT INTO points (match_id, point_index, start_time_ms, end_time_ms, winner)
           VALUES (?1, ?2, ?3, NULL, NULL)"#,
        params![match_id, point_index, start_time_ms],
    )?;
    Ok(conn.last_insert_rowid())
}

fn end_point(conn: &Connection, point_id: i64, end_time_ms: u32, winner: &str) -> Result<(), rusqlite::Error> {
    conn.execute(
        "UPDATE points SET end_time_ms = ?1, winner = ?2 WHERE id = ?3",
        params![end_time_ms, winner, point_id],
    )?;
    Ok(())
}

fn end_point_for_goal(
    conn: &Connection,
    logger: &SqliteEventLogger,
    match_id: i64,
    time_ms: u32,
    player: super::types::PlayerId,
) -> Result<(), rusqlite::Error> {
    let winner = match player {
        super::types::PlayerId::L => "left",
        super::types::PlayerId::R => "right",
    };
    if let Some(point_id) = logger.current_point_id.lock().ok().and_then(|g| *g) {
        end_point(conn, point_id, time_ms, winner)?;
    }
    let next_index = if let Ok(mut guard) = logger.current_point_index.lock() {
        *guard += 1;
        *guard
    } else {
        return Ok(());
    };
    let next_point_id = insert_point(conn, match_id, next_index, time_ms)?;
    if let Ok(mut guard) = logger.current_point_id.lock() {
        *guard = Some(next_point_id);
    }
    Ok(())
}

/// System to flush EventBus events to SQLite
///
/// This bridges the EventBus (in-memory events) to SQLite storage.
/// Call this system periodically to persist events.
pub fn flush_events_to_sqlite(
    mut event_bus: ResMut<super::bus::EventBus>,
    logger: Option<Res<SqliteEventLogger>>,
) {
    let Some(logger) = logger else {
        return;
    };

    if !logger.is_enabled() {
        // Still drain the bus to prevent buildup
        let _ = event_bus.export_events();
        return;
    }

    let events = event_bus.export_events();
    if !events.is_empty() {
        logger.log_events(&events);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::types::PlayerId;

    fn create_test_logger() -> SqliteEventLogger {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();
        let session_id = create_session(&conn, "test").unwrap();
        SqliteEventLogger {
            conn: Mutex::new(conn),
            session_id,
            current_match_id: Mutex::new(None),
            current_point_id: Mutex::new(None),
            current_point_index: Mutex::new(0),
            enabled: true,
        }
    }

    #[test]
    fn test_start_and_end_match() {
        let logger = create_test_logger();

        let match_id = logger.start_match(1, "Test Level", "Human", "AI", 12345);
        assert!(match_id.is_some());
        assert!(logger.current_match_id().is_some());

        logger.end_match(3, 2, 45.5);
        assert!(logger.current_match_id().is_none());
    }

    #[test]
    fn test_log_events() {
        let logger = create_test_logger();
        logger.start_match(1, "Test Level", "Human", "AI", 12345);

        // Log some events
        logger.log_event(100, &GameEvent::Pickup { player: PlayerId::L });
        logger.log_event(200, &GameEvent::Goal {
            player: PlayerId::L,
            score_left: 1,
            score_right: 0,
        });

        // Verify events were logged
        let count = logger.event_count().unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_batch_log_events() {
        let logger = create_test_logger();
        logger.start_match(1, "Test Level", "Human", "AI", 12345);

        let events = vec![
            (100, GameEvent::Pickup { player: PlayerId::L }),
            (150, GameEvent::ShotStart {
                player: PlayerId::L,
                pos: (-200.0, -350.0),
                quality: 0.8,
            }),
            (200, GameEvent::ShotRelease {
                player: PlayerId::L,
                charge: 0.7,
                angle: 45.0,
                power: 600.0,
            }),
        ];

        logger.log_events(&events);

        let count = logger.event_count().unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn test_disabled_logger() {
        let logger = SqliteEventLogger::disabled();
        assert!(!logger.is_enabled());

        let match_id = logger.start_match(1, "Test", "A", "B", 0);
        assert!(match_id.is_none());

        // Should not panic
        logger.log_event(0, &GameEvent::ResetScores);
        logger.end_match(0, 0, 0.0);
    }
}
