//! Headless simulation runner

use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use rand::Rng;
use std::time::Duration;

use crate::ai::{
    AiCapabilities, AiNavState, AiProfileDatabase, AiState, InputState, NavGraph, ai_decision_update,
    ai_navigation_update, mark_nav_dirty_on_level_change, rebuild_nav_graph,
    shot_quality::evaluate_shot_quality,
};
use crate::ball::{
    Ball, BallState, CurrentPalette, Velocity, ball_collisions, ball_follow_holder,
    ball_gravity, ball_player_collision, ball_spin, ball_state_update, apply_velocity, pickup_ball,
};
use crate::events::{
    emit_game_events, snapshot_ball, snapshot_player, EmitterConfig, EventBuffer, EventBus,
    EventEmitterState, GameConfig, GameEvent,
};
use crate::palettes::PaletteDatabase;
use crate::constants::*;
use crate::levels::LevelDatabase;
use crate::player::{
    HoldingBall, JumpState, Player, Team, apply_gravity, apply_input, check_collisions,
};
use crate::scoring::{CurrentLevel, Score, check_scoring};
use crate::world::Basket;
use crate::player::TargetBasket;
use crate::shooting::{ChargingShot, LastShotInfo, throw_ball, update_shot_charge};
use crate::ui::PhysicsTweaks;
use crate::steal::{StealContest, StealCooldown, StealTracker, steal_cooldown_update};

use super::config::SimConfig;
use super::control::{SimControl, SimEventBuffer};
use super::db::SimDatabase;
use super::metrics::{MatchResult, SimMetrics};
use super::setup::sim_setup;
use super::shot_test::run_shot_test;

/// Get the effective level for a match.
/// If config.level is None, picks a random non-debug level (excluding Pit).
fn get_effective_level(config: &SimConfig, level_db: &LevelDatabase, seed: u64) -> u32 {
    if let Some(level) = config.level {
        return level;
    }

    // Build list of valid levels (exclude debug levels and "Pit")
    let valid_levels: Vec<u32> = (1..=level_db.len() as u32)
        .filter(|&level| {
            if let Some(lvl) = level_db.get((level - 1) as usize) {
                !lvl.debug && lvl.name != "Pit"
            } else {
                false
            }
        })
        .collect();

    if valid_levels.is_empty() {
        return 3; // Fallback to Islands
    }

    // Use seed to pick a level deterministically
    let idx = (seed as usize) % valid_levels.len();
    valid_levels[idx]
}

