//! Automated post-training analysis
//!
//! Generates analysis reports and Claude prompts from training session data.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

use chrono::Local;

use crate::simulation::db::{SimDatabase, MatchSummary, DistanceAnalysis, InputAnalysis, GoalTransition};

/// Per-game analysis metrics
#[derive(Debug, Clone, Default)]
pub struct GameAnalysis {
    pub game_number: u32,
    pub level_name: String,
    pub duration_secs: f32,
    pub human_score: u32,
    pub ai_score: u32,
    pub possession: PossessionStats,
    pub shots: ShotStats,
    pub steals: StealStats,
    pub ai_behavior: AiBehaviorStats,
    pub ai_movement: MovementStats,
    pub notes: Option<String>,
}

/// Possession statistics
#[derive(Debug, Clone, Default)]
pub struct PossessionStats {
    /// Percentage of time human held ball (0-100)
    pub human_pct: f32,
    /// Percentage of time AI held ball (0-100)
    pub ai_pct: f32,
    /// Percentage of time ball was free (0-100)
    pub free_pct: f32,
    /// Number of times human picked up the ball
    pub human_pickups: u32,
    /// Number of times AI picked up the ball
    pub ai_pickups: u32,
}

/// Shot statistics
#[derive(Debug, Clone, Default)]
pub struct ShotStats {
    pub human_shots: u32,
    pub human_goals: u32,
    pub ai_shots: u32,
    pub ai_goals: u32,
}

impl ShotStats {
    pub fn human_accuracy(&self) -> f32 {
        if self.human_shots == 0 {
            0.0
        } else {
            (self.human_goals as f32 / self.human_shots as f32) * 100.0
        }
    }

    pub fn ai_accuracy(&self) -> f32 {
        if self.ai_shots == 0 {
            0.0
        } else {
            (self.ai_goals as f32 / self.ai_shots as f32) * 100.0
        }
    }
}

/// Steal statistics
#[derive(Debug, Clone, Default)]
pub struct StealStats {
    pub human_attempts: u32,
    pub human_successes: u32,
    pub ai_attempts: u32,
    pub ai_successes: u32,
}

impl StealStats {
    pub fn human_success_rate(&self) -> f32 {
        if self.human_attempts == 0 {
            0.0
        } else {
            (self.human_successes as f32 / self.human_attempts as f32) * 100.0
        }
    }

    pub fn ai_success_rate(&self) -> f32 {
        if self.ai_attempts == 0 {
            0.0
        } else {
            (self.ai_successes as f32 / self.ai_attempts as f32) * 100.0
        }
    }
}

/// AI behavior statistics
#[derive(Debug, Clone, Default)]
pub struct AiBehaviorStats {
    /// Goal distribution: goal name -> count
    pub goal_distribution: HashMap<String, u32>,
    /// Number of goal transitions
    pub goal_transitions: u32,
    /// Detected oscillation patterns (rapid goal switches)
    pub oscillation_count: u32,
    /// Time spent in each goal (seconds)
    pub goal_time_breakdown: HashMap<String, f32>,
    /// Longest single stretch in one goal (seconds)
    pub longest_goal_duration_secs: f32,
    /// Goal during which most time was spent
    pub dominant_goal: String,
    /// Time dominant goal was active (seconds)
    pub dominant_goal_time: f32,
}

/// AI movement statistics
#[derive(Debug, Clone, Default)]
pub struct MovementStats {
    /// Time spent moving left (move_x < -0.1)
    pub time_moving_left_secs: f32,
    /// Time spent moving right (move_x > 0.1)
    pub time_moving_right_secs: f32,
    /// Time spent stationary (|move_x| <= 0.1)
    pub time_stationary_secs: f32,
    /// Average distance to opponent when opponent has ball
    pub avg_distance_to_opponent: f32,
    /// Closing rate: positive = closing gap, negative = opening gap (px/s)
    pub closing_rate: f32,
    /// Time spent stuck (input active but < 5px position change over 0.5s)
    pub time_stuck_secs: f32,
    /// Average X position during game
    pub avg_x_position: f32,
    /// X position range (min, max)
    pub x_position_range: (f32, f32),
    /// Per-goal movement breakdown
    pub per_goal_movement: HashMap<String, GoalMovementStats>,
}

/// Movement stats broken down by AI goal
#[derive(Debug, Clone, Default)]
pub struct GoalMovementStats {
    /// Time in this goal (seconds)
    pub time_secs: f32,
    /// Time moving left
    pub time_moving_left_secs: f32,
    /// Time moving right
    pub time_moving_right_secs: f32,
    /// Time stationary
    pub time_stationary_secs: f32,
    /// Average distance to opponent
    pub avg_distance: f32,
    /// Closing rate
    pub closing_rate: f32,
}

/// Pursuit protocol analysis - aggregated across all iterations
#[derive(Debug, Clone, Default)]
pub struct PursuitAnalysis {
    /// Number of pursuit iterations
    pub iterations: u32,
    /// Iterations where AI scored (caught player)
    pub ai_catches: u32,
    /// Iterations where player scored
    pub player_scores: u32,
    /// Iterations that timed out without a score
    pub timeouts: u32,
    /// Average distance between AI and player across all iterations
    pub avg_distance: f32,
    /// Minimum distance achieved across all iterations
    pub min_distance: f32,
    /// Average closing rate (positive = AI closing gap)
    pub avg_closing_rate: f32,
    /// Total time AI was stuck (input active, no movement)
    pub total_stuck_time_secs: f32,
    /// Pursuit score (higher = better pursuit, 0-100)
    pub pursuit_score: f32,
    /// Per-iteration stats
    pub iteration_stats: Vec<PursuitIterationStats>,
    /// AI profile name
    pub ai_profile: String,
}

/// Per-iteration pursuit statistics
#[derive(Debug, Clone, Default)]
pub struct PursuitIterationStats {
    /// Iteration number
    pub iteration: u32,
    /// Duration in seconds
    pub duration_secs: f32,
    /// Outcome: "ai_catch", "player_score", "timeout"
    pub outcome: String,
    /// Average distance to opponent
    pub avg_distance: f32,
    /// Minimum distance achieved
    pub min_distance: f32,
    /// Closing rate (positive = closing gap)
    pub closing_rate: f32,
    /// Time stuck (seconds)
    pub stuck_time_secs: f32,
    /// Position range (how much AI moved)
    pub position_range: f32,
}

