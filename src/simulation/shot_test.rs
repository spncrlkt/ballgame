//! Shot accuracy testing
//!
//! Tests shot accuracy from various positions, classifying outcomes
//! as goal, overshoot, or undershoot.
//!
//! Refactored to reuse a single Bevy app per position, resetting
//! entity state between shots instead of creating new apps.

use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use std::time::Duration;

use crate::ball::{
    Ball, BallPlayerContact, BallPulse, BallRolling, BallShotGrace, BallSpin, BallState, BallStyle,
    CurrentPalette, Velocity, apply_velocity, ball_collisions, ball_gravity, ball_player_collision,
    ball_spin, ball_state_update,
};
use crate::constants::*;
use crate::levels::LevelDatabase;
use crate::palettes::PaletteDatabase;
use crate::player::{
    CoyoteTimer, Facing, Grounded, HoldingBall, JumpState, Player, TargetBasket, Team,
    apply_gravity, apply_input, check_collisions,
};
use crate::scoring::{CurrentLevel, Score, check_scoring};
use crate::shooting::{ChargingShot, LastShotInfo, throw_ball, update_shot_charge};
use crate::steal::{StealContest, StealCooldown};
use crate::ui::PhysicsTweaks;
use crate::world::{Basket, Collider, Platform};
use crate::ai::InputState;

use super::config::SimConfig;

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