/// Run a single match and return the result
pub fn run_match(config: &SimConfig, seed: u64, level_db: &LevelDatabase, profile_db: &AiProfileDatabase) -> MatchResult {
    // Determine effective level (random if not specified)
    let level = get_effective_level(config, level_db, seed);

    // Create a minimal Bevy app
    let mut app = App::new();

    // Minimal plugins for headless operation
    app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
        Duration::from_secs_f32(1.0 / 60.0), // 60 FPS equivalent
    )));
    // Add transform plugin for GlobalTransform propagation (needed for collision)
    app.add_plugins(bevy::transform::TransformPlugin);

    // Set up fixed timestep for physics (1/60 second)
    app.insert_resource(Time::<Fixed>::from_duration(Duration::from_secs_f32(1.0 / 60.0)));

    // Game resources
    app.insert_resource((*level_db).clone());
    app.insert_resource((*profile_db).clone());
    app.init_resource::<Score>();
    app.insert_resource(CurrentLevel(level));
    app.init_resource::<StealContest>();
    app.init_resource::<StealTracker>();
    app.init_resource::<NavGraph>();
    app.init_resource::<AiCapabilities>();
    app.init_resource::<PhysicsTweaks>();
    app.init_resource::<LastShotInfo>();
    app.insert_resource(CurrentPalette(0)); // Use first palette for simulation
    app.init_resource::<PaletteDatabase>();
    app.insert_resource(EventBus::new());

    // Event logging buffer
    let mut event_buffer = SimEventBuffer {
        buffer: EventBuffer::new(),
        enabled: config.db_path.is_some(),
        emitter_state: EventEmitterState::with_config(EmitterConfig {
            track_both_ai_goals: true,
        }),
    };

    // Start session if logging enabled
    if event_buffer.enabled {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
        event_buffer.buffer.start_session(&timestamp);

        // Log match start event
        let level_name = level_db
            .get((level - 1) as usize)
            .map(|l| l.name.clone())
            .unwrap_or_else(|| format!("Level {}", level));
        event_buffer.buffer.log(0.0, GameEvent::MatchStart {
            level,
            level_name,
            left_profile: config.left_profile.clone(),
            right_profile: config.right_profile.clone(),
            seed,
        });

        // Log game configuration (all tunable parameters)
        event_buffer.buffer.log(0.0, GameEvent::Config(GameConfig {
            // Physics
            gravity_rise: GRAVITY_RISE,
            gravity_fall: GRAVITY_FALL,
            jump_velocity: JUMP_VELOCITY,
            move_speed: MOVE_SPEED,
            ground_accel: GROUND_ACCEL,
            air_accel: AIR_ACCEL,
            // Ball physics
            ball_gravity: BALL_GRAVITY,
            ball_bounce: BALL_BOUNCE,
            ball_air_friction: BALL_AIR_FRICTION,
            ball_ground_friction: BALL_GROUND_FRICTION,
            // Shooting
            shot_max_power: SHOT_MAX_POWER,
            shot_max_speed: SHOT_MAX_SPEED,
            shot_charge_time: SHOT_CHARGE_TIME,
            shot_max_variance: SHOT_MAX_VARIANCE,
            shot_min_variance: SHOT_MIN_VARIANCE,
            // Steal
            steal_range: STEAL_RANGE,
            steal_success_chance: STEAL_SUCCESS_CHANCE,
            steal_cooldown: STEAL_COOLDOWN,
            // Presets not tracked in simulation (uses defaults)
            preset_movement: None,
            preset_ball: None,
            preset_shooting: None,
            preset_composite: None,
        }));
    }
    app.insert_resource(event_buffer);

    // Simulation resources
    app.insert_resource(SimControl {
        config: config.clone(),
        should_exit: false,
        current_seed: seed,
    });
    app.insert_resource(SimMetrics::new());

    // Startup system
    app.add_systems(Startup, sim_setup);
    // Mark nav graph dirty after first frame so GlobalTransform is populated
    app.add_systems(PostStartup, |mut nav_graph: ResMut<NavGraph>| {
        nav_graph.dirty = true;
    });

    // Game systems (no rendering/UI)
    app.add_systems(
        Update,
        (
            mark_nav_dirty_on_level_change,
            rebuild_nav_graph,
            ai_navigation_update,
            ai_decision_update,
        )
            .chain(),
    );

    app.add_systems(Update, (steal_cooldown_update, metrics_update, emit_simulation_events));

    app.add_systems(
        FixedUpdate,
        (
            apply_input,
            apply_gravity,
            ball_gravity,
            ball_spin,
            apply_velocity,
            check_collisions,
            ball_collisions,
            ball_state_update,
            ball_player_collision,
            ball_follow_holder,
            pickup_ball,
            steal_cooldown_update,
            update_shot_charge,
            throw_ball,
            check_scoring,
            sim_check_end_conditions,
        )
            .chain(),
    );

    // Run Startup first to spawn entities
    app.finish();
    app.cleanup();
    app.update(); // This runs Startup, First, etc.

    // Then run simulation loop with manual scheduling at 60Hz
    let fixed_dt = Duration::from_secs_f32(1.0 / 60.0);

    loop {
        // Advance all time resources consistently
        app.world_mut()
            .resource_mut::<Time<Virtual>>()
            .advance_by(fixed_dt);
        app.world_mut()
            .resource_mut::<Time<Real>>()
            .advance_by(fixed_dt);
        app.world_mut()
            .resource_mut::<Time<Fixed>>()
            .advance_by(fixed_dt);

        // Run Update schedule (AI decisions)
        app.world_mut().run_schedule(Update);

        // Run FixedUpdate schedule (physics)
        app.world_mut().run_schedule(FixedUpdate);

        let control = app.world().resource::<SimControl>();
        if control.should_exit {
            break;
        }
    }

    // Extract results - clone the values we need to avoid borrow conflicts
    let (elapsed, score_left, score_right, left_stats, right_stats) = {
        let metrics = app.world().resource::<SimMetrics>();
        let score = app.world().resource::<Score>();
        (
            metrics.elapsed,
            score.left,
            score.right,
            metrics.left.clone(),
            metrics.right.clone(),
        )
    };

    // Warn about broken AI behavior but don't fail (for debugging)
    let total_shots = left_stats.shots_attempted + right_stats.shots_attempted;
    let _total_steals = left_stats.steals_attempted + right_stats.steals_attempted;

    if score_left == 0 && score_right == 0 {
        eprintln!(
            "WARNING: 0-0 game on level {} ({} vs {}, seed {}). \
             AI is not scoring. Left shots: {}, Right shots: {}",
            level, config.left_profile, config.right_profile, seed,
            left_stats.shots_attempted, right_stats.shots_attempted
        );
    }

    if total_shots < 10 {
        eprintln!(
            "WARNING: Only {} shots in 60s on level {} ({} vs {}, seed {}). \
             AI is not shooting enough. Left: {}, Right: {}",
            total_shots, level, config.left_profile, config.right_profile, seed,
            left_stats.shots_attempted, right_stats.shots_attempted
        );
    }

    // Note: steals_attempted metric is not implemented, so skip this check
    // if total_steals == 0 { ... }

    let level_name = level_db
        .get((level - 1) as usize)
        .map(|l| l.name.clone())
        .unwrap_or_else(|| format!("Level {}", level));

    let mut result = MatchResult {
        level,
        level_name,
        left_profile: config.left_profile.clone(),
        right_profile: config.right_profile.clone(),
        duration: elapsed,
        score_left,
        score_right,
        winner: String::new(),
        left_stats,
        right_stats,
        seed,
        events: Vec::new(),
    };

    result.left_stats.finalize();
    result.right_stats.finalize();
    result.determine_winner();

    if let Some(mut event_buffer) = app.world_mut().get_resource_mut::<SimEventBuffer>() {
        if event_buffer.enabled {
            event_buffer.buffer.log(elapsed, GameEvent::MatchEnd {
                score_left,
                score_right,
                duration: elapsed,
            });
            result.events = event_buffer.buffer.drain_events();
        }
    }

    result
}