impl PursuitAnalysis {
    /// Calculate pursuit score (0-100, higher is better)
    fn calculate_pursuit_score(&self) -> f32 {
        if self.iterations == 0 {
            return 0.0;
        }

        let mut score = 50.0; // Start at 50

        // Bonus for catching player
        let catch_rate = self.ai_catches as f32 / self.iterations as f32;
        score += catch_rate * 20.0;

        // Penalty for timeouts
        let timeout_rate = self.timeouts as f32 / self.iterations as f32;
        score -= timeout_rate * 15.0;

        // Bonus for positive closing rate
        if self.avg_closing_rate > 0.0 {
            score += (self.avg_closing_rate / 50.0).min(10.0);
        } else {
            score += self.avg_closing_rate / 25.0; // Penalty for negative
        }

        // Penalty for being stuck
        let avg_stuck_pct = if self.iteration_stats.is_empty() {
            0.0
        } else {
            let total_duration: f32 = self.iteration_stats.iter().map(|s| s.duration_secs).sum();
            if total_duration > 0.0 {
                self.total_stuck_time_secs / total_duration * 100.0
            } else {
                0.0
            }
        };
        score -= avg_stuck_pct * 0.3;

        // Bonus for low average distance
        if self.avg_distance < 200.0 {
            score += (200.0 - self.avg_distance) / 10.0;
        }

        score.clamp(0.0, 100.0)
    }
}

/// Full session analysis
#[derive(Debug, Clone)]
pub struct SessionAnalysis {
    pub session_id: String,
    pub protocol: String,
    pub protocol_description: String,
    pub ai_profile: String,
    pub games: Vec<GameAnalysis>,
    pub aggregate: AggregateStats,
    pub insights: Vec<String>,
    pub weaknesses: Vec<String>,
}

/// Aggregate statistics across all games
#[derive(Debug, Clone, Default)]
pub struct AggregateStats {
    pub total_games: u32,
    pub human_wins: u32,
    pub ai_wins: u32,
    pub total_duration_secs: f32,
    pub avg_human_possession: f32,
    pub avg_ai_possession: f32,
    pub total_human_shots: u32,
    pub total_ai_shots: u32,
    pub total_human_goals: u32,
    pub total_ai_goals: u32,
    pub total_human_steals: u32,
    pub total_ai_steals: u32,
    pub total_human_steal_attempts: u32,
    pub total_ai_steal_attempts: u32,
}

impl AggregateStats {
    pub fn human_shot_accuracy(&self) -> f32 {
        if self.total_human_shots == 0 {
            0.0
        } else {
            (self.total_human_goals as f32 / self.total_human_shots as f32) * 100.0
        }
    }

    pub fn ai_shot_accuracy(&self) -> f32 {
        if self.total_ai_shots == 0 {
            0.0
        } else {
            (self.total_ai_goals as f32 / self.total_ai_shots as f32) * 100.0
        }
    }

    pub fn human_steal_rate(&self) -> f32 {
        if self.total_human_steal_attempts == 0 {
            0.0
        } else {
            (self.total_human_steals as f32 / self.total_human_steal_attempts as f32) * 100.0
        }
    }

    pub fn ai_steal_rate(&self) -> f32 {
        if self.total_ai_steal_attempts == 0 {
            0.0
        } else {
            (self.total_ai_steals as f32 / self.total_ai_steal_attempts as f32) * 100.0
        }
    }
}

use super::protocol::TrainingProtocol;

/// Analyze a training session from SQLite database.
pub fn analyze_session_from_db(
    db: &SimDatabase,
    session_id: &str,
    protocol: TrainingProtocol,
) -> Option<SessionAnalysis> {
    // Get session matches
    let matches = db.get_session_matches(session_id).ok()?;
    if matches.is_empty() {
        return None;
    }

    let mut games = Vec::new();
    let mut aggregate = AggregateStats::default();

    // Get AI profile from first match
    let ai_profile = matches.first().map(|m| m.right_profile.clone()).unwrap_or_else(|| "Unknown".to_string());

    // Analyze each match
    for (idx, match_info) in matches.iter().enumerate() {
        let game_analysis = analyze_game_from_db(db, match_info, (idx + 1) as u32);

        // Update aggregate stats
        aggregate.total_games += 1;
        if match_info.score_left > match_info.score_right {
            aggregate.human_wins += 1;
        } else if match_info.score_right > match_info.score_left {
            aggregate.ai_wins += 1;
        }
        aggregate.total_duration_secs += game_analysis.duration_secs;
        aggregate.avg_human_possession += game_analysis.possession.human_pct;
        aggregate.avg_ai_possession += game_analysis.possession.ai_pct;
        aggregate.total_human_shots += game_analysis.shots.human_shots;
        aggregate.total_ai_shots += game_analysis.shots.ai_shots;
        aggregate.total_human_goals += game_analysis.shots.human_goals;
        aggregate.total_ai_goals += game_analysis.shots.ai_goals;
        aggregate.total_human_steals += game_analysis.steals.human_successes;
        aggregate.total_ai_steals += game_analysis.steals.ai_successes;
        aggregate.total_human_steal_attempts += game_analysis.steals.human_attempts;
        aggregate.total_ai_steal_attempts += game_analysis.steals.ai_attempts;

        games.push(game_analysis);
    }

    // Average possession percentages
    if aggregate.total_games > 0 {
        aggregate.avg_human_possession /= aggregate.total_games as f32;
        aggregate.avg_ai_possession /= aggregate.total_games as f32;
    }

    // Generate insights and weaknesses
    let insights = generate_insights(&aggregate, &games);
    let weaknesses = identify_weaknesses(&aggregate, &games);

    Some(SessionAnalysis {
        session_id: session_id.to_string(),
        protocol: protocol.cli_name().to_string(),
        protocol_description: protocol.description().to_string(),
        ai_profile,
        games,
        aggregate,
        insights,
        weaknesses,
    })
}

