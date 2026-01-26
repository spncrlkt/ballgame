//! Simulation setup systems
//!
//! Contains entity spawning for headless simulation.

use bevy::prelude::*;

use crate::ai::{AiGoal, AiNavState, AiProfileDatabase, AiState, InputState};
use crate::ball::{
    Ball, BallPlayerContact, BallPulse, BallRolling, BallShotGrace, BallSpin, BallState, BallStyle,
    Velocity,
};
use crate::constants::*;
use crate::levels::LevelDatabase;
use crate::player::{CoyoteTimer, Facing, Grounded, JumpState, Player, TargetBasket, Team};
use crate::scoring::CurrentLevel;
use crate::shooting::ChargingShot;
use crate::steal::StealCooldown;
use crate::world::{Basket, Collider, CornerRamp, Platform};

use super::control::SimControl;

/// Setup system for simulation
pub fn sim_setup(
    mut commands: Commands,
    level_db: Res<LevelDatabase>,
    profile_db: Res<AiProfileDatabase>,
    control: Res<SimControl>,
    current_level: Res<CurrentLevel>,
) {
    let config = &control.config;

    // Find profile IDs
    let left_profile_id = profile_db
        .get_by_name(&config.left_profile)
        .map(|p| p.id.clone())
        .unwrap_or_else(|| profile_db.default_profile().id.clone());
    let right_profile_id = profile_db
        .get_by_name(&config.right_profile)
        .map(|p| p.id.clone())
        .unwrap_or_else(|| profile_db.default_profile().id.clone());

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
                profile_id: left_profile_id,
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
                profile_id: right_profile_id,
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
    if let Some(level) = level_db.get_by_id(&current_level.0) {
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

/// Spawn corner steps for simulation (matching the main game's behavior)
pub fn spawn_corner_steps(
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
