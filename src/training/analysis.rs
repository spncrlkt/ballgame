//! Automated post-training analysis
//!
//! Generates analysis reports and Claude prompts from training session data.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

use chrono::Local;

use crate::events::evlog_parser::{parse_evlog, ParsedEvlog};
use crate::events::PlayerId;
use crate::training::state::GameResult;

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

/// Full session analysis
#[derive(Debug, Clone)]
pub struct SessionAnalysis {
    pub session_id: String,
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

/// Analyze a training session from evlog files
pub fn analyze_session(session_dir: &Path, game_results: &[GameResult]) -> SessionAnalysis {
    let mut games = Vec::new();
    let mut aggregate = AggregateStats::default();

    // Get AI profile from first game result
    let ai_profile = game_results
        .first()
        .map(|_| {
            // Parse first evlog to get AI profile
            if let Some(first) = game_results.first() {
                if let Ok(parsed) = parse_evlog(&first.evlog_path) {
                    return parsed.metadata.right_profile.clone();
                }
            }
            "Unknown".to_string()
        })
        .unwrap_or_else(|| "Unknown".to_string());

    // Session ID from directory name
    let session_id = session_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    // Analyze each game
    for result in game_results {
        if let Ok(parsed) = parse_evlog(&result.evlog_path) {
            let game_analysis = analyze_game(&parsed, result);

            // Update aggregate stats
            aggregate.total_games += 1;
            if result.human_score > result.ai_score {
                aggregate.human_wins += 1;
            } else {
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
    }

    // Average possession percentages
    if aggregate.total_games > 0 {
        aggregate.avg_human_possession /= aggregate.total_games as f32;
        aggregate.avg_ai_possession /= aggregate.total_games as f32;
    }

    // Generate insights and weaknesses
    let insights = generate_insights(&aggregate, &games);
    let weaknesses = identify_weaknesses(&aggregate, &games);

    SessionAnalysis {
        session_id,
        ai_profile,
        games,
        aggregate,
        insights,
        weaknesses,
    }
}

/// Analyze a single game from parsed evlog
fn analyze_game(parsed: &ParsedEvlog, result: &GameResult) -> GameAnalysis {
    // Calculate possession from ticks
    let possession = calculate_possession(parsed);

    // Shot stats
    let shots = ShotStats {
        human_shots: parsed.shots_for(PlayerId::L) as u32,
        human_goals: parsed.goals_for(PlayerId::L) as u32,
        ai_shots: parsed.shots_for(PlayerId::R) as u32,
        ai_goals: parsed.goals_for(PlayerId::R) as u32,
    };

    // Steal stats
    let steals = StealStats {
        human_attempts: parsed.steal_attempts_for(PlayerId::L) as u32,
        human_successes: parsed.steal_successes_for(PlayerId::L) as u32,
        ai_attempts: parsed.steal_attempts_for(PlayerId::R) as u32,
        ai_successes: parsed.steal_successes_for(PlayerId::R) as u32,
    };

    // AI behavior analysis
    let ai_behavior = analyze_ai_behavior(parsed);

    // AI movement analysis
    let ai_movement = calculate_movement_stats(parsed);

    GameAnalysis {
        game_number: result.game_number,
        level_name: result.level_name.clone(),
        duration_secs: result.duration_secs,
        human_score: result.human_score,
        ai_score: result.ai_score,
        possession,
        shots,
        steals,
        ai_behavior,
        ai_movement,
        notes: result.notes.clone(),
    }
}

/// Calculate possession percentages from tick data
fn calculate_possession(parsed: &ParsedEvlog) -> PossessionStats {
    if parsed.ticks.is_empty() {
        return PossessionStats::default();
    }

    let mut human_ticks = 0u32;
    let mut ai_ticks = 0u32;
    let mut free_ticks = 0u32;

    for tick in &parsed.ticks {
        match tick.ball_state {
            'H' => {
                // Ball is held - determine by which player based on position proximity
                let ball_pos = tick.ball_pos;
                let left_dist = (ball_pos.0 - tick.left_pos.0).powi(2)
                    + (ball_pos.1 - tick.left_pos.1).powi(2);
                let right_dist = (ball_pos.0 - tick.right_pos.0).powi(2)
                    + (ball_pos.1 - tick.right_pos.1).powi(2);

                if left_dist < right_dist {
                    human_ticks += 1;
                } else {
                    ai_ticks += 1;
                }
            }
            'I' => {
                // Ball in flight - count as free
                free_ticks += 1;
            }
            _ => {
                // 'F' or other - free ball
                free_ticks += 1;
            }
        }
    }

    let total = (human_ticks + ai_ticks + free_ticks) as f32;
    if total == 0.0 {
        return PossessionStats::default();
    }

    PossessionStats {
        human_pct: (human_ticks as f32 / total) * 100.0,
        ai_pct: (ai_ticks as f32 / total) * 100.0,
        free_pct: (free_ticks as f32 / total) * 100.0,
        human_pickups: parsed.pickups_for(PlayerId::L) as u32,
        ai_pickups: parsed.pickups_for(PlayerId::R) as u32,
    }
}

/// Analyze AI behavior patterns
fn analyze_ai_behavior(parsed: &ParsedEvlog) -> AiBehaviorStats {
    let mut goal_distribution: HashMap<String, u32> = HashMap::new();
    let mut goal_time_breakdown: HashMap<String, f32> = HashMap::new();
    let mut goal_transitions = 0u32;
    let mut oscillation_count = 0u32;

    // Track recent goals for oscillation detection
    let mut recent_goals: Vec<&str> = Vec::new();
    const OSCILLATION_WINDOW: usize = 6;
    const OSCILLATION_THRESHOLD: usize = 3; // 3+ switches in window = oscillation

    // Get AI goals sorted by time
    let ai_goals: Vec<_> = parsed
        .ai_goals
        .iter()
        .filter(|g| g.player == PlayerId::R)
        .collect();

    let match_end_ms = parsed.max_time_ms;
    let mut last_goal: Option<(&str, u32)> = None; // (goal_name, start_time_ms)
    let mut longest_goal_duration_secs = 0.0f32;

    for ai_goal in &ai_goals {
        let goal = ai_goal.goal.as_str();
        let time_ms = ai_goal.time_ms;

        // Count distribution
        *goal_distribution.entry(goal.to_string()).or_insert(0) += 1;

        // Calculate time spent in previous goal
        if let Some((prev_goal, prev_time)) = last_goal {
            let duration_secs = (time_ms - prev_time) as f32 / 1000.0;
            *goal_time_breakdown.entry(prev_goal.to_string()).or_insert(0.0) += duration_secs;

            if duration_secs > longest_goal_duration_secs {
                longest_goal_duration_secs = duration_secs;
            }

            // Count transitions
            if prev_goal != goal {
                goal_transitions += 1;

                // Track for oscillation detection
                recent_goals.push(goal);
                if recent_goals.len() > OSCILLATION_WINDOW {
                    recent_goals.remove(0);
                }

                // Check for oscillation (rapid switching between goals)
                if recent_goals.len() >= OSCILLATION_WINDOW {
                    let unique_goals: std::collections::HashSet<_> = recent_goals.iter().collect();
                    if unique_goals.len() <= 2 && recent_goals.len() >= OSCILLATION_THRESHOLD {
                        oscillation_count += 1;
                    }
                }
            }
        }

        last_goal = Some((goal, time_ms));
    }

    // Account for time from last goal to match end
    if let Some((last_goal_name, last_time)) = last_goal {
        let duration_secs = (match_end_ms - last_time) as f32 / 1000.0;
        *goal_time_breakdown
            .entry(last_goal_name.to_string())
            .or_insert(0.0) += duration_secs;

        if duration_secs > longest_goal_duration_secs {
            longest_goal_duration_secs = duration_secs;
        }
    }

    // Find dominant goal (most time spent)
    let (dominant_goal, dominant_goal_time) = goal_time_breakdown
        .iter()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(k, v)| (k.clone(), *v))
        .unwrap_or_default();

    AiBehaviorStats {
        goal_distribution,
        goal_transitions,
        oscillation_count,
        goal_time_breakdown,
        longest_goal_duration_secs,
        dominant_goal,
        dominant_goal_time,
    }
}

/// Calculate AI movement statistics from tick and input data
fn calculate_movement_stats(parsed: &ParsedEvlog) -> MovementStats {
    if parsed.ticks.is_empty() {
        return MovementStats::default();
    }

    // Collect AI (right player) inputs
    let ai_inputs: Vec<_> = parsed.inputs_for(PlayerId::R).collect();

    let mut time_moving_left_secs = 0.0f32;
    let mut time_moving_right_secs = 0.0f32;
    let mut time_stationary_secs = 0.0f32;

    let mut total_distance = 0.0f32;
    let mut distance_samples = 0u32;

    let mut closing_distance_sum = 0.0f32;
    let mut closing_samples = 0u32;

    let mut x_positions: Vec<f32> = Vec::new();
    let mut time_stuck_secs = 0.0f32;

    // Per-goal movement tracking
    let mut per_goal_movement: HashMap<String, GoalMovementStats> = HashMap::new();

    // Build a map of time -> AI goal
    let mut current_goal = String::new();
    let ai_goals: Vec<_> = parsed
        .ai_goals
        .iter()
        .filter(|g| g.player == PlayerId::R)
        .collect();
    let mut goal_idx = 0;

    // Stuck detection: track position history
    const STUCK_WINDOW_MS: u32 = 500;
    const STUCK_THRESHOLD_PX: f32 = 5.0;
    let mut position_history: Vec<(u32, f32)> = Vec::new(); // (time_ms, x_pos)

    // Process ticks with their corresponding inputs
    let mut prev_tick: Option<&crate::events::evlog_parser::TickData> = None;

    for tick in &parsed.ticks {
        let time_ms = tick.time_ms;

        // Update current goal
        while goal_idx < ai_goals.len() && ai_goals[goal_idx].time_ms <= time_ms {
            current_goal = ai_goals[goal_idx].goal.clone();
            goal_idx += 1;
        }

        // Find input for this tick (closest by time)
        let input = ai_inputs
            .iter()
            .filter(|i| i.time_ms <= time_ms)
            .last()
            .map(|i| i.move_x)
            .unwrap_or(0.0);

        // Calculate time delta from previous tick
        let dt_secs = if let Some(prev) = prev_tick {
            (time_ms - prev.time_ms) as f32 / 1000.0
        } else {
            0.0
        };

        // Movement direction tracking
        if input < -0.1 {
            time_moving_left_secs += dt_secs;
        } else if input > 0.1 {
            time_moving_right_secs += dt_secs;
        } else {
            time_stationary_secs += dt_secs;
        }

        // Track position
        let ai_x = tick.right_pos.0;
        let human_x = tick.left_pos.0;
        x_positions.push(ai_x);

        // Distance to opponent
        let distance = (ai_x - human_x).abs();
        total_distance += distance;
        distance_samples += 1;

        // Closing rate (change in distance)
        if let Some(prev) = prev_tick {
            let prev_distance = (prev.right_pos.0 - prev.left_pos.0).abs();
            let distance_change = prev_distance - distance; // positive = closing
            if dt_secs > 0.0 {
                closing_distance_sum += distance_change / dt_secs;
                closing_samples += 1;
            }
        }

        // Stuck detection
        position_history.push((time_ms, ai_x));
        // Remove old entries
        position_history.retain(|(t, _)| time_ms - *t <= STUCK_WINDOW_MS);

        if position_history.len() >= 2 && input.abs() > 0.1 {
            let oldest = position_history.first().unwrap();
            let position_change = (ai_x - oldest.1).abs();
            if position_change < STUCK_THRESHOLD_PX {
                time_stuck_secs += dt_secs;
            }
        }

        // Per-goal movement tracking
        if !current_goal.is_empty() {
            let goal_stats = per_goal_movement
                .entry(current_goal.clone())
                .or_default();
            goal_stats.time_secs += dt_secs;
            if input < -0.1 {
                goal_stats.time_moving_left_secs += dt_secs;
            } else if input > 0.1 {
                goal_stats.time_moving_right_secs += dt_secs;
            } else {
                goal_stats.time_stationary_secs += dt_secs;
            }
            goal_stats.avg_distance += distance * dt_secs; // weighted sum, will divide later
        }

        prev_tick = Some(tick);
    }

    // Finalize per-goal averages
    for stats in per_goal_movement.values_mut() {
        if stats.time_secs > 0.0 {
            stats.avg_distance /= stats.time_secs;
        }
    }

    // Calculate closing rate per goal
    // (This is a simplification - we already have aggregate closing rate)

    let avg_distance_to_opponent = if distance_samples > 0 {
        total_distance / distance_samples as f32
    } else {
        0.0
    };

    let closing_rate = if closing_samples > 0 {
        closing_distance_sum / closing_samples as f32
    } else {
        0.0
    };

    let avg_x_position = if !x_positions.is_empty() {
        x_positions.iter().sum::<f32>() / x_positions.len() as f32
    } else {
        0.0
    };

    let x_position_range = if !x_positions.is_empty() {
        let min = x_positions.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = x_positions.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        (min, max)
    } else {
        (0.0, 0.0)
    };

    MovementStats {
        time_moving_left_secs,
        time_moving_right_secs,
        time_stationary_secs,
        avg_distance_to_opponent,
        closing_rate,
        time_stuck_secs,
        avg_x_position,
        x_position_range,
        per_goal_movement,
    }
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
    md.push_str(&format!("# Training Session Analysis\n\n"));
    md.push_str(&format!("**Session:** {}\n", analysis.session_id));
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
    use crate::events::evlog_parser::parse_evlog_content;

    // Sample evlog with AI movement data
    const SAMPLE_EVLOG_WITH_MOVEMENT: &str = r#"
T:00000|SE|test_session|2026-01-25
T:00000|MS|3|Test Level|Player|Aggressive|12345
T:00000|AG|R|ChaseBall
T:00100|T|1|-300.0,-350.0|0.0,0.0|300.0,-350.0|0.0,0.0|-300.0,-350.0|0.0,0.0|H
T:00100|I|L|0.0|-
T:00100|I|R|-1.0|-
T:00200|AG|R|InterceptDefense
T:00200|T|2|-300.0,-350.0|0.0,0.0|290.0,-350.0|-100.0,0.0|-300.0,-350.0|0.0,0.0|H
T:00200|I|L|0.0|-
T:00200|I|R|-1.0|-
T:00300|T|3|-300.0,-350.0|0.0,0.0|280.0,-350.0|-100.0,0.0|-300.0,-350.0|0.0,0.0|H
T:00300|I|L|0.0|-
T:00300|I|R|-1.0|-
T:00400|T|4|-300.0,-350.0|0.0,0.0|270.0,-350.0|-100.0,0.0|-300.0,-350.0|0.0,0.0|H
T:00400|I|L|0.0|-
T:00400|I|R|-1.0|-
T:00500|T|5|-290.0,-350.0|100.0,0.0|260.0,-350.0|-100.0,0.0|-290.0,-350.0|0.0,0.0|H
T:00500|I|L|1.0|-
T:00500|I|R|-1.0|-
T:01000|ME|0|0|1.0
"#;

    #[test]
    fn test_ai_behavior_with_goal_time_breakdown() {
        let parsed = parse_evlog_content(SAMPLE_EVLOG_WITH_MOVEMENT);
        let behavior = analyze_ai_behavior(&parsed);

        // Should have 2 goals tracked
        assert!(behavior.goal_distribution.contains_key("ChaseBall"));
        assert!(behavior.goal_distribution.contains_key("InterceptDefense"));

        // Should have goal time breakdown
        assert!(!behavior.goal_time_breakdown.is_empty());

        // InterceptDefense should be dominant (more time spent)
        assert_eq!(behavior.dominant_goal, "InterceptDefense");
        assert!(behavior.dominant_goal_time > 0.0);

        // Should have 1 transition
        assert_eq!(behavior.goal_transitions, 1);
    }

    #[test]
    fn test_movement_stats() {
        let parsed = parse_evlog_content(SAMPLE_EVLOG_WITH_MOVEMENT);
        let movement = calculate_movement_stats(&parsed);

        // AI should be moving left (input -1.0 throughout)
        assert!(movement.time_moving_left_secs > 0.0);
        assert!(movement.time_moving_right_secs < 0.1); // nearly 0

        // Should have per-goal movement data
        assert!(!movement.per_goal_movement.is_empty());
        assert!(movement.per_goal_movement.contains_key("InterceptDefense"));

        // Position range should be tracked
        assert!(movement.x_position_range.0 <= movement.x_position_range.1);
    }

    #[test]
    fn test_identify_movement_weaknesses() {
        // Create a minimal GameAnalysis with movement issues
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
                time_stuck_secs: 8.0, // 40% of game - should trigger weakness
                closing_rate: -15.0,  // falling behind
                x_position_range: (280.0, 320.0), // only 40px range
                ..Default::default()
            },
            notes: None,
        };

        let aggregate = AggregateStats {
            total_games: 1,
            ..Default::default()
        };

        let weaknesses = identify_weaknesses(&aggregate, &[game]);

        // Should detect stuck pattern
        assert!(weaknesses.iter().any(|w| w.contains("stuck")));

        // Should detect falling behind
        assert!(weaknesses.iter().any(|w| w.contains("falling behind")));

        // Should detect monotonic movement
        assert!(weaknesses.iter().any(|w| w.contains("monotonic")));

        // Should detect small position range
        assert!(weaknesses.iter().any(|w| w.contains("position range")));

        // Should detect dominant goal with few transitions
        assert!(weaknesses.iter().any(|w| w.contains("InterceptDefense")));
    }