/// Update metrics during simulation
/// Uses fixed timestep for headless mode consistency
fn metrics_update(
    mut metrics: ResMut<SimMetrics>,
    players: Query<(Entity, &Transform, &Team, &AiState, &JumpState, &AiNavState, Option<&HoldingBall>, &TargetBasket), With<Player>>,
    balls: Query<(&Transform, &BallState), With<Ball>>,
    baskets: Query<(&Transform, &Basket)>,
    score: Res<Score>,
) {
    // Use fixed timestep for consistent headless simulation (60 FPS)
    const FIXED_DT: f32 = 1.0 / 60.0;
    let dt = FIXED_DT;
    metrics.elapsed += dt;
    metrics.time_since_score += dt;

    // Detect shot release (ball transitions from Held to InFlight)
    for (_ball_transform, ball_state) in &balls {
        match ball_state {
            BallState::InFlight { shooter, .. } => {
                // Ball is in flight - check if it was just released
                if metrics.prev_ball_holder.is_some() {
                    // Shot was just released - record position and quality
                    if let Ok((_, shooter_transform, shooter_team, _, _, _, _, target_basket)) = players.get(*shooter) {
                        let pos = shooter_transform.translation.truncate();

                        // Find target basket position
                        let basket_pos = baskets
                            .iter()
                            .find(|(_, b)| **b == target_basket.0)
                            .map(|(t, _)| t.translation.truncate())
                            .unwrap_or_default();

                        let quality = evaluate_shot_quality(pos, basket_pos);

                        match shooter_team {
                            Team::Left => {
                                metrics.left.shots_attempted += 1;
                                metrics.left.shot_positions_sum_x += pos.x;
                                metrics.left.shot_positions_sum_y += pos.y;
                                metrics.left.shot_quality_sum += quality;
                            }
                            Team::Right => {
                                metrics.right.shots_attempted += 1;
                                metrics.right.shot_positions_sum_x += pos.x;
                                metrics.right.shot_positions_sum_y += pos.y;
                                metrics.right.shot_quality_sum += quality;
                            }
                        }
                    }
                    metrics.prev_ball_holder = None;
                }
            }
            BallState::Held(holder) => {
                metrics.prev_ball_holder = Some(*holder);
            }
            BallState::Free => {
                metrics.prev_ball_holder = None;
            }
        }
    }

    // Track player stats
    for (_entity, transform, team, ai_state, jump_state, nav_state, holding, _target_basket) in &players {
        let pos = transform.translation.truncate();
        let goal_name = format!("{:?}", ai_state.current_goal);
        let has_ball = holding.is_some();

        // Get player index for array access
        let idx = match team {
            Team::Left => 0,
            Team::Right => 1,
        };

        // Track jumps: increment when is_jumping transitions false → true
        let currently_jumping = jump_state.is_jumping;
        if currently_jumping && !metrics.prev_jumping[idx] {
            match team {
                Team::Left => metrics.left.jumps += 1,
                Team::Right => metrics.right.jumps += 1,
            }
        }
        metrics.prev_jumping[idx] = currently_jumping;

        // Track nav completions: increment when path completes successfully
        let nav_active = nav_state.active;
        let path_len = nav_state.current_path.len();
        let path_complete = nav_state.path_complete();

        // Detect completion: was active with a path, now complete or inactive
        if metrics.prev_nav_active[idx]
            && metrics.prev_nav_path_len[idx] > 0
            && (path_complete || !nav_active)
        {
            // Path finished - count as completed if we made progress
            if nav_state.path_index > 0 || path_complete {
                match team {
                    Team::Left => metrics.left.nav_paths_completed += 1,
                    Team::Right => metrics.right.nav_paths_completed += 1,
                }
            } else {
                // Cleared without progress - count as failed
                match team {
                    Team::Left => metrics.left.nav_paths_failed += 1,
                    Team::Right => metrics.right.nav_paths_failed += 1,
                }
            }
        }
        metrics.prev_nav_active[idx] = nav_active;
        metrics.prev_nav_path_len[idx] = path_len;

        match team {
            Team::Left => {
                // Track distance
                if let Some(last) = metrics.last_pos_left {
                    metrics.left.distance_traveled += pos.distance(last);
                }
                metrics.last_pos_left = Some(pos);

                // Track possession time
                if has_ball {
                    metrics.left.possession_time += dt;
                }

                // Track goal time
                *metrics.left.goal_time.entry(goal_name).or_insert(0.0) += dt;
            }
            Team::Right => {
                // Track distance
                if let Some(last) = metrics.last_pos_right {
                    metrics.right.distance_traveled += pos.distance(last);
                }
                metrics.last_pos_right = Some(pos);

                // Track possession time
                if has_ball {
                    metrics.right.possession_time += dt;
                }

                // Track goal time
                *metrics.right.goal_time.entry(goal_name).or_insert(0.0) += dt;
            }
        }
    }

    // Check for score changes
    let prev_left = metrics.left.goals;
    let prev_right = metrics.right.goals;

    if score.left > prev_left {
        metrics.left.goals = score.left;
        metrics.left.shots_made += 1;
        metrics.time_since_score = 0.0;
    }
    if score.right > prev_right {
        metrics.right.goals = score.right;
        metrics.right.shots_made += 1;
        metrics.time_since_score = 0.0;
    }
}

