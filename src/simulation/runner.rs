//! Headless simulation runner

use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use rand::Rng;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
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
use crate::events::{EventBuffer, GameConfig, GameEvent, PlayerId};
use crate::palettes::PaletteDatabase;
use crate::constants::*;
use crate::levels::LevelDatabase;
use crate::player::{
    CoyoteTimer, Facing, Grounded, HoldingBall, JumpState, Player, TargetBasket, Team,
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

/// Resource for event logging in simulation
#[derive(Resource, Default)]
pub struct SimEventBuffer {
    pub buffer: EventBuffer,
    pub enabled: bool,
    pub log_dir: PathBuf,
    /// Track previous score for detecting score changes
    pub prev_score_left: u32,
    pub prev_score_right: u32,
    /// Track previous ball holder for possession events
    pub prev_ball_holder: Option<Entity>,
    /// Track previous charging state for shot events
    pub prev_charging: [bool; 2],
    /// Track last tick time for 50ms sampling
    pub last_tick_time: f32,
    /// Frame counter for tick events
    pub tick_frame_count: u64,
    /// Track previous AI goals for change detection
    pub prev_ai_goal_left: Option<String>,
    pub prev_ai_goal_right: Option<String>,
    /// Track previous steal cooldowns for steal detection
    pub prev_steal_cooldowns: [f32; 2],
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

    // Event logging buffer
    let mut event_buffer = SimEventBuffer {
        buffer: EventBuffer::new(),
        enabled: config.log_events,
        log_dir: PathBuf::from(&config.log_dir),
        prev_score_left: 0,
        prev_score_right: 0,
        prev_ball_holder: None,
        prev_charging: [false, false],
        last_tick_time: 0.0,
        tick_frame_count: 0,
        prev_ai_goal_left: None,
        prev_ai_goal_right: None,
        prev_steal_cooldowns: [0.0, 0.0],
    };

    // Start session if logging enabled
    if event_buffer.enabled {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
        event_buffer.buffer.start_session(&timestamp);

        // Log match start event
        let level_name = level_db
            .get((config.level - 1) as usize)
            .map(|l| l.name.clone())
            .unwrap_or_else(|| format!("Level {}", config.level));
        event_buffer.buffer.log(0.0, GameEvent::MatchStart {
            level: config.level,
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

    // Run until match ends
    loop {
        app.update();

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

    let level_name = level_db
        .get((config.level - 1) as usize)
        .map(|l| l.name.clone())
        .unwrap_or_else(|| format!("Level {}", config.level));

    let mut result = MatchResult {
        level: config.level,
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
    };

    result.left_stats.finalize();
    result.right_stats.finalize();
    result.determine_winner();

    // Write event log if enabled
    {
        let mut event_buffer = app.world_mut().resource_mut::<SimEventBuffer>();
        if event_buffer.enabled {
            // Log match end event
            event_buffer.buffer.log(elapsed, GameEvent::MatchEnd {
                score_left,
                score_right,
                duration: elapsed,
            });

            // Write buffer to file
            write_event_log(&event_buffer);
        }
    }

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
    players: Query<(&Transform, &Team, &AiState, &JumpState, &AiNavState, Option<&HoldingBall>), With<Player>>,
    score: Res<Score>,
) {
    let dt = time.delta_secs();
    metrics.elapsed += dt;
    metrics.time_since_score += dt;

    // Track player stats
    for (transform, team, ai_state, jump_state, nav_state, holding) in &players {
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
fn emit_simulation_events(
    mut event_buffer: ResMut<SimEventBuffer>,
    metrics: Res<SimMetrics>,
    score: Res<Score>,
    steal_contest: Res<StealContest>,
    players: Query<(Entity, &Team, &Transform, &Velocity, &ChargingShot, &AiState, &StealCooldown, Option<&HoldingBall>), With<Player>>,
    balls: Query<(&Transform, &Velocity, &BallState), With<Ball>>,
) {
    if !event_buffer.enabled {
        return;
    }

    let time = metrics.elapsed;

    // === Tick events at 50ms (20 Hz) ===
    if time - event_buffer.last_tick_time >= 0.05 {
        event_buffer.last_tick_time = time;
        event_buffer.tick_frame_count += 1;
        let frame = event_buffer.tick_frame_count;

        // Collect player data
        let mut left_pos = (0.0, 0.0);
        let mut left_vel = (0.0, 0.0);
        let mut right_pos = (0.0, 0.0);
        let mut right_vel = (0.0, 0.0);

        for (_, team, transform, velocity, _, _, _, _) in &players {
            let pos = (transform.translation.x, transform.translation.y);
            let vel = (velocity.0.x, velocity.0.y);
            match team {
                Team::Left => {
                    left_pos = pos;
                    left_vel = vel;
                }
                Team::Right => {
                    right_pos = pos;
                    right_vel = vel;
                }
            }
        }

        // Collect ball data (only one ball in the game)
        let (ball_pos, ball_vel, ball_state_char) = balls
            .iter()
            .next()
            .map(|(transform, velocity, ball_state)| {
                let pos = (transform.translation.x, transform.translation.y);
                let vel = (velocity.0.x, velocity.0.y);
                let state = match ball_state {
                    BallState::Free => 'F',
                    BallState::Held(_) => 'H',
                    BallState::InFlight { .. } => 'I',
                };
                (pos, vel, state)
            })
            .unwrap_or(((0.0, 0.0), (0.0, 0.0), 'F'));

        event_buffer.buffer.log(time, GameEvent::Tick {
            frame,
            left_pos,
            left_vel,
            right_pos,
            right_vel,
            ball_pos,
            ball_vel,
            ball_state: ball_state_char,
        });
    }

    // === Detect score changes (Goal events) ===
    if score.left > event_buffer.prev_score_left {
        event_buffer.buffer.log(time, GameEvent::Goal {
            player: PlayerId::L,
            score_left: score.left,
            score_right: score.right,
        });
        event_buffer.prev_score_left = score.left;
    }
    if score.right > event_buffer.prev_score_right {
        event_buffer.buffer.log(time, GameEvent::Goal {
            player: PlayerId::R,
            score_left: score.left,
            score_right: score.right,
        });
        event_buffer.prev_score_right = score.right;
    }

    // === AI goal change detection ===
    for (_, team, _, _, _, ai_state, _, _) in &players {
        let goal_str = format!("{:?}", ai_state.current_goal);
        let (prev, player_id) = match team {
            Team::Left => (&mut event_buffer.prev_ai_goal_left, PlayerId::L),
            Team::Right => (&mut event_buffer.prev_ai_goal_right, PlayerId::R),
        };

        if prev.as_ref() != Some(&goal_str) {
            *prev = Some(goal_str.clone());
            event_buffer.buffer.log(time, GameEvent::AiGoal {
                player: player_id,
                goal: goal_str,
            });
        }
    }

    // === Steal event detection ===
    // Detect steal attempts when cooldown resets to max (meaning a steal just happened)
    for (_, team, _, _, _, _, steal_cooldown, _) in &players {
        let idx = match team {
            Team::Left => 0,
            Team::Right => 1,
        };
        let player_id = match team {
            Team::Left => PlayerId::L,
            Team::Right => PlayerId::R,
        };

        let current_cooldown = steal_cooldown.0;
        let prev_cooldown = event_buffer.prev_steal_cooldowns[idx];

        // Detect if cooldown just jumped up (steal was attempted)
        // Attacker gets STEAL_COOLDOWN (0.3s) on success, STEAL_FAIL_COOLDOWN (0.5s) on fail
        // Victim gets STEAL_VICTIM_COOLDOWN (1.0s) - we skip these (not an attempt)
        // Detect attacker cooldowns only (< 0.6s to exclude victim's 1.0s)
        let is_attacker_cooldown = current_cooldown > 0.25 && current_cooldown < 0.6;
        let cooldown_just_set = prev_cooldown < 0.1;

        if is_attacker_cooldown && cooldown_just_set {
            event_buffer.buffer.log(time, GameEvent::StealAttempt {
                attacker: player_id,
            });
            // Check StealContest for success/fail (fail_flash_timer > 0 means fail)
            if steal_contest.fail_flash_timer > 0.0 {
                event_buffer.buffer.log(time, GameEvent::StealFail {
                    attacker: player_id,
                });
            } else {
                event_buffer.buffer.log(time, GameEvent::StealSuccess {
                    attacker: player_id,
                });
            }
        }
        event_buffer.prev_steal_cooldowns[idx] = current_cooldown;
    }

    // === Track ball possession changes and shot charging ===
    for (entity, team, transform, _, charging, _, _, holding) in &players {
        let player_id = match team {
            Team::Left => PlayerId::L,
            Team::Right => PlayerId::R,
        };
        let idx = match team {
            Team::Left => 0,
            Team::Right => 1,
        };

        // Track pickup/drop
        let is_holding = holding.is_some();
        let was_holding = event_buffer.prev_ball_holder == Some(entity);

        if is_holding && !was_holding {
            event_buffer.buffer.log(time, GameEvent::Pickup { player: player_id });
            event_buffer.prev_ball_holder = Some(entity);
        }

        // Detect shot charging start - now with actual position
        let is_charging = charging.charge_time > 0.0;
        if is_charging && !event_buffer.prev_charging[idx] {
            event_buffer.buffer.log(time, GameEvent::ShotStart {
                player: player_id,
                pos: (transform.translation.x, transform.translation.y),
                quality: 0.5, // Could calculate based on position
            });
        }
        event_buffer.prev_charging[idx] = is_charging;
    }

    // === Detect when ball becomes free (drop or shot release) ===
    for (_, _, ball_state) in &balls {
        match ball_state {
            BallState::InFlight { shooter, power } => {
                // If ball just became InFlight, log shot release
                if event_buffer.prev_ball_holder.is_some() {
                    let player_id = if Some(*shooter) == event_buffer.prev_ball_holder {
                        players.iter()
                            .find(|(e, _, _, _, _, _, _, _)| *e == *shooter)
                            .map(|(_, team, _, _, _, _, _, _)| match team {
                                Team::Left => PlayerId::L,
                                Team::Right => PlayerId::R,
                            })
                    } else {
                        None
                    };

                    if let Some(pid) = player_id {
                        event_buffer.buffer.log(time, GameEvent::ShotRelease {
                            player: pid,
                            charge: 0.5,
                            angle: 60.0,
                            power: *power,
                        });
                    }
                    event_buffer.prev_ball_holder = None;
                }
            }
            BallState::Free => {
                // If ball just became Free after being Held, it was a drop
                if event_buffer.prev_ball_holder.is_some() {
                    if let Some((_, team, _, _, _, _, _, _)) = players.iter()
                        .find(|(e, _, _, _, _, _, _, _)| Some(*e) == event_buffer.prev_ball_holder)
                    {
                        let player_id = match team {
                            Team::Left => PlayerId::L,
                            Team::Right => PlayerId::R,
                        };
                        event_buffer.buffer.log(time, GameEvent::Drop { player: player_id });
                    }
                    event_buffer.prev_ball_holder = None;
                }
            }
            BallState::Held(_) => {
                // Ball is held - already tracked above
            }
        }
    }
}

/// Write the event buffer to a .evlog file
fn write_event_log(event_buffer: &SimEventBuffer) {
    if !event_buffer.enabled {
        return;
    }

    // Create log directory if needed
    if let Err(e) = fs::create_dir_all(&event_buffer.log_dir) {
        eprintln!("Failed to create log directory: {}", e);
        return;
    }

    // Generate filename from session ID
    let session_id = event_buffer.buffer.session_id();
    if session_id.is_empty() {
        return;
    }

    let filename = format!("{}.evlog", &session_id[..session_id.len().min(36)]);
    let path = event_buffer.log_dir.join(filename);

    // Serialize and write
    let content = event_buffer.buffer.serialize();
    match File::create(&path) {
        Ok(mut file) => {
            if let Err(e) = file.write_all(content.as_bytes()) {
                eprintln!("Failed to write event log: {}", e);
            }
        }
        Err(e) => {
            eprintln!("Failed to create event log file: {}", e);
        }
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

        super::config::SimMode::ShotTest { shots_per_position } => {
            run_shot_test(&config, *shots_per_position, &level_db);
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

// ============================================================================
// Shot Accuracy Test
// ============================================================================

/// Shot outcome classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShotOutcome {
    Goal,
    Overshoot,
    Undershoot,
}

/// Results for a single shooting position
#[derive(Debug, Default)]
struct PositionResult {
    x: f32,
    goals: u32,
    overshoots: u32,
    undershoots: u32,
}

impl PositionResult {
    fn total(&self) -> u32 {
        self.goals + self.overshoots + self.undershoots
    }

    fn over_under_ratio(&self) -> f32 {
        let misses = self.overshoots + self.undershoots;
        if misses == 0 {
            0.5 // No misses, consider balanced
        } else {
            self.overshoots as f32 / misses as f32
        }
    }
}

/// Resource for shot test control
#[derive(Resource)]
struct ShotTestControl {
    phase: ShotTestPhase,
    shots_remaining: u32,
    current_position_idx: usize,
    positions: Vec<f32>, // X positions to test from
    basket_y: f32,
    ball_max_y: f32,     // Track highest point of ball during flight
    shot_in_flight: bool,
    frame_count: u32,
    results: Vec<PositionResult>,
    should_exit: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShotTestPhase {
    Setup,      // Position player, give ball
    Charging,   // Charge shot to full
    InFlight,   // Ball is flying
    Settling,   // Wait for ball to settle
    NextShot,   // Reset for next shot
}

/// Run the shot accuracy test
fn run_shot_test(config: &SimConfig, shots_per_position: u32, level_db: &LevelDatabase) {
    println!("Shot Accuracy Test");
    println!("==================\n");

    // Shooting positions (X coordinates) - from close to far
    // Right basket is at ~740 (wall_inner - basket_push_in)
    // Positions: close (500), medium (300), far (0), very far (-200)
    let positions = vec![500.0, 300.0, 0.0, -200.0];

    // Get basket height from level
    let level_idx = (config.level - 1) as usize;
    let basket_y = level_db
        .get(level_idx)
        .map(|l| ARENA_FLOOR_Y + l.basket_height)
        .unwrap_or(ARENA_FLOOR_Y + 150.0);

    println!(
        "Testing from {} positions, {} shots each",
        positions.len(),
        shots_per_position
    );
    println!("Basket Y: {:.0}", basket_y);
    println!();

    // Run test for each position
    let mut all_results = Vec::new();

    for (pos_idx, &pos_x) in positions.iter().enumerate() {
        print!(
            "Position {} (x={:+.0}): ",
            pos_idx + 1,
            pos_x
        );
        use std::io::Write;
        std::io::stdout().flush().ok();

        let result = run_shot_test_position(
            pos_x,
            basket_y,
            shots_per_position,
            level_db,
            config.level,
        );

        println!(
            "G:{} O:{} U:{} (over/under: {:.0}%)",
            result.goals,
            result.overshoots,
            result.undershoots,
            result.over_under_ratio() * 100.0
        );

        all_results.push(result);
    }

    // Summary
    println!("\n==================");
    println!("Summary:");

    let total_goals: u32 = all_results.iter().map(|r| r.goals).sum();
    let total_over: u32 = all_results.iter().map(|r| r.overshoots).sum();
    let total_under: u32 = all_results.iter().map(|r| r.undershoots).sum();
    let total_shots: u32 = all_results.iter().map(|r| r.total()).sum();
    let total_misses = total_over + total_under;

    let overall_ratio = if total_misses > 0 {
        total_over as f32 / total_misses as f32
    } else {
        0.5
    };

    println!(
        "  Total: {} shots, {} goals ({:.0}%)",
        total_shots,
        total_goals,
        100.0 * total_goals as f32 / total_shots as f32
    );
    println!(
        "  Misses: {} overshoot, {} undershoot",
        total_over, total_under
    );
    println!("  Over/Under ratio: {:.1}%", overall_ratio * 100.0);

    // Pass/Fail based on 40-60% target
    let balanced = overall_ratio >= 0.4 && overall_ratio <= 0.6;
    if balanced {
        println!("\n  Result: PASS (ratio within 40-60%)");
    } else {
        println!("\n  Result: FAIL (ratio outside 40-60%)");
        if overall_ratio > 0.6 {
            println!("  -> Shots are overshooting too often");
        } else {
            println!("  -> Shots are undershooting too often");
        }
    }
}

/// Run shot test for a single position
fn run_shot_test_position(
    pos_x: f32,
    basket_y: f32,
    shots: u32,
    level_db: &LevelDatabase,
    level: u32,
) -> PositionResult {
    let mut result = PositionResult {
        x: pos_x,
        ..Default::default()
    };

    for _ in 0..shots {
        let outcome = run_single_shot(pos_x, basket_y, level_db, level);
        match outcome {
            ShotOutcome::Goal => result.goals += 1,
            ShotOutcome::Overshoot => result.overshoots += 1,
            ShotOutcome::Undershoot => result.undershoots += 1,
        }
    }

    result
}

/// Run a single shot and return the outcome
fn run_single_shot(
    player_x: f32,
    basket_y: f32,
    level_db: &LevelDatabase,
    level: u32,
) -> ShotOutcome {
    // Create minimal Bevy app for single shot
    let mut app = App::new();

    app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
        Duration::from_secs_f32(1.0 / 60.0),
    )));
    app.add_plugins(bevy::transform::TransformPlugin);

    // Resources
    app.insert_resource((*level_db).clone());
    app.init_resource::<Score>();
    app.insert_resource(CurrentLevel(level));
    app.init_resource::<StealContest>();
    app.init_resource::<PhysicsTweaks>();
    app.init_resource::<LastShotInfo>();
    app.insert_resource(CurrentPalette(0));
    app.init_resource::<PaletteDatabase>();

    // Shot test control
    app.insert_resource(ShotTestControl {
        phase: ShotTestPhase::Setup,
        shots_remaining: 1,
        current_position_idx: 0,
        positions: vec![player_x],
        basket_y,
        ball_max_y: f32::MIN,
        shot_in_flight: false,
        frame_count: 0,
        results: vec![PositionResult::default()],
        should_exit: false,
    });

    // Startup
    let player_x_clone = player_x;
    let level_clone = level;
    app.add_systems(Startup, move |commands: Commands, level_db: Res<LevelDatabase>| {
        shot_test_setup(commands, &level_db, player_x_clone, level_clone);
    });

    // Game systems
    app.add_systems(
        FixedUpdate,
        (
            shot_test_control,
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
            update_shot_charge,
            throw_ball,
            check_scoring,
            shot_test_track_ball,
        )
            .chain(),
    );

    // Run until shot completes
    loop {
        app.update();

        let control = app.world().resource::<ShotTestControl>();
        if control.should_exit {
            break;
        }

        // Safety timeout (5 seconds = 300 frames)
        if control.frame_count > 300 {
            break;
        }
    }

    // Determine outcome
    let control = app.world().resource::<ShotTestControl>();
    let score = app.world().resource::<Score>();

    if score.left > 0 {
        ShotOutcome::Goal
    } else if control.ball_max_y > basket_y {
        ShotOutcome::Overshoot
    } else {
        ShotOutcome::Undershoot
    }
}

/// Setup for shot test
fn shot_test_setup(
    mut commands: Commands,
    level_db: &LevelDatabase,
    player_x: f32,
    level: u32,
) {
    let level_idx = (level - 1) as usize;
    let level_def = level_db.get(level_idx);

    // Floor position
    let floor_y = ARENA_FLOOR_Y;
    let player_y = floor_y + 20.0 + PLAYER_SIZE.y / 2.0; // On top of floor

    // Spawn player (left team, shooting at right basket)
    let player_entity = commands
        .spawn((
            Transform::from_translation(Vec3::new(player_x, player_y, 0.0)),
            Sprite {
                custom_size: Some(PLAYER_SIZE),
                ..default()
            },
            Player,
            Velocity::default(),
            Grounded(true),
            CoyoteTimer::default(),
            JumpState::default(),
            Facing(1.0), // Face right
            ChargingShot::default(),
            TargetBasket(Basket::Right),
            Collider,
            Team::Left,
            InputState::default(),
            StealCooldown::default(),
        ))
        .id();

    // Spawn ball held by player
    let ball_entity = commands
        .spawn((
            Transform::from_translation(Vec3::new(player_x, player_y, 0.0)),
            Sprite {
                custom_size: Some(BALL_SIZE),
                ..default()
            },
            Ball,
            BallState::Held(player_entity),
            Velocity::default(),
            BallPlayerContact::default(),
            BallPulse::default(),
            BallRolling::default(),
            BallShotGrace::default(),
            BallSpin::default(),
            BallStyle::new("wedges"),
        ))
        .id();

    commands.entity(player_entity).insert(HoldingBall(ball_entity));

    // Spawn floor
    commands.spawn((
        Sprite {
            custom_size: Some(Vec2::new(ARENA_WIDTH - WALL_THICKNESS * 2.0, 40.0)),
            ..default()
        },
        Transform::from_xyz(0.0, floor_y, 0.0),
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

    // Spawn baskets
    if let Some(level_def) = level_def {
        let basket_y = floor_y + level_def.basket_height;
        let wall_inner = ARENA_WIDTH / 2.0 - WALL_THICKNESS;
        let right_basket_x = wall_inner - level_def.basket_push_in;

        // Only need the right basket for this test
        commands.spawn((
            Sprite {
                custom_size: Some(BASKET_SIZE),
                ..default()
            },
            Transform::from_xyz(right_basket_x, basket_y, 0.0),
            Basket::Right,
        ));

        // Left basket (for completeness)
        let left_basket_x = -wall_inner + level_def.basket_push_in;
        commands.spawn((
            Sprite {
                custom_size: Some(BASKET_SIZE),
                ..default()
            },
            Transform::from_xyz(left_basket_x, basket_y, 0.0),
            Basket::Left,
        ));
    }
}

/// Control system for shot test - manages charging and releasing
fn shot_test_control(
    mut control: ResMut<ShotTestControl>,
    mut players: Query<(&mut InputState, &ChargingShot), With<Player>>,
    balls: Query<&BallState, With<Ball>>,
) {
    control.frame_count += 1;

    for (mut input, charging) in &mut players {
        match control.phase {
            ShotTestPhase::Setup => {
                // Start charging immediately
                input.throw_held = true;
                control.phase = ShotTestPhase::Charging;
            }
            ShotTestPhase::Charging => {
                // Keep charging until full (SHOT_CHARGE_TIME = 1.6s = ~96 frames)
                input.throw_held = true;
                if charging.charge_time >= SHOT_CHARGE_TIME {
                    // Release!
                    input.throw_held = false;
                    input.throw_released = true;
                    control.phase = ShotTestPhase::InFlight;
                    control.ball_max_y = f32::MIN;
                }
            }
            ShotTestPhase::InFlight => {
                input.throw_held = false;
                input.throw_released = false;

                // Check if ball is no longer in flight
                for ball_state in &balls {
                    if !matches!(ball_state, BallState::InFlight { .. }) {
                        control.phase = ShotTestPhase::Settling;
                    }
                }
            }
            ShotTestPhase::Settling => {
                // Wait a few frames for ball to settle / score to register
                if control.frame_count > 10 {
                    control.should_exit = true;
                }
            }
            ShotTestPhase::NextShot => {
                // Not used in single-shot mode
                control.should_exit = true;
            }
        }
    }
}

/// Track ball's maximum height during flight
fn shot_test_track_ball(
    mut control: ResMut<ShotTestControl>,
    balls: Query<(&Transform, &BallState), With<Ball>>,
) {
    if control.phase == ShotTestPhase::InFlight {
        for (transform, ball_state) in &balls {
            if matches!(ball_state, BallState::InFlight { .. }) {
                let y = transform.translation.y;
                if y > control.ball_max_y {
                    control.ball_max_y = y;
                }
            }
        }
    }
}
