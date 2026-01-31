//! Team interaction analysis for training sessions
//!
//! Analyzes pass events, blocks, and team coordination from training databases.

use std::collections::HashMap;
use std::path::Path;

use rusqlite::Connection;

type AnyResult<T> = Result<T, Box<dyn std::error::Error>>;

/// Summary statistics for a single character
#[derive(Debug, Clone, Default)]
pub struct CharacterTeamStats {
    pub character_id: String,
    pub passes_attempted: usize,
    pub passes_completed: usize,
    pub passes_intercepted: usize,
    pub passes_missed: usize,
    pub passes_received: usize,
    pub blocks_activated: usize,
    pub blocks_intercepted: usize,
}

impl CharacterTeamStats {
    pub fn pass_completion_rate(&self) -> f32 {
        if self.passes_attempted == 0 {
            0.0
        } else {
            self.passes_completed as f32 / self.passes_attempted as f32 * 100.0
        }
    }

    pub fn block_success_rate(&self) -> f32 {
        if self.blocks_activated == 0 {
            0.0
        } else {
            self.blocks_intercepted as f32 / self.blocks_activated as f32 * 100.0
        }
    }
}

/// Summary statistics for a team (Left or Right)
#[derive(Debug, Clone, Default)]
pub struct TeamStats {
    pub team_id: String,
    pub total_passes_attempted: usize,
    pub total_passes_completed: usize,
    pub total_passes_intercepted: usize,
    pub total_passes_missed: usize,
    pub total_blocks_activated: usize,
    pub total_blocks_intercepted: usize,
    pub characters: Vec<CharacterTeamStats>,
}

impl TeamStats {
    pub fn pass_completion_rate(&self) -> f32 {
        if self.total_passes_attempted == 0 {
            0.0
        } else {
            self.total_passes_completed as f32 / self.total_passes_attempted as f32 * 100.0
        }
    }

    pub fn turnover_rate(&self) -> f32 {
        if self.total_passes_attempted == 0 {
            0.0
        } else {
            (self.total_passes_intercepted + self.total_passes_missed) as f32
                / self.total_passes_attempted as f32
                * 100.0
        }
    }
}

/// Pass event with timing
#[derive(Debug, Clone)]
pub struct PassEvent {
    pub time_ms: i64,
    pub match_id: i64,
    pub from_character: String,
    pub to_character: String,
    pub outcome: PassOutcome,
}

/// Outcome of a pass attempt
#[derive(Debug, Clone, PartialEq)]
pub enum PassOutcome {
    Completed,
    Intercepted,
    Missed,
    Unknown, // Pass initiated but no outcome recorded
}

/// Block event with timing
#[derive(Debug, Clone)]
pub struct BlockEvent {
    pub time_ms: i64,
    pub match_id: i64,
    pub character: String,
    pub event_type: BlockEventType,
}

#[derive(Debug, Clone)]
pub enum BlockEventType {
    Activated,
    Deactivated,
    Intercepted { ball_state: char },
}

/// Full team interaction report
#[derive(Debug, Clone)]
pub struct TeamInteractionReport {
    pub db_path: String,
    pub session_count: usize,
    pub match_count: usize,
    pub total_events: usize,
    pub left_team: TeamStats,
    pub right_team: TeamStats,
    pub pass_events: Vec<PassEvent>,
    pub block_events: Vec<BlockEvent>,
    pub pass_timing_analysis: PassTimingAnalysis,
}

/// Analysis of pass timing patterns
#[derive(Debug, Clone, Default)]
pub struct PassTimingAnalysis {
    pub avg_pass_interval_ms: f32,
    pub min_pass_interval_ms: i64,
    pub max_pass_interval_ms: i64,
    pub passes_under_500ms: usize,
    pub passes_under_1000ms: usize,
    pub passes_under_2000ms: usize,
}

