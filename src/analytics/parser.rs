//! SQLite analytics parser
//!
//! Builds analytics-friendly match structures from the SQLite event store.

use std::path::Path;

use rusqlite::params;

use crate::events::{CharacterId, GameEvent, parse_event};
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
    pub goals: Vec<(f32, CharacterId, u32, u32)>, // (time, scorer, score_left, score_right)
    /// Shot events: (time, character, charge, angle, power)
    pub shots: Vec<(f32, CharacterId, f32, f32, f32)>,
    /// Shot starts: (time, character)
    pub shot_starts: Vec<(f32, CharacterId)>,
    /// Pickup events: (time, character)
    pub pickups: Vec<(f32, CharacterId)>,
    /// Drop events: (time, character)
    pub drops: Vec<(f32, CharacterId)>,
    /// Steal attempts: (time, attacker)
    pub steal_attempts: Vec<(f32, CharacterId)>,
    /// Steal successes: (time, attacker)
    pub steal_successes: Vec<(f32, CharacterId)>,
    /// Steal failures: (time, attacker)
    pub steal_failures: Vec<(f32, CharacterId)>,
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

    /// Get profile for a character
    pub fn profile_for(&self, character: CharacterId) -> &str {
        match character.team() {
            crate::events::TeamId::Left => &self.left_profile,
            crate::events::TeamId::Right => &self.right_profile,
        }
    }

    /// Get score for a character's team
    pub fn score_for(&self, character: CharacterId) -> u32 {
        match character.team() {
            crate::events::TeamId::Left => self.score_left,
            crate::events::TeamId::Right => self.score_right,
        }
    }

    /// Count shots for a character
    pub fn shots_for(&self, character: CharacterId) -> usize {
        self.shots
            .iter()
            .filter(|(_, c, _, _, _)| *c == character)
            .count()
    }

    /// Count shots for a team (all characters on that team)
    pub fn shots_for_team(&self, team: crate::events::TeamId) -> usize {
        self.shots
            .iter()
            .filter(|(_, c, _, _, _)| c.team() == team)
            .count()
    }

    /// Count goals for a character
    pub fn goals_for(&self, character: CharacterId) -> usize {
        self.goals
            .iter()
            .filter(|(_, c, _, _)| *c == character)
            .count()
    }

    /// Count goals for a team (all characters on that team)
    pub fn goals_for_team(&self, team: crate::events::TeamId) -> usize {
        self.goals
            .iter()
            .filter(|(_, c, _, _)| c.team() == team)
            .count()
    }

    /// Count steal attempts for a character
    pub fn steal_attempts_for(&self, character: CharacterId) -> usize {
        self.steal_attempts
            .iter()
            .filter(|(_, c)| *c == character)
            .count()
    }

    /// Count steal attempts for a team
    pub fn steal_attempts_for_team(&self, team: crate::events::TeamId) -> usize {
        self.steal_attempts
            .iter()
            .filter(|(_, c)| c.team() == team)
            .count()
    }

    /// Count steal successes for a character
    pub fn steal_successes_for(&self, character: CharacterId) -> usize {
        self.steal_successes
            .iter()
            .filter(|(_, c)| *c == character)
            .count()
    }

    /// Count steal successes for a team
    pub fn steal_successes_for_team(&self, team: crate::events::TeamId) -> usize {
        self.steal_successes
            .iter()
            .filter(|(_, c)| c.team() == team)
            .count()
    }

    /// Count pickups for a character
    pub fn pickups_for(&self, character: CharacterId) -> usize {
        self.pickups.iter().filter(|(_, c)| *c == character).count()
    }

    /// Count pickups for a team
    pub fn pickups_for_team(&self, team: crate::events::TeamId) -> usize {
        self.pickups.iter().filter(|(_, c)| c.team() == team).count()
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
            // Prefer *2 variants (CharacterId), fall back to legacy PlayerId variants
            GameEvent::Goal2 {
                character,
                score_left: left,
                score_right: right,
            } => goals.push((time_secs, character, left, right)),
            GameEvent::Goal {
                player,
                score_left: left,
                score_right: right,
            } => goals.push((time_secs, player.to_character_id(), left, right)),
            GameEvent::ShotRelease2 {
                character,
                charge,
                angle,
                power,
            } => shots.push((time_secs, character, charge, angle, power)),
            GameEvent::ShotRelease {
                player,
                charge,
                angle,
                power,
            } => shots.push((time_secs, player.to_character_id(), charge, angle, power)),
            GameEvent::ShotStart2 { character, .. } => shot_starts.push((time_secs, character)),
            GameEvent::ShotStart { player, .. } => shot_starts.push((time_secs, player.to_character_id())),
            GameEvent::Pickup2 { character } => pickups.push((time_secs, character)),
            GameEvent::Pickup { player } => pickups.push((time_secs, player.to_character_id())),
            GameEvent::Drop2 { character } => drops.push((time_secs, character)),
            GameEvent::Drop { player } => drops.push((time_secs, player.to_character_id())),
            GameEvent::StealAttempt2 { attacker } => steal_attempts.push((time_secs, attacker)),
            GameEvent::StealAttempt { attacker } => steal_attempts.push((time_secs, attacker.to_character_id())),
            GameEvent::StealSuccess2 { attacker } => steal_successes.push((time_secs, attacker)),
            GameEvent::StealSuccess { attacker } => steal_successes.push((time_secs, attacker.to_character_id())),
            GameEvent::StealFail2 { attacker } => steal_failures.push((time_secs, attacker)),
            GameEvent::StealFail { attacker } => steal_failures.push((time_secs, attacker.to_character_id())),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::TeamId;

    fn create_test_match() -> ParsedMatch {
        ParsedMatch {
            session_id: "test-session".to_string(),
            level: 1,
            level_name: "Test Level".to_string(),
            left_profile: "Aggressive".to_string(),
            right_profile: "Passive".to_string(),
            seed: 12345,
            duration: 60.0,
            score_left: 3,
            score_right: 2,
            goals: vec![
                (5.0, CharacterId::L0, 1, 0),
                (15.0, CharacterId::R0, 1, 1),
                (25.0, CharacterId::L1, 2, 1),
                (35.0, CharacterId::L0, 3, 1),
                (45.0, CharacterId::R1, 3, 2),
            ],
            shots: vec![
                (4.5, CharacterId::L0, 0.8, 45.0, 600.0),
                (14.5, CharacterId::R0, 0.7, 50.0, 550.0),
                (24.5, CharacterId::L1, 0.9, 40.0, 650.0),
                (34.5, CharacterId::L0, 0.85, 48.0, 620.0),
                (44.5, CharacterId::R1, 0.75, 55.0, 580.0),
                (50.0, CharacterId::L0, 0.6, 60.0, 500.0), // missed shot
            ],
            shot_starts: vec![
                (4.0, CharacterId::L0),
                (14.0, CharacterId::R0),
                (24.0, CharacterId::L1),
                (34.0, CharacterId::L0),
                (44.0, CharacterId::R1),
                (49.5, CharacterId::L0),
            ],
            pickups: vec![
                (3.0, CharacterId::L0),
                (10.0, CharacterId::R0),
                (20.0, CharacterId::L1),
                (30.0, CharacterId::L0),
                (40.0, CharacterId::R1),
                (48.0, CharacterId::L0),
            ],
            drops: vec![(55.0, CharacterId::L0)],
            steal_attempts: vec![
                (8.0, CharacterId::R0),
                (28.0, CharacterId::R1),
                (38.0, CharacterId::L1),
            ],
            steal_successes: vec![(28.0, CharacterId::R1)],
            steal_failures: vec![(8.0, CharacterId::R0), (38.0, CharacterId::L1)],
        }
    }

    #[test]
    fn test_winner_left() {
        let mut m = create_test_match();
        m.score_left = 5;
        m.score_right = 3;
        assert_eq!(m.winner(), "left");
    }

    #[test]
    fn test_winner_right() {
        let mut m = create_test_match();
        m.score_left = 2;
        m.score_right = 4;
        assert_eq!(m.winner(), "right");
    }

    #[test]
    fn test_winner_tie() {
        let mut m = create_test_match();
        m.score_left = 3;
        m.score_right = 3;
        assert_eq!(m.winner(), "tie");
    }

    #[test]
    fn test_profile_for_character() {
        let m = create_test_match();
        assert_eq!(m.profile_for(CharacterId::L0), "Aggressive");
        assert_eq!(m.profile_for(CharacterId::L1), "Aggressive");
        assert_eq!(m.profile_for(CharacterId::R0), "Passive");
        assert_eq!(m.profile_for(CharacterId::R1), "Passive");
    }

    #[test]
    fn test_score_for_character() {
        let m = create_test_match();
        assert_eq!(m.score_for(CharacterId::L0), 3);
        assert_eq!(m.score_for(CharacterId::L1), 3); // Same team, same score
        assert_eq!(m.score_for(CharacterId::R0), 2);
        assert_eq!(m.score_for(CharacterId::R1), 2);
    }

    #[test]
    fn test_goals_for_character() {
        let m = create_test_match();
        // L0 scored 2 goals (at 5.0 and 35.0)
        assert_eq!(m.goals_for(CharacterId::L0), 2);
        // L1 scored 1 goal (at 25.0)
        assert_eq!(m.goals_for(CharacterId::L1), 1);
        // R0 scored 1 goal (at 15.0)
        assert_eq!(m.goals_for(CharacterId::R0), 1);
        // R1 scored 1 goal (at 45.0)
        assert_eq!(m.goals_for(CharacterId::R1), 1);
    }

    #[test]
    fn test_goals_for_team() {
        let m = create_test_match();
        // Left team: L0 (2) + L1 (1) = 3
        assert_eq!(m.goals_for_team(TeamId::Left), 3);
        // Right team: R0 (1) + R1 (1) = 2
        assert_eq!(m.goals_for_team(TeamId::Right), 2);
    }

    #[test]
    fn test_shots_for_character() {
        let m = create_test_match();
        // L0 took 3 shots
        assert_eq!(m.shots_for(CharacterId::L0), 3);
        // L1 took 1 shot
        assert_eq!(m.shots_for(CharacterId::L1), 1);
        // R0 took 1 shot
        assert_eq!(m.shots_for(CharacterId::R0), 1);
        // R1 took 1 shot
        assert_eq!(m.shots_for(CharacterId::R1), 1);
    }

    #[test]
    fn test_shots_for_team() {
        let m = create_test_match();
        // Left team: L0 (3) + L1 (1) = 4
        assert_eq!(m.shots_for_team(TeamId::Left), 4);
        // Right team: R0 (1) + R1 (1) = 2
        assert_eq!(m.shots_for_team(TeamId::Right), 2);
    }

    #[test]
    fn test_steal_attempts_for_character() {
        let m = create_test_match();
        assert_eq!(m.steal_attempts_for(CharacterId::L0), 0);
        assert_eq!(m.steal_attempts_for(CharacterId::L1), 1);
        assert_eq!(m.steal_attempts_for(CharacterId::R0), 1);
        assert_eq!(m.steal_attempts_for(CharacterId::R1), 1);
    }

    #[test]
    fn test_steal_attempts_for_team() {
        let m = create_test_match();
        // Left team: L1 attempted 1
        assert_eq!(m.steal_attempts_for_team(TeamId::Left), 1);
        // Right team: R0 (1) + R1 (1) = 2
        assert_eq!(m.steal_attempts_for_team(TeamId::Right), 2);
    }

    #[test]
    fn test_steal_successes_for_character() {
        let m = create_test_match();
        assert_eq!(m.steal_successes_for(CharacterId::L0), 0);
        assert_eq!(m.steal_successes_for(CharacterId::L1), 0);
        assert_eq!(m.steal_successes_for(CharacterId::R0), 0);
        assert_eq!(m.steal_successes_for(CharacterId::R1), 1);
    }

    #[test]
    fn test_steal_successes_for_team() {
        let m = create_test_match();
        assert_eq!(m.steal_successes_for_team(TeamId::Left), 0);
        assert_eq!(m.steal_successes_for_team(TeamId::Right), 1);
    }

    #[test]
    fn test_pickups_for_character() {
        let m = create_test_match();
        assert_eq!(m.pickups_for(CharacterId::L0), 3);
        assert_eq!(m.pickups_for(CharacterId::L1), 1);
        assert_eq!(m.pickups_for(CharacterId::R0), 1);
        assert_eq!(m.pickups_for(CharacterId::R1), 1);
    }

    #[test]
    fn test_pickups_for_team() {
        let m = create_test_match();
        // Left team: L0 (3) + L1 (1) = 4
        assert_eq!(m.pickups_for_team(TeamId::Left), 4);
        // Right team: R0 (1) + R1 (1) = 2
        assert_eq!(m.pickups_for_team(TeamId::Right), 2);
    }

    #[test]
    fn test_empty_match() {
        let m = ParsedMatch::default();
        assert_eq!(m.winner(), "tie");
        assert_eq!(m.goals_for(CharacterId::L0), 0);
        assert_eq!(m.goals_for_team(TeamId::Left), 0);
        assert_eq!(m.shots_for(CharacterId::L0), 0);
        assert_eq!(m.shots_for_team(TeamId::Right), 0);
        assert_eq!(m.steal_attempts_for(CharacterId::R0), 0);
        assert_eq!(m.pickups_for(CharacterId::L1), 0);
    }

    #[test]
    fn test_all_characters_coverage() {
        let m = create_test_match();
        // Verify all CharacterId variants work with each method
        for character in CharacterId::all() {
            // These should not panic
            let _ = m.goals_for(character);
            let _ = m.shots_for(character);
            let _ = m.steal_attempts_for(character);
            let _ = m.steal_successes_for(character);
            let _ = m.pickups_for(character);
            let _ = m.profile_for(character);
            let _ = m.score_for(character);
        }
    }
}