/// Analyze a single game from SQLite database
fn analyze_game_from_db(db: &SimDatabase, match_info: &MatchSummary, game_number: u32) -> GameAnalysis {
    let match_id = match_info.id;

    // Get event statistics
    let event_stats = db.get_match_event_stats(match_id).ok();

    // Get distance analysis
    let distance_analysis = db.analyze_distance(match_id).ok();

    // Get AI input analysis
    let input_analysis = db.analyze_ai_inputs(match_id).ok();

    // Get goal transitions
    let goal_transitions = db.get_goal_transitions(match_id).ok().unwrap_or_default();

    // Build shot stats from events
    let shots = if let Some(_stats) = &event_stats {
        // Count shots and goals by player from events
        let shot_events = db.get_events_by_type(match_id, "SR").ok().unwrap_or_default();
        let goal_events = db.get_events_by_type(match_id, "G").ok().unwrap_or_default();

        let (human_shots, ai_shots) = count_events_by_player(&shot_events);
        let (human_goals, ai_goals) = count_events_by_player(&goal_events);

        ShotStats {
            human_shots,
            human_goals,
            ai_shots,
            ai_goals,
        }
    } else {
        ShotStats::default()
    };

    // Build steal stats from events
    let steals = {
        let steal_attempt_events = db.get_events_by_type(match_id, "SA").ok().unwrap_or_default();
        let steal_success_events = db.get_events_by_type(match_id, "S+").ok().unwrap_or_default();

        let (human_attempts, ai_attempts) = count_events_by_player(&steal_attempt_events);
        let (human_successes, ai_successes) = count_events_by_player(&steal_success_events);

        StealStats {
            human_attempts,
            human_successes,
            ai_attempts,
            ai_successes,
        }
    };

    // Build possession stats (approximate from tick data)
    let possession = calculate_possession_from_db(db, match_id);

    // Build AI behavior stats from goal transitions
    let ai_behavior = build_ai_behavior_from_transitions(&goal_transitions, match_info.duration);

    // Build movement stats from input analysis
    let tick_stats = analyze_movement_from_ticks(db, match_id);
    let ai_movement = build_movement_from_input_analysis(
        input_analysis.as_ref(),
        distance_analysis.as_ref(),
        tick_stats,
    );

    GameAnalysis {
        game_number,
        level_name: match_info.level_name.clone(),
        duration_secs: match_info.duration,
        human_score: match_info.score_left,
        ai_score: match_info.score_right,
        possession,
        shots,
        steals,
        ai_behavior,
        ai_movement,
        notes: None,
    }
}

/// Count events by player (L = human, R = AI)
fn count_events_by_player(events: &[crate::simulation::db::EventRecord]) -> (u32, u32) {
    let mut human = 0u32;
    let mut ai = 0u32;

    for event in events {
        // Event data format: T:NNNNN|TYPE|player|...
        let parts: Vec<&str> = event.data.split('|').collect();
        if parts.len() >= 3 {
            match parts[2] {
                "L" => human += 1,
                "R" => ai += 1,
                _ => {}
            }
        }
    }

    (human, ai)
}

/// Calculate possession stats from tick events in database
fn calculate_possession_from_db(db: &SimDatabase, match_id: i64) -> PossessionStats {
    let tick_events = db.get_events_by_type(match_id, "T").ok().unwrap_or_default();

    if tick_events.is_empty() {
        return PossessionStats::default();
    }

    let mut human_ticks = 0u32;
    let mut ai_ticks = 0u32;
    let mut free_ticks = 0u32;

    for event in &tick_events {
        // Parse tick data: T:NNNNN|T|frame|left_pos|left_vel|right_pos|right_vel|ball_pos|ball_vel|state
        let parts: Vec<&str> = event.data.split('|').collect();
        if parts.len() < 10 {
            continue;
        }

        let state = parts[9];
        match state {
            "H" => {
                // Ball is held - determine holder by proximity
                if let (Some(left_pos), Some(right_pos), Some(ball_pos)) = (
                    parse_pos_simple(parts[3]),
                    parse_pos_simple(parts[5]),
                    parse_pos_simple(parts[7]),
                ) {
                    let left_dist = (ball_pos.0 - left_pos.0).powi(2) + (ball_pos.1 - left_pos.1).powi(2);
                    let right_dist = (ball_pos.0 - right_pos.0).powi(2) + (ball_pos.1 - right_pos.1).powi(2);

                    if left_dist < right_dist {
                        human_ticks += 1;
                    } else {
                        ai_ticks += 1;
                    }
                }
            }
            "I" | "F" | _ => {
                free_ticks += 1;
            }
        }
    }

    // Count pickups from P events
    let pickup_events = db.get_events_by_type(match_id, "P").ok().unwrap_or_default();
    let (human_pickups, ai_pickups) = count_events_by_player(&pickup_events);

    let total = (human_ticks + ai_ticks + free_ticks) as f32;
    if total == 0.0 {
        return PossessionStats::default();
    }

    PossessionStats {
        human_pct: (human_ticks as f32 / total) * 100.0,
        ai_pct: (ai_ticks as f32 / total) * 100.0,
        free_pct: (free_ticks as f32 / total) * 100.0,
        human_pickups,
        ai_pickups,
    }
}

/// Parse position string "x,y" into tuple
fn parse_pos_simple(s: &str) -> Option<(f32, f32)> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() == 2 {
        let x = parts[0].parse::<f32>().ok()?;
        let y = parts[1].parse::<f32>().ok()?;
        Some((x, y))
    } else {
        None
    }
}

/// Build AI behavior stats from goal transitions
fn build_ai_behavior_from_transitions(transitions: &[GoalTransition], duration_secs: f32) -> AiBehaviorStats {
    let mut goal_distribution: HashMap<String, u32> = HashMap::new();
    let mut goal_time_breakdown: HashMap<String, f32> = HashMap::new();
    let mut goal_transitions_count = 0u32;
    let mut oscillation_count = 0u32;

    // Track recent goals for oscillation detection
    let mut recent_goals: Vec<&str> = Vec::new();
    const OSCILLATION_WINDOW: usize = 6;

    // Filter for AI (R) goals
    let ai_goals: Vec<_> = transitions.iter().filter(|t| t.player == "R").collect();

    let mut last_goal: Option<(&str, u32)> = None;
    let mut longest_goal_duration_secs = 0.0f32;
    let match_end_ms = (duration_secs * 1000.0) as u32;

    for ai_goal in &ai_goals {
        let goal = ai_goal.goal.as_str();
        let time_ms = ai_goal.time_ms;

        // Count distribution
        *goal_distribution.entry(goal.to_string()).or_insert(0) += 1;

        // Calculate time spent in previous goal
        if let Some((prev_goal, prev_time)) = last_goal {
            let duration = (time_ms - prev_time) as f32 / 1000.0;
            *goal_time_breakdown.entry(prev_goal.to_string()).or_insert(0.0) += duration;

            if duration > longest_goal_duration_secs {
                longest_goal_duration_secs = duration;
            }

            // Count transitions
            if prev_goal != goal {
                goal_transitions_count += 1;

                // Track for oscillation detection
                recent_goals.push(goal);
                if recent_goals.len() > OSCILLATION_WINDOW {
                    recent_goals.remove(0);
                }

                // Check for oscillation
                if recent_goals.len() >= OSCILLATION_WINDOW {
                    let unique_goals: std::collections::HashSet<_> = recent_goals.iter().collect();
                    if unique_goals.len() <= 2 {
                        oscillation_count += 1;
                    }
                }
            }
        }

        last_goal = Some((goal, time_ms));
    }

    // Account for time from last goal to match end
    if let Some((last_goal_name, last_time)) = last_goal {
        let duration = (match_end_ms.saturating_sub(last_time)) as f32 / 1000.0;
        *goal_time_breakdown.entry(last_goal_name.to_string()).or_insert(0.0) += duration;

        if duration > longest_goal_duration_secs {
            longest_goal_duration_secs = duration;
        }
    }

    // Find dominant goal
    let (dominant_goal, dominant_goal_time) = goal_time_breakdown
        .iter()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(k, v)| (k.clone(), *v))
        .unwrap_or_default();

    AiBehaviorStats {
        goal_distribution,
        goal_transitions: goal_transitions_count,
        oscillation_count,
        goal_time_breakdown,
        longest_goal_duration_secs,
        dominant_goal,
        dominant_goal_time,
    }
}

