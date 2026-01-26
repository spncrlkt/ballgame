//! Replay playback systems

use bevy::prelude::*;

use crate::ball::{Ball, BallState, BallStyle, Velocity};
use crate::constants::*;
use crate::levels::LevelDatabase;
use crate::player::{Facing, Player, Team};
use crate::scoring::CurrentLevel;
use crate::world::{Basket, Collider, Platform};

use super::ReplayData;
use super::state::ReplayState;

/// Hermite interpolation for smooth curves using position + velocity
fn hermite_interp(p0: Vec2, v0: Vec2, p1: Vec2, v1: Vec2, t: f32, dt_secs: f32) -> Vec2 {
    let t2 = t * t;
    let t3 = t2 * t;

    // Scale velocities by time delta (50ms = 0.05s)
    let v0_scaled = v0 * dt_secs;
    let v1_scaled = v1 * dt_secs;

    let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
    let h10 = t3 - 2.0 * t2 + t;
    let h01 = -2.0 * t3 + 3.0 * t2;
    let h11 = t3 - t2;

    p0 * h00 + v0_scaled * h10 + p1 * h01 + v1_scaled * h11
}

/// Setup system for replay mode - spawns minimal entities needed
pub fn replay_setup(
    mut commands: Commands,
    replay_data: Res<ReplayData>,
    level_db: Res<LevelDatabase>,
    mut current_level: ResMut<CurrentLevel>,
) {
    info!(
        "Setting up replay: level {}, profiles {} vs {}",
        replay_data.match_info.level,
        replay_data.match_info.left_profile,
        replay_data.match_info.right_profile
    );

    // Set current level from replay
    current_level.0 = replay_data.match_info.level;

    // Get initial positions from first tick (or use defaults)
    let (left_pos, right_pos, ball_pos) = if let Some(first) = replay_data.ticks.first() {
        (first.left_pos, first.right_pos, first.ball_pos)
    } else {
        (
            Vec2::new(PLAYER_SPAWN_LEFT.x, PLAYER_SPAWN_LEFT.y),
            Vec2::new(PLAYER_SPAWN_RIGHT.x, PLAYER_SPAWN_RIGHT.y),
            Vec2::new(BALL_SPAWN.x, BALL_SPAWN.y),
        )
    };

    // Spawn left player (simplified - just sprite)
    commands.spawn((
        Sprite {
            color: Color::srgb(0.2, 0.6, 0.9), // Blue-ish
            custom_size: Some(PLAYER_SIZE),
            ..default()
        },
        Transform::from_xyz(left_pos.x, left_pos.y, 1.0),
        Player,
        Team::Left,
        Facing::default(),
        Velocity::default(),
    ));

    // Spawn right player
    commands.spawn((
        Sprite {
            color: Color::srgb(0.9, 0.3, 0.2), // Red-ish
            custom_size: Some(PLAYER_SIZE),
            ..default()
        },
        Transform::from_xyz(right_pos.x, right_pos.y, 1.0),
        Player,
        Team::Right,
        Facing(-1.0),
        Velocity::default(),
    ));

    // Spawn ball
    commands.spawn((
        Sprite {
            color: Color::WHITE,
            custom_size: Some(BALL_SIZE),
            ..default()
        },
        Transform::from_xyz(ball_pos.x, ball_pos.y, 2.0),
        Ball,
        BallState::default(),
        BallStyle::new("wedges"),
        Velocity::default(),
    ));

    // Spawn arena floor
    commands.spawn((
        Sprite {
            color: Color::srgb(0.3, 0.3, 0.3),
            custom_size: Some(Vec2::new(ARENA_WIDTH - WALL_THICKNESS * 2.0, 40.0)),
            ..default()
        },
        Transform::from_xyz(0.0, ARENA_FLOOR_Y, 0.0),
        Platform,
    ));

    // Spawn walls
    commands.spawn((
        Sprite {
            color: Color::srgb(0.3, 0.3, 0.3),
            custom_size: Some(Vec2::new(WALL_THICKNESS, 5000.0)),
            ..default()
        },
        Transform::from_xyz(-ARENA_WIDTH / 2.0 + WALL_THICKNESS / 2.0, 2000.0, 0.0),
        Platform,
    ));
    commands.spawn((
        Sprite {
            color: Color::srgb(0.3, 0.3, 0.3),
            custom_size: Some(Vec2::new(WALL_THICKNESS, 5000.0)),
            ..default()
        },
        Transform::from_xyz(ARENA_WIDTH / 2.0 - WALL_THICKNESS / 2.0, 2000.0, 0.0),
        Platform,
    ));

    // Spawn level platforms from level database
    let level_idx = (replay_data.match_info.level as usize).saturating_sub(1);
    if let Some(level) = level_db.get(level_idx) {
        // Spawn platforms
        for platform in &level.platforms {
            match platform {
                crate::levels::PlatformDef::Mirror { x, y, width } => {
                    // Left
                    commands.spawn((
                        Sprite {
                            color: Color::srgb(0.3, 0.3, 0.3),
                            custom_size: Some(Vec2::new(*width, 20.0)),
                            ..default()
                        },
                        Transform::from_xyz(-x, ARENA_FLOOR_Y + y, 0.0),
                        Platform,
                        Collider,
                    ));
                    // Right
                    commands.spawn((
                        Sprite {
                            color: Color::srgb(0.3, 0.3, 0.3),
                            custom_size: Some(Vec2::new(*width, 20.0)),
                            ..default()
                        },
                        Transform::from_xyz(*x, ARENA_FLOOR_Y + y, 0.0),
                        Platform,
                        Collider,
                    ));
                }
                crate::levels::PlatformDef::Center { y, width } => {
                    commands.spawn((
                        Sprite {
                            color: Color::srgb(0.3, 0.3, 0.3),
                            custom_size: Some(Vec2::new(*width, 20.0)),
                            ..default()
                        },
                        Transform::from_xyz(0.0, ARENA_FLOOR_Y + y, 0.0),
                        Platform,
                        Collider,
                    ));
                }
            }
        }

        // Spawn baskets
        let basket_y = ARENA_FLOOR_Y + level.basket_height;
        let wall_inner = ARENA_WIDTH / 2.0 - WALL_THICKNESS;
        let left_basket_x = -wall_inner + level.basket_push_in;
        let right_basket_x = wall_inner - level.basket_push_in;

        commands.spawn((
            Sprite {
                color: Color::srgb(0.2, 0.6, 0.9),
                custom_size: Some(BASKET_SIZE),
                ..default()
            },
            Transform::from_xyz(left_basket_x, basket_y, 0.0),
            Basket::Left,
        ));
        commands.spawn((
            Sprite {
                color: Color::srgb(0.9, 0.3, 0.2),
                custom_size: Some(BASKET_SIZE),
                ..default()
            },
            Transform::from_xyz(right_basket_x, basket_y, 0.0),
            Basket::Right,
        ));
    }
}

