//! Headless simulation runner

use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use rand::Rng;
use std::time::Duration;

use crate::ai::{
    AiGoal, AiNavState, AiProfileDatabase, AiState, InputState, NavGraph, ai_decision_update,
    ai_navigation_update, mark_nav_dirty_on_level_change, rebuild_nav_graph,
};
use crate::ball::{
    Ball, BallPlayerContact, BallPulse, BallRolling, BallShotGrace, BallSpin, BallState, BallStyle,
    CurrentPalette, Velocity, ball_collisions, ball_follow_holder, ball_gravity, ball_player_collision,
    ball_spin, ball_state_update, apply_velocity, pickup_ball,
};
use crate::palettes::PaletteDatabase;
use crate::constants::*;
use crate::levels::LevelDatabase;
use crate::player::{
    CoyoteTimer, Facing, Grounded, JumpState, Player, TargetBasket, Team,
    apply_gravity, apply_input, check_collisions,
};
use crate::scoring::{CurrentLevel, Score, check_scoring};
use crate::shooting::{ChargingShot, LastShotInfo, throw_ball, update_shot_charge};
use crate::ui::PhysicsTweaks;
use crate::steal::{StealContest, StealCooldown, steal_cooldown_update};
use crate::world::{Basket, Collider, CornerRamp, Platform};

use super::config::SimConfig;
use super::metrics::{MatchResult, SimMetrics};

/// Resource to control simulation
#[derive(Resource)]
pub struct SimControl {
    pub config: SimConfig,
    pub should_exit: bool,
    pub current_seed: u64,
}

/// Run a single match and return the result
pub fn run_match(config: &SimConfig, seed: u64, level_db: &LevelDatabase, profile_db: &AiProfileDatabase) -> MatchResult {
    // Create a minimal Bevy app
    let mut app = App::new();

    // Minimal plugins for headless operation
    app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
        Duration::from_secs_f32(1.0 / 60.0), // 60 FPS equivalent
    )));
    // Add transform plugin for GlobalTransform propagation (needed for collision)
    app.add_plugins(bevy::transform::TransformPlugin);

    // Game resources
    app.insert_resource((*level_db).clone());
    app.insert_resource((*profile_db).clone());
    app.init_resource::<Score>();
    app.insert_resource(CurrentLevel(config.level));
    app.init_resource::<StealContest>();
    app.init_resource::<NavGraph>();
    app.init_resource::<PhysicsTweaks>();
    app.init_resource::<LastShotInfo>();
    app.insert_resource(CurrentPalette(0)); // Use first palette for simulation
    app.init_resource::<PaletteDatabase>();

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

    app.add_systems(Update, (steal_cooldown_update, metrics_update));

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

    // Run until match ends
    loop {
        app.update();

        let control = app.world().resource::<SimControl>();
        if control.should_exit {
            break;
        }
    }

    // Extract results
    let metrics = app.world().resource::<SimMetrics>();
    let score = app.world().resource::<Score>();
    let level_name = level_db
        .get((config.level - 1) as usize)
        .map(|l| l.name.clone())
        .unwrap_or_else(|| format!("Level {}", config.level));

    let mut result = MatchResult {
        level: config.level,
        level_name,
        left_profile: config.left_profile.clone(),
        right_profile: config.right_profile.clone(),
        duration: metrics.elapsed,
        score_left: score.left,
        score_right: score.right,
        winner: String::new(),
        left_stats: metrics.left.clone(),
        right_stats: metrics.right.clone(),
        seed,
    };

    result.left_stats.finalize();
    result.right_stats.finalize();
    result.determine_winner();

    result
}