/// Build movement stats from input analysis
fn build_movement_from_input_analysis(
    input: Option<&InputAnalysis>,
    distance: Option<&DistanceAnalysis>,
    tick_stats: Option<MovementTickStats>,
) -> MovementStats {
    let input = match input {
        Some(i) => i,
        None => return MovementStats::default(),
    };

    // Approximate time from frame counts (assuming ~60 FPS)
    let frame_to_secs = |frames: u32| frames as f32 / 60.0;

    let time_moving_left_secs = frame_to_secs(input.move_left_frames);
    let time_moving_right_secs = frame_to_secs(input.move_right_frames);
    let time_stationary_secs = frame_to_secs(input.stationary_frames);

    // Get distance stats
    let avg_distance_to_opponent = distance.map(|d| d.avg_distance).unwrap_or(0.0);

    let (avg_x_position, x_position_range, closing_rate, time_stuck_secs) = tick_stats
        .map(|stats| {
            (
                stats.avg_x_position,
                (stats.min_x_position, stats.max_x_position),
                stats.closing_rate,
                stats.time_stuck_secs,
            )
        })
        .unwrap_or((0.0, (0.0, 0.0), 0.0, 0.0));

    MovementStats {
        time_moving_left_secs,
        time_moving_right_secs,
        time_stationary_secs,
        avg_distance_to_opponent,
        closing_rate,
        time_stuck_secs,
        avg_x_position,
        x_position_range,
        per_goal_movement: HashMap::new(), // Would need per-tick goal correlation
    }
}

#[derive(Clone, Copy)]
struct MovementTickStats {
    min_x_position: f32,
    max_x_position: f32,
    avg_x_position: f32,
    closing_rate: f32,
    time_stuck_secs: f32,
}

fn analyze_movement_from_ticks(db: &SimDatabase, match_id: i64) -> Option<MovementTickStats> {
    let tick_events = db.get_events_by_type(match_id, "T").ok()?;
    if tick_events.is_empty() {
        return None;
    }

    let mut min_x = f32::MAX;
    let mut max_x = f32::MIN;
    let mut sum_x = 0.0f32;
    let mut x_samples = 0u32;
    let mut total_closing_rate = 0.0f32;
    let mut closing_samples = 0u32;
    let mut time_stuck_secs = 0.0f32;

    let mut prev_right_pos: Option<(f32, f32)> = None;
    let mut prev_distance: Option<f32> = None;
    let mut prev_time_ms: Option<u32> = None;
    let stuck_threshold = 3.0f32;

    for event in &tick_events {
        let parts: Vec<&str> = event.data.split('|').collect();
        if parts.len() < 6 {
            continue;
        }

        let left_pos = parse_pos_simple(parts[3]);
        let right_pos = parse_pos_simple(parts[5]);

        let Some((rx, ry)) = right_pos else {
            continue;
        };
        sum_x += rx;
        x_samples += 1;
        min_x = min_x.min(rx);
        max_x = max_x.max(rx);

        if let Some(prev_pos) = prev_right_pos {
            let dx = rx - prev_pos.0;
            let dy = ry - prev_pos.1;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist < stuck_threshold {
                if let Some(prev_ms) = prev_time_ms {
                    let dt = (event.time_ms.saturating_sub(prev_ms)) as f32 / 1000.0;
                    time_stuck_secs += dt;
                }
            }
        }
        prev_right_pos = Some((rx, ry));

        if let (Some((lx, ly)), Some(prev_dist), Some(prev_ms)) = (left_pos, prev_distance, prev_time_ms) {
            let distance = ((rx - lx).powi(2) + (ry - ly).powi(2)).sqrt();
            let dt = (event.time_ms.saturating_sub(prev_ms)) as f32 / 1000.0;
            if dt > 0.0 {
                total_closing_rate += (prev_dist - distance) / dt;
                closing_samples += 1;
            }
            prev_distance = Some(distance);
        } else if let Some((lx, ly)) = left_pos {
            let distance = ((rx - lx).powi(2) + (ry - ly).powi(2)).sqrt();
            prev_distance = Some(distance);
        }

        prev_time_ms = Some(event.time_ms);
    }

    if x_samples == 0 {
        return None;
    }

    let avg_x = sum_x / x_samples as f32;
    let closing_rate = if closing_samples > 0 {
        total_closing_rate / closing_samples as f32
    } else {
        0.0
    };

    Some(MovementTickStats {
        min_x_position: if min_x == f32::MAX { 0.0 } else { min_x },
        max_x_position: if max_x == f32::MIN { 0.0 } else { max_x },
        avg_x_position: avg_x,
        closing_rate,
        time_stuck_secs,
    })
}

/// Analyze a pursuit protocol session from SQLite.
pub fn analyze_pursuit_session_from_db(
    db: &SimDatabase,
    session_id: &str,
) -> Option<PursuitAnalysis> {
    let matches = db.get_session_matches(session_id).ok()?;
    if matches.is_empty() {
        return None;
    }

    let mut analysis = PursuitAnalysis::default();
    analysis.ai_profile = matches
        .first()
        .map(|m| m.right_profile.clone())
        .unwrap_or_else(|| "Unknown".to_string());

    let mut total_distance = 0.0f32;
    let mut total_closing_rate = 0.0f32;
    let mut min_distance = f32::MAX;

    for (idx, match_info) in matches.iter().enumerate() {
        analysis.iterations += 1;

        let outcome = if match_info.score_right > 0 {
            analysis.ai_catches += 1;
            "ai_catch"
        } else if match_info.score_left > 0 {
            analysis.player_scores += 1;
            "player_score"
        } else {
            analysis.timeouts += 1;
            "timeout"
        };

        let distance = db.analyze_distance(match_info.id).ok().unwrap_or_default();
        let iter_min_distance = distance.min_distance;

        if iter_min_distance < min_distance {
            min_distance = iter_min_distance;
        }

        total_distance += distance.avg_distance;
        total_closing_rate += 0.0;

        analysis.iteration_stats.push(PursuitIterationStats {
            iteration: (idx + 1) as u32,
            duration_secs: match_info.duration,
            outcome: outcome.to_string(),
            avg_distance: distance.avg_distance,
            min_distance: iter_min_distance,
            closing_rate: 0.0,
            stuck_time_secs: 0.0,
            position_range: 0.0,
        });
    }

    if analysis.iterations > 0 {
        analysis.avg_distance = total_distance / analysis.iterations as f32;
        analysis.avg_closing_rate = total_closing_rate / analysis.iterations as f32;
        analysis.min_distance = if min_distance == f32::MAX { 0.0 } else { min_distance };
    }

    analysis.pursuit_score = analysis.calculate_pursuit_score();

    Some(analysis)
}

