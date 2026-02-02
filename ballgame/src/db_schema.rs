//! Shared database schema definitions
//!
//! Both SqliteEventLogger (real-time training) and SimDatabase (batch simulation)
//! use overlapping schemas. This module centralizes the table definitions to avoid
//! divergence and make schema updates easier.

/// Core tables used by all database types
///
/// These tables form the foundation of game event storage:
/// - sessions: Groups of matches (training session, tournament, etc.)
/// - matches: Individual games with scoring and participants
/// - points: Individual scoring sequences within matches
/// - events: Detailed game events with timestamps
/// - player_stats: Aggregated statistics per match
pub const CORE_SCHEMA: &str = r#"
    CREATE TABLE IF NOT EXISTS sessions (
        id TEXT PRIMARY KEY,
        created_at TEXT NOT NULL,
        session_type TEXT NOT NULL,
        config_json TEXT,
        display_name TEXT,
        run_started_at TEXT,
        run_finished_at TEXT,
        run_elapsed_secs REAL,
        matches_planned INTEGER,
        matches_played INTEGER,
        duration_limit_secs REAL,
        stalemate_timeout_secs REAL,
        parallel_threads INTEGER,
        run_timeout_secs REAL,
        mode TEXT,
        profiles_count INTEGER,
        levels_count INTEGER,
        matches_per_pair INTEGER,
        matches_per_level INTEGER
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
        winner TEXT NOT NULL,
        game_mode TEXT DEFAULT '1v1'
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

    CREATE TABLE IF NOT EXISTS events (
        id INTEGER PRIMARY KEY,
        match_id INTEGER REFERENCES matches(id),
        point_id INTEGER REFERENCES points(id),
        time_ms INTEGER NOT NULL,
        tick_frame INTEGER NOT NULL DEFAULT 0,
        event_type TEXT NOT NULL,
        data TEXT NOT NULL,
        created_at TEXT DEFAULT CURRENT_TIMESTAMP
    );

    -- Core indexes
    CREATE INDEX IF NOT EXISTS idx_matches_session ON matches(session_id);
    CREATE INDEX IF NOT EXISTS idx_matches_profiles ON matches(left_profile, right_profile);
    CREATE INDEX IF NOT EXISTS idx_matches_level ON matches(level);
    CREATE INDEX IF NOT EXISTS idx_matches_game_mode ON matches(game_mode);
    CREATE INDEX IF NOT EXISTS idx_player_stats_match ON player_stats(match_id);
    CREATE INDEX IF NOT EXISTS idx_events_match ON events(match_id);
    CREATE INDEX IF NOT EXISTS idx_events_point ON events(point_id);
    CREATE INDEX IF NOT EXISTS idx_events_type ON events(event_type);
    CREATE INDEX IF NOT EXISTS idx_events_time ON events(match_id, time_ms);
    CREATE INDEX IF NOT EXISTS idx_events_tick ON events(match_id, tick_frame);
    CREATE INDEX IF NOT EXISTS idx_points_match ON points(match_id);
"#;

/// Training-specific tables (real-time logging with SqliteEventLogger)
///
/// These tables support interactive training sessions with human players:
/// - input_sources: Track which devices/AI are providing input
/// - match_characters: Character assignments and AI profiles
/// - debug_events: High-frequency state samples for analysis
pub const TRAINING_SCHEMA: &str = r#"
    -- Input sources for a match (keyboard, gamepads, AI)
    CREATE TABLE IF NOT EXISTS input_sources (
        id INTEGER PRIMARY KEY,
        match_id INTEGER REFERENCES matches(id),
        source_id INTEGER NOT NULL,
        source_type TEXT NOT NULL,
        source_detail TEXT,
        UNIQUE(match_id, source_id)
    );

    -- Character assignments for a match (which characters exist and their initial controllers)
    CREATE TABLE IF NOT EXISTS match_characters (
        id INTEGER PRIMARY KEY,
        match_id INTEGER REFERENCES matches(id),
        character_id TEXT NOT NULL,
        initial_source_id INTEGER,
        ai_profile TEXT,
        UNIQUE(match_id, character_id)
    );

    -- Debug sample table for manual reachability capture
    CREATE TABLE IF NOT EXISTS debug_events (
        id INTEGER PRIMARY KEY,
        match_id INTEGER REFERENCES matches(id),
        time_ms INTEGER NOT NULL,
        tick_frame INTEGER NOT NULL,
        player TEXT NOT NULL,
        pos_x REAL NOT NULL,
        pos_y REAL NOT NULL,
        vel_x REAL NOT NULL,
        vel_y REAL NOT NULL,
        input_move_x REAL NOT NULL,
        input_jump INTEGER NOT NULL,
        grounded INTEGER NOT NULL,
        is_jumping INTEGER NOT NULL,
        coyote_timer REAL NOT NULL,
        jump_buffer_timer REAL NOT NULL,
        facing REAL NOT NULL,
        nav_active INTEGER NOT NULL,
        nav_path_index INTEGER NOT NULL,
        nav_action TEXT,
        level_id TEXT NOT NULL,
        human_controlled INTEGER NOT NULL,
        created_at TEXT DEFAULT CURRENT_TIMESTAMP
    );

    -- Training-specific indexes
    CREATE INDEX IF NOT EXISTS idx_input_sources_match ON input_sources(match_id);
    CREATE INDEX IF NOT EXISTS idx_match_characters_match ON match_characters(match_id);
    CREATE INDEX IF NOT EXISTS idx_debug_match ON debug_events(match_id);
    CREATE INDEX IF NOT EXISTS idx_debug_time ON debug_events(match_id, time_ms);
    CREATE INDEX IF NOT EXISTS idx_debug_tick ON debug_events(match_id, tick_frame);
"#;

/// Bracket tournament tables (batch simulation with SimDatabase)
///
/// These tables support elimination bracket tournaments:
/// - bracket_tournaments: Tournament configuration and results
/// - bracket_entries: Participants with seeding and final placement
/// - bracket_matches: Bracket match slots (best-of series)
/// - bracket_games: Individual games within a bracket match
pub const BRACKET_SCHEMA: &str = r#"
    CREATE TABLE IF NOT EXISTS bracket_tournaments (
        id INTEGER PRIMARY KEY,
        session_id TEXT REFERENCES sessions(id),
        format_best_of INTEGER NOT NULL,
        format_score_limit INTEGER NOT NULL,
        format_duration_limit REAL NOT NULL,
        seeding_method TEXT NOT NULL,
        entrant_count INTEGER NOT NULL,
        champion_profile TEXT,
        is_complete INTEGER NOT NULL DEFAULT 0
    );

    CREATE TABLE IF NOT EXISTS bracket_entries (
        id INTEGER PRIMARY KEY,
        tournament_id INTEGER REFERENCES bracket_tournaments(id),
        entry_index INTEGER NOT NULL,
        profile_name TEXT NOT NULL,
        seed INTEGER NOT NULL,
        final_placement INTEGER,
        match_wins INTEGER DEFAULT 0,
        match_losses INTEGER DEFAULT 0,
        game_wins INTEGER DEFAULT 0,
        game_losses INTEGER DEFAULT 0
    );

    CREATE TABLE IF NOT EXISTS bracket_matches (
        id INTEGER PRIMARY KEY,
        tournament_id INTEGER REFERENCES bracket_tournaments(id),
        bracket_match_id INTEGER NOT NULL,
        side TEXT NOT NULL,
        round INTEGER NOT NULL,
        match_in_round INTEGER NOT NULL,
        player1_entry_idx INTEGER,
        player2_entry_idx INTEGER,
        player1_wins INTEGER,
        player2_wins INTEGER,
        winner_idx INTEGER
    );

    CREATE TABLE IF NOT EXISTS bracket_games (
        id INTEGER PRIMARY KEY,
        bracket_match_id INTEGER REFERENCES bracket_matches(id),
        game_index INTEGER NOT NULL,
        match_id INTEGER REFERENCES matches(id),
        level INTEGER NOT NULL,
        level_name TEXT NOT NULL,
        player1_score INTEGER NOT NULL,
        player2_score INTEGER NOT NULL,
        winner INTEGER NOT NULL,
        duration_secs REAL NOT NULL,
        seed INTEGER NOT NULL
    );

    -- Bracket-specific indexes
    CREATE INDEX IF NOT EXISTS idx_bracket_tournaments_session ON bracket_tournaments(session_id);
    CREATE INDEX IF NOT EXISTS idx_bracket_entries_tournament ON bracket_entries(tournament_id);
    CREATE INDEX IF NOT EXISTS idx_bracket_matches_tournament ON bracket_matches(tournament_id);
    CREATE INDEX IF NOT EXISTS idx_bracket_games_match ON bracket_games(bracket_match_id);
"#;

/// AI Client participant tracking tables
///
/// These tables support tracking which AI clients (vs embedded profiles) participated
/// in matches, enabling analysis of different AI architectures:
/// - match_participants: Individual participant info per slot per match
/// - team_compositions: Named team pairings for analysis
pub const PARTICIPANT_SCHEMA: &str = r#"
    -- Track all 4 participants in each match (for 2v2 format)
    -- This extends the simple left_profile/right_profile in matches table
    CREATE TABLE IF NOT EXISTS match_participants (
        id INTEGER PRIMARY KEY,
        match_id INTEGER REFERENCES matches(id),
        slot_id INTEGER NOT NULL,         -- 0=L0, 1=L1, 2=R0, 3=R1
        character_id TEXT NOT NULL,       -- "L0", "L1", "R0", "R1"
        team TEXT NOT NULL,               -- "left" or "right"
        participant_type TEXT NOT NULL,   -- "profile" or "client"
        participant_id TEXT NOT NULL,     -- profile name or client ID
        client_version TEXT,              -- from Hello message (clients only)
        UNIQUE(match_id, slot_id)
    );

    -- Named team compositions for tournament analysis
    -- A "team" is a pair of participants that play together
    CREATE TABLE IF NOT EXISTS team_compositions (
        id INTEGER PRIMARY KEY,
        team_name TEXT NOT NULL UNIQUE,           -- e.g., "v1-duo", "v1+v2", "Balanced-duo"
        slot_0_type TEXT NOT NULL,                -- "profile" or "client"
        slot_0_id TEXT NOT NULL,                  -- profile name or client ID
        slot_1_type TEXT NOT NULL,                -- "profile" or "client"
        slot_1_id TEXT NOT NULL,                  -- profile name or client ID
        created_at TEXT DEFAULT CURRENT_TIMESTAMP
    );

    -- Participant-specific indexes
    CREATE INDEX IF NOT EXISTS idx_match_participants_match ON match_participants(match_id);
    CREATE INDEX IF NOT EXISTS idx_match_participants_type ON match_participants(participant_type);
    CREATE INDEX IF NOT EXISTS idx_match_participants_id ON match_participants(participant_id);
    CREATE INDEX IF NOT EXISTS idx_match_participants_team ON match_participants(team);
"#;

/// Initialize all schemas for a training database
///
/// Call this when creating a new SqliteEventLogger database.
pub fn init_training_schema(conn: &rusqlite::Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(CORE_SCHEMA)?;
    conn.execute_batch(TRAINING_SCHEMA)?;
    Ok(())
}

/// Initialize all schemas for a simulation database
///
/// Call this when creating a new SimDatabase.
pub fn init_simulation_schema(conn: &rusqlite::Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(CORE_SCHEMA)?;
    conn.execute_batch(BRACKET_SCHEMA)?;
    conn.execute_batch(PARTICIPANT_SCHEMA)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_training_schema_valid() {
        let conn = Connection::open_in_memory().unwrap();
        init_training_schema(&conn).expect("Training schema should be valid SQL");

        // Verify key tables exist
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='sessions'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_simulation_schema_valid() {
        let conn = Connection::open_in_memory().unwrap();
        init_simulation_schema(&conn).expect("Simulation schema should be valid SQL");

        // Verify bracket tables exist
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='bracket_tournaments'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }
}
