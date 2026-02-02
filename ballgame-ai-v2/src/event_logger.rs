//! SQLite event logging for AI client
//!
//! Logs client events to a timestamped SQLite database for analysis.

use chrono::Local;
use rusqlite::{Connection, params};
use std::path::Path;
use std::sync::Mutex;

use ballgame_protocol::{AgentInput, CharacterId, GameStateSnapshot};

// Simple logging macros for v2 (no tracing dependency)
macro_rules! info {
    ($($arg:tt)*) => { println!("[INFO] {}", format!($($arg)*)); };
}
macro_rules! warn {
    ($($arg:tt)*) => { eprintln!("[WARN] {}", format!($($arg)*)); };
}

/// Database directory
const DB_DIR: &str = "db";

/// SQLite event logger for AI client
pub struct ClientEventLogger {
    conn: Mutex<Connection>,
    session_id: String,
    current_match_id: Mutex<Option<i64>>,
    enabled: bool,
}

impl ClientEventLogger {
    /// Create a new client event logger
    pub fn new(client_name: &str) -> Option<Self> {
        // Ensure db directory exists
        if let Err(e) = std::fs::create_dir_all(DB_DIR) {
            warn!("Failed to create db directory: {}", e);
            return None;
        }

        // Generate timestamped filename
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let db_path = format!("{}/ai_client_{}_{}.db", DB_DIR, client_name, timestamp);
        let path = Path::new(&db_path);

        let conn = match Connection::open(path) {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to open database: {}", e);
                return None;
            }
        };

        // Enable WAL mode
        if let Err(e) = conn.execute_batch("PRAGMA journal_mode=WAL;") {
            warn!("Failed to set WAL mode: {}", e);
        }

        // Initialize schema
        if let Err(e) = init_schema(&conn) {
            warn!("Failed to init schema: {}", e);
            return None;
        }

        // Create session
        let session_id = match create_session(&conn, client_name) {
            Ok(id) => id,
            Err(e) => {
                warn!("Failed to create session: {}", e);
                return None;
            }
        };

        info!("Client SQLite logger initialized: {}", db_path);

        Some(Self {
            conn: Mutex::new(conn),
            session_id,
            current_match_id: Mutex::new(None),
            enabled: true,
        })
    }

    /// Start a new match
    pub fn start_match(&self, character: CharacterId, server_url: &str) -> Option<i64> {
        if !self.enabled {
            return None;
        }

        let conn = self.conn.lock().ok()?;
        let display_name = short_uuid();

        let result = conn.execute(
            r#"INSERT INTO matches
               (session_id, display_name, character, server_url, start_time)
               VALUES (?1, ?2, ?3, ?4, datetime('now'))"#,
            params![
                self.session_id,
                display_name,
                character.to_string(),
                server_url,
            ],
        );

        match result {
            Ok(_) => {
                let match_id = conn.last_insert_rowid();
                *self.current_match_id.lock().ok()? = Some(match_id);
                info!("Started client match {} as {}", match_id, character);
                Some(match_id)
            }
            Err(e) => {
                warn!("Failed to start match: {}", e);
                None
            }
        }
    }

    /// End the current match
    pub fn end_match(&self, reason: &str) {
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

        if let Err(e) = conn.execute(
            "UPDATE matches SET end_time = datetime('now'), end_reason = ?1 WHERE id = ?2",
            params![reason, match_id],
        ) {
            warn!("Failed to end match: {}", e);
        } else {
            info!("Ended client match {}: {}", match_id, reason);
        }

        if let Ok(mut guard) = self.current_match_id.lock() {
            *guard = None;
        }
    }

    /// Log a game state received from server
    pub fn log_state(&self, tick: u64, state: &GameStateSnapshot, our_char: CharacterId) {
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

        // Find our agent in the state
        let our_agent = state.agents.iter().find(|a| a.character == our_char);

        let (pos_x, pos_y, vel_x, vel_y, holding_ball, grounded) = match our_agent {
            Some(a) => (
                a.position.x,
                a.position.y,
                a.velocity.x,
                a.velocity.y,
                a.holding_ball,
                a.grounded,
            ),
            None => (0.0, 0.0, 0.0, 0.0, false, false),
        };

        if let Err(e) = conn.execute(
            r#"INSERT INTO states
               (match_id, tick, score_left, score_right, ball_x, ball_y,
                our_x, our_y, our_vx, our_vy, holding_ball, grounded)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)"#,
            params![
                match_id,
                tick as i64,
                state.score.left,
                state.score.right,
                state.ball.position.x,
                state.ball.position.y,
                pos_x,
                pos_y,
                vel_x,
                vel_y,
                holding_ball,
                grounded,
            ],
        ) {
            warn!("Failed to log state: {}", e);
        }
    }

    /// Log a decision/input sent to server
    pub fn log_decision(&self, tick: u64, input: &AgentInput, goal: &str) {
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

        if let Err(e) = conn.execute(
            r#"INSERT INTO decisions
               (match_id, tick, goal, move_x, jump, action)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#,
            params![
                match_id,
                tick as i64,
                goal,
                input.move_x,
                input.jump_pressed,
                input.action_pressed,
            ],
        ) {
            warn!("Failed to log decision: {}", e);
        }
    }

    /// Log a connection event
    pub fn log_event(&self, event_type: &str, details: &str) {
        if !self.enabled {
            return;
        }

        let match_id = self.current_match_id.lock().ok().and_then(|g| *g);

        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(_) => return,
        };

        if let Err(e) = conn.execute(
            r#"INSERT INTO events (match_id, event_type, details, timestamp)
               VALUES (?1, ?2, ?3, datetime('now'))"#,
            params![match_id, event_type, details],
        ) {
            warn!("Failed to log event: {}", e);
        }
    }

    /// Check if logging is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