impl TeamInteractionReport {
    pub fn to_markdown(&self) -> String {
        let mut out = String::new();

        out.push_str("# Team Interaction Analysis\n\n");
        out.push_str(&format!("Database: `{}`\n\n", self.db_path));
        out.push_str(&format!("Sessions: {}\n", self.session_count));
        out.push_str(&format!("Matches: {}\n", self.match_count));
        out.push_str(&format!("Total events analyzed: {}\n\n", self.total_events));

        // Summary table
        out.push_str("## Team Summary\n\n");
        out.push_str("| Metric | Left Team | Right Team |\n");
        out.push_str("|--------|-----------|------------|\n");
        out.push_str(&format!(
            "| Passes Attempted | {} | {} |\n",
            self.left_team.total_passes_attempted, self.right_team.total_passes_attempted
        ));
        out.push_str(&format!(
            "| Passes Completed | {} | {} |\n",
            self.left_team.total_passes_completed, self.right_team.total_passes_completed
        ));
        out.push_str(&format!(
            "| Pass Completion % | {:.1}% | {:.1}% |\n",
            self.left_team.pass_completion_rate(),
            self.right_team.pass_completion_rate()
        ));
        out.push_str(&format!(
            "| Passes Intercepted | {} | {} |\n",
            self.left_team.total_passes_intercepted, self.right_team.total_passes_intercepted
        ));
        out.push_str(&format!(
            "| Passes Missed | {} | {} |\n",
            self.left_team.total_passes_missed, self.right_team.total_passes_missed
        ));
        out.push_str(&format!(
            "| Turnover Rate | {:.1}% | {:.1}% |\n",
            self.left_team.turnover_rate(),
            self.right_team.turnover_rate()
        ));
        out.push_str(&format!(
            "| Blocks Activated | {} | {} |\n",
            self.left_team.total_blocks_activated, self.right_team.total_blocks_activated
        ));
        out.push_str(&format!(
            "| Blocks Intercepted | {} | {} |\n\n",
            self.left_team.total_blocks_intercepted, self.right_team.total_blocks_intercepted
        ));

        // Per-character breakdown
        out.push_str("## Character Breakdown\n\n");
        out.push_str("### Left Team\n\n");
        self.format_character_table(&mut out, &self.left_team.characters);

        out.push_str("### Right Team\n\n");
        self.format_character_table(&mut out, &self.right_team.characters);

        // Pass timing analysis
        out.push_str("## Pass Timing Analysis\n\n");
        if self.pass_timing_analysis.avg_pass_interval_ms > 0.0 {
            out.push_str(&format!(
                "- Average interval between passes: {:.0}ms\n",
                self.pass_timing_analysis.avg_pass_interval_ms
            ));
            out.push_str(&format!(
                "- Min/Max interval: {}ms / {}ms\n",
                self.pass_timing_analysis.min_pass_interval_ms,
                self.pass_timing_analysis.max_pass_interval_ms
            ));
            out.push_str(&format!(
                "- Quick passes (<500ms): {}\n",
                self.pass_timing_analysis.passes_under_500ms
            ));
            out.push_str(&format!(
                "- Fast passes (<1s): {}\n",
                self.pass_timing_analysis.passes_under_1000ms
            ));
            out.push_str(&format!(
                "- Normal passes (<2s): {}\n\n",
                self.pass_timing_analysis.passes_under_2000ms
            ));
        } else {
            out.push_str("No pass timing data available.\n\n");
        }

        // Recent pass events (last 20)
        if !self.pass_events.is_empty() {
            out.push_str("## Recent Pass Events\n\n");
            out.push_str("| Time (ms) | Match | From | To | Outcome |\n");
            out.push_str("|-----------|-------|------|----|---------|\n");
            for event in self.pass_events.iter().rev().take(20) {
                let outcome_str = match event.outcome {
                    PassOutcome::Completed => "✓ Completed",
                    PassOutcome::Intercepted => "✗ Intercepted",
                    PassOutcome::Missed => "✗ Missed",
                    PassOutcome::Unknown => "? Unknown",
                };
                out.push_str(&format!(
                    "| {} | {} | {} | {} | {} |\n",
                    event.time_ms,
                    event.match_id,
                    event.from_character,
                    event.to_character,
                    outcome_str
                ));
            }
            out.push('\n');
        }

        // Recent block events (last 20)
        if !self.block_events.is_empty() {
            out.push_str("## Recent Block Events\n\n");
            out.push_str("| Time (ms) | Match | Character | Event |\n");
            out.push_str("|-----------|-------|-----------|-------|\n");
            for event in self.block_events.iter().rev().take(20) {
                let event_str = match &event.event_type {
                    BlockEventType::Activated => "Activated".to_string(),
                    BlockEventType::Deactivated => "Deactivated".to_string(),
                    BlockEventType::Intercepted { ball_state } => {
                        format!("Intercepted ({})", ball_state)
                    }
                };
                out.push_str(&format!(
                    "| {} | {} | {} | {} |\n",
                    event.time_ms, event.match_id, event.character, event_str
                ));
            }
            out.push('\n');
        }

        out
    }