/// Setup system for simulation
fn sim_setup(
    mut commands: Commands,
    level_db: Res<LevelDatabase>,
    profile_db: Res<AiProfileDatabase>,
    control: Res<SimControl>,
    current_level: Res<CurrentLevel>,
) {
    let config = &control.config;

    // Find profile indices
    let left_idx = profile_db
        .profiles()
        .iter()
        .position(|p| p.name == config.left_profile)
        .unwrap_or(0);
    let right_idx = profile_db
        .profiles()
        .iter()
        .position(|p| p.name == config.right_profile)
        .unwrap_or(0);

    // Spawn left player (AI controlled)
    commands
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
        ))
        .insert((
            InputState::default(),
            AiState {
                current_goal: AiGoal::ChaseBall,
                profile_index: left_idx,
                ..default()
            },
            AiNavState::default(),
            StealCooldown::default(),
        ));

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

    // Spawn ball
    commands.spawn((
        Transform::from_translation(BALL_SPAWN),
        Sprite {
            custom_size: Some(BALL_SIZE),
            ..default()
        },
        Ball,
        BallState::default(),
        Velocity::default(),
        BallPlayerContact::default(),
        BallPulse::default(),
        BallRolling::default(),
        BallShotGrace::default(),
        BallSpin::default(),
        BallStyle::new("wedges"),
    ));

    // Spawn arena floor
    commands.spawn((
        Sprite {
            custom_size: Some(Vec2::new(ARENA_WIDTH - WALL_THICKNESS * 2.0, 40.0)),
            ..default()
        },
        Transform::from_xyz(0.0, ARENA_FLOOR_Y, 0.0),
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
            spawn_corner_steps(
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

/// Update metrics during simulation
fn metrics_update(
    time: Res<Time>,
    mut metrics: ResMut<SimMetrics>,
    players: Query<(&Transform, &Team, &AiState, Option<&crate::player::HoldingBall>), With<Player>>,
    score: Res<Score>,
) {
    let dt = time.delta_secs();
    metrics.elapsed += dt;
    metrics.time_since_score += dt;

    // Track player stats
    for (transform, team, ai_state, holding) in &players {
        let pos = transform.translation.truncate();
        let goal_name = format!("{:?}", ai_state.current_goal);
        let has_ball = holding.is_some();

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

/// Main simulation entry point
pub fn run_simulation(config: SimConfig) {
    // Load databases
    let level_db = LevelDatabase::load_from_file(LEVELS_FILE);
    let profile_db = AiProfileDatabase::default();

    // Get profile list
    let profiles: Vec<String> = profile_db.profiles().iter().map(|p| p.name.clone()).collect();

    // Get level names
    let mut level_names = std::collections::HashMap::new();
    for i in 0..level_db.len() {
        if let Some(level) = level_db.get(i) {
            level_names.insert((i + 1) as u32, level.name.clone());
        }
    }

    match &config.mode {
        super::config::SimMode::Single => {
            let seed = config.seed.unwrap_or_else(|| rand::thread_rng().r#gen());
            if !config.quiet {
                println!(
                    "Running single match: {} vs {} on {} (seed: {})",
                    config.left_profile,
                    config.right_profile,
                    level_names.get(&config.level).unwrap_or(&"?".to_string()),
                    seed
                );
            }

            let result = run_match(&config, seed, &level_db, &profile_db);
            output_result(&result, &config);
        }

        super::config::SimMode::MultiMatch { count } => {
            if !config.quiet {
                println!(
                    "Running {} matches: {} vs {} on {}",
                    count,
                    config.left_profile,
                    config.right_profile,
                    level_names.get(&config.level).unwrap_or(&"?".to_string())
                );
            }

            let mut results = Vec::new();
            let base_seed = config.seed.unwrap_or_else(|| rand::thread_rng().r#gen());

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

            if let Some(output_file) = &config.output_file {
                let json = serde_json::to_string_pretty(&results).unwrap();
                std::fs::write(output_file, json).expect("Failed to write output");
                println!("Results written to {}", output_file);
            }
        }

        super::config::SimMode::Tournament { matches_per_pair } => {
            if !config.quiet {
                println!(
                    "Running tournament: {} profiles, {} matches per pair",
                    profiles.len(),
                    matches_per_pair
                );
            }

            let mut tournament = super::metrics::TournamentResult::new();
            let base_seed = config.seed.unwrap_or_else(|| rand::thread_rng().r#gen());
            let mut match_num = 0;
            let total_matches = profiles.len() * (profiles.len() - 1) * (*matches_per_pair as usize);

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

            if !config.quiet {
                println!("\rTournament complete. {} matches played.", total_matches);
            }

            tournament.calculate_win_rates();
            println!("{}", tournament.format_table(&profiles));

            if let Some(output_file) = &config.output_file {
                let json = serde_json::to_string_pretty(&tournament).unwrap();
                std::fs::write(output_file, json).expect("Failed to write output");
                println!("Results written to {}", output_file);
            }
        }

        super::config::SimMode::LevelSweep { matches_per_level } => {
            let num_levels = level_db.len();
            if !config.quiet {
                println!(
                    "Running level sweep: {} on {} levels, {} matches each",
                    config.left_profile, num_levels, matches_per_level
                );
            }

            let mut sweep = super::metrics::LevelSweepResult::new(&config.left_profile);
            let base_seed = config.seed.unwrap_or_else(|| rand::thread_rng().r#gen());
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
                    match_config.level = level as u32;

                    let seed = base_seed.wrapping_add(match_num as u64);
                    let result = run_match(&match_config, seed, &level_db, &profile_db);

                    sweep
                        .results_by_level
                        .entry(level as u32)
                        .or_default()
                        .push(result);
                }
            }

            if !config.quiet {
                println!("\rLevel sweep complete.");
            }

            sweep.calculate_stats();
            println!("{}", sweep.format_table(&level_names));

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

/// Spawn corner steps for simulation (matching the main game's behavior)
fn spawn_corner_steps(
    commands: &mut Commands,
    step_count: usize,
    corner_height: f32,
    corner_width: f32,
    step_push_in: f32,
) {
    // Wall inner edges
    let left_wall_inner = -ARENA_WIDTH / 2.0 + WALL_THICKNESS;
    let right_wall_inner = ARENA_WIDTH / 2.0 - WALL_THICKNESS;

    // Step dimensions
    let step_height = corner_height / step_count as f32;
    let step_width = corner_width / step_count as f32;

    // Floor top surface
    let floor_top = ARENA_FLOOR_Y + 20.0;

    // Left steps: go from wall (high) toward center (low)
    for i in 0..step_count {
        let step_num = (step_count - 1 - i) as f32; // Reverse so 0 is lowest
        let y = floor_top + step_height * (step_num + 0.5);

        let (x, width) = if i == 0 {
            // Top step extends from wall to step_push_in + step_width
            let right_edge = left_wall_inner + step_push_in + step_width;
            let center = (left_wall_inner + right_edge) / 2.0;
            let full_width = right_edge - left_wall_inner;
            (center, full_width)
        } else {
            (
                left_wall_inner + step_push_in + step_width * (i as f32 + 0.5),
                step_width,
            )
        };

        commands.spawn((
            Sprite {
                custom_size: Some(Vec2::new(width, CORNER_STEP_THICKNESS)),
                ..default()
            },
            Transform::from_xyz(x, y, 0.0),
            Platform,
            Collider,
            CornerRamp,
        ));
    }

    // Right steps: mirror of left
    for i in 0..step_count {
        let step_num = (step_count - 1 - i) as f32;
        let y = floor_top + step_height * (step_num + 0.5);

        let (x, width) = if i == 0 {
            let left_edge = right_wall_inner - step_push_in - step_width;
            let center = (right_wall_inner + left_edge) / 2.0;
            let full_width = right_wall_inner - left_edge;
            (center, full_width)
        } else {
            (
                right_wall_inner - step_push_in - step_width * (i as f32 + 0.5),
                step_width,
            )
        };

        commands.spawn((
            Sprite {
                custom_size: Some(Vec2::new(width, CORNER_STEP_THICKNESS)),
                ..default()
            },
            Transform::from_xyz(x, y, 0.0),
            Platform,
            Collider,
            CornerRamp,
        ));
    }
}