/// Check end conditions for simulation
fn sim_check_end_conditions(
    metrics: Res<SimMetrics>,
    mut control: ResMut<SimControl>,
    score: Res<Score>,
) {
    let config = &control.config;

    // Time limit
    if metrics.elapsed >= config.duration_limit {
        control.should_exit = true;
        return;
    }

    // Score limit
    if config.score_limit > 0
        && (score.left >= config.score_limit || score.right >= config.score_limit)
    {
        control.should_exit = true;
        return;
    }

    // Stalemate
    if metrics.time_since_score >= config.stalemate_timeout && (score.left > 0 || score.right > 0) {
        control.should_exit = true;
    }
}

/// Emit game events during simulation
///
/// This is a thin wrapper around the shared `emit_game_events` function.
fn emit_simulation_events(
    mut event_buffer: ResMut<SimEventBuffer>,
    metrics: Res<SimMetrics>,
    score: Res<Score>,
    steal_contest: Res<StealContest>,
    players: Query<
        (
            Entity,
            &Team,
            &Transform,
            &Velocity,
            &ChargingShot,
            &AiState,
            &StealCooldown,
            Option<&HoldingBall>,
            &InputState,
        ),
        With<Player>,
    >,
    balls: Query<(&Transform, &Velocity, &BallState), With<Ball>>,
) {
    if !event_buffer.enabled {
        return;
    }

    let time = metrics.elapsed;

    // Convert query results to snapshots
    let player_snapshots: Vec<_> = players
        .iter()
        .map(|(entity, team, transform, velocity, charging, ai_state, steal_cooldown, holding, input_state)| {
            snapshot_player(
                entity,
                team,
                transform,
                velocity,
                charging,
                ai_state,
                steal_cooldown,
                holding,
                input_state,
            )
        })
        .collect();

    let ball_snapshot = balls
        .iter()
        .next()
        .map(|(transform, velocity, state)| snapshot_ball(transform, velocity, state));

    // Destructure to get separate mutable borrows
    let SimEventBuffer {
        ref mut emitter_state,
        ref mut buffer,
        ..
    } = *event_buffer;

    // Use the shared emitter
    emit_game_events(
        emitter_state,
        buffer,
        time,
        &score,
        &steal_contest,
        &player_snapshots,
        ball_snapshot.as_ref(),
    );
}


