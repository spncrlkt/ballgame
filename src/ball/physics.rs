//! Ball physics systems

use bevy::prelude::*;
use rand::Rng;

use crate::ball::components::*;
use crate::constants::*;
use crate::player::Velocity;
use crate::ui::PhysicsTweaks;
use crate::world::{BasketRim, CornerRamp, Platform};

/// Apply velocity to all entities with Velocity component
pub fn apply_velocity(mut query: Query<(&mut Transform, &Velocity)>, time: Res<Time>) {
    for (mut transform, velocity) in &mut query {
        transform.translation.x += velocity.0.x * time.delta_secs();
        transform.translation.y += velocity.0.y * time.delta_secs();
    }
}

/// Apply gravity and friction to ball
pub fn ball_gravity(
    tweaks: Res<PhysicsTweaks>,
    mut query: Query<(&mut Velocity, &BallState, &BallRolling, &mut BallShotGrace), With<Ball>>,
    time: Res<Time>,
) {
    for (mut velocity, state, rolling, mut grace) in &mut query {
        // Decrement grace timer
        if grace.0 > 0.0 {
            grace.0 = (grace.0 - time.delta_secs()).max(0.0);
        }

        match state {
            BallState::Free | BallState::InFlight { .. } => {
                if rolling.0 {
                    // Rolling on ground - no gravity, apply rolling friction (skip if grace active)
                    velocity.0.y = 0.0;
                    if grace.0 <= 0.0 {
                        velocity.0.x *= tweaks.ball_roll_friction.powf(time.delta_secs());
                    }
                } else {
                    // In air - apply gravity, apply air friction only if no grace
                    velocity.0.y -= tweaks.ball_gravity * time.delta_secs();
                    if grace.0 <= 0.0 {
                        velocity.0.x *= tweaks.ball_air_friction.powf(time.delta_secs());
                    }
                }
            }
            BallState::Held(_) => {
                // Ball follows player, no gravity
                velocity.0 = Vec2::ZERO;
            }
        }
    }
}