/// Resource for shot test control - manages multiple shots per app
#[derive(Resource)]
struct ShotTestControl {
    phase: ShotTestPhase,
    shots_remaining: u32,
    player_x: f32,
    basket_y: f32,
    ball_max_y: f32,
    frame_count: u32,
    settle_start_frame: u32,
    // Accumulated results
    goals: u32,
    overshoots: u32,
    undershoots: u32,
    // Exit flag
    all_done: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShotTestPhase {
    Setup,
    Charging,
    InFlight,
    Settling,
    NextShot,
}

/// Run the shot accuracy test
pub fn run_shot_test(config: &SimConfig, shots_per_position: u32, level_db: &LevelDatabase) {
    println!("Shot Accuracy Test");
    println!("==================\n");

    // Shooting positions (X coordinates) - from close to far
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

    // Run test for each position - one app per position (not per shot!)
    let mut all_results = Vec::new();

    for (pos_idx, &pos_x) in positions.iter().enumerate() {
        print!(
            "Position {} (x={:+.0}): ",
            pos_idx + 1,
            pos_x
        );
        use std::io::Write;
        std::io::stdout().flush().ok();

        let result = run_shots_at_position(
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

/// Run all shots at a single position using ONE app instance
fn run_shots_at_position(
    pos_x: f32,
    basket_y: f32,
    shots: u32,
    level_db: &LevelDatabase,
    level: u32,
) -> PositionResult {
    // Create ONE app for all shots at this position
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

    // Shot test control - runs ALL shots for this position
    app.insert_resource(ShotTestControl {
        phase: ShotTestPhase::Setup,
        shots_remaining: shots,
        player_x: pos_x,
        basket_y,
        ball_max_y: f32::MIN,
        frame_count: 0,
        settle_start_frame: 0,
        goals: 0,
        overshoots: 0,
        undershoots: 0,
        all_done: false,
    });

    // Startup - spawn arena and initial entities
    let player_x_clone = pos_x;
    let level_clone = level;
    app.add_systems(Startup, move |commands: Commands, level_db: Res<LevelDatabase>| {
        shot_test_setup(commands, &level_db, player_x_clone, level_clone);
    });

    // Game systems
    app.add_systems(
        FixedUpdate,
        (
            shot_test_control_system,
            apply_input,
            apply_gravity,
            ball_gravity,
            ball_spin,
            apply_velocity,
            check_collisions,
            ball_collisions,
            ball_state_update,
            ball_player_collision,
            crate::ball::ball_follow_holder,
            crate::ball::pickup_ball,
            update_shot_charge,
            throw_ball,
            check_scoring,
            shot_test_track_ball,
            shot_test_reset_system,
        )
            .chain(),
    );

    // Run until all shots complete
    loop {
        app.update();

        let control = app.world().resource::<ShotTestControl>();
        if control.all_done {
            break;
        }

        // Safety timeout (30 seconds = 1800 frames for all shots)
        if control.frame_count > 1800 {
            break;
        }
    }

    // Extract accumulated results
    let control = app.world().resource::<ShotTestControl>();
    PositionResult {
        goals: control.goals,
        overshoots: control.overshoots,
        undershoots: control.undershoots,
    }
}

/// Setup for shot test - spawns arena and initial player/ball
fn shot_test_setup(
    mut commands: Commands,
    level_db: &LevelDatabase,
    player_x: f32,
    level: u32,
) {
    let level_idx = (level - 1) as usize;
    let level_def = level_db.get(level_idx);

    let floor_y = ARENA_FLOOR_Y;
    let player_y = floor_y + 20.0 + PLAYER_SIZE.y / 2.0;

    // Spawn player
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
            Facing(1.0),
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
        let left_basket_x = -wall_inner + level_def.basket_push_in;

        commands.spawn((
            Sprite {
                custom_size: Some(BASKET_SIZE),
                ..default()
            },
            Transform::from_xyz(right_basket_x, basket_y, 0.0),
            Basket::Right,
        ));
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

/// Control system - manages charging and releasing shots
fn shot_test_control_system(
    mut control: ResMut<ShotTestControl>,
    mut players: Query<(&mut InputState, &ChargingShot), With<Player>>,
    balls: Query<&BallState, With<Ball>>,
) {
    control.frame_count += 1;

    for (mut input, charging) in &mut players {
        match control.phase {
            ShotTestPhase::Setup => {
                input.throw_held = true;
                control.phase = ShotTestPhase::Charging;
            }
            ShotTestPhase::Charging => {
                input.throw_held = true;
                if charging.charge_time >= SHOT_CHARGE_TIME {
                    input.throw_held = false;
                    input.throw_released = true;
                    control.phase = ShotTestPhase::InFlight;
                    control.ball_max_y = f32::MIN;
                }
            }
            ShotTestPhase::InFlight => {
                input.throw_held = false;
                input.throw_released = false;

                for ball_state in &balls {
                    if !matches!(ball_state, BallState::InFlight { .. }) {
                        control.phase = ShotTestPhase::Settling;
                        control.settle_start_frame = control.frame_count;
                    }
                }
            }
            ShotTestPhase::Settling => {
                // Wait 10 frames for score to register
                if control.frame_count - control.settle_start_frame > 10 {
                    control.phase = ShotTestPhase::NextShot;
                }
            }
            ShotTestPhase::NextShot => {
                // Handled by reset system
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

/// Reset system - records result and resets for next shot
fn shot_test_reset_system(
    mut control: ResMut<ShotTestControl>,
    mut score: ResMut<Score>,
    mut players: Query<(
        Entity,
        &mut Transform,
        &mut Velocity,
        &mut ChargingShot,
        &mut InputState,
        Option<&HoldingBall>,
    ), (With<Player>, Without<Ball>)>,
    mut balls: Query<(
        Entity,
        &mut Transform,
        &mut Velocity,
        &mut BallState,
        &mut BallSpin,
        &mut BallRolling,
        &mut BallShotGrace,
    ), (With<Ball>, Without<Player>)>,
    mut commands: Commands,
) {
    if control.phase != ShotTestPhase::NextShot {
        return;
    }

    // Record result from this shot
    if score.left > 0 {
        control.goals += 1;
    } else if control.ball_max_y > control.basket_y {
        control.overshoots += 1;
    } else {
        control.undershoots += 1;
    }

    control.shots_remaining -= 1;

    if control.shots_remaining == 0 {
        control.all_done = true;
        return;
    }

    // Reset for next shot
    let floor_y = ARENA_FLOOR_Y;
    let player_y = floor_y + 20.0 + PLAYER_SIZE.y / 2.0;
    let player_x = control.player_x;

    // Reset score
    score.left = 0;
    score.right = 0;

    // Reset player
    for (player_entity, mut transform, mut velocity, mut charging, mut input, holding) in &mut players {
        transform.translation = Vec3::new(player_x, player_y, 0.0);
        velocity.0 = Vec2::ZERO;
        charging.charge_time = 0.0;
        input.throw_held = false;
        input.throw_released = false;

        // Remove HoldingBall if present (will re-add below)
        if holding.is_some() {
            commands.entity(player_entity).remove::<HoldingBall>();
        }
    }

    // Reset ball
    for (ball_entity, mut transform, mut velocity, mut ball_state, mut spin, mut rolling, mut grace) in &mut balls {
        transform.translation = Vec3::new(player_x, player_y, 0.0);
        velocity.0 = Vec2::ZERO;
        spin.0 = 0.0;
        rolling.0 = false;
        grace.0 = 0.0;

        // Find player entity and set ball as held
        for (player_entity, ..) in &players {
            *ball_state = BallState::Held(player_entity);
            commands.entity(player_entity).insert(HoldingBall(ball_entity));
            break;
        }
    }

    // Reset control state
    control.phase = ShotTestPhase::Setup;
    control.ball_max_y = f32::MIN;
}