/// Main simulation entry point
pub fn run_simulation(config: SimConfig) {
    // Load databases
    let level_db = LevelDatabase::load_from_file(LEVELS_FILE);
    let profile_db = AiProfileDatabase::default();

    // Initialize parallel execution if requested
    if config.parallel > 0 {
        super::parallel::init_parallel(config.parallel);
    }

    // Initialize database if requested
    let db = config.db_path.as_ref().map(|path| {
        match SimDatabase::open(std::path::Path::new(path)) {
            Ok(db) => {
                if !config.quiet {
                    println!("Using database: {}", path);
                }
                Some(db)
            }
            Err(e) => {
                eprintln!("Warning: Failed to open database {}: {}", path, e);
                None
            }
        }
    }).flatten();

    // Get profile list
    let profiles: Vec<String> = profile_db.profiles().iter().map(|p| p.name.clone()).collect();

    // Get level names
    let mut level_names = std::collections::HashMap::new();
    for i in 0..level_db.len() {
        if let Some(level) = level_db.get(i) {
            level_names.insert((i + 1) as u32, level.name.clone());
        }
    }

    // Helper to format level name for display
    let level_display = |level: Option<u32>| -> String {
        match level {
            Some(l) => level_names.get(&l).cloned().unwrap_or_else(|| format!("Level {}", l)),
            None => "Random".to_string(),
        }
    };

    match &config.mode {
        super::config::SimMode::Single => {
            let seed = config.seed.unwrap_or_else(|| rand::thread_rng().r#gen());
            if !config.quiet {
                println!(
                    "Running single match: {} vs {} on {} (seed: {})",
                    config.left_profile,
                    config.right_profile,
                    level_display(config.level),
                    seed
                );
            }

            let result = run_match(&config, seed, &level_db, &profile_db);
            output_result(&result, &config);
            if let Some(ref db) = db {
                store_results_in_db(db, "single", std::slice::from_ref(&result), &config);
            }
        }

        super::config::SimMode::MultiMatch { count } => {
            let parallel_mode = config.parallel > 0;
            if !config.quiet {
                println!(
                    "Running {} matches{}: {} vs {} on {}",
                    count,
                    if parallel_mode { format!(" (parallel, {} threads)", config.parallel) } else { String::new() },
                    config.left_profile,
                    config.right_profile,
                    level_display(config.level)
                );
            }

            let base_seed = config.seed.unwrap_or_else(|| rand::thread_rng().r#gen());

            let results = if parallel_mode {
                // Parallel execution
                super::parallel::run_multi_match_parallel(&config, *count, base_seed, &level_db, &profile_db)
            } else {
                // Sequential execution
                let mut results = Vec::new();
                for i in 0..*count {
                    if !config.quiet {
                        print!("\rMatch {}/{}...", i + 1, count);
                        use std::io::Write;
                        std::io::stdout().flush().ok();
                    }

                    let seed = base_seed.wrapping_add(i as u64);
                    let result = run_match(&config, seed, &level_db, &profile_db);
                    results.push(result);
                }
                results
            };

            if !config.quiet {
                println!("\rCompleted {} matches.", count);
            }

            // Aggregate results
            let wins: u32 = results.iter().filter(|r| r.winner == "left").count() as u32;
            let ties: u32 = results.iter().filter(|r| r.winner == "tie").count() as u32;
            let total_left: u32 = results.iter().map(|r| r.score_left).sum();
            let total_right: u32 = results.iter().map(|r| r.score_right).sum();

            println!(
                "\nResults: {} wins - {} ties - {} losses",
                wins,
                ties,
                count - wins - ties
            );
            println!(
                "Total score: {} - {} (avg: {:.1} - {:.1})",
                total_left,
                total_right,
                total_left as f32 / *count as f32,
                total_right as f32 / *count as f32
            );

            // Store in database if enabled
            if let Some(ref db) = db {
                store_results_in_db(db, "multi_match", &results, &config);
            }

            if let Some(output_file) = &config.output_file {
                let json = serde_json::to_string_pretty(&results).unwrap();
                std::fs::write(output_file, json).expect("Failed to write output");
                println!("Results written to {}", output_file);
            }
        }

        super::config::SimMode::Tournament { matches_per_pair } => {
            let parallel_mode = config.parallel > 0;
            let total_matches = profiles.len() * (profiles.len() - 1) * (*matches_per_pair as usize);

            if !config.quiet {
                println!(
                    "Running tournament{}: {} profiles, {} matches per pair ({} total)",
                    if parallel_mode { format!(" (parallel, {} threads)", config.parallel) } else { String::new() },
                    profiles.len(),
                    matches_per_pair,
                    total_matches
                );
            }

            let base_seed = config.seed.unwrap_or_else(|| rand::thread_rng().r#gen());
            let mut tournament = super::metrics::TournamentResult::new();

            if parallel_mode {
                // Parallel execution
                tournament.matches = super::parallel::run_tournament_parallel(
                    &config, *matches_per_pair, base_seed, &level_db, &profile_db
                );
            } else {
                // Sequential execution
                let mut match_num = 0;
                for left in &profiles {
                    for right in &profiles {
                        if left == right {
                            continue;
                        }

                        for _i in 0..*matches_per_pair {
                            match_num += 1;
                            if !config.quiet {
                                print!(
                                    "\rMatch {}/{}: {} vs {}...",
                                    match_num, total_matches, left, right
                                );
                                use std::io::Write;
                                std::io::stdout().flush().ok();
                            }

                            let mut match_config = config.clone();
                            match_config.left_profile = left.clone();
                            match_config.right_profile = right.clone();

                            let seed = base_seed.wrapping_add(match_num as u64);
                            let result = run_match(&match_config, seed, &level_db, &profile_db);
                            tournament.matches.push(result);
                        }
                    }
                }
            }

            if !config.quiet {
                println!("\rTournament complete. {} matches played.", total_matches);
            }

            tournament.calculate_win_rates();
            println!("{}", tournament.format_table(&profiles));

            // Store in database if enabled
            if let Some(ref db) = db {
                store_results_in_db(db, "tournament", &tournament.matches, &config);
            }

            if let Some(output_file) = &config.output_file {
                let json = serde_json::to_string_pretty(&tournament).unwrap();
                std::fs::write(output_file, json).expect("Failed to write output");
                println!("Results written to {}", output_file);
            }
        }

        super::config::SimMode::LevelSweep { matches_per_level } => {
            let parallel_mode = config.parallel > 0;
            let num_levels = level_db.len();

            if !config.quiet {
                println!(
                    "Running level sweep{}: {} on {} levels, {} matches each",
                    if parallel_mode { format!(" (parallel, {} threads)", config.parallel) } else { String::new() },
                    config.left_profile, num_levels, matches_per_level
                );
            }

            let mut sweep = super::metrics::LevelSweepResult::new(&config.left_profile);
            let base_seed = config.seed.unwrap_or_else(|| rand::thread_rng().r#gen());

            if parallel_mode {
                // Parallel execution - run all matches then group by level
                let results = super::parallel::run_level_sweep_parallel(
                    &config, *matches_per_level, base_seed, &level_db, &profile_db
                );
                for result in results {
                    sweep.results_by_level.entry(result.level).or_default().push(result);
                }
            } else {
                // Sequential execution
                let mut match_num = 0;
                for level in 1..=num_levels {
                    // Skip debug level
                    if level_db.get(level - 1).is_some_and(|l| l.debug) {
                        continue;
                    }

                    for i in 0..*matches_per_level {
                        match_num += 1;
                        if !config.quiet {
                            print!(
                                "\rLevel {} match {}/{}...",
                                level,
                                i + 1,
                                matches_per_level
                            );
                            use std::io::Write;
                            std::io::stdout().flush().ok();
                        }

                        let mut match_config = config.clone();
                        match_config.level = Some(level as u32);

                        let seed = base_seed.wrapping_add(match_num as u64);
                        let result = run_match(&match_config, seed, &level_db, &profile_db);

                        sweep
                            .results_by_level
                            .entry(level as u32)
                            .or_default()
                            .push(result);
                    }
                }
            }

            if !config.quiet {
                println!("\rLevel sweep complete.");
            }

            sweep.calculate_stats();
            println!("{}", sweep.format_table(&level_names));

            // Store in database if enabled
            if let Some(ref db) = db {
                let all_results: Vec<_> = sweep.results_by_level.values()
                    .flat_map(|v| v.iter().cloned())
                    .collect();
                store_results_in_db(db, "level_sweep", &all_results, &config);
            }

            if let Some(output_file) = &config.output_file {
                let json = serde_json::to_string_pretty(&sweep).unwrap();
                std::fs::write(output_file, json).expect("Failed to write output");
                println!("Results written to {}", output_file);
            }
        }

        super::config::SimMode::Regression => {
            println!("Regression testing not yet implemented.");
            println!("Would compare current AI performance to baseline metrics.");
        }

        super::config::SimMode::ShotTest { shots_per_position } => {
            run_shot_test(&config, *shots_per_position, &level_db);
        }

        super::config::SimMode::GhostTrial { path } => {
            run_ghost_trials(&config, path, &level_db, &profile_db);
        }
    }
}

fn output_result(result: &MatchResult, config: &SimConfig) {
    let json = serde_json::to_string_pretty(result).unwrap();

    if let Some(output_file) = &config.output_file {
        std::fs::write(output_file, &json).expect("Failed to write output");
        println!("Results written to {}", output_file);
    } else {
        println!("{}", json);
    }
}

/// Store match results in the database
fn store_results_in_db(db: &SimDatabase, session_type: &str, results: &[MatchResult], config: &SimConfig) {
    // Create session
    let config_json = serde_json::to_string(config).ok();
    let session_id = match db.create_session(session_type, config_json.as_deref()) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("Warning: Failed to create database session: {}", e);
            return;
        }
    };

    // Insert each result
    let mut stored = 0;
    for result in results {
        match db.insert_match(&session_id, result) {
            Ok(match_id) => {
                stored += 1;
                if !result.events.is_empty() {
                    if let Err(e) = db.insert_events_with_points(match_id, result.duration, &result.events) {
                        eprintln!("Warning: Failed to store match events: {}", e);
                    }
                }
            }
            Err(e) => eprintln!("Warning: Failed to store match result: {}", e),
        }
    }

    if !config.quiet && stored > 0 {
        println!("Stored {} results in database", stored);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::metrics::PlayerStats;

    #[test]
    fn test_store_results_in_db_persists_events() {
        let db = SimDatabase::open_in_memory().unwrap();
        let mut result = MatchResult {
            level: 1,
            level_name: "Test".to_string(),
            left_profile: "Balanced".to_string(),
            right_profile: "Balanced".to_string(),
            duration: 1.0,
            score_left: 1,
            score_right: 0,
            winner: "left".to_string(),
            left_stats: PlayerStats::default(),
            right_stats: PlayerStats::default(),
            seed: 123,
            events: Vec::new(),
        };
        result.events.push((0, GameEvent::ResetScores));

        store_results_in_db(&db, "test", std::slice::from_ref(&result), &SimConfig::default());

        let match_id: i64 = db
            .conn()
            .query_row("SELECT MAX(id) FROM matches", [], |row| row.get(0))
            .unwrap();
        let event_count = db.event_count(match_id).unwrap();
        assert!(event_count > 0);
    }
}

/// Run ghost trials from a file or directory
fn run_ghost_trials(
    config: &SimConfig,
    path: &str,
    level_db: &LevelDatabase,
    profile_db: &AiProfileDatabase,
) {
    let path = std::path::Path::new(path);

    // Collect ghost files
    let ghost_files: Vec<std::path::PathBuf> = if path.is_dir() {
        std::fs::read_dir(path)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .filter(|p| p.extension().map_or(false, |ext| ext == "ghost"))
                    .collect()
            })
            .unwrap_or_default()
    } else if path.extension().map_or(false, |ext| ext == "ghost") {
        vec![path.to_path_buf()]
    } else {
        eprintln!("Error: {} is not a .ghost file or directory", path.display());
        return;
    };

    if ghost_files.is_empty() {
        eprintln!("No .ghost files found in {}", path.display());
        return;
    }

    if !config.quiet {
        println!("Running {} ghost trials against {}", ghost_files.len(), config.right_profile);
        println!();
    }

    let mut results = Vec::new();
    let mut defended = 0;
    let mut scored = 0;

    for ghost_path in &ghost_files {
        let trial = match super::ghost::load_ghost_trial(ghost_path) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Failed to load {}: {}", ghost_path.display(), e);
                continue;
            }
        };

        let seed = config.seed.unwrap_or_else(|| rand::thread_rng().r#gen());
        let result = run_ghost_trial(config, &trial, seed, level_db, profile_db);

        if !config.quiet {
            let status = if result.ai_defended() { "DEFENDED" } else { "SCORED" };
            println!(
                "  {} [{}]: {} ({:.1}s, {:.0}% survival)",
                trial.source_file,
                status,
                result.outcome,
                result.duration,
                result.survival_ratio() * 100.0
            );
        }

        if result.ai_defended() {
            defended += 1;
        } else {
            scored += 1;
        }

        results.push(result);
    }

    // Summary
    println!();
    println!("=== Ghost Trial Summary ===");
    println!("AI Profile: {}", config.right_profile);
    println!("Total trials: {}", results.len());
    println!("AI defended: {} ({:.0}%)", defended, defended as f32 / results.len() as f32 * 100.0);
    println!("Ghost scored: {} ({:.0}%)", scored, scored as f32 / results.len() as f32 * 100.0);

    // Output JSON if requested
    if let Some(output_file) = &config.output_file {
        let json = serde_json::to_string_pretty(&results).unwrap();
        std::fs::write(output_file, json).expect("Failed to write output");
        println!("Results written to {}", output_file);
    }
}

