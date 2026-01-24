//! Event log parser for analytics

use std::fs;
use std::path::Path;

use crate::events::{parse_event, GameEvent, PlayerId};

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
    let content = fs::read_to_string(path).ok()?;
    let mut parsed = ParsedMatch::default();

    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }

        if let Some((time_ms, event)) = parse_event(line) {
            let time = time_ms as f32 / 1000.0;

            match event {
                GameEvent::SessionStart { session_id, .. } => {
                    parsed.session_id = session_id;
                }
                GameEvent::MatchStart {
                    level,
                    level_name,
                    left_profile,
                    right_profile,
                    seed,
                } => {
                    parsed.level = level;
                    parsed.level_name = level_name;
                    parsed.left_profile = left_profile;
                    parsed.right_profile = right_profile;
                    parsed.seed = seed;
                }
                GameEvent::MatchEnd {
                    score_left,
                    score_right,
                    duration,
                } => {
                    parsed.score_left = score_left;
                    parsed.score_right = score_right;
                    parsed.duration = duration;
                }
                GameEvent::Goal {
                    player,
                    score_left,
                    score_right,
                } => {
                    parsed.goals.push((time, player, score_left, score_right));
                }
                GameEvent::ShotStart { player, .. } => {
                    parsed.shot_starts.push((time, player));
                }
                GameEvent::ShotRelease {
                    player,
                    charge,
                    angle,
                    power,
                } => {
                    parsed.shots.push((time, player, charge, angle, power));
                }
                GameEvent::Pickup { player } => {
                    parsed.pickups.push((time, player));
                }
                GameEvent::Drop { player } => {
                    parsed.drops.push((time, player));
                }
                GameEvent::StealAttempt { attacker } => {
                    parsed.steal_attempts.push((time, attacker));
                }
                GameEvent::StealSuccess { attacker } => {
                    parsed.steal_successes.push((time, attacker));
                }
                GameEvent::StealFail { attacker } => {
                    parsed.steal_failures.push((time, attacker));
                }
                _ => {}
            }
        }
    }

    // Only return if we have a valid match
    if parsed.duration > 0.0 {
        Some(parsed)
    } else {
        None
    }
}

/// Parse all event logs in a directory
pub fn parse_all_logs(dir: &Path) -> Vec<ParsedMatch> {
    let mut matches = Vec::new();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "evlog")
                && let Some(parsed) = parse_event_log(&path)
            {
                matches.push(parsed);
            }
        }
    }

    matches
}