/// Generate pursuit-specific markdown report
pub fn format_pursuit_analysis_markdown(analysis: &PursuitAnalysis) -> String {
    let mut md = String::new();

    md.push_str("# Pursuit Protocol Analysis\n\n");
    md.push_str(&format!("**AI Profile:** {}\n\n", analysis.ai_profile));

    // Summary
    md.push_str("## Summary\n\n");
    md.push_str(&format!(
        "**Pursuit Score:** {:.1}/100\n\n",
        analysis.pursuit_score
    ));

    md.push_str("| Metric | Value |\n");
    md.push_str("|--------|-------|\n");
    md.push_str(&format!("| Iterations | {} |\n", analysis.iterations));
    md.push_str(&format!(
        "| AI Catches | {} ({:.0}%) |\n",
        analysis.ai_catches,
        if analysis.iterations > 0 {
            analysis.ai_catches as f32 / analysis.iterations as f32 * 100.0
        } else {
            0.0
        }
    ));
    md.push_str(&format!(
        "| Player Scores | {} ({:.0}%) |\n",
        analysis.player_scores,
        if analysis.iterations > 0 {
            analysis.player_scores as f32 / analysis.iterations as f32 * 100.0
        } else {
            0.0
        }
    ));
    md.push_str(&format!(
        "| Timeouts | {} ({:.0}%) |\n",
        analysis.timeouts,
        if analysis.iterations > 0 {
            analysis.timeouts as f32 / analysis.iterations as f32 * 100.0
        } else {
            0.0
        }
    ));
    md.push_str(&format!(
        "| Avg Distance | {:.0}px |\n",
        analysis.avg_distance
    ));
    md.push_str(&format!(
        "| Min Distance | {:.0}px |\n",
        analysis.min_distance
    ));
    md.push_str(&format!(
        "| Avg Closing Rate | {:.1}px/s {} |\n",
        analysis.avg_closing_rate,
        if analysis.avg_closing_rate > 0.0 {
            "(closing)"
        } else if analysis.avg_closing_rate < 0.0 {
            "(falling behind)"
        } else {
            ""
        }
    ));
    md.push_str(&format!(
        "| Total Stuck Time | {:.1}s |\n\n",
        analysis.total_stuck_time_secs
    ));

    // Per-iteration breakdown
    md.push_str("## Per-Iteration Breakdown\n\n");
    md.push_str("| # | Duration | Outcome | Avg Dist | Min Dist | Closing | Stuck | Range |\n");
    md.push_str("|---|----------|---------|----------|----------|---------|-------|-------|\n");

    for stats in &analysis.iteration_stats {
        md.push_str(&format!(
            "| {} | {:.1}s | {} | {:.0}px | {:.0}px | {:.1}px/s | {:.1}s | {:.0}px |\n",
            stats.iteration,
            stats.duration_secs,
            stats.outcome,
            stats.avg_distance,
            stats.min_distance,
            stats.closing_rate,
            stats.stuck_time_secs,
            stats.position_range
        ));
    }

    md.push('\n');

    // Interpretation
    md.push_str("## Interpretation\n\n");

    if analysis.pursuit_score >= 70.0 {
        md.push_str("**PASS:** AI demonstrates good pursuit behavior.\n");
    } else if analysis.pursuit_score >= 50.0 {
        md.push_str("**MARGINAL:** AI shows some pursuit but with issues.\n");
    } else {
        md.push_str("**FAIL:** AI is not effectively pursuing the player.\n");
    }

    // Specific issues
    if analysis.avg_closing_rate < 0.0 {
        md.push_str("\n- **Issue:** Negative closing rate - AI is falling behind instead of chasing.\n");
    }
    if analysis.total_stuck_time_secs > 5.0 {
        md.push_str(&format!(
            "\n- **Issue:** AI was stuck for {:.1}s total - movement may be blocked.\n",
            analysis.total_stuck_time_secs
        ));
    }
    if analysis.timeouts > analysis.iterations / 2 {
        md.push_str("\n- **Issue:** Majority of iterations timed out - AI not reaching the player.\n");
    }

    md
}

/// Generate insights from session data
fn generate_insights(aggregate: &AggregateStats, games: &[GameAnalysis]) -> Vec<String> {
    let mut insights = Vec::new();

    // Win rate insight
    if aggregate.total_games > 0 {
        let human_win_rate = (aggregate.human_wins as f32 / aggregate.total_games as f32) * 100.0;
        if human_win_rate >= 80.0 {
            insights.push(format!(
                "Human dominated: {:.0}% win rate ({}/{})",
                human_win_rate, aggregate.human_wins, aggregate.total_games
            ));
        } else if human_win_rate <= 20.0 {
            insights.push(format!(
                "AI dominated: {:.0}% AI win rate ({}/{})",
                100.0 - human_win_rate,
                aggregate.ai_wins,
                aggregate.total_games
            ));
        }
    }

    // Possession insight
    if aggregate.avg_human_possession > 60.0 {
        insights.push(format!(
            "Human controlled possession: {:.1}% average",
            aggregate.avg_human_possession
        ));
    } else if aggregate.avg_ai_possession > 60.0 {
        insights.push(format!(
            "AI controlled possession: {:.1}% average",
            aggregate.avg_ai_possession
        ));
    }

    // Shot activity insight
    if aggregate.total_ai_shots == 0 && aggregate.total_games > 1 {
        insights.push("AI never attempted a shot across all games".to_string());
    } else if aggregate.total_ai_shots > 0 {
        let ai_acc = aggregate.ai_shot_accuracy();
        if ai_acc < 20.0 {
            insights.push(format!(
                "AI shot accuracy very low: {:.1}% ({}/{})",
                ai_acc, aggregate.total_ai_goals, aggregate.total_ai_shots
            ));
        } else if ai_acc > 50.0 {
            insights.push(format!(
                "AI shot accuracy high: {:.1}% ({}/{})",
                ai_acc, aggregate.total_ai_goals, aggregate.total_ai_shots
            ));
        }
    }

    // Steal insight
    if aggregate.total_ai_steal_attempts == 0 && aggregate.total_games > 1 {
        insights.push("AI never attempted a steal across all games".to_string());
    }

    // Per-game patterns
    let mut ai_goal_totals: HashMap<String, u32> = HashMap::new();
    for game in games {
        for (goal, count) in &game.ai_behavior.goal_distribution {
            *ai_goal_totals.entry(goal.clone()).or_insert(0) += count;
        }
    }

    // Find dominant AI goal
    if let Some((dominant_goal, count)) = ai_goal_totals.iter().max_by_key(|(_, c)| *c) {
        let total: u32 = ai_goal_totals.values().sum();
        if total > 0 {
            let pct = (*count as f32 / total as f32) * 100.0;
            if pct > 50.0 {
                insights.push(format!(
                    "AI spent {:.0}% of time in '{}' goal",
                    pct, dominant_goal
                ));
            }
        }
    }

    insights
}