/// Run a single ghost trial
pub fn run_ghost_trial(
    config: &SimConfig,
    trial: &super::ghost::GhostTrial,
    seed: u64,
    level_db: &LevelDatabase,
    profile_db: &AiProfileDatabase,
) -> super::ghost::GhostTrialResult {
    use bevy::app::ScheduleRunnerPlugin;
    use super::ghost::{GhostOutcome, GhostPlaybackState, GhostTrialResult, max_tick};

    // Create a minimal Bevy app
    let mut app = App::new();

    // Minimal plugins for headless operation
    app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
        Duration::from_secs_f32(1.0 / 60.0),
    )));
    app.add_plugins(bevy::transform::TransformPlugin);

    // Use the trial's level
    let level = trial.level;

    // Game resources
    app.insert_resource((*level_db).clone());
    app.insert_resource((*profile_db).clone());
    app.init_resource::<Score>();
    app.insert_resource(CurrentLevel(level));
    app.init_resource::<StealContest>();
    app.init_resource::<StealTracker>();
    app.init_resource::<NavGraph>();
    app.init_resource::<PhysicsTweaks>();
    app.init_resource::<LastShotInfo>();
    app.insert_resource(CurrentPalette(0));
    app.init_resource::<PaletteDatabase>();

    // Ghost trial resources
    app.insert_resource(trial.clone());
    app.insert_resource(GhostPlaybackState::default());

    // Simulation control - use a modified config for ghost trial
    let mut ghost_config = config.clone();
    ghost_config.level = Some(level);
    ghost_config.left_profile = "Ghost".to_string(); // Ghost player (not AI)

    app.insert_resource(SimControl {
        config: ghost_config,
        should_exit: false,
        current_seed: seed,
    });
    app.insert_resource(SimMetrics::new());

    // Startup system - custom setup for ghost trials
    app.add_systems(Startup, ghost_trial_setup);
    app.add_systems(PostStartup, |mut nav_graph: ResMut<NavGraph>| {
        nav_graph.dirty = true;
    });

    // AI systems in Update
    app.add_systems(
        Update,
        (
            mark_nav_dirty_on_level_change,
            rebuild_nav_graph,
            ai_navigation_update,
            ai_decision_update,
        )
            .chain(),
    );

    app.add_systems(Update, (steal_cooldown_update, super::ghost::ghost_check_end_conditions));

    // Ghost input and physics in FixedUpdate for consistent timing
    app.add_systems(
        FixedUpdate,
        (
            super::ghost::ghost_input_system, // Apply ghost inputs first
            apply_input,
            apply_gravity,
            ball_gravity,
            ball_spin,
            apply_velocity,
            check_collisions,
            ball_collisions,
            ball_state_update,
            ball_player_collision,
            ball_follow_holder,
            pickup_ball,
            steal_cooldown_update,
            update_shot_charge,
            throw_ball,
            check_scoring,
        )
            .chain(),
    );

    // Run until trial ends
    loop {
        app.update();

        let control = app.world().resource::<SimControl>();
        if control.should_exit {
            break;
        }
    }

    // Extract results
    let playback = app.world().resource::<GhostPlaybackState>();
    let outcome = playback.outcome.unwrap_or(GhostOutcome::TimeLimit);
    let end_tick = playback.end_tick;
    let total_ticks = max_tick(trial);
    let duration = end_tick as f32 / 1000.0;

    GhostTrialResult {
        source_file: trial.source_file.clone(),
        level: trial.level,
        level_name: trial.level_name.clone(),
        ai_profile: config.right_profile.clone(),
        outcome,
        duration,
        end_tick,
        total_ticks,
        originally_scored: trial.originally_scored,
    }
}

