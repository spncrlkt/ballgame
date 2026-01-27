//! SQLite database for simulation results
//!
//! Provides persistent storage and querying of simulation results.
//! Uses WAL mode for concurrent reads during writes.

use bevy::prelude::Vec2;
use rusqlite::{Connection, OptionalExtension, Result, params};
use std::path::Path;

use super::metrics::{MatchResult, PlayerStats};
use crate::events::{GameEvent, parse_event, serialize_event};
use crate::replay::{MatchInfo, ReplayData, TickFrame, TimedEvent};

/// Database wrapper for simulation results
pub struct SimDatabase {
    conn: Connection,
}

impl SimDatabase {
    /// Get a reference to the underlying connection
    ///
    /// Use sparingly - prefer using the typed methods on SimDatabase.
    pub fn conn(&self) -> &Connection {
        &self.conn
    }
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
        let _ = self
            .conn
            .execute("ALTER TABLE sessions ADD COLUMN display_name TEXT", []);
        let _ = self
            .conn
            .execute("ALTER TABLE matches ADD COLUMN display_name TEXT", []);
        let _ = self
            .conn
            .execute("ALTER TABLE events ADD COLUMN point_id INTEGER", []);
        Ok(())
    }

    /// Create a new session and return its ID
    pub fn create_session(&self, session_type: &str, config_json: Option<&str>) -> Result<String> {
        let id = uuid::Uuid::new_v4();
        let id_str = id.to_string();
        let display_name = id.simple().to_string()[..16].to_string();
        let created_at = chrono::Utc::now().to_rfc3339();

        self.conn.execute(
            "INSERT INTO sessions (id, created_at, session_type, config_json, display_name) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id_str, created_at, session_type, config_json, display_name],
        )?;

        Ok(id_str)
    }

    /// Insert a match result and return the match ID
    pub fn insert_match(&self, session_id: &str, result: &MatchResult) -> Result<i64> {
        let display_name = short_uuid();
        self.conn.execute(
            r#"INSERT INTO matches
               (session_id, display_name, seed, level, level_name, left_profile, right_profile,
                score_left, score_right, duration_secs, winner)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)"#,
            params![
                session_id,
                display_name,
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
             FROM matches WHERE 1=1",
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
        self.conn
            .query_row("SELECT COUNT(*) FROM matches", [], |row| row.get(0))
    }

    /// Get session count
    pub fn session_count(&self) -> Result<u64> {
        self.conn
            .query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))
    }

    /// Insert a batch of events for a match
    pub fn insert_events(&self, match_id: i64, events: &[(u32, &str, &str)]) -> Result<()> {
        let mut stmt = self.conn.prepare(
            "INSERT INTO events (match_id, point_id, time_ms, event_type, data) VALUES (?1, NULL, ?2, ?3, ?4)"
        )?;

        for (time_ms, event_type, data) in events {
            stmt.execute(params![match_id, time_ms, event_type, data])?;
        }

        Ok(())
    }

    /// Insert a single event for a match
    pub fn insert_event(
        &self,
        match_id: i64,
        time_ms: u32,
        event_type: &str,
        data: &str,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO events (match_id, point_id, time_ms, event_type, data) VALUES (?1, NULL, ?2, ?3, ?4)",
            params![match_id, time_ms, event_type, data],
        )?;
        Ok(())
    }

    /// Insert points and events, assigning point_id to each event.
    pub fn insert_events_with_points(
        &self,
        match_id: i64,
        duration_secs: f32,
        events: &[(u32, GameEvent)],
    ) -> Result<()> {
        if events.is_empty() {
            return Ok(());
        }

        self.conn.execute("BEGIN TRANSACTION", [])?;

        let mut point_index = 1u32;
        let mut point_id = self.insert_point(match_id, point_index, 0)?;

        for (time_ms, event) in events {
            let data = serialize_event(*time_ms, event);
            let event_type = event.type_code();
            self.conn.execute(
                "INSERT INTO events (match_id, point_id, time_ms, event_type, data) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![match_id, point_id, time_ms, event_type, data],
            )?;

            if let GameEvent::Goal { player, .. } = event {
                let winner = match player {
                    crate::events::PlayerId::L => "left",
                    crate::events::PlayerId::R => "right",
                };
                self.end_point(point_id, *time_ms, winner)?;
                point_index += 1;
                point_id = self.insert_point(match_id, point_index, *time_ms)?;
            }
        }

        let end_time_ms = (duration_secs * 1000.0).round() as u32;
        self.end_point(point_id, end_time_ms, "none")?;
        self.conn.execute("COMMIT", [])?;
        Ok(())
    }

    /// Get events for a match
    pub fn get_events(&self, match_id: i64) -> Result<Vec<EventRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, point_id, time_ms, event_type, data FROM events WHERE match_id = ?1 ORDER BY time_ms"
        )?;

        let rows = stmt.query_map(params![match_id], |row| {
            Ok(EventRecord {
                id: row.get(0)?,
                point_id: row.get(1)?,
                time_ms: row.get(2)?,
                event_type: row.get(3)?,
                data: row.get(4)?,
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
            "SELECT id, point_id, time_ms, event_type, data FROM events WHERE match_id = ?1 AND event_type = ?2 ORDER BY time_ms"
        )?;

        let rows = stmt.query_map(params![match_id, event_type], |row| {
            Ok(EventRecord {
                id: row.get(0)?,
                point_id: row.get(1)?,
                time_ms: row.get(2)?,
                event_type: row.get(3)?,
                data: row.get(4)?,
            })
        })?;

        rows.collect()
    }

    /// Find a match ID in a session by 1-based game index.
    pub fn find_match_by_session(&self, session_id: &str, game_num: u32) -> Result<Option<i64>> {
        if game_num == 0 {
            return Ok(None);
        }

        let offset = (game_num - 1) as i64;
        self.conn
            .query_row(
                "SELECT id FROM matches WHERE session_id = ?1 ORDER BY id ASC LIMIT 1 OFFSET ?2",
                params![session_id, offset],
                |row| row.get(0),
            )
            .optional()
    }

    /// Load replay data from SQLite for a match.
    pub fn load_replay_data(&self, match_id: i64) -> std::result::Result<ReplayData, String> {
        let (session_id, level, level_name, left_profile, right_profile, seed) = self
            .conn
            .query_row(
                "SELECT session_id, level, level_name, left_profile, right_profile, seed FROM matches WHERE id = ?1",
                params![match_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, u32>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, i64>(5)?,
                    ))
                },
            )
            .map_err(|e| e.to_string())?;

        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, time_ms, data FROM events WHERE match_id = ?1 ORDER BY time_ms ASC, id ASC",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(params![match_id], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, u32>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?;

        let mut ticks = Vec::new();
        let mut events = Vec::new();
        let mut max_time_ms = 0u32;

        for row in rows {
            let (event_id, time_ms, data) = row.map_err(|e| e.to_string())?;
            let (_, event) = parse_event(&data).ok_or_else(|| {
                format!("Failed to parse event {} for match {}", event_id, match_id)
            })?;

            if time_ms > max_time_ms {
                max_time_ms = time_ms;
            }

            match event {
                GameEvent::Tick {
                    frame,
                    left_pos,
                    left_vel,
                    right_pos,
                    right_vel,
                    ball_pos,
                    ball_vel,
                    ball_state,
                } => {
                    ticks.push(TickFrame {
                        time_ms,
                        frame,
                        left_pos: Vec2::new(left_pos.0, left_pos.1),
                        left_vel: Vec2::new(left_vel.0, left_vel.1),
                        right_pos: Vec2::new(right_pos.0, right_pos.1),
                        right_vel: Vec2::new(right_vel.0, right_vel.1),
                        ball_pos: Vec2::new(ball_pos.0, ball_pos.1),
                        ball_vel: Vec2::new(ball_vel.0, ball_vel.1),
                        ball_state,
                    });
                }
                _ => {
                    events.push(TimedEvent { time_ms, event });
                }
            }
        }

        Ok(ReplayData {
            session_id,
            match_info: MatchInfo {
                level,
                level_name,
                left_profile,
                right_profile,
                seed: seed as u64,
            },
            ticks,
            events,
            duration_ms: max_time_ms,
        })
    }
}

