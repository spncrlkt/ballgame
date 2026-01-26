//! SQLite database for simulation results
//!
//! Provides persistent storage and querying of simulation results.
//! Uses WAL mode for concurrent reads during writes.

use rusqlite::{Connection, Result, params};
use std::path::Path;

use super::metrics::{MatchResult, PlayerStats};

/// Database wrapper for simulation results
pub struct SimDatabase {
    conn: Connection,
}

impl SimDatabase {
    /// Open or create a database at the given path
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;

        // Enable WAL mode for concurrent reads during writes
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;

        // Set busy timeout for parallel access
        conn.busy_timeout(std::time::Duration::from_secs(5))?;

        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    /// Open an in-memory database (for testing)
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    /// Initialize the database schema
    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                created_at TEXT NOT NULL,
                session_type TEXT NOT NULL,
                config_json TEXT
            );

            CREATE TABLE IF NOT EXISTS matches (
                id INTEGER PRIMARY KEY,
                session_id TEXT REFERENCES sessions(id),
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
                time_ms INTEGER NOT NULL,
                event_type TEXT NOT NULL,
                data TEXT NOT NULL,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP
            );

            CREATE INDEX IF NOT EXISTS idx_events_match ON events(match_id);
            CREATE INDEX IF NOT EXISTS idx_events_type ON events(event_type);
            CREATE INDEX IF NOT EXISTS idx_events_time ON events(match_id, time_ms);
            "#,
        )?;
        Ok(())
    }

    /// Create a new session and return its ID
    pub fn create_session(&self, session_type: &str, config_json: Option<&str>) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let created_at = chrono::Utc::now().to_rfc3339();

        self.conn.execute(
            "INSERT INTO sessions (id, created_at, session_type, config_json) VALUES (?1, ?2, ?3, ?4)",
            params![id, created_at, session_type, config_json],
        )?;

        Ok(id)
    }

    /// Insert a match result and return the match ID
    pub fn insert_match(&self, session_id: &str, result: &MatchResult) -> Result<i64> {
        self.conn.execute(
            r#"INSERT INTO matches
               (session_id, seed, level, level_name, left_profile, right_profile,
                score_left, score_right, duration_secs, winner)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"#,
            params![
                session_id,
                result.seed as i64,
                result.level,
                result.level_name,
                result.left_profile,
                result.right_profile,
                result.score_left,
                result.score_right,
                result.duration,
                result.winner,
            ],
        )?;

        let match_id = self.conn.last_insert_rowid();

        // Insert player stats
        self.insert_player_stats(match_id, "left", &result.left_stats)?;
        self.insert_player_stats(match_id, "right", &result.right_stats)?;

        Ok(match_id)
    }

    /// Insert player stats for a match
    fn insert_player_stats(&self, match_id: i64, side: &str, stats: &PlayerStats) -> Result<()> {
        self.conn.execute(
            r#"INSERT INTO player_stats
               (match_id, side, goals, shots_attempted, shots_made, steals_attempted,
                steals_successful, possession_time, distance_traveled, jumps,
                nav_paths_completed, nav_paths_failed, avg_shot_x, avg_shot_y, avg_shot_quality)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)"#,
            params![
                match_id,
                side,
                stats.goals,
                stats.shots_attempted,
                stats.shots_made,
                stats.steals_attempted,
                stats.steals_successful,
                stats.possession_time,
                stats.distance_traveled,
                stats.jumps,
                stats.nav_paths_completed,
                stats.nav_paths_failed,
                stats.avg_shot_x,
                stats.avg_shot_y,
                stats.avg_shot_quality,
            ],
        )?;
        Ok(())
    }

    /// Get aggregate stats for a profile
    pub fn get_profile_stats(&self, profile: &str) -> Result<ProfileStats> {
        let mut stmt = self.conn.prepare(
            r#"SELECT
                COUNT(*) as matches,
                SUM(CASE WHEN winner = 'left' AND left_profile = ?1 THEN 1
                         WHEN winner = 'right' AND right_profile = ?1 THEN 1 ELSE 0 END) as wins,
                SUM(CASE WHEN winner = 'tie' THEN 1 ELSE 0 END) as ties,
                AVG(CASE WHEN left_profile = ?1 THEN score_left ELSE score_right END) as avg_score,
                AVG(CASE WHEN left_profile = ?1 THEN score_right ELSE score_left END) as avg_opp_score
               FROM matches
               WHERE left_profile = ?1 OR right_profile = ?1"#,
        )?;

        let result = stmt.query_row(params![profile], |row| {
            Ok(ProfileStats {
                profile: profile.to_string(),
                matches: row.get(0)?,
                wins: row.get(1)?,
                ties: row.get(2)?,
                avg_score: row.get(3)?,
                avg_opponent_score: row.get(4)?,
            })
        })?;

        Ok(result)
    }

    /// Get match results with optional filtering
    pub fn query_matches(&self, filter: &MatchFilter) -> Result<Vec<MatchSummary>> {
        let mut sql = String::from(
            "SELECT id, level, level_name, left_profile, right_profile,
                    score_left, score_right, duration_secs, winner
             FROM matches WHERE 1=1"
        );

        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(profile) = &filter.profile {
            sql.push_str(" AND (left_profile = ? OR right_profile = ?)");
            params.push(Box::new(profile.clone()));
            params.push(Box::new(profile.clone()));
        }

        if let Some(level) = filter.level {
            sql.push_str(" AND level = ?");
            params.push(Box::new(level as i32));
        }

        if let Some(limit) = filter.limit {
            sql.push_str(" LIMIT ?");
            params.push(Box::new(limit as i32));
        }

        let mut stmt = self.conn.prepare(&sql)?;

        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(param_refs.as_slice(), |row| {
            Ok(MatchSummary {
                id: row.get(0)?,
                level: row.get(1)?,
                level_name: row.get(2)?,
                left_profile: row.get(3)?,
                right_profile: row.get(4)?,
                score_left: row.get(5)?,
                score_right: row.get(6)?,
                duration: row.get(7)?,
                winner: row.get(8)?,
            })
        })?;

        rows.collect()
    }

    /// Get match count
    pub fn match_count(&self) -> Result<u64> {
        self.conn.query_row(
            "SELECT COUNT(*) FROM matches",
            [],
            |row| row.get(0),
        )
    }

    /// Get session count
    pub fn session_count(&self) -> Result<u64> {
        self.conn.query_row(
            "SELECT COUNT(*) FROM sessions",
            [],
            |row| row.get(0),
        )
    }

    /// Insert a batch of events for a match
    pub fn insert_events(&self, match_id: i64, events: &[(u32, &str, &str)]) -> Result<()> {
        let mut stmt = self.conn.prepare(
            "INSERT INTO events (match_id, time_ms, event_type, data) VALUES (?1, ?2, ?3, ?4)"
        )?;

        for (time_ms, event_type, data) in events {
            stmt.execute(params![match_id, time_ms, event_type, data])?;
        }

        Ok(())
    }

    /// Insert a single event for a match
    pub fn insert_event(&self, match_id: i64, time_ms: u32, event_type: &str, data: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO events (match_id, time_ms, event_type, data) VALUES (?1, ?2, ?3, ?4)",
            params![match_id, time_ms, event_type, data],
        )?;
        Ok(())
    }

    /// Get events for a match
    pub fn get_events(&self, match_id: i64) -> Result<Vec<EventRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, time_ms, event_type, data FROM events WHERE match_id = ?1 ORDER BY time_ms"
        )?;

        let rows = stmt.query_map(params![match_id], |row| {
            Ok(EventRecord {
                id: row.get(0)?,
                time_ms: row.get(1)?,
                event_type: row.get(2)?,
                data: row.get(3)?,
            })
        })?;

        rows.collect()
    }

    /// Get event count for a match
    pub fn event_count(&self, match_id: i64) -> Result<u64> {
        self.conn.query_row(
            "SELECT COUNT(*) FROM events WHERE match_id = ?1",
            params![match_id],
            |row| row.get(0),
        )
    }

    /// Get events by type for a match
    pub fn get_events_by_type(&self, match_id: i64, event_type: &str) -> Result<Vec<EventRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, time_ms, event_type, data FROM events WHERE match_id = ?1 AND event_type = ?2 ORDER BY time_ms"
        )?;

        let rows = stmt.query_map(params![match_id, event_type], |row| {
            Ok(EventRecord {
                id: row.get(0)?,
                time_ms: row.get(1)?,
                event_type: row.get(2)?,
                data: row.get(3)?,
            })
        })?;

        rows.collect()
    }
}