/// Handle ball collisions with platforms
pub fn ball_collisions(
    tweaks: Res<PhysicsTweaks>,
    mut ball_query: Query<
        (
            &mut Transform,
            &mut Velocity,
            &BallState,
            &Sprite,
            &mut BallRolling,
        ),
        With<Ball>,
    >,
    platform_query: Query<
        (&GlobalTransform, &Sprite, Option<&BasketRim>, Option<&CornerRamp>),
        (With<Platform>, Without<Ball>),
    >,
) {
    let mut rng = rand::thread_rng();

    for (mut ball_transform, mut ball_velocity, state, ball_sprite, mut rolling) in &mut ball_query
    {
        // Skip collision for held balls
        if matches!(state, BallState::Held(_)) {
            continue;
        }

        // Check if ball is in flight (thrown) - rims only collide with thrown/free balls
        let is_thrown_or_free = matches!(state, BallState::Free | BallState::InFlight { .. });

        let ball_size = ball_sprite.custom_size.unwrap_or(BALL_SIZE);
        let ball_half = ball_size / 2.0;

        // Track if ball has ground contact this frame (for rolling detection)
        let was_rolling = rolling.0;
        let mut has_ground_contact = false;

        for (platform_global, platform_sprite, maybe_rim, maybe_step) in &platform_query {
            // Skip rim collisions for non-thrown balls (though held is already filtered above)
            if maybe_rim.is_some() && !is_thrown_or_free {
                continue;
            }

            let platform_size = platform_sprite
                .custom_size
                .unwrap_or(Vec2::new(100.0, 20.0));
            let platform_half = platform_size / 2.0;

            let ball_pos = ball_transform.translation.truncate();
            let platform_pos = platform_global.translation().truncate();

            let diff = ball_pos - platform_pos;
            let overlap_x = ball_half.x + platform_half.x - diff.x.abs();
            let overlap_y = ball_half.y + platform_half.y - diff.y.abs();

            if overlap_x <= 0.0 || overlap_y <= 0.0 {
                continue;
            }

            let is_step = maybe_step.is_some();

            // Resolve collision with bounce
            if overlap_y < overlap_x {
                // Vertical collision
                if diff.y > 0.0 {
                    // Ball above platform (landed on floor)
                    has_ground_contact = true;
                    // Position slightly into platform so collision is detected next frame
                    ball_transform.translation.y =
                        platform_pos.y + platform_half.y + ball_half.y - COLLISION_EPSILON;
                    if ball_velocity.0.y < 0.0 {
                        if is_step {
                            // Erratic step bounce - keep more velocity, add random deflection
                            let speed = ball_velocity.0.length();
                            let deflect_angle = rng.gen_range(-STEP_DEFLECT_ANGLE_MAX..STEP_DEFLECT_ANGLE_MAX);
                            let deflect_rad = deflect_angle.to_radians();

                            // Reflect Y, then rotate by random angle
                            let reflected = Vec2::new(ball_velocity.0.x, -ball_velocity.0.y);
                            let cos_a = deflect_rad.cos();
                            let sin_a = deflect_rad.sin();
                            let rotated = Vec2::new(
                                reflected.x * cos_a - reflected.y * sin_a,
                                reflected.x * sin_a + reflected.y * cos_a,
                            );
                            ball_velocity.0 = rotated.normalize() * speed * STEP_BOUNCE_RETENTION;
                            rolling.0 = false;
                        } else {
                            // Normal floor bounce
                            ball_velocity.0.x *= BALL_GROUND_FRICTION;

                            let post_bounce_vel = ball_velocity.0.y.abs() * tweaks.ball_bounce;
                            let max_bounce_height =
                                (post_bounce_vel * post_bounce_vel) / (2.0 * tweaks.ball_gravity);

                            if max_bounce_height > ball_size.y * BALL_BOUNCE_HEIGHT_MULT {
                                ball_velocity.0.y = -ball_velocity.0.y * tweaks.ball_bounce;
                                rolling.0 = false;
                            } else {
                                ball_velocity.0.y = 0.0;
                                rolling.0 = true;
                            }
                        }
                    }
                } else {
                    // Ball below platform (hit ceiling)
                    ball_transform.translation.y = platform_pos.y - platform_half.y - ball_half.y;
                    if ball_velocity.0.y > 0.0 {
                        if is_step {
                            // Erratic step bounce from below
                            let speed = ball_velocity.0.length();
                            let deflect_angle = rng.gen_range(-STEP_DEFLECT_ANGLE_MAX..STEP_DEFLECT_ANGLE_MAX);
                            let deflect_rad = deflect_angle.to_radians();
                            let reflected = Vec2::new(ball_velocity.0.x, -ball_velocity.0.y);
                            let cos_a = deflect_rad.cos();
                            let sin_a = deflect_rad.sin();
                            let rotated = Vec2::new(
                                reflected.x * cos_a - reflected.y * sin_a,
                                reflected.x * sin_a + reflected.y * cos_a,
                            );
                            ball_velocity.0 = rotated.normalize() * speed * STEP_BOUNCE_RETENTION;
                        } else {
                            ball_velocity.0.y = -ball_velocity.0.y * tweaks.ball_bounce;
                        }
                    }
                }
            } else {
                // Horizontal collision - bounce off walls/step sides
                if diff.x > 0.0 {
                    ball_transform.translation.x = platform_pos.x + platform_half.x + ball_half.x;
                } else {
                    ball_transform.translation.x = platform_pos.x - platform_half.x - ball_half.x;
                }
                if is_step {
                    // Erratic step side bounce
                    let speed = ball_velocity.0.length();
                    let deflect_angle = rng.gen_range(-STEP_DEFLECT_ANGLE_MAX..STEP_DEFLECT_ANGLE_MAX);
                    let deflect_rad = deflect_angle.to_radians();
                    let reflected = Vec2::new(-ball_velocity.0.x, ball_velocity.0.y);
                    let cos_a = deflect_rad.cos();
                    let sin_a = deflect_rad.sin();
                    let rotated = Vec2::new(
                        reflected.x * cos_a - reflected.y * sin_a,
                        reflected.x * sin_a + reflected.y * cos_a,
                    );
                    ball_velocity.0 = rotated.normalize() * speed * STEP_BOUNCE_RETENTION;
                } else {
                    ball_velocity.0.x = -ball_velocity.0.x * tweaks.ball_bounce;
                }
            }
        }

        // If ball was rolling but lost ground contact, start falling
        if was_rolling && !has_ground_contact {
            rolling.0 = false;
        }
    }
}

/// Update ball state (InFlight -> Free when slow)
pub fn ball_state_update(mut ball_query: Query<(&Velocity, &mut BallState), With<Ball>>) {
    for (velocity, mut state) in &mut ball_query {
        // InFlight balls become Free when they slow down enough
        if matches!(*state, BallState::InFlight { .. }) {
            let speed = velocity.0.length();
            if speed < BALL_FREE_SPEED {
                *state = BallState::Free;
            }
        }
    }
}