fn init_schema(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            client_name TEXT NOT NULL,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS matches (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT NOT NULL,
            display_name TEXT,
            character TEXT NOT NULL,
            server_url TEXT NOT NULL,
            start_time TEXT,
            end_time TEXT,
            end_reason TEXT,
            FOREIGN KEY (session_id) REFERENCES sessions(id)
        );

        CREATE TABLE IF NOT EXISTS states (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            match_id INTEGER NOT NULL,
            tick INTEGER NOT NULL,
            score_left INTEGER,
            score_right INTEGER,
            ball_x REAL,
            ball_y REAL,
            our_x REAL,
            our_y REAL,
            our_vx REAL,
            our_vy REAL,
            holding_ball INTEGER,
            grounded INTEGER,
            FOREIGN KEY (match_id) REFERENCES matches(id)
        );

        CREATE TABLE IF NOT EXISTS decisions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            match_id INTEGER NOT NULL,
            tick INTEGER NOT NULL,
            goal TEXT,
            move_x REAL,
            jump INTEGER,
            action INTEGER,
            FOREIGN KEY (match_id) REFERENCES matches(id)
        );

        CREATE TABLE IF NOT EXISTS events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            match_id INTEGER,
            event_type TEXT NOT NULL,
            details TEXT,
            timestamp TEXT,
            FOREIGN KEY (match_id) REFERENCES matches(id)
        );

        CREATE INDEX IF NOT EXISTS idx_states_match_tick ON states(match_id, tick);
        CREATE INDEX IF NOT EXISTS idx_decisions_match_tick ON decisions(match_id, tick);
        CREATE INDEX IF NOT EXISTS idx_events_match ON events(match_id);
        "#,
    )
}

fn create_session(conn: &Connection, client_name: &str) -> Result<String, rusqlite::Error> {
    let id = uuid::Uuid::new_v4().to_string();
    let created_at = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO sessions (id, client_name, created_at) VALUES (?1, ?2, ?3)",
        params![id, client_name, created_at],
    )?;

    Ok(id)
}

fn short_uuid() -> String {
    let full = uuid::Uuid::new_v4().simple().to_string();
    full[..16].to_string()
}
