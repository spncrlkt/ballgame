//! Event log parser for analytics
//!
//! Uses the unified evlog parser and provides analytics-specific types and helpers.

use std::path::Path;

use crate::events::{parse_evlog, parse_all_evlogs, PlayerId};

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
        self.shots.iter().filter(|(_, p, _, _, _)| *p == player).count()
    }

    /// Count goals for a player
    pub fn goals_for(&self, player: PlayerId) -> usize {
        self.goals.iter().filter(|(_, p, _, _)| *p == player).count()
    }

    /// Count steal attempts for a player
    pub fn steal_attempts_for(&self, player: PlayerId) -> usize {
        self.steal_attempts.iter().filter(|(_, p)| *p == player).count()
    }

    /// Count steal successes for a player
    pub fn steal_successes_for(&self, player: PlayerId) -> usize {
        self.steal_successes.iter().filter(|(_, p)| *p == player).count()
    }

    /// Count pickups for a player
    pub fn pickups_for(&self, player: PlayerId) -> usize {
        self.pickups.iter().filter(|(_, p)| *p == player).count()
    }
}

/// Parse a single event log file
pub fn parse_event_log(path: &Path) -> Option<ParsedMatch> {
    let parsed = parse_evlog(path).ok()?;

    if !parsed.is_valid() {
        return None;
    }

    // Convert unified format to analytics format
    let goals: Vec<_> = parsed
        .goals
        .iter()
        .map(|g| (g.time, g.player, g.score_left, g.score_right))
        .collect();

    let shots: Vec<_> = parsed
        .shots
        .iter()
        .map(|s| (s.time, s.player, s.charge, s.angle, s.power))
        .collect();

    let shot_starts: Vec<_> = parsed
        .shot_starts
        .iter()
        .map(|s| (s.time, s.player))
        .collect();

    let pickups: Vec<_> = parsed
        .pickups
        .iter()
        .map(|p| (p.time, p.player))
        .collect();

    let drops: Vec<_> = parsed
        .drops
        .iter()
        .map(|d| (d.time, d.player))
        .collect();

    let steal_attempts: Vec<_> = parsed
        .steal_attempts
        .iter()
        .map(|s| (s.time, s.attacker))
        .collect();

    let steal_successes: Vec<_> = parsed
        .steal_successes
        .iter()
        .map(|s| (s.time, s.attacker))
        .collect();

    let steal_failures: Vec<_> = parsed
        .steal_failures
        .iter()
        .map(|s| (s.time, s.attacker))
        .collect();

    Some(ParsedMatch {
        session_id: parsed.metadata.session_id,
        level: parsed.metadata.level,
        level_name: parsed.metadata.level_name,
        left_profile: parsed.metadata.left_profile,
        right_profile: parsed.metadata.right_profile,
        seed: parsed.metadata.seed,
        duration: parsed.metadata.duration,
        score_left: parsed.metadata.score_left,
        score_right: parsed.metadata.score_right,
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

/// Parse all event logs in a directory
pub fn parse_all_logs(dir: &Path) -> Vec<ParsedMatch> {
    parse_all_evlogs(dir)
        .into_iter()
        .map(|parsed| ParsedMatch {
            session_id: parsed.metadata.session_id,
            level: parsed.metadata.level,
            level_name: parsed.metadata.level_name,
            left_profile: parsed.metadata.left_profile,
            right_profile: parsed.metadata.right_profile,
            seed: parsed.metadata.seed,
            duration: parsed.metadata.duration,
            score_left: parsed.metadata.score_left,
            score_right: parsed.metadata.score_right,
            goals: parsed.goals.iter().map(|g| (g.time, g.player, g.score_left, g.score_right)).collect(),
            shots: parsed.shots.iter().map(|s| (s.time, s.player, s.charge, s.angle, s.power)).collect(),
            shot_starts: parsed.shot_starts.iter().map(|s| (s.time, s.player)).collect(),
            pickups: parsed.pickups.iter().map(|p| (p.time, p.player)).collect(),
            drops: parsed.drops.iter().map(|d| (d.time, d.player)).collect(),
            steal_attempts: parsed.steal_attempts.iter().map(|s| (s.time, s.attacker)).collect(),
            steal_successes: parsed.steal_successes.iter().map(|s| (s.time, s.attacker)).collect(),
            steal_failures: parsed.steal_failures.iter().map(|s| (s.time, s.attacker)).collect(),
        })
        .collect()
}
