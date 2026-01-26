//! Unified event log parser
//!
//! Consolidates parsing logic for replay and analytics systems.
//! Both systems share the same .evlog file format but need different views of the data.

use std::fs;
use std::path::Path;

use super::format::parse_event;
use super::types::{GameEvent, PlayerId};

/// Parsed event with timestamp
#[derive(Debug, Clone)]
pub struct TimestampedEvent {
    /// Time in milliseconds from match start
    pub time_ms: u32,
    /// The parsed event
    pub event: GameEvent,
}

/// Match metadata extracted from SessionStart and MatchStart events
#[derive(Debug, Clone, Default)]
pub struct MatchMetadata {
    /// Session ID
    pub session_id: String,
    /// Level number (1-based)
    pub level: u32,
    /// Level name
    pub level_name: String,
    /// Left player AI profile
    pub left_profile: String,
    /// Right player AI profile
    pub right_profile: String,
    /// RNG seed for reproducibility
    pub seed: u64,
    /// Match duration in seconds (from MatchEnd)
    pub duration: f32,
    /// Final left score (from MatchEnd)
    pub score_left: u32,
    /// Final right score (from MatchEnd)
    pub score_right: u32,
}

impl MatchMetadata {
    /// Determine winner from final scores
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

    /// Get final score for a player side
    pub fn score_for(&self, player: PlayerId) -> u32 {
        match player {
            PlayerId::L => self.score_left,
            PlayerId::R => self.score_right,
        }
    }
}

/// Tick frame data for position interpolation (replay)
#[derive(Debug, Clone)]
pub struct TickData {
    /// Time in milliseconds
    pub time_ms: u32,
    /// Frame number
    pub frame: u64,
    /// Left player position (x, y)
    pub left_pos: (f32, f32),
    /// Left player velocity (x, y)
    pub left_vel: (f32, f32),
    /// Right player position (x, y)
    pub right_pos: (f32, f32),
    /// Right player velocity (x, y)
    pub right_vel: (f32, f32),
    /// Ball position (x, y)
    pub ball_pos: (f32, f32),
    /// Ball velocity (x, y)
    pub ball_vel: (f32, f32),
    /// Ball state: 'F' = Free, 'H' = Held, 'I' = InFlight
    pub ball_state: char,
}

/// Goal event data
#[derive(Debug, Clone)]
pub struct GoalData {
    /// Time in seconds
    pub time: f32,
    /// Scoring player
    pub player: PlayerId,
    /// Left score after goal
    pub score_left: u32,
    /// Right score after goal
    pub score_right: u32,
}

/// Shot event data (release)
#[derive(Debug, Clone)]
pub struct ShotData {
    /// Time in seconds
    pub time: f32,
    /// Shooting player
    pub player: PlayerId,
    /// Charge time in seconds
    pub charge: f32,
    /// Launch angle in degrees
    pub angle: f32,
    /// Launch power/speed
    pub power: f32,
}

/// Shot start event data
#[derive(Debug, Clone)]
pub struct ShotStartData {
    /// Time in seconds
    pub time: f32,
    /// Shooting player
    pub player: PlayerId,
}

/// Ball pickup event data
#[derive(Debug, Clone)]
pub struct PickupData {
    /// Time in seconds
    pub time: f32,
    /// Player who picked up the ball
    pub player: PlayerId,
}

/// Ball drop event data
#[derive(Debug, Clone)]
pub struct DropData {
    /// Time in seconds
    pub time: f32,
    /// Player who dropped the ball
    pub player: PlayerId,
}

/// Steal attempt event data
#[derive(Debug, Clone)]
pub struct StealAttemptData {
    /// Time in seconds
    pub time: f32,
    /// Player attempting the steal
    pub attacker: PlayerId,
}

/// Steal success event data
#[derive(Debug, Clone)]
pub struct StealSuccessData {
    /// Time in seconds
    pub time: f32,
    /// Player who succeeded
    pub attacker: PlayerId,
}

