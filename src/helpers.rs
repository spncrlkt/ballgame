//! Utility functions for ballgame

use bevy::prelude::*;
use rand::Rng;

use crate::constants::{ARENA_WIDTH, WALL_THICKNESS};

/// Axis for bounce reflection
pub enum ReflectAxis {
    /// Reflect on vertical axis (horizontal collision - negate X)
    Vertical,
    /// Reflect on horizontal axis (vertical collision - negate Y)
    Horizontal,
}

/// Apply bounce deflection with random angle variance.
/// Used for step and rim bounces to create unpredictable ball behavior.
pub fn apply_bounce_deflection(
    velocity: &mut Vec2,
    axis: ReflectAxis,
    deflect_max: f32,
    retention: f32,
    rng: &mut impl Rng,
) {
    let speed = velocity.length();
    let deflect_angle = rng.gen_range(-deflect_max..deflect_max);
    let deflect_rad = deflect_angle.to_radians();

    // Reflect velocity on the appropriate axis
    let reflected = match axis {
        ReflectAxis::Vertical => Vec2::new(-velocity.x, velocity.y),
        ReflectAxis::Horizontal => Vec2::new(velocity.x, -velocity.y),
    };

    // Rotate by random deflection angle
    let cos_a = deflect_rad.cos();
    let sin_a = deflect_rad.sin();
    let rotated = Vec2::new(
        reflected.x * cos_a - reflected.y * sin_a,
        reflected.x * sin_a + reflected.y * cos_a,
    );

    *velocity = rotated.normalize() * speed * retention;
}

/// Move a value toward a target by a maximum delta
pub fn move_toward(current: f32, target: f32, max_delta: f32) -> f32 {
    if (target - current).abs() <= max_delta {
        target
    } else {
        current + (target - current).signum() * max_delta
    }
}

/// Calculate basket X positions from wall offset
pub fn basket_x_from_offset(offset: f32) -> (f32, f32) {
    let wall_inner = ARENA_WIDTH / 2.0 - WALL_THICKNESS;
    let left_x = -wall_inner + offset;
    let right_x = wall_inner - offset;
    (left_x, right_x)
}