/// Setup system for ghost trials
/// Left player is ghost-controlled, right player is AI-controlled
/// Ball starts with left player (ghost)
fn ghost_trial_setup(
    mut commands: Commands,
    level_db: Res<LevelDatabase>,
    profile_db: Res<AiProfileDatabase>,
    control: Res<SimControl>,
    current_level: Res<CurrentLevel>,
    _trial: Res<super::ghost::GhostTrial>,
) {
    use crate::ai::{AiGoal, AiNavState, AiState};
    use crate::ball::{
        Ball, BallPlayerContact, BallPulse, BallRolling, BallShotGrace, BallSpin, BallState, BallStyle,
    };
    use crate::player::{CoyoteTimer, Facing, Grounded, TargetBasket};
    use crate::shooting::ChargingShot;
    use crate::world::{Basket, Collider, Platform};

    let config = &control.config;

    // Find AI profile index for right player
    let right_idx = profile_db
        .profiles()
        .iter()
        .position(|p| p.name == config.right_profile)
        .unwrap_or(0);

    // Spawn left player (Ghost controlled - no AI)
    let left_player = commands
        .spawn((
            Transform::from_translation(PLAYER_SPAWN_LEFT),
            Sprite {
                custom_size: Some(PLAYER_SIZE),
                ..default()
            },
            Player,
            Velocity::default(),
            Grounded(false),
            CoyoteTimer::default(),
            JumpState::default(),
            Facing::default(),
            ChargingShot::default(),
            TargetBasket(Basket::Right),
            Collider,
            Team::Left,
            InputState::default(), // Ghost inputs will be written here
            StealCooldown::default(),
        ))
        .id();

    // Spawn right player (AI controlled)
    commands
        .spawn((
            Transform::from_translation(PLAYER_SPAWN_RIGHT),
            Sprite {
                custom_size: Some(PLAYER_SIZE),
                ..default()
            },
            Player,
            Velocity::default(),
            Grounded(false),
            CoyoteTimer::default(),
            JumpState::default(),
            Facing(-1.0),
            ChargingShot::default(),
            TargetBasket(Basket::Left),
            Collider,
            Team::Right,
        ))
        .insert((
            InputState::default(),
            AiState {
                current_goal: AiGoal::ChaseBall,
                profile_index: right_idx,
                ..default()
            },
            AiNavState::default(),
            StealCooldown::default(),
        ));

    // Spawn ball - give it to the ghost (left player)
    let ball_entity = commands.spawn((
        Transform::from_translation(PLAYER_SPAWN_LEFT + Vec3::new(10.0, 0.0, 0.0)),
        Sprite {
            custom_size: Some(BALL_SIZE),
            ..default()
        },
        Ball,
        BallState::Held(left_player), // Ghost starts with the ball
        Velocity::default(),
        BallSpin::default(),
        BallPlayerContact::default(),
        BallPulse { timer: 0.0 },
        BallRolling(false),
        BallShotGrace::default(),
        BallStyle("wedges".to_string()),
        Collider,
    )).id();

    // Add HoldingBall to the ghost player so they can throw
    commands.entity(left_player).insert(HoldingBall(ball_entity));

    // Spawn floor
    commands.spawn((
        Sprite {
            custom_size: Some(Vec2::new(ARENA_WIDTH + 200.0, 40.0)),
            ..default()
        },
        Transform::from_xyz(0.0, ARENA_FLOOR_Y - 20.0, 0.0),
        Platform,
        Collider,
    ));

    // Spawn walls
    commands.spawn((
        Sprite {
            custom_size: Some(Vec2::new(WALL_THICKNESS, 5000.0)),
            ..default()
        },
        Transform::from_xyz(-ARENA_WIDTH / 2.0 + WALL_THICKNESS / 2.0, 2000.0, 0.0),
        Platform,
        Collider,
    ));
    commands.spawn((
        Sprite {
            custom_size: Some(Vec2::new(WALL_THICKNESS, 5000.0)),
            ..default()
        },
        Transform::from_xyz(ARENA_WIDTH / 2.0 - WALL_THICKNESS / 2.0, 2000.0, 0.0),
        Platform,
        Collider,
    ));

    // Spawn level platforms
    let level_idx = (current_level.0 - 1) as usize;
    if let Some(level) = level_db.get(level_idx) {
        for platform in &level.platforms {
            match platform {
                crate::levels::PlatformDef::Mirror { x, y, width } => {
                    // Left
                    commands.spawn((
                        Sprite {
                            custom_size: Some(Vec2::new(*width, 20.0)),
                            ..default()
                        },
                        Transform::from_xyz(-x, ARENA_FLOOR_Y + y, 0.0),
                        Platform,
                        Collider,
                        crate::world::LevelPlatform,
                    ));
                    // Right
                    commands.spawn((
                        Sprite {
                            custom_size: Some(Vec2::new(*width, 20.0)),
                            ..default()
                        },
                        Transform::from_xyz(*x, ARENA_FLOOR_Y + y, 0.0),
                        Platform,
                        Collider,
                        crate::world::LevelPlatform,
                    ));
                }
                crate::levels::PlatformDef::Center { y, width } => {
                    commands.spawn((
                        Sprite {
                            custom_size: Some(Vec2::new(*width, 20.0)),
                            ..default()
                        },
                        Transform::from_xyz(0.0, ARENA_FLOOR_Y + y, 0.0),
                        Platform,
                        Collider,
                        crate::world::LevelPlatform,
                    ));
                }
            }
        }

        // Spawn corner steps if level has them
        if level.step_count > 0 {
            super::setup::spawn_corner_steps(
                &mut commands,
                level.step_count,
                level.corner_height,
                level.corner_width,
                level.step_push_in,
            );
        }

        // Spawn baskets (baskets need Sprite for scoring check)
        let basket_y = ARENA_FLOOR_Y + level.basket_height;
        let wall_inner = ARENA_WIDTH / 2.0 - WALL_THICKNESS;
        let left_basket_x = -wall_inner + level.basket_push_in;
        let right_basket_x = wall_inner - level.basket_push_in;

        commands.spawn((
            Sprite {
                custom_size: Some(BASKET_SIZE),
                ..default()
            },
            Transform::from_xyz(left_basket_x, basket_y, 0.0),
            Basket::Left,
        ));
        commands.spawn((
            Sprite {
                custom_size: Some(BASKET_SIZE),
                ..default()
            },
            Transform::from_xyz(right_basket_x, basket_y, 0.0),
            Basket::Right,
        ));
    }
}