/// A record from the events table
#[derive(Debug, Clone)]
pub struct EventRecord {
    pub id: i64,
    pub time_ms: u32,
    pub event_type: String,
    pub data: String,
}

/// Aggregate stats for a profile
#[derive(Debug, Clone)]
pub struct ProfileStats {
    pub profile: String,
    pub matches: u32,
    pub wins: u32,
    pub ties: u32,
    pub avg_score: f64,
    pub avg_opponent_score: f64,
}

impl ProfileStats {
    pub fn win_rate(&self) -> f64 {
        if self.matches == 0 {
            0.0
        } else {
            self.wins as f64 / self.matches as f64
        }
    }

    pub fn losses(&self) -> u32 {
        self.matches.saturating_sub(self.wins + self.ties)
    }
}

/// Filter for querying matches
#[derive(Debug, Clone, Default)]
pub struct MatchFilter {
    pub profile: Option<String>,
    pub level: Option<u32>,
    pub limit: Option<u32>,
}

/// Summary of a match (without full stats)
#[derive(Debug, Clone)]
pub struct MatchSummary {
    pub id: i64,
    pub level: u32,
    pub level_name: String,
    pub left_profile: String,
    pub right_profile: String,
    pub score_left: u32,
    pub score_right: u32,
    pub duration: f32,
    pub winner: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::metrics::PlayerStats;