/// A record from the events table
#[derive(Debug, Clone)]
pub struct EventRecord {
    pub id: i64,
    pub point_id: Option<i64>,
    pub time_ms: u32,
    pub event_type: String,
    pub data: String,
}

impl SimDatabase {
    fn insert_point(&self, match_id: i64, point_index: u32, start_time_ms: u32) -> Result<i64> {
        self.conn.execute(
            r#"INSERT INTO points (match_id, point_index, start_time_ms, end_time_ms, winner)
               VALUES (?1, ?2, ?3, NULL, NULL)"#,
            params![match_id, point_index, start_time_ms],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    fn end_point(&self, point_id: i64, end_time_ms: u32, winner: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE points SET end_time_ms = ?1, winner = ?2 WHERE id = ?3",
            params![end_time_ms, winner, point_id],
        )?;
        Ok(())
    }
}

fn short_uuid() -> String {
    let full = uuid::Uuid::new_v4().simple().to_string();
    full[..16].to_string()
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

//=============================================================================
// Analysis Data Structures
//=============================================================================

/// Distance analysis metrics for a match
#[derive(Debug, Clone, Default)]
pub struct DistanceAnalysis {
    pub min_distance: f32,
    pub avg_distance: f32,
    pub max_distance: f32,
    pub ticks_within_60px: u32,
    pub ticks_within_100px: u32,
    pub ticks_within_200px: u32,
    pub total_ticks: u32,
    pub closest_moment_ms: Option<u32>,
}

/// AI input analysis metrics for a match
#[derive(Debug, Clone, Default)]
pub struct InputAnalysis {
    pub move_left_frames: u32,
    pub move_right_frames: u32,
    pub stationary_frames: u32,
    pub jump_presses: u32,
    pub pickup_presses: u32,
    pub throw_presses: u32,
    pub total_frames: u32,
}

/// Closest moment during a match
#[derive(Debug, Clone)]
pub struct ClosestMoment {
    pub time_ms: u32,
    pub distance: f32,
    pub left_pos: (f32, f32),
    pub right_pos: (f32, f32),
}

/// Goal transition data
#[derive(Debug, Clone)]
pub struct GoalTransition {
    pub time_ms: u32,
    pub player: String,
    pub goal: String,
}

/// Session summary for analysis
#[derive(Debug, Clone)]
pub struct SessionSummary {
    pub session_id: String,
    pub session_type: String,
    pub created_at: String,
    pub match_count: u32,
    pub total_duration_secs: f32,
}

/// Match with event statistics
#[derive(Debug, Clone)]
pub struct MatchEventStats {
    pub match_id: i64,
    pub level_name: String,
    pub left_profile: String,
    pub right_profile: String,
    pub score_left: u32,
    pub score_right: u32,
    pub duration_secs: f32,
    pub event_count: u32,
    pub tick_count: u32,
    pub goal_count: u32,
    pub shot_count: u32,
    pub steal_count: u32,
}

//=============================================================================
// Analysis Query Methods
//=============================================================================

impl SimDatabase {
    /// Get session summary
    pub fn get_session_summary(&self, session_id: &str) -> Result<SessionSummary> {
        self.conn.query_row(
            r#"SELECT
                s.id, s.session_type, s.created_at,
                COUNT(m.id) as match_count,
                COALESCE(SUM(m.duration_secs), 0.0) as total_duration
               FROM sessions s
               LEFT JOIN matches m ON m.session_id = s.id
               WHERE s.id = ?1
               GROUP BY s.id"#,
            params![session_id],
            |row| {
                Ok(SessionSummary {
                    session_id: row.get(0)?,
                    session_type: row.get(1)?,
                    created_at: row.get(2)?,
                    match_count: row.get(3)?,
                    total_duration_secs: row.get(4)?,
                })
            },
        )
    }

    /// Get matches for a session
    pub fn get_session_matches(&self, session_id: &str) -> Result<Vec<MatchSummary>> {
        let mut stmt = self.conn.prepare(
            r#"SELECT id, level, level_name, left_profile, right_profile,
                      score_left, score_right, duration_secs, winner
               FROM matches
               WHERE session_id = ?1
               ORDER BY id"#,
        )?;

        let rows = stmt.query_map(params![session_id], |row| {
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

    /// Get match event statistics
    pub fn get_match_event_stats(&self, match_id: i64) -> Result<MatchEventStats> {
        self.conn.query_row(
            r#"SELECT
                m.id, m.level_name, m.left_profile, m.right_profile,
                m.score_left, m.score_right, m.duration_secs,
                (SELECT COUNT(*) FROM events WHERE match_id = m.id) as event_count,
                (SELECT COUNT(*) FROM events WHERE match_id = m.id AND event_type = 'T') as tick_count,
                (SELECT COUNT(*) FROM events WHERE match_id = m.id AND event_type = 'G') as goal_count,
                (SELECT COUNT(*) FROM events WHERE match_id = m.id AND event_type = 'SR') as shot_count,
                (SELECT COUNT(*) FROM events WHERE match_id = m.id AND event_type IN ('SA', 'S+', 'S-', 'SO')) as steal_count
               FROM matches m
               WHERE m.id = ?1"#,
            params![match_id],
            |row| {
                Ok(MatchEventStats {
                    match_id: row.get(0)?,
                    level_name: row.get(1)?,
                    left_profile: row.get(2)?,
                    right_profile: row.get(3)?,
                    score_left: row.get(4)?,
                    score_right: row.get(5)?,
                    duration_secs: row.get(6)?,
                    event_count: row.get(7)?,
                    tick_count: row.get(8)?,
                    goal_count: row.get(9)?,
                    shot_count: row.get(10)?,
                    steal_count: row.get(11)?,
                })
            },
        )
    }

    /// Get AI goal transitions for a match
    pub fn get_goal_transitions(&self, match_id: i64) -> Result<Vec<GoalTransition>> {
        let mut stmt = self.conn.prepare(
            "SELECT time_ms, data FROM events WHERE match_id = ?1 AND event_type = 'AG' ORDER BY time_ms"
        )?;

        let rows = stmt.query_map(params![match_id], |row| {
            let time_ms: u32 = row.get(0)?;
            let data: String = row.get(1)?;

            // Parse data format: T:NNNNN|AG|player|goal
            // The data column contains the full serialized event
            let parts: Vec<&str> = data.split('|').collect();
            let (player, goal) = if parts.len() >= 4 {
                (parts[2].to_string(), parts[3].to_string())
            } else {
                ("?".to_string(), "?".to_string())
            };

            Ok(GoalTransition {
                time_ms,
                player,
                goal,
            })
        })?;

        rows.collect()
    }

    /// Count events by type for a match
    pub fn count_events_by_type(&self, match_id: i64) -> Result<Vec<(String, u32)>> {
        let mut stmt = self.conn.prepare(
            "SELECT event_type, COUNT(*) FROM events WHERE match_id = ?1 GROUP BY event_type ORDER BY COUNT(*) DESC"
        )?;

        let rows = stmt.query_map(params![match_id], |row| Ok((row.get(0)?, row.get(1)?)))?;

        rows.collect()
    }

    /// Get the most recent session ID
    pub fn get_latest_session(&self) -> Result<Option<String>> {
        let result = self.conn.query_row(
            "SELECT id FROM sessions ORDER BY created_at DESC LIMIT 1",
            [],
            |row| row.get(0),
        );

        match result {
            Ok(id) => Ok(Some(id)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Get matches for the most recent session
    pub fn get_latest_session_matches(&self) -> Result<Vec<MatchSummary>> {
        if let Some(session_id) = self.get_latest_session()? {
            self.get_session_matches(&session_id)
        } else {
            Ok(Vec::new())
        }
    }

    /// Get tick events with parsed position data for distance analysis
    ///
    /// This parses the Tick events (format: T:NNNNN|T|frame|left_pos|left_vel|right_pos|right_vel|ball_pos|ball_vel|state)
    /// and calculates distance between players.
    pub fn analyze_distance(&self, match_id: i64) -> Result<DistanceAnalysis> {
        let tick_events = self.get_events_by_type(match_id, "T")?;

        if tick_events.is_empty() {
            return Ok(DistanceAnalysis::default());
        }

        let mut min_distance = f32::MAX;
        let mut max_distance = f32::MIN;
        let mut total_distance = 0.0f32;
        let mut closest_moment_ms = 0u32;
        let mut ticks_within_60 = 0u32;
        let mut ticks_within_100 = 0u32;
        let mut ticks_within_200 = 0u32;

        for event in &tick_events {
            // Parse: T:NNNNN|T|frame|left_pos|left_vel|right_pos|right_vel|ball_pos|ball_vel|state
            let parts: Vec<&str> = event.data.split('|').collect();
            if parts.len() < 6 {
                continue;
            }

            // left_pos is at index 3, right_pos is at index 5
            let left_pos = parse_pos(parts[3]);
            let right_pos = parse_pos(parts[5]);

            if let (Some((lx, ly)), Some((rx, ry))) = (left_pos, right_pos) {
                let distance = ((rx - lx).powi(2) + (ry - ly).powi(2)).sqrt();

                total_distance += distance;

                if distance < min_distance {
                    min_distance = distance;
                    closest_moment_ms = event.time_ms;
                }
                if distance > max_distance {
                    max_distance = distance;
                }

                if distance < 60.0 {
                    ticks_within_60 += 1;
                }
                if distance < 100.0 {
                    ticks_within_100 += 1;
                }
                if distance < 200.0 {
                    ticks_within_200 += 1;
                }
            }
        }

        let total_ticks = tick_events.len() as u32;
        let avg_distance = if total_ticks > 0 {
            total_distance / total_ticks as f32
        } else {
            0.0
        };

        Ok(DistanceAnalysis {
            min_distance: if min_distance == f32::MAX {
                0.0
            } else {
                min_distance
            },
            avg_distance,
            max_distance: if max_distance == f32::MIN {
                0.0
            } else {
                max_distance
            },
            ticks_within_60px: ticks_within_60,
            ticks_within_100px: ticks_within_100,
            ticks_within_200px: ticks_within_200,
            total_ticks,
            closest_moment_ms: if total_ticks > 0 {
                Some(closest_moment_ms)
            } else {
                None
            },
        })
    }

    /// Analyze AI input patterns for a match
    ///
    /// Parses ControllerInput events (CI) for the AI player (R)
    pub fn analyze_ai_inputs(&self, match_id: i64) -> Result<InputAnalysis> {
        let ci_events = self.get_events_by_type(match_id, "CI")?;

        let mut move_left = 0u32;
        let mut move_right = 0u32;
        let mut stationary = 0u32;
        let mut jump_presses = 0u32;
        let mut pickup_presses = 0u32;
        let mut throw_presses = 0u32;
        let mut total_frames = 0u32;

        for event in &ci_events {
            // Parse: T:NNNNN|CI|player|source|move_x|jump|jump_pressed|throw|throw_released|pickup
            let parts: Vec<&str> = event.data.split('|').collect();
            if parts.len() < 10 {
                continue;
            }

            // Only analyze AI (R) inputs
            if parts[2] != "R" {
                continue;
            }

            total_frames += 1;

            // move_x is at index 4
            if let Ok(move_x) = parts[4].parse::<f32>() {
                if move_x < -0.1 {
                    move_left += 1;
                } else if move_x > 0.1 {
                    move_right += 1;
                } else {
                    stationary += 1;
                }
            }

            // jump_pressed is at index 6
            if parts[6] == "1" {
                jump_presses += 1;
            }

            // throw is at index 7
            if parts[7] == "1" {
                throw_presses += 1;
            }

            // pickup is at index 9
            if parts[9] == "1" {
                pickup_presses += 1;
            }
        }

        Ok(InputAnalysis {
            move_left_frames: move_left,
            move_right_frames: move_right,
            stationary_frames: stationary,
            jump_presses,
            pickup_presses,
            throw_presses,
            total_frames,
        })
    }

    /// Get closest moments (where distance < threshold)
    pub fn get_closest_moments(&self, match_id: i64, threshold: f32) -> Result<Vec<ClosestMoment>> {
        let tick_events = self.get_events_by_type(match_id, "T")?;
        let mut moments = Vec::new();

        for event in &tick_events {
            let parts: Vec<&str> = event.data.split('|').collect();
            if parts.len() < 6 {
                continue;
            }

            let left_pos = parse_pos(parts[3]);
            let right_pos = parse_pos(parts[5]);

            if let (Some((lx, ly)), Some((rx, ry))) = (left_pos, right_pos) {
                let distance = ((rx - lx).powi(2) + (ry - ly).powi(2)).sqrt();

                if distance < threshold {
                    moments.push(ClosestMoment {
                        time_ms: event.time_ms,
                        distance,
                        left_pos: (lx, ly),
                        right_pos: (rx, ry),
                    });
                }
            }
        }

        // Sort by distance (closest first)
        moments.sort_by(|a, b| {
            a.distance
                .partial_cmp(&b.distance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(moments)
    }
}

/// Parse a position string "x,y" into (f32, f32)
fn parse_pos(s: &str) -> Option<(f32, f32)> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 2 {
        return None;
    }
    let x = parts[0].parse().ok()?;
    let y = parts[1].parse().ok()?;
    Some((x, y))
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
            events: Vec::new(),
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
            result.winner = if i % 2 == 0 {
                "left".to_string()
            } else {
                "right".to_string()
            };
            db.insert_match(&session_id, &result).unwrap();
        }

        let stats = db.get_profile_stats("Balanced").unwrap();
        assert_eq!(stats.matches, 3);
        assert_eq!(stats.wins, 2); // left won matches 0 and 2
    }
}