    #[test]
    fn test_real_session_analysis() {
        // Test against actual evlog file if it exists
        use crate::events::evlog_parser::parse_evlog;
        use std::path::Path;

        let evlog_path = Path::new("training_logs/session_20260125_172911/game_1_level7.evlog");
        if !evlog_path.exists() {
            // Skip test if file doesn't exist
            return;
        }

        let parsed = parse_evlog(evlog_path).expect("Should parse evlog");

        // Verify inputs were parsed
        assert!(!parsed.inputs.is_empty(), "Should have input data");

        // Verify AI inputs exist
        let ai_inputs: Vec<_> = parsed.inputs_for(PlayerId::R).collect();
        assert!(!ai_inputs.is_empty(), "Should have AI input data");

        // Run analysis
        let behavior = analyze_ai_behavior(&parsed);
        let movement = calculate_movement_stats(&parsed);

        // Verify goal time breakdown matches expected from analysis.md
        // InterceptDefense should be dominant at ~97%
        assert_eq!(behavior.dominant_goal, "InterceptDefense");
        assert!(behavior.dominant_goal_time > 15.0, "InterceptDefense should be >15s");

        // Verify movement stats are reasonable
        // From analysis.md: Left: 6.3s, Right: 3.8s, Still: 11.0s
        assert!(movement.time_moving_left_secs > 5.0, "Should have >5s moving left");
        assert!(movement.time_moving_right_secs > 2.0, "Should have >2s moving right");
        assert!(movement.time_stationary_secs > 8.0, "Should have >8s stationary");

        // Position range should be ~978px (from -443 to 535)
        let range = movement.x_position_range.1 - movement.x_position_range.0;
        assert!(range > 500.0, "Position range should be >500px, got {}", range);

        // Closing rate should be positive (AI was closing gap)
        assert!(movement.closing_rate > 0.0, "Closing rate should be positive (closing gap)");

        // Stuck time should be around 3.2s (15%)
        assert!(movement.time_stuck_secs > 2.0, "Should have >2s stuck time");
        assert!(movement.time_stuck_secs < 6.0, "Should have <6s stuck time");

        println!("Real session analysis verified:");
        println!("  Dominant goal: {} ({:.1}s)", behavior.dominant_goal, behavior.dominant_goal_time);
        println!("  Movement: L={:.1}s R={:.1}s Still={:.1}s",
            movement.time_moving_left_secs,
            movement.time_moving_right_secs,
            movement.time_stationary_secs);
        println!("  Position range: {:.0} to {:.0} ({:.0}px)",
            movement.x_position_range.0, movement.x_position_range.1, range);
        println!("  Closing rate: {:.1}px/s", movement.closing_rate);
        println!("  Stuck time: {:.1}s", movement.time_stuck_secs);
    }
}
