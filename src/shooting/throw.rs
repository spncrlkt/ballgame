//! Ball throwing system

use crate::calculate_shot_trajectory;
use bevy::prelude::*;
use rand::Rng;

use crate::ai::AiInput;
use crate::ball::{Ball, BallRolling, BallShotGrace, BallState, Velocity};
use crate::constants::*;
use crate::player::{Grounded, HoldingBall, Player, TargetBasket};
use crate::shooting::{ChargingShot, LastShotInfo};
use crate::ui::PhysicsTweaks;
use crate::world::Basket;

/// Execute throw when button is released.
/// All players read from their AiInput component.
pub fn throw_ball(
    tweaks: Res<PhysicsTweaks>,
    mut commands: Commands,
    mut shot_info: ResMut<LastShotInfo>,
    mut player_query: Query<
        (
            Entity,
            &Transform,
            &Velocity,
            &TargetBasket,
            &Grounded,
            &mut ChargingShot,
            &mut AiInput,
            Option<&HoldingBall>,
        ),
        With<Player>,
    >,
    mut ball_query: Query<
        (
            &mut Velocity,
            &mut BallState,
            &mut BallRolling,
            &mut BallShotGrace,
        ),
        (With<Ball>, Without<Player>),
    >,
    basket_query: Query<(&Transform, &Basket), Without<Player>>,
) {
    for (
        player_entity,
        player_transform,
        player_velocity,
        target,
        grounded,
        mut charging,
        mut input,
        holding,
    ) in &mut player_query
    {
        if !input.throw_released {
            continue;
        }

        // Consume the throw_released flag
        input.throw_released = false;

        let Some(holding_ball) = holding else {
            // Not holding a ball - reset charge since they released the button
            charging.charge_time = 0.0;
            continue;
        };

        let Ok((mut ball_velocity, mut ball_state, mut rolling, mut grace)) =
            ball_query.get_mut(holding_ball.0)
        else {
            continue;
        };

        // Ball is being thrown - no longer rolling, start grace period
        rolling.0 = false;
        grace.0 = SHOT_GRACE_PERIOD;

        // Calculate charge percentage (0.0 to 1.0)
        let charge_pct = (charging.charge_time / tweaks.shot_charge_time).min(1.0);

        let mut rng = rand::thread_rng();
        let player_pos = player_transform.translation.truncate();

        // Find closest basket matching the target type
        let target_basket_pos = basket_query
            .iter()
            .filter(|(_, basket)| **basket == target.0)
            .min_by(|(a, _), (b, _)| {
                let dist_a = player_pos.distance_squared(a.translation.truncate());
                let dist_b = player_pos.distance_squared(b.translation.truncate());
                dist_a.partial_cmp(&dist_b).unwrap()
            })
            .map(|(transform, _)| transform.translation.truncate());

        // Calculate optimal trajectory to basket
        let trajectory = if let Some(basket_pos) = target_basket_pos {
            calculate_shot_trajectory(
                player_pos.x,
                player_pos.y,
                basket_pos.x,
                basket_pos.y,
                BALL_GRAVITY,
            )
        } else {
            None
        };

        // Base variance from charge level: 50% at 0 charge → 2% at full charge
        let base_variance =
            SHOT_MAX_VARIANCE - (SHOT_MAX_VARIANCE - SHOT_MIN_VARIANCE) * charge_pct;
        let mut variance = base_variance;

        // Air shot penalty: +10% variance when airborne
        let air_penalty = if !grounded.0 {
            SHOT_AIR_VARIANCE_PENALTY
        } else {
            0.0
        };
        variance += air_penalty;

        // Horizontal movement penalty: 0-10% variance based on horizontal speed
        let move_penalty =
            (player_velocity.0.x.abs() / MOVE_SPEED).min(1.0) * SHOT_MOVE_VARIANCE_PENALTY;
        variance += move_penalty;

        // Get base angle, required speed, and distance variance from trajectory
        let (base_angle, required_speed, distance_variance) = if let Some(traj) = &trajectory {
            (traj.angle, traj.required_speed, traj.distance_variance)
        } else {
            // Fallback (shouldn't happen now) - 60° toward target with max speed
            let fallback_angle = if let Some(basket_pos) = target_basket_pos {
                if basket_pos.x >= player_pos.x {
                    SHOT_DEFAULT_ANGLE.to_radians() // 60° right
                } else {
                    std::f32::consts::PI - SHOT_DEFAULT_ANGLE.to_radians() // 120° left
                }
            } else {
                SHOT_DEFAULT_ANGLE.to_radians() // Default: 60° right
            };
            (fallback_angle, SHOT_MAX_SPEED, 0.0)
        };

        // Add distance variance to total
        variance += distance_variance;

        // Apply variance to angle (max ±30° at full variance), biased 5% upward
        let max_angle_variance = 30.0_f32.to_radians();
        let angle_variance = (rng.gen_range(-variance..variance) + 0.05) * max_angle_variance;
        let final_angle = base_angle + angle_variance;

        // Half power for quick shots (< 400ms charge)
        let power_multiplier = if charging.charge_time < SHOT_QUICK_THRESHOLD {
            0.5
        } else {
            1.0
        };

        // Boost speed by 10% to compensate for undershoot, then apply ±10% randomness
        let speed_randomness = rng.gen_range(0.9..1.1);
        let uncapped_speed = required_speed * 1.10 * speed_randomness * power_multiplier;

        // Hard cap at SHOT_HARD_CAP
        let final_speed = uncapped_speed.min(SHOT_HARD_CAP);

        // Convert angle + speed to velocity (simple and direct!)
        // Angle is absolute: 0=right, π/2=up, π=left
        let vx = final_speed * final_angle.cos();
        let vy = final_speed * final_angle.sin();

        // Set ball velocity
        ball_velocity.0.x = vx;
        ball_velocity.0.y = vy;

        *ball_state = BallState::InFlight {
            shooter: player_entity,
            power: final_speed,
        };

        // Record shot info for debug display
        *shot_info = LastShotInfo {
            angle_degrees: final_angle.to_degrees(),
            speed: final_speed,
            base_variance,
            air_penalty,
            move_penalty,
            distance_variance,
            required_speed,
            total_variance: variance,
            target: Some(target.0),
        };

        // Reset charge and release ball
        charging.charge_time = 0.0;
        commands.entity(player_entity).remove::<HoldingBall>();
    }
}