    fn format_character_table(&self, out: &mut String, characters: &[CharacterTeamStats]) {
        if characters.is_empty() {
            out.push_str("No character data available.\n\n");
            return;
        }

        out.push_str("| Character | Pass Att | Pass Cmp | Pass % | Intercepted | Missed | Received | Blocks | Block Int |\n");
        out.push_str("|-----------|----------|----------|--------|-------------|--------|----------|--------|-----------|\n");
        for c in characters {
            out.push_str(&format!(
                "| {} | {} | {} | {:.1}% | {} | {} | {} | {} | {} |\n",
                c.character_id,
                c.passes_attempted,
                c.passes_completed,
                c.pass_completion_rate(),
                c.passes_intercepted,
                c.passes_missed,
                c.passes_received,
                c.blocks_activated,
                c.blocks_intercepted
            ));
        }
        out.push('\n');
    }
}

/// Run team interaction analysis on a training database
pub fn run_team_interaction_analysis(db_path: &Path) -> AnyResult<TeamInteractionReport> {
    let conn = Connection::open(db_path)?;

    // Count sessions
    let session_count: usize = conn
        .query_row("SELECT COUNT(*) FROM sessions", [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap_or(0) as usize;

    // Count matches
    let match_count: usize = conn
        .query_row("SELECT COUNT(*) FROM matches", [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap_or(0) as usize;

    // Initialize character stats
    let mut char_stats: HashMap<String, CharacterTeamStats> = HashMap::new();
    for id in ["L0", "L1", "R0", "R1"] {
        char_stats.insert(
            id.to_string(),
            CharacterTeamStats {
                character_id: id.to_string(),
                ..Default::default()
            },
        );
    }

    // Collect all pass and block events
    let mut pass_events = Vec::new();
    let mut block_events = Vec::new();
    let mut total_events = 0usize;

    // Query events table for team interaction events
    let mut stmt = conn.prepare(
        "SELECT match_id, time_ms, event_type, data FROM events
         WHERE event_type IN ('PA', 'PC', 'PI', 'PM', 'BA', 'BD', 'BI')
         ORDER BY match_id, time_ms",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;

    // Track pending passes (PA events that haven't been resolved yet)
    let mut pending_passes: HashMap<(i64, String), PassEvent> = HashMap::new();

    for row in rows {
        let (match_id, time_ms, event_type, data) = row?;
        total_events += 1;

        match event_type.as_str() {
            "PA" => {
                // Pass initiated: T:XXXXX|PA|from|to
                if let Some((from, to)) = parse_pass_data(&data) {
                    let event = PassEvent {
                        time_ms,
                        match_id,
                        from_character: from.clone(),
                        to_character: to.clone(),
                        outcome: PassOutcome::Unknown,
                    };
                    // Track as pending
                    pending_passes.insert((match_id, from.clone()), event.clone());
                    pass_events.push(event);

                    if let Some(stats) = char_stats.get_mut(&from) {
                        stats.passes_attempted += 1;
                    }
                }
            }
            "PC" => {
                // Pass completed
                if let Some((passer, receiver)) = parse_pass_data(&data) {
                    // Update pending pass outcome
                    if let Some(pending) = pending_passes.remove(&(match_id, passer.clone())) {
                        // Find and update the event in pass_events
                        for event in pass_events.iter_mut().rev() {
                            if event.match_id == pending.match_id
                                && event.from_character == pending.from_character
                                && event.outcome == PassOutcome::Unknown
                            {
                                event.outcome = PassOutcome::Completed;
                                break;
                            }
                        }
                    }

                    if let Some(stats) = char_stats.get_mut(&passer) {
                        stats.passes_completed += 1;
                    }
                    if let Some(stats) = char_stats.get_mut(&receiver) {
                        stats.passes_received += 1;
                    }
                }
            }
            "PI" => {
                // Pass intercepted
                if let Some((passer, _interceptor)) = parse_pass_data(&data) {
                    // Update pending pass outcome
                    if let Some(pending) = pending_passes.remove(&(match_id, passer.clone())) {
                        for event in pass_events.iter_mut().rev() {
                            if event.match_id == pending.match_id
                                && event.from_character == pending.from_character
                                && event.outcome == PassOutcome::Unknown
                            {
                                event.outcome = PassOutcome::Intercepted;
                                break;
                            }
                        }
                    }

                    if let Some(stats) = char_stats.get_mut(&passer) {
                        stats.passes_intercepted += 1;
                    }
                }
            }
            "PM" => {
                // Pass missed
                if let Some((passer, _target)) = parse_pass_data(&data) {
                    // Update pending pass outcome
                    if let Some(pending) = pending_passes.remove(&(match_id, passer.clone())) {
                        for event in pass_events.iter_mut().rev() {
                            if event.match_id == pending.match_id
                                && event.from_character == pending.from_character
                                && event.outcome == PassOutcome::Unknown
                            {
                                event.outcome = PassOutcome::Missed;
                                break;
                            }
                        }
                    }

                    if let Some(stats) = char_stats.get_mut(&passer) {
                        stats.passes_missed += 1;
                    }
                }
            }
            "BA" => {
                // Block activated
                if let Some(character) = parse_single_character(&data) {
                    block_events.push(BlockEvent {
                        time_ms,
                        match_id,
                        character: character.clone(),
                        event_type: BlockEventType::Activated,
                    });
                    if let Some(stats) = char_stats.get_mut(&character) {
                        stats.blocks_activated += 1;
                    }
                }
            }
            "BD" => {
                // Block deactivated
                if let Some(character) = parse_single_character(&data) {
                    block_events.push(BlockEvent {
                        time_ms,
                        match_id,
                        character,
                        event_type: BlockEventType::Deactivated,
                    });
                }
            }
            "BI" => {
                // Block intercepted
                if let Some((character, ball_state)) = parse_block_intercept(&data) {
                    block_events.push(BlockEvent {
                        time_ms,
                        match_id,
                        character: character.clone(),
                        event_type: BlockEventType::Intercepted { ball_state },
                    });
                    if let Some(stats) = char_stats.get_mut(&character) {
                        stats.blocks_intercepted += 1;
                    }
                }
            }
            _ => {}
        }
    }

    // Calculate pass timing analysis
    let pass_timing_analysis = calculate_pass_timing(&pass_events);

    // Build team stats
    let left_chars: Vec<CharacterTeamStats> = ["L0", "L1"]
        .iter()
        .filter_map(|id| char_stats.get(*id).cloned())
        .collect();

    let right_chars: Vec<CharacterTeamStats> = ["R0", "R1"]
        .iter()
        .filter_map(|id| char_stats.get(*id).cloned())
        .collect();

    let left_team = TeamStats {
        team_id: "Left".to_string(),
        total_passes_attempted: left_chars.iter().map(|c| c.passes_attempted).sum(),
        total_passes_completed: left_chars.iter().map(|c| c.passes_completed).sum(),
        total_passes_intercepted: left_chars.iter().map(|c| c.passes_intercepted).sum(),
        total_passes_missed: left_chars.iter().map(|c| c.passes_missed).sum(),
        total_blocks_activated: left_chars.iter().map(|c| c.blocks_activated).sum(),
        total_blocks_intercepted: left_chars.iter().map(|c| c.blocks_intercepted).sum(),
        characters: left_chars,
    };

    let right_team = TeamStats {
        team_id: "Right".to_string(),
        total_passes_attempted: right_chars.iter().map(|c| c.passes_attempted).sum(),
        total_passes_completed: right_chars.iter().map(|c| c.passes_completed).sum(),
        total_passes_intercepted: right_chars.iter().map(|c| c.passes_intercepted).sum(),
        total_passes_missed: right_chars.iter().map(|c| c.passes_missed).sum(),
        total_blocks_activated: right_chars.iter().map(|c| c.blocks_activated).sum(),
        total_blocks_intercepted: right_chars.iter().map(|c| c.blocks_intercepted).sum(),
        characters: right_chars,
    };

    Ok(TeamInteractionReport {
        db_path: db_path.display().to_string(),
        session_count,
        match_count,
        total_events,
        left_team,
        right_team,
        pass_events,
        block_events,
        pass_timing_analysis,
    })
}

/// Parse pass data from event format: "T:XXXXX|PA|from|to" -> data is "from|to"
fn parse_pass_data(data: &str) -> Option<(String, String)> {
    // Data format after event parsing: "from|to" or just the data portion
    let parts: Vec<&str> = data.split('|').collect();

    // Handle format: "L0|L1" (from|to)
    if parts.len() >= 2 {
        return Some((parts[0].to_string(), parts[1].to_string()));
    }

    // Handle format with timestamp prefix: "T:00100|PA|L0|L1"
    if parts.len() >= 4 && parts[1] == "PA" {
        return Some((parts[2].to_string(), parts[3].to_string()));
    }

    None
}

/// Parse single character from event data
fn parse_single_character(data: &str) -> Option<String> {
    let parts: Vec<&str> = data.split('|').collect();
    if !parts.is_empty() {
        let char_str = parts[0].trim();
        if ["L0", "L1", "R0", "R1"].contains(&char_str) {
            return Some(char_str.to_string());
        }
    }
    None
}

/// Parse block intercept data: "character|ball_state"
fn parse_block_intercept(data: &str) -> Option<(String, char)> {
    let parts: Vec<&str> = data.split('|').collect();
    if parts.len() >= 2 {
        let char_str = parts[0].trim();
        if ["L0", "L1", "R0", "R1"].contains(&char_str)
            && let Some(ball_state) = parts[1].chars().next()
        {
            return Some((char_str.to_string(), ball_state));
        }
    }
    None
}

/// Calculate pass timing statistics
fn calculate_pass_timing(pass_events: &[PassEvent]) -> PassTimingAnalysis {
    if pass_events.len() < 2 {
        return PassTimingAnalysis::default();
    }

    let mut intervals: Vec<i64> = Vec::new();
    let mut prev_time: Option<i64> = None;

    for event in pass_events {
        if let Some(prev) = prev_time {
            let interval = event.time_ms - prev;
            if interval > 0 && interval < 60000 {
                // Ignore intervals > 1 minute (probably different matches)
                intervals.push(interval);
            }
        }
        prev_time = Some(event.time_ms);
    }

    if intervals.is_empty() {
        return PassTimingAnalysis::default();
    }

    let sum: i64 = intervals.iter().sum();
    let avg = sum as f32 / intervals.len() as f32;
    let min = *intervals.iter().min().unwrap_or(&0);
    let max = *intervals.iter().max().unwrap_or(&0);

    let under_500 = intervals.iter().filter(|&&i| i < 500).count();
    let under_1000 = intervals.iter().filter(|&&i| i < 1000).count();
    let under_2000 = intervals.iter().filter(|&&i| i < 2000).count();

    PassTimingAnalysis {
        avg_pass_interval_ms: avg,
        min_pass_interval_ms: min,
        max_pass_interval_ms: max,
        passes_under_500ms: under_500,
        passes_under_1000ms: under_1000,
        passes_under_2000ms: under_2000,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pass_data() {
        assert_eq!(
            parse_pass_data("L0|L1"),
            Some(("L0".to_string(), "L1".to_string()))
        );
        assert_eq!(
            parse_pass_data("R1|R0"),
            Some(("R1".to_string(), "R0".to_string()))
        );
    }

    #[test]
    fn test_parse_single_character() {
        assert_eq!(parse_single_character("L0"), Some("L0".to_string()));
        assert_eq!(parse_single_character("R1|extra"), Some("R1".to_string()));
        assert_eq!(parse_single_character("invalid"), None);
    }

    #[test]
    fn test_parse_block_intercept() {
        assert_eq!(
            parse_block_intercept("L0|P"),
            Some(("L0".to_string(), 'P'))
        );
        assert_eq!(
            parse_block_intercept("R1|S"),
            Some(("R1".to_string(), 'S'))
        );
    }

    #[test]
    fn test_character_stats_rates() {
        let stats = CharacterTeamStats {
            character_id: "L0".to_string(),
            passes_attempted: 10,
            passes_completed: 7,
            passes_intercepted: 2,
            passes_missed: 1,
            passes_received: 5,
            blocks_activated: 4,
            blocks_intercepted: 1,
        };
        assert!((stats.pass_completion_rate() - 70.0).abs() < 0.1);
        assert!((stats.block_success_rate() - 25.0).abs() < 0.1);
    }

    #[test]
    fn test_team_stats_rates() {
        let team = TeamStats {
            team_id: "Left".to_string(),
            total_passes_attempted: 20,
            total_passes_completed: 14,
            total_passes_intercepted: 4,
            total_passes_missed: 2,
            total_blocks_activated: 8,
            total_blocks_intercepted: 2,
            characters: vec![],
        };
        assert!((team.pass_completion_rate() - 70.0).abs() < 0.1);
        assert!((team.turnover_rate() - 30.0).abs() < 0.1);
    }
}