/// Steal failure event data
#[derive(Debug, Clone)]
pub struct StealFailData {
    /// Time in seconds
    pub time: f32,
    /// Player who failed
    pub attacker: PlayerId,
}

/// AI goal change event data
#[derive(Debug, Clone)]
pub struct AiGoalData {
    /// Time in milliseconds
    pub time_ms: u32,
    /// Player whose goal changed
    pub player: PlayerId,
    /// New goal description
    pub goal: String,
}

/// Input state snapshot data
#[derive(Debug, Clone)]
pub struct InputData {
    /// Time in milliseconds
    pub time_ms: u32,
    /// Player this input belongs to
    pub player: PlayerId,
    /// Horizontal movement (-1.0 to 1.0)
    pub move_x: f32,
    /// Jump button pressed
    pub jump: bool,
    /// Throw button held
    pub throw: bool,
    /// Pickup button pressed
    pub pickup: bool,
}

/// Parsed event log with all data accessible
#[derive(Debug, Clone, Default)]
pub struct ParsedEvlog {
    /// Match metadata
    pub metadata: MatchMetadata,
    /// Tick frames for position interpolation (sorted by time)
    pub ticks: Vec<TickData>,
    /// All timestamped events (for replay timeline)
    pub raw_events: Vec<TimestampedEvent>,
    /// Goal events
    pub goals: Vec<GoalData>,
    /// Shot release events
    pub shots: Vec<ShotData>,
    /// Shot start events
    pub shot_starts: Vec<ShotStartData>,
    /// Ball pickup events
    pub pickups: Vec<PickupData>,
    /// Ball drop events
    pub drops: Vec<DropData>,
    /// Steal attempt events
    pub steal_attempts: Vec<StealAttemptData>,
    /// Steal success events
    pub steal_successes: Vec<StealSuccessData>,
    /// Steal failure events
    pub steal_failures: Vec<StealFailData>,
    /// AI goal change events
    pub ai_goals: Vec<AiGoalData>,
    /// Input state snapshots (sorted by time)
    pub inputs: Vec<InputData>,
    /// Maximum observed timestamp (milliseconds)
    pub max_time_ms: u32,
}

impl ParsedEvlog {
    /// Check if this is a valid, complete match log
    pub fn is_valid(&self) -> bool {
        self.metadata.duration > 0.0
    }

    /// Get duration in milliseconds (uses MatchEnd duration or max observed time)
    pub fn duration_ms(&self) -> u32 {
        if self.metadata.duration > 0.0 {
            (self.metadata.duration * 1000.0) as u32
        } else {
            self.max_time_ms
        }
    }

    // --- Analytics helpers ---

    /// Count shots for a player
    pub fn shots_for(&self, player: PlayerId) -> usize {
        self.shots.iter().filter(|s| s.player == player).count()
    }

    /// Count goals for a player
    pub fn goals_for(&self, player: PlayerId) -> usize {
        self.goals.iter().filter(|g| g.player == player).count()
    }

    /// Count steal attempts for a player
    pub fn steal_attempts_for(&self, player: PlayerId) -> usize {
        self.steal_attempts
            .iter()
            .filter(|s| s.attacker == player)
            .count()
    }

    /// Count steal successes for a player
    pub fn steal_successes_for(&self, player: PlayerId) -> usize {
        self.steal_successes
            .iter()
            .filter(|s| s.attacker == player)
            .count()
    }

    /// Count pickups for a player
    pub fn pickups_for(&self, player: PlayerId) -> usize {
        self.pickups.iter().filter(|p| p.player == player).count()
    }

    /// Get inputs for a specific player (sorted by time)
    pub fn inputs_for(&self, player: PlayerId) -> impl Iterator<Item = &InputData> {
        self.inputs.iter().filter(move |i| i.player == player)
    }

    // --- Replay helpers ---