/// Main playback system - advances time and interpolates positions
pub fn replay_playback(
    time: Res<Time>,
    replay_data: Res<ReplayData>,
    mut state: ResMut<ReplayState>,
    mut players: Query<(&mut Transform, &Team), With<Player>>,
    mut ball: Query<(&mut Transform, &mut BallState), (With<Ball>, Without<Player>)>,
) {
    // Don't advance if paused (unless stepping)
    if state.is_paused && !state.is_stepping {
        return;
    }
    state.is_stepping = false;

    // Advance time
    if !state.is_paused {
        let delta_ms = (time.delta_secs() * 1000.0 * state.playback_speed) as u32;
        state.current_time_ms = state.current_time_ms.saturating_add(delta_ms);

        // Check if we've reached the end
        if state.current_time_ms >= replay_data.duration_ms {
            state.current_time_ms = replay_data.duration_ms;
            state.finished = true;
            state.is_paused = true;
        }
    }

    // Find bracket for interpolation
    let Some((prev, next, t)) = replay_data.find_bracket(state.current_time_ms) else {
        return;
    };

    // Tick interval in seconds for Hermite interpolation
    let dt_secs = 0.05; // 50ms

    // Interpolate player positions
    let left_pos = hermite_interp(prev.left_pos, prev.left_vel, next.left_pos, next.left_vel, t, dt_secs);
    let right_pos = hermite_interp(prev.right_pos, prev.right_vel, next.right_pos, next.right_vel, t, dt_secs);

    for (mut transform, team) in &mut players {
        match team {
            Team::Left => {
                transform.translation.x = left_pos.x;
                transform.translation.y = left_pos.y;
            }
            Team::Right => {
                transform.translation.x = right_pos.x;
                transform.translation.y = right_pos.y;
            }
        }
    }

    // Interpolate ball position
    let ball_pos = hermite_interp(prev.ball_pos, prev.ball_vel, next.ball_pos, next.ball_vel, t, dt_secs);

    for (mut transform, mut ball_state) in &mut ball {
        transform.translation.x = ball_pos.x;
        transform.translation.y = ball_pos.y;

        // Update ball state from current frame
        let current_state = if t < 0.5 { prev.ball_state } else { next.ball_state };
        *ball_state = match current_state {
            'H' => BallState::Held(Entity::PLACEHOLDER), // Simplified
            'I' => BallState::InFlight {
                shooter: Entity::PLACEHOLDER,
                power: 500.0,
            },
            _ => BallState::Free,
        };
    }
}

/// Input handler for replay controls
pub fn replay_input_handler(
    keyboard: Res<ButtonInput<KeyCode>>,
    replay_data: Res<ReplayData>,
    mut state: ResMut<ReplayState>,
) {
    // Space: Toggle pause
    if keyboard.just_pressed(KeyCode::Space) {
        state.toggle_pause();
    }

    // Left/Right arrows: Adjust speed
    if keyboard.just_pressed(KeyCode::ArrowRight) {
        state.speed_up();
    }
    if keyboard.just_pressed(KeyCode::ArrowLeft) {
        state.speed_down();
    }

    // Period (.): Step forward one tick (when paused)
    if keyboard.just_pressed(KeyCode::Period) {
        state.step_forward();
    }

    // Comma (,): Step backward one tick (when paused)
    if keyboard.just_pressed(KeyCode::Comma) {
        state.step_backward();
    }

    // Home: Jump to start
    if keyboard.just_pressed(KeyCode::Home) {
        state.jump_to_start();
    }

    // End: Jump to end
    if keyboard.just_pressed(KeyCode::End) {
        state.jump_to_end(replay_data.duration_ms);
    }

    // Escape: Could be used to exit replay mode
    // (handled elsewhere if needed)
}
