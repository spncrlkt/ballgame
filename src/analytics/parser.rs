//! SQLite analytics parser
//!
//! Builds analytics-friendly match structures from the SQLite event store.

use std::path::Path;

use rusqlite::params;

use crate::events::{GameEvent, PlayerId, parse_event};
use crate::simulation::SimDatabase;

/// Parsed match data from an event log
#[derive(Debug, Clone, Default)]
pub struct ParsedMatch {
    /// Session ID
    pub session_id: String,
    /// Level number
    pub level: u32,
    /// Level name
    pub level_name: String,
    /// Left player profile
    pub left_profile: String,
    /// Right player profile
    pub right_profile: String,
    /// RNG seed
    pub seed: u64,
    /// Match duration in seconds
    pub duration: f32,
    /// Final scores
    pub score_left: u32,
    pub score_right: u32,
    /// Goal events with timestamps
    pub goals: Vec<(f32, PlayerId, u32, u32)>, // (time, scorer, score_left, score_right)
    /// Shot events: (time, player, charge, angle, power)
    pub shots: Vec<(f32, PlayerId, f32, f32, f32)>,
    /// Shot starts: (time, player)
    pub shot_starts: Vec<(f32, PlayerId)>,
    /// Pickup events: (time, player)
    pub pickups: Vec<(f32, PlayerId)>,
    /// Drop events: (time, player)
    pub drops: Vec<(f32, PlayerId)>,
    /// Steal attempts: (time, attacker)
    pub steal_attempts: Vec<(f32, PlayerId)>,
    /// Steal successes: (time, attacker)
    pub steal_successes: Vec<(f32, PlayerId)>,
    /// Steal failures: (time, attacker)
    pub steal_failures: Vec<(f32, PlayerId)>,
}

impl ParsedMatch {
    /// Determine winner from scores
    pub fn winner(&self) -> &str {
        if self.score_left > self.score_right {
            "left"
        } else if self.score_right > self.score_left {
            "right"
        } else {
            "tie"
        }
    }

    /// Get profile for a player side
    pub fn profile_for(&self, player: PlayerId) -> &str {
        match player {
            PlayerId::L => &self.left_profile,
            PlayerId::R => &self.right_profile,
        }
    }

    /// Get score for a player side
    pub fn score_for(&self, player: PlayerId) -> u32 {
        match player {
            PlayerId::L => self.score_left,
            PlayerId::R => self.score_right,
        }
    }

    /// Count shots for a player
    pub fn shots_for(&self, player: PlayerId) -> usize {
        self.shots
            .iter()
            .filter(|(_, p, _, _, _)| *p == player)
            .count()
    }

    /// Count goals for a player
    pub fn goals_for(&self, player: PlayerId) -> usize {
        self.goals
            .iter()
            .filter(|(_, p, _, _)| *p == player)
            .count()
    }

    /// Count steal attempts for a player
    pub fn steal_attempts_for(&self, player: PlayerId) -> usize {
        self.steal_attempts
            .iter()
            .filter(|(_, p)| *p == player)
            .count()
    }

    /// Count steal successes for a player
    pub fn steal_successes_for(&self, player: PlayerId) -> usize {
        self.steal_successes
            .iter()
            .filter(|(_, p)| *p == player)
            .count()
    }

    /// Count pickups for a player
    pub fn pickups_for(&self, player: PlayerId) -> usize {
        self.pickups.iter().filter(|(_, p)| *p == player).count()
    }
}

/// Parse a single match from SQLite by match ID.
pub fn parse_match_from_db(db: &SimDatabase, match_id: i64) -> Option<ParsedMatch> {
    let (session_id, level, level_name, left_profile, right_profile, seed, duration, score_left, score_right) =
        db.conn()
            .query_row(
                "SELECT session_id, level, level_name, left_profile, right_profile, seed, duration_secs, score_left, score_right FROM matches WHERE id = ?1",
                params![match_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, u32>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, i64>(5)?,
                        row.get::<_, f32>(6)?,
                        row.get::<_, u32>(7)?,
                        row.get::<_, u32>(8)?,
                    ))
                },
            )
            .ok()?;

    let mut goals = Vec::new();
    let mut shots = Vec::new();
    let mut shot_starts = Vec::new();
    let mut pickups = Vec::new();
    let mut drops = Vec::new();
    let mut steal_attempts = Vec::new();
    let mut steal_successes = Vec::new();
    let mut steal_failures = Vec::new();

    let events = db.get_events(match_id).ok()?;
    for event in events {
        let Some((time_ms, parsed)) = parse_event(&event.data) else {
            continue;
        };
        let time_secs = time_ms as f32 / 1000.0;
        match parsed {
            GameEvent::Goal {
                player,
                score_left: left,
                score_right: right,
            } => goals.push((time_secs, player, left, right)),
            GameEvent::ShotRelease {
                player,
                charge,
                angle,
                power,
            } => shots.push((time_secs, player, charge, angle, power)),
            GameEvent::ShotStart { player, .. } => shot_starts.push((time_secs, player)),
            GameEvent::Pickup { player } => pickups.push((time_secs, player)),
            GameEvent::Drop { player } => drops.push((time_secs, player)),
            GameEvent::StealAttempt { attacker } => steal_attempts.push((time_secs, attacker)),
            GameEvent::StealSuccess { attacker } => steal_successes.push((time_secs, attacker)),
            GameEvent::StealFail { attacker } => steal_failures.push((time_secs, attacker)),
            _ => {}
        }
    }

    Some(ParsedMatch {
        session_id,
        level,
        level_name,
        left_profile,
        right_profile,
        seed: seed as u64,
        duration,
        score_left,
        score_right,
        goals,
        shots,
        shot_starts,
        pickups,
        drops,
        steal_attempts,
        steal_successes,
        steal_failures,
    })
}

/// Parse all matches from a SQLite database.
pub fn parse_all_matches_from_db(db_path: &Path) -> Vec<ParsedMatch> {
    let db = match SimDatabase::open(db_path) {
        Ok(db) => db,
        Err(_) => return Vec::new(),
    };

    let mut stmt = match db.conn().prepare("SELECT id FROM matches ORDER BY id") {
        Ok(stmt) => stmt,
        Err(_) => return Vec::new(),
    };

    let rows = match stmt.query_map([], |row| row.get::<_, i64>(0)) {
        Ok(rows) => rows,
        Err(_) => return Vec::new(),
    };

    rows.filter_map(|id| id.ok())
        .filter_map(|id| parse_match_from_db(&db, id))
        .collect()
}