    /// Find ticks in a time range
    pub fn ticks_in_range(&self, start_ms: u32, end_ms: u32) -> impl Iterator<Item = &TickData> {
        self.ticks
            .iter()
            .filter(move |t| t.time_ms >= start_ms && t.time_ms <= end_ms)
    }

    /// Find two tick frames bracketing a time for interpolation
    /// Returns (prev_tick, next_tick, interpolation_factor)
    pub fn find_tick_bracket(&self, time_ms: u32) -> Option<(&TickData, &TickData, f32)> {
        if self.ticks.is_empty() {
            return None;
        }

        // Binary search for insertion point
        let idx = self.ticks.partition_point(|t| t.time_ms <= time_ms);

        if idx == 0 {
            let first = &self.ticks[0];
            return Some((first, first, 0.0));
        }
        if idx >= self.ticks.len() {
            let last = self.ticks.last().unwrap();
            return Some((last, last, 1.0));
        }

        let prev = &self.ticks[idx - 1];
        let next = &self.ticks[idx];

        let t = if next.time_ms > prev.time_ms {
            (time_ms - prev.time_ms) as f32 / (next.time_ms - prev.time_ms) as f32
        } else {
            0.0
        };

        Some((prev, next, t))
    }

    /// Get events before a given time (for replay timeline)
    pub fn events_before(&self, time_ms: u32) -> impl Iterator<Item = &TimestampedEvent> {
        self.raw_events.iter().filter(move |e| e.time_ms <= time_ms)
    }

    /// Get the most recent AI goal for a player at a given time
    pub fn current_ai_goal(&self, time_ms: u32, player: PlayerId) -> Option<&str> {
        self.ai_goals
            .iter()
            .filter(|g| g.time_ms <= time_ms && g.player == player)
            .last()
            .map(|g| g.goal.as_str())
    }
}

/// Parse an event log file into a structured format
pub fn parse_evlog<P: AsRef<Path>>(path: P) -> Result<ParsedEvlog, String> {
    let content = fs::read_to_string(path.as_ref())
        .map_err(|e| format!("Failed to read evlog file: {}", e))?;

    Ok(parse_evlog_content(&content))
}