    fn sample_result() -> MatchResult {
        MatchResult {
            level: 3,
            level_name: "Test Level".to_string(),
            left_profile: "Balanced".to_string(),
            right_profile: "Aggressive".to_string(),
            duration: 45.5,
            score_left: 3,
            score_right: 2,
            winner: "left".to_string(),
            left_stats: PlayerStats::default(),
            right_stats: PlayerStats::default(),
            seed: 12345,
        }
    }

    #[test]
    fn test_create_database() {
        let db = SimDatabase::open_in_memory().unwrap();
        assert_eq!(db.match_count().unwrap(), 0);
        assert_eq!(db.session_count().unwrap(), 0);
    }

    #[test]
    fn test_insert_match() {
        let db = SimDatabase::open_in_memory().unwrap();
        let session_id = db.create_session("test", None).unwrap();

        let result = sample_result();
        let match_id = db.insert_match(&session_id, &result).unwrap();

        assert!(match_id > 0);
        assert_eq!(db.match_count().unwrap(), 1);
    }

    #[test]
    fn test_query_matches() {
        let db = SimDatabase::open_in_memory().unwrap();
        let session_id = db.create_session("test", None).unwrap();

        db.insert_match(&session_id, &sample_result()).unwrap();

        let filter = MatchFilter {
            profile: Some("Balanced".to_string()),
            ..Default::default()
        };

        let matches = db.query_matches(&filter).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].left_profile, "Balanced");
    }

    #[test]
    fn test_profile_stats() {
        let db = SimDatabase::open_in_memory().unwrap();
        let session_id = db.create_session("test", None).unwrap();

        // Insert 3 matches
        for i in 0..3 {
            let mut result = sample_result();
            result.seed = i;
            result.winner = if i % 2 == 0 { "left".to_string() } else { "right".to_string() };
            db.insert_match(&session_id, &result).unwrap();
        }

        let stats = db.get_profile_stats("Balanced").unwrap();
        assert_eq!(stats.matches, 3);
        assert_eq!(stats.wins, 2); // left won matches 0 and 2
    }
}