/// Identify AI weaknesses from session data
fn identify_weaknesses(aggregate: &AggregateStats, games: &[GameAnalysis]) -> Vec<String> {
    let mut weaknesses = Vec::new();

    // No shooting
    if aggregate.total_ai_shots == 0 && aggregate.total_games > 0 {
        weaknesses.push("AI never shoots - may be stuck in defensive/chase mode".to_string());
    }

    // Very low shooting
    if aggregate.total_ai_shots > 0 && aggregate.total_ai_shots < aggregate.total_games {
        weaknesses.push(format!(
            "AI rarely shoots ({} shots in {} games) - may not recognize shooting opportunities",
            aggregate.total_ai_shots, aggregate.total_games
        ));
    }

    // Poor accuracy
    if aggregate.total_ai_shots >= 3 && aggregate.ai_shot_accuracy() < 15.0 {
        weaknesses.push(format!(
            "AI shot accuracy critically low ({:.1}%) - shot positioning/timing issues",
            aggregate.ai_shot_accuracy()
        ));
    }

    // No steals attempted
    if aggregate.total_ai_steal_attempts == 0 && aggregate.total_games > 0 {
        weaknesses.push(
            "AI never attempts steals - may not be engaging in defensive pressure".to_string(),
        );
    }

    // Low steal success
    if aggregate.total_ai_steal_attempts >= 5 && aggregate.ai_steal_rate() < 10.0 {
        weaknesses.push(format!(
            "AI steal success rate very low ({:.1}%) - may be attempting steals out of range",
            aggregate.ai_steal_rate()
        ));
    }

    // Possession issues
    if aggregate.avg_ai_possession < 20.0 && aggregate.total_games > 1 {
        weaknesses.push(format!(
            "AI possession very low ({:.1}%) - may not be competing for ball effectively",
            aggregate.avg_ai_possession
        ));
    }

    // Goal oscillation
    let total_oscillations: u32 = games.iter().map(|g| g.ai_behavior.oscillation_count).sum();
    if total_oscillations > aggregate.total_games {
        weaknesses.push(format!(
            "AI shows goal oscillation ({} instances) - rapid switching between goals indicates indecision",
            total_oscillations
        ));
    }

    // Check for "stuck" in single goal
    let mut stuck_games = 0;
    for game in games {
        if game.ai_behavior.goal_transitions < 3 && game.duration_secs > 30.0 {
            stuck_games += 1;
        }
    }
    if stuck_games > aggregate.total_games / 2 {
        weaknesses.push(format!(
            "AI appears stuck in single goal in {}/{} games - may not be reacting to game state changes",
            stuck_games, aggregate.total_games
        ));
    }

    // Movement-based weaknesses
    for game in games {
        let mv = &game.ai_movement;
        let duration = game.duration_secs;

        // Stuck pattern detection (input active but position not changing)
        if mv.time_stuck_secs > duration * 0.25 && duration > 5.0 {
            weaknesses.push(format!(
                "Game {}: AI stuck for {:.1}s ({:.0}% of game) - moving input active but position unchanged",
                game.game_number,
                mv.time_stuck_secs,
                (mv.time_stuck_secs / duration) * 100.0
            ));
        }

        // Not closing gap during defense
        if mv.closing_rate < -10.0 {
            weaknesses.push(format!(
                "Game {}: AI falling behind opponent (closing rate: {:.1}px/s) - not chasing effectively",
                game.game_number, mv.closing_rate
            ));
        }

        // Monotonic movement (always moving same direction)
        let total_movement_time = mv.time_moving_left_secs + mv.time_moving_right_secs;
        if total_movement_time > 3.0 {
            let left_pct = mv.time_moving_left_secs / total_movement_time;
            let right_pct = mv.time_moving_right_secs / total_movement_time;
            if left_pct > 0.90 {
                weaknesses.push(format!(
                    "Game {}: AI movement monotonic ({:.0}% left) - may be targeting wrong position",
                    game.game_number,
                    left_pct * 100.0
                ));
            } else if right_pct > 0.90 {
                weaknesses.push(format!(
                    "Game {}: AI movement monotonic ({:.0}% right) - may be targeting wrong position",
                    game.game_number,
                    right_pct * 100.0
                ));
            }
        }

        // Position outlier (AI stayed in unexpected area)
        let x_range = mv.x_position_range.1 - mv.x_position_range.0;
        if x_range < 100.0 && duration > 10.0 {
            weaknesses.push(format!(
                "Game {}: AI position range very small ({:.0}px) - AI barely moved across arena",
                game.game_number, x_range
            ));
        }
    }

    // Dominant goal time check
    for game in games {
        let behavior = &game.ai_behavior;
        if !behavior.dominant_goal.is_empty() && game.duration_secs > 5.0 {
            let dominant_pct = (behavior.dominant_goal_time / game.duration_secs) * 100.0;
            if dominant_pct > 80.0 && behavior.goal_transitions < 3 {
                weaknesses.push(format!(
                    "Game {}: AI spent {:.0}% of time in '{}' with only {} transitions - limited goal diversity",
                    game.game_number,
                    dominant_pct,
                    behavior.dominant_goal,
                    behavior.goal_transitions
                ));
            }
        }
    }

    weaknesses
}

/// Write analysis files to session directory
pub fn write_analysis_files(session_dir: &Path, analysis: &SessionAnalysis) -> io::Result<()> {
    // Write analysis.md
    let md_content = format_analysis_markdown(analysis);
    let md_path = session_dir.join("analysis.md");
    fs::write(&md_path, md_content)?;

    // Write claude prompt file
    let prompt = generate_claude_prompt(session_dir, analysis);
    let timestamp = Local::now().format("%Y%m%d_%H%M").to_string();
    let prompt_path = session_dir.join(format!("claude_prompt_{}.txt", timestamp));
    fs::write(&prompt_path, prompt)?;

    println!("Analysis written to: {}", md_path.display());
    println!("Claude prompt written to: {}", prompt_path.display());

    Ok(())
}