/// Parse event log content (for testing or in-memory parsing)
pub fn parse_evlog_content(content: &str) -> ParsedEvlog {
    let mut parsed = ParsedEvlog::default();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((time_ms, event)) = parse_event(line) {
            parsed.max_time_ms = parsed.max_time_ms.max(time_ms);
            let time = time_ms as f32 / 1000.0;

            match &event {
                GameEvent::SessionStart { session_id, .. } => {
                    parsed.metadata.session_id = session_id.clone();
                }
                GameEvent::MatchStart {
                    level,
                    level_name,
                    left_profile,
                    right_profile,
                    seed,
                } => {
                    parsed.metadata.level = *level;
                    parsed.metadata.level_name = level_name.clone();
                    parsed.metadata.left_profile = left_profile.clone();
                    parsed.metadata.right_profile = right_profile.clone();
                    parsed.metadata.seed = *seed;
                }
                GameEvent::MatchEnd {
                    score_left,
                    score_right,
                    duration,
                } => {
                    parsed.metadata.score_left = *score_left;
                    parsed.metadata.score_right = *score_right;
                    parsed.metadata.duration = *duration;
                }
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
                    parsed.ticks.push(TickData {
                        time_ms,
                        frame: *frame,
                        left_pos: *left_pos,
                        left_vel: *left_vel,
                        right_pos: *right_pos,
                        right_vel: *right_vel,
                        ball_pos: *ball_pos,
                        ball_vel: *ball_vel,
                        ball_state: *ball_state,
                    });
                }
                GameEvent::Goal {
                    player,
                    score_left,
                    score_right,
                } => {
                    parsed.goals.push(GoalData {
                        time,
                        player: *player,
                        score_left: *score_left,
                        score_right: *score_right,
                    });
                    // Also store in raw_events for replay
                    parsed.raw_events.push(TimestampedEvent {
                        time_ms,
                        event: event.clone(),
                    });
                }
                GameEvent::ShotStart { player, .. } => {
                    parsed.shot_starts.push(ShotStartData {
                        time,
                        player: *player,
                    });
                    parsed.raw_events.push(TimestampedEvent {
                        time_ms,
                        event: event.clone(),
                    });
                }
                GameEvent::ShotRelease {
                    player,
                    charge,
                    angle,
                    power,
                } => {
                    parsed.shots.push(ShotData {
                        time,
                        player: *player,
                        charge: *charge,
                        angle: *angle,
                        power: *power,
                    });
                    parsed.raw_events.push(TimestampedEvent {
                        time_ms,
                        event: event.clone(),
                    });
                }
                GameEvent::Pickup { player } => {
                    parsed.pickups.push(PickupData {
                        time,
                        player: *player,
                    });
                    parsed.raw_events.push(TimestampedEvent {
                        time_ms,
                        event: event.clone(),
                    });
                }
                GameEvent::Drop { player } => {
                    parsed.drops.push(DropData {
                        time,
                        player: *player,
                    });
                    parsed.raw_events.push(TimestampedEvent {
                        time_ms,
                        event: event.clone(),
                    });
                }
                GameEvent::StealAttempt { attacker } => {
                    parsed.steal_attempts.push(StealAttemptData {
                        time,
                        attacker: *attacker,
                    });
                    parsed.raw_events.push(TimestampedEvent {
                        time_ms,
                        event: event.clone(),
                    });
                }
                GameEvent::StealSuccess { attacker } => {
                    parsed.steal_successes.push(StealSuccessData {
                        time,
                        attacker: *attacker,
                    });
                    parsed.raw_events.push(TimestampedEvent {
                        time_ms,
                        event: event.clone(),
                    });
                }
                GameEvent::StealFail { attacker } => {
                    parsed.steal_failures.push(StealFailData {
                        time,
                        attacker: *attacker,
                    });
                    parsed.raw_events.push(TimestampedEvent {
                        time_ms,
                        event: event.clone(),
                    });
                }
                GameEvent::AiGoal { player, goal } => {
                    parsed.ai_goals.push(AiGoalData {
                        time_ms,
                        player: *player,
                        goal: goal.clone(),
                    });
                    parsed.raw_events.push(TimestampedEvent {
                        time_ms,
                        event: event.clone(),
                    });
                }
                GameEvent::Input {
                    player,
                    move_x,
                    jump,
                    throw,
                    pickup,
                } => {
                    parsed.inputs.push(InputData {
                        time_ms,
                        player: *player,
                        move_x: *move_x,
                        jump: *jump,
                        throw: *throw,
                        pickup: *pickup,
                    });
                }
                _ => {
                    // Ignore Config and other events
                }
            }
        }
    }

    // Sort ticks, events, and inputs by time (should already be sorted, but ensure)
    parsed.ticks.sort_by_key(|t| t.time_ms);
    parsed.raw_events.sort_by_key(|e| e.time_ms);
    parsed.inputs.sort_by_key(|i| i.time_ms);

    parsed
}

/// Parse all event logs in a directory
pub fn parse_all_evlogs<P: AsRef<Path>>(dir: P) -> Vec<ParsedEvlog> {
    let mut results = Vec::new();

    if let Ok(entries) = fs::read_dir(dir.as_ref()) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "evlog") {
                if let Ok(parsed) = parse_evlog(&path) {
                    if parsed.is_valid() {
                        results.push(parsed);
                    }
                }
            }
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    // Uses the actual compact format: T:NNNNN|CODE|data...
    const SAMPLE_EVLOG: &str = r#"
