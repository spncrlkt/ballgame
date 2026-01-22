//! Utility functions for ballgame

use crate::constants::{ARENA_WIDTH, WALL_THICKNESS};

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