/// Generate the analysis markdown report
fn format_analysis_markdown(analysis: &SessionAnalysis) -> String {
    let mut md = String::new();

    // Header
    md.push_str("# Training Session Analysis\n\n");
    md.push_str(&format!("**Session:** {}\n", analysis.session_id));
    md.push_str(&format!("**Protocol:** {}\n", analysis.protocol));
    md.push_str(&format!("**Goal:** {}\n", analysis.protocol_description));
    md.push_str(&format!("**AI Profile:** {}\n\n", analysis.ai_profile));

    // Session Summary
    md.push_str("## Session Summary\n\n");
    md.push_str("| Metric | Human | AI |\n");
    md.push_str("|--------|-------|----|\n");
    md.push_str(&format!(
        "| Wins | {} | {} |\n",
        analysis.aggregate.human_wins, analysis.aggregate.ai_wins
    ));
    md.push_str(&format!(
        "| Avg Possession | {:.1}% | {:.1}% |\n",
        analysis.aggregate.avg_human_possession, analysis.aggregate.avg_ai_possession
    ));
    md.push_str(&format!(
        "| Total Shots | {} | {} |\n",
        analysis.aggregate.total_human_shots, analysis.aggregate.total_ai_shots
    ));
    md.push_str(&format!(
        "| Total Goals | {} | {} |\n",
        analysis.aggregate.total_human_goals, analysis.aggregate.total_ai_goals
    ));
    md.push_str(&format!(
        "| Shot Accuracy | {:.1}% | {:.1}% |\n",
        analysis.aggregate.human_shot_accuracy(),
        analysis.aggregate.ai_shot_accuracy()
    ));
    md.push_str(&format!(
        "| Steal Attempts | {} | {} |\n",
        analysis.aggregate.total_human_steal_attempts, analysis.aggregate.total_ai_steal_attempts
    ));
    md.push_str(&format!(
        "| Steal Success | {:.1}% | {:.1}% |\n",
        analysis.aggregate.human_steal_rate(),
        analysis.aggregate.ai_steal_rate()
    ));
    md.push_str(&format!(
        "| Total Duration | {:.1}s | - |\n\n",
        analysis.aggregate.total_duration_secs
    ));

    // Per-Game Breakdown
    md.push_str("## Per-Game Breakdown\n\n");
    for game in &analysis.games {
        md.push_str(&format!(
            "### Game {} - {} ({}-{})\n\n",
            game.game_number, game.level_name, game.human_score, game.ai_score
        ));
        md.push_str(&format!("- Duration: {:.1}s\n", game.duration_secs));
        md.push_str(&format!(
            "- Possession: Human {:.1}% | AI {:.1}% | Free {:.1}%\n",
            game.possession.human_pct, game.possession.ai_pct, game.possession.free_pct
        ));
        md.push_str(&format!(
            "- Shots: Human {} ({} goals) | AI {} ({} goals)\n",
            game.shots.human_shots,
            game.shots.human_goals,
            game.shots.ai_shots,
            game.shots.ai_goals
        ));
        md.push_str(&format!(
            "- Steals: Human {}/{} | AI {}/{}\n",
            game.steals.human_successes,
            game.steals.human_attempts,
            game.steals.ai_successes,
            game.steals.ai_attempts
        ));

        // AI Goals breakdown
        if !game.ai_behavior.goal_distribution.is_empty() {
            md.push_str("- AI Goals: ");
            let goals: Vec<_> = game
                .ai_behavior
                .goal_distribution
                .iter()
                .map(|(k, v)| format!("{}: {}", k, v))
                .collect();
            md.push_str(&goals.join(", "));
            md.push_str(&format!(
                " ({} transitions)\n",
                game.ai_behavior.goal_transitions
            ));
        }

        if game.ai_behavior.oscillation_count > 0 {
            md.push_str(&format!(
                "- **Oscillation detected:** {} instances\n",
                game.ai_behavior.oscillation_count
            ));
        }

        if let Some(notes) = &game.notes {
            md.push_str(&format!("- **Player Notes:** {}\n", notes));
        }

        md.push('\n');
    }

    // AI Movement Analysis
    md.push_str("## AI Movement Analysis\n\n");

    for game in &analysis.games {
        let mv = &game.ai_movement;
        let behavior = &game.ai_behavior;

        md.push_str(&format!("### Game {} Movement\n\n", game.game_number));

        // Overall movement stats
        md.push_str(&format!(
            "- **Movement Time:** Left: {:.1}s | Right: {:.1}s | Still: {:.1}s\n",
            mv.time_moving_left_secs, mv.time_moving_right_secs, mv.time_stationary_secs
        ));
        md.push_str(&format!(
            "- **Position Range:** {:.0}px to {:.0}px (span: {:.0}px)\n",
            mv.x_position_range.0,
            mv.x_position_range.1,
            mv.x_position_range.1 - mv.x_position_range.0
        ));
        md.push_str(&format!(
            "- **Avg Distance to Opponent:** {:.0}px\n",
            mv.avg_distance_to_opponent
        ));
        md.push_str(&format!(
            "- **Closing Rate:** {:.1}px/s {}\n",
            mv.closing_rate,
            if mv.closing_rate > 0.0 {
                "(closing gap)"
            } else if mv.closing_rate < 0.0 {
                "(falling behind)"
            } else {
                "(neutral)"
            }
        ));

        if mv.time_stuck_secs > 0.5 {
            md.push_str(&format!(
                "- **Stuck Time:** {:.1}s ({:.0}% of game)\n",
                mv.time_stuck_secs,
                (mv.time_stuck_secs / game.duration_secs) * 100.0
            ));
        }

        // Per-goal breakdown table
        if !mv.per_goal_movement.is_empty() {
            md.push_str("\n**Per-Goal Breakdown:**\n\n");
            md.push_str("| Goal | Time | Left | Right | Still | Avg Dist |\n");
            md.push_str("|------|------|------|-------|-------|----------|\n");

            // Sort goals by time for consistent output
            let mut goals: Vec<_> = mv.per_goal_movement.iter().collect();
            goals.sort_by(|a, b| {
                b.1.time_secs
                    .partial_cmp(&a.1.time_secs)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            for (goal_name, stats) in goals {
                let pct = if game.duration_secs > 0.0 {
                    (stats.time_secs / game.duration_secs) * 100.0
                } else {
                    0.0
                };
                md.push_str(&format!(
                    "| {} | {:.1}s ({:.0}%) | {:.1}s | {:.1}s | {:.1}s | {:.0}px |\n",
                    goal_name,
                    stats.time_secs,
                    pct,
                    stats.time_moving_left_secs,
                    stats.time_moving_right_secs,
                    stats.time_stationary_secs,
                    stats.avg_distance
                ));
            }
        }

        // Goal time breakdown
        if !behavior.goal_time_breakdown.is_empty() {
            md.push_str("\n**Goal Time Breakdown:**\n");
            let mut goals: Vec<_> = behavior.goal_time_breakdown.iter().collect();
            goals.sort_by(|a, b| {
                b.1.partial_cmp(a.1).unwrap_or(std::cmp::Ordering::Equal)
            });
            for (goal, time) in goals {
                let pct = if game.duration_secs > 0.0 {
                    (*time / game.duration_secs) * 100.0
                } else {
                    0.0
                };
                md.push_str(&format!("- {}: {:.1}s ({:.0}%)\n", goal, time, pct));
            }
        }

        if !behavior.dominant_goal.is_empty() {
            md.push_str(&format!(
                "\n**Dominant Goal:** {} ({:.1}s, longest stretch: {:.1}s)\n",
                behavior.dominant_goal, behavior.dominant_goal_time, behavior.longest_goal_duration_secs
            ));
        }

        md.push('\n');
    }

    // Insights
    if !analysis.insights.is_empty() {
        md.push_str("## Insights\n\n");
        for insight in &analysis.insights {
            md.push_str(&format!("- {}\n", insight));
        }
        md.push('\n');
    }

    // Weaknesses
    if !analysis.weaknesses.is_empty() {
        md.push_str("## AI Weaknesses\n\n");
        for weakness in &analysis.weaknesses {
            md.push_str(&format!("- {}\n", weakness));
        }
        md.push('\n');
    }

    md
}

/// Generate a Claude prompt for AI analysis
pub fn generate_claude_prompt(session_dir: &Path, analysis: &SessionAnalysis) -> String {
    let mut prompt = String::new();

    prompt.push_str("## Training Session Analysis Request\n\n");
    prompt.push_str(&format!(
        "**Protocol:** {} - {}\n\n",
        analysis.protocol, analysis.protocol_description
    ));
    prompt.push_str(&format!(
        "I completed a training session against the **{}** AI.\n\n",
        analysis.ai_profile
    ));

    prompt.push_str(&format!(
        "**Analysis File:** {}/analysis.md\n\n",
        session_dir.display()
    ));

    prompt.push_str("### Session Overview\n");
    prompt.push_str(&format!(
        "- Games: {} | Human Wins: {} | AI Wins: {}\n",
        analysis.aggregate.total_games, analysis.aggregate.human_wins, analysis.aggregate.ai_wins
    ));
    prompt.push_str(&format!(
        "- Human Possession: {:.1}% | AI Possession: {:.1}%\n",
        analysis.aggregate.avg_human_possession, analysis.aggregate.avg_ai_possession
    ));
    prompt.push_str(&format!(
        "- AI Shots: {} (Accuracy: {:.1}%)\n",
        analysis.aggregate.total_ai_shots,
        analysis.aggregate.ai_shot_accuracy()
    ));
    prompt.push_str(&format!(
        "- AI Steals: {}/{} ({:.1}%)\n\n",
        analysis.aggregate.total_ai_steals,
        analysis.aggregate.total_ai_steal_attempts,
        analysis.aggregate.ai_steal_rate()
    ));

    // Movement summary
    prompt.push_str("### Movement Summary\n");
    for game in &analysis.games {
        let mv = &game.ai_movement;
        prompt.push_str(&format!(
            "- Game {}: Closing rate {:.1}px/s, stuck {:.1}s, pos range {:.0}px\n",
            game.game_number,
            mv.closing_rate,
            mv.time_stuck_secs,
            mv.x_position_range.1 - mv.x_position_range.0
        ));
    }
    prompt.push('\n');

    // Weaknesses
    if !analysis.weaknesses.is_empty() {
        prompt.push_str("### Identified Weaknesses\n");
        for weakness in &analysis.weaknesses {
            prompt.push_str(&format!("- {}\n", weakness));
        }
        prompt.push('\n');
    }

    // Player notes
    let notes: Vec<_> = analysis
        .games
        .iter()
        .filter_map(|g| g.notes.as_ref().map(|n| format!("Game {}: {}", g.game_number, n)))
        .collect();
    if !notes.is_empty() {
        prompt.push_str("### Player Notes\n");
        for note in &notes {
            prompt.push_str(&format!("- {}\n", note));
        }
        prompt.push('\n');
    }

    prompt.push_str("### Request\n");
    prompt.push_str("1. Read the analysis.md file for full metrics (includes AI Movement Analysis section)\n");
    prompt.push_str("2. Focus on movement issues: stuck patterns, monotonic movement, negative closing rate\n");
    prompt.push_str("3. Examine AI code in `src/ai/` (decision.rs, goals.rs, navigation.rs)\n");
    prompt.push_str("4. Check goal time breakdown - is AI spending too long in wrong goals?\n");
    prompt.push_str("5. Suggest specific code changes to improve AI tracking/positioning\n");
    prompt.push_str("6. Recommend ai_profiles.txt parameter adjustments if relevant\n");

    prompt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identify_movement_weaknesses() {
        let game = GameAnalysis {
            game_number: 1,
            level_name: "Test".to_string(),
            duration_secs: 20.0,
            human_score: 2,
            ai_score: 0,
            possession: PossessionStats::default(),
            shots: ShotStats::default(),
            steals: StealStats::default(),
            ai_behavior: AiBehaviorStats {
                dominant_goal: "InterceptDefense".to_string(),
                dominant_goal_time: 18.0,
                goal_transitions: 1,
                ..Default::default()
            },
            ai_movement: MovementStats {
                time_moving_left_secs: 15.0,
                time_moving_right_secs: 0.5,
                time_stationary_secs: 4.5,
                time_stuck_secs: 8.0,
                closing_rate: -15.0,
                x_position_range: (280.0, 320.0),
                ..Default::default()
            },
            notes: None,
        };

        let aggregate = AggregateStats {
            total_games: 1,
            ..Default::default()
        };

        let weaknesses = identify_weaknesses(&aggregate, &[game]);

        assert!(weaknesses.iter().any(|w| w.contains("stuck")));
        assert!(weaknesses.iter().any(|w| w.contains("falling behind")));
        assert!(weaknesses.iter().any(|w| w.contains("monotonic")));
        assert!(weaknesses.iter().any(|w| w.contains("position range")));
        assert!(weaknesses.iter().any(|w| w.contains("InterceptDefense")));
    }
}