T:00000|SE|test_session|2026-01-24
T:00000|MS|3|Test Level|Balanced|Aggressive|12345
T:00100|T|1|100.0,50.0|-5.0,0.0|400.0,50.0|2.0,0.0|250.0,100.0|0.0,-50.0|F
T:00200|T|2|95.0,50.0|-5.0,0.0|402.0,50.0|2.0,0.0|250.0,75.0|0.0,-50.0|F
T:00500|PU|L
T:01000|SS|L|100.0,50.0|0.85
T:01500|SR|L|0.50|45.0|300.0
T:02000|G|L|1|0
T:05000|SA|R
T:05100|S-|R
T:10000|ME|1|0|10.0
"#;

    #[test]
    fn test_parse_evlog_content() {
        let parsed = parse_evlog_content(SAMPLE_EVLOG);

        assert!(parsed.is_valid());
        assert_eq!(parsed.metadata.session_id, "test_session");
        assert_eq!(parsed.metadata.level, 3);
        assert_eq!(parsed.metadata.level_name, "Test Level");
        assert_eq!(parsed.metadata.left_profile, "Balanced");
        assert_eq!(parsed.metadata.right_profile, "Aggressive");
        assert_eq!(parsed.metadata.seed, 12345);
        assert_eq!(parsed.metadata.duration, 10.0);
        assert_eq!(parsed.metadata.score_left, 1);
        assert_eq!(parsed.metadata.score_right, 0);
    }

    #[test]
    fn test_ticks() {
        let parsed = parse_evlog_content(SAMPLE_EVLOG);

        assert_eq!(parsed.ticks.len(), 2);
        assert_eq!(parsed.ticks[0].time_ms, 100);
        assert_eq!(parsed.ticks[0].frame, 1);
        assert_eq!(parsed.ticks[0].ball_state, 'F');
    }

    #[test]
    fn test_events() {
        let parsed = parse_evlog_content(SAMPLE_EVLOG);

        assert_eq!(parsed.goals.len(), 1);
        assert_eq!(parsed.goals[0].player, PlayerId::L);

        assert_eq!(parsed.shots.len(), 1);
        assert_eq!(parsed.shots[0].charge, 0.5);

        assert_eq!(parsed.pickups.len(), 1);
        assert_eq!(parsed.steal_attempts.len(), 1);
        assert_eq!(parsed.steal_failures.len(), 1);
    }

    #[test]
    fn test_find_tick_bracket() {
        let parsed = parse_evlog_content(SAMPLE_EVLOG);

        // Between ticks
        let (prev, next, t) = parsed.find_tick_bracket(150).unwrap();
        assert_eq!(prev.time_ms, 100);
        assert_eq!(next.time_ms, 200);
        assert!((t - 0.5).abs() < 0.01);

        // Before first tick
        let (prev, next, t) = parsed.find_tick_bracket(50).unwrap();
        assert_eq!(prev.time_ms, 100);
        assert_eq!(next.time_ms, 100);
        assert_eq!(t, 0.0);

        // After last tick
        let (prev, next, t) = parsed.find_tick_bracket(300).unwrap();
        assert_eq!(prev.time_ms, 200);
        assert_eq!(next.time_ms, 200);
        assert_eq!(t, 1.0);
    }

    #[test]
    fn test_analytics_helpers() {
        let parsed = parse_evlog_content(SAMPLE_EVLOG);

        assert_eq!(parsed.goals_for(PlayerId::L), 1);
        assert_eq!(parsed.goals_for(PlayerId::R), 0);
        assert_eq!(parsed.shots_for(PlayerId::L), 1);
        assert_eq!(parsed.pickups_for(PlayerId::L), 1);
        assert_eq!(parsed.steal_attempts_for(PlayerId::R), 1);
    }

    #[test]
    fn test_metadata_helpers() {
        let parsed = parse_evlog_content(SAMPLE_EVLOG);

        assert_eq!(parsed.metadata.winner(), "left");
        assert_eq!(parsed.metadata.profile_for(PlayerId::L), "Balanced");
        assert_eq!(parsed.metadata.profile_for(PlayerId::R), "Aggressive");
        assert_eq!(parsed.metadata.score_for(PlayerId::L), 1);
    }
}
