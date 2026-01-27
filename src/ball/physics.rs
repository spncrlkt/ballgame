//! Ball physics systems

use bevy::prelude::*;

use crate::ball::components::*;
use crate::constants::*;
use crate::helpers::{ReflectAxis, apply_bounce_deflection};
use crate::player::Velocity;
use crate::tuning::PhysicsTweaks;
use crate::world::{BasketRim, CornerRamp, Platform};

/// Apply velocity to all entities with Velocity component
pub fn apply_velocity(mut query: Query<(&mut Transform, &Velocity)>, time: Res<Time>) {
    // Use minimum dt for headless mode compatibility
    let dt = time.delta_secs().max(1.0 / 60.0);

    for (mut transform, velocity) in &mut query {
        transform.translation.x += velocity.0.x * dt;
        transform.translation.y += velocity.0.y * dt;
    }
}

/// Apply gravity and friction to ball
pub fn ball_gravity(
    tweaks: Res<PhysicsTweaks>,
    mut query: Query<(&mut Velocity, &BallState, &BallRolling, &mut BallShotGrace), With<Ball>>,
    time: Res<Time>,
) {
    // Use minimum dt for headless mode compatibility
    let dt = time.delta_secs().max(1.0 / 60.0);

    for (mut velocity, state, rolling, mut grace) in &mut query {
        // Decrement grace timer
        if grace.0 > 0.0 {
            grace.0 = (grace.0 - dt).max(0.0);
        }

        match state {
            BallState::Free | BallState::InFlight { .. } => {
                if rolling.0 {
                    // Rolling on ground - no gravity, apply rolling friction (skip if grace active)
                    velocity.0.y = 0.0;
                    if grace.0 <= 0.0 {
                        velocity.0.x *= tweaks.ball_roll_friction.powf(dt);
                    }
                } else {
                    // In air - apply gravity, apply air friction only if no grace
                    velocity.0.y -= tweaks.ball_gravity * dt;
                    if grace.0 <= 0.0 {
                        velocity.0.x *= tweaks.ball_air_friction.powf(dt);
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
        (
            &GlobalTransform,
            &Sprite,
            Option<&BasketRim>,
            Option<&CornerRamp>,
        ),
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

        for (platform_global_transform, platform_sprite, maybe_rim, maybe_step) in &platform_query {
            // Skip rim collisions for non-thrown balls (though held is already filtered above)
            if maybe_rim.is_some() && !is_thrown_or_free {
                continue;
            }

            let platform_size = platform_sprite
                .custom_size
                .unwrap_or(Vec2::new(100.0, 20.0));
            let platform_half = platform_size / 2.0;

            let ball_pos = ball_transform.translation.truncate();
            let platform_pos = platform_global_transform.translation().truncate();

            let diff = ball_pos - platform_pos;
            let overlap_x = ball_half.x + platform_half.x - diff.x.abs();
            let overlap_y = ball_half.y + platform_half.y - diff.y.abs();

            if overlap_x <= 0.0 || overlap_y <= 0.0 {
                continue;
            }

            let is_step = maybe_step.is_some();
            let is_rim = maybe_rim.is_some();

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
                            apply_bounce_deflection(
                                &mut ball_velocity.0,
                                ReflectAxis::Horizontal,
                                STEP_DEFLECT_ANGLE_MAX,
                                STEP_BOUNCE_RETENTION,
                                &mut rng,
                            );
                            rolling.0 = false;
                        } else if is_rim {
                            // Rim bounce - snappy but less chaotic than steps
                            apply_bounce_deflection(
                                &mut ball_velocity.0,
                                ReflectAxis::Horizontal,
                                RIM_DEFLECT_ANGLE_MAX,
                                RIM_BOUNCE_RETENTION,
                                &mut rng,
                            );
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
                            apply_bounce_deflection(
                                &mut ball_velocity.0,
                                ReflectAxis::Horizontal,
                                STEP_DEFLECT_ANGLE_MAX,
                                STEP_BOUNCE_RETENTION,
                                &mut rng,
                            );
                        } else if is_rim {
                            // Rim bounce from below
                            apply_bounce_deflection(
                                &mut ball_velocity.0,
                                ReflectAxis::Horizontal,
                                RIM_DEFLECT_ANGLE_MAX,
                                RIM_BOUNCE_RETENTION,
                                &mut rng,
                            );
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
                    apply_bounce_deflection(
                        &mut ball_velocity.0,
                        ReflectAxis::Vertical,
                        STEP_DEFLECT_ANGLE_MAX,
                        STEP_BOUNCE_RETENTION,
                        &mut rng,
                    );
                } else if is_rim {
                    // Rim side bounce
                    apply_bounce_deflection(
                        &mut ball_velocity.0,
                        ReflectAxis::Vertical,
                        RIM_DEFLECT_ANGLE_MAX,
                        RIM_BOUNCE_RETENTION,
                        &mut rng,
                    );
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

/// Update ball spin/rotation based on velocity
pub fn ball_spin(
    time: Res<Time>,
    mut query: Query<
        (
            &mut Transform,
            &mut BallSpin,
            &Velocity,
            &BallState,
            &BallRolling,
        ),
        With<Ball>,
    >,
) {
    // Use minimum dt for headless mode compatibility
    let dt = time.delta_secs().max(1.0 / 60.0);

    for (mut transform, mut spin, velocity, state, rolling) in &mut query {
        // When ball is held, reset rotation to upright
        if matches!(state, BallState::Held(_)) {
            transform.rotation = Quat::IDENTITY;
            spin.0 = 0.0;
            continue;
        }

        // Update spin based on horizontal velocity
        // Negative because positive vx = clockwise rotation (rolling right)
        if rolling.0 {
            // Rolling: spin matches ground velocity exactly (no slip)
            // v = ωr → ω = v/r
            spin.0 = -velocity.0.x / (BALL_SIZE.x / 2.0);
        } else {
            // In flight: spin based on velocity with decay
            spin.0 = -velocity.0.x * BALL_SPIN_FACTOR;
            spin.0 *= BALL_SPIN_DECAY.powf(dt);
        }

        // Apply rotation
        transform.rotate_z(spin.0 * dt);
    }
}

/// Animate display balls with row-based wave effects
/// One row activates at a time, cycling every 5 seconds
pub fn display_ball_wave(
    time: Res<Time>,
    mut wave: ResMut<DisplayBallWave>,
    mut query: Query<(&DisplayBall, &mut Transform, &mut DisplayBallSpin)>,
) {
    let dt = time.delta_secs();

    // Update timer
    wave.timer += dt;
    if wave.timer > wave.cycle_period {
        wave.timer -= wave.cycle_period;
    }

    for (display, mut transform, mut spin) in &mut query {
        // Get intensity for this ball's row
        let intensity = wave.row_intensity(display.row);

        // Scale effect - all balls in active row pulse together
        if intensity > 0.01 {
            transform.scale = Vec3::splat(1.0 + intensity * 0.25);
            // Start spin when row becomes active
            if spin.velocity.abs() < 0.1 {
                spin.velocity = 6.0 * intensity;
            }
        } else {
            transform.scale = Vec3::ONE;
        }

        // Apply spin and decay
        if spin.velocity.abs() > 0.01 {
            transform.rotate_z(spin.velocity * dt);
            spin.velocity *= 0.92_f32.powf(dt * 60.0);
        } else {
            spin.velocity = 0.0;
        }
    }
}
