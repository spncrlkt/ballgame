//! AI capabilities - precomputed physics values and reachability queries.
//!
//! This module provides a single source of truth for physics-derived values.
//! Other AI modules query this resource instead of computing physics formulas.

use bevy::prelude::*;

use crate::constants::*;

/// Precomputed physics capabilities for AI decision-making.
/// Created once at startup, used by navigation and decision systems.
#[derive(Resource)]
pub struct AiCapabilities {
    /// Maximum height AI can reach with a full jump (v²/2g)
    pub max_jump_height: f32,
    /// Time to reach max jump height (v/g)
    pub time_to_peak: f32,
    /// Horizontal distance covered during a max height jump (up and down)
    pub max_jump_reach: f32,
}

impl Default for AiCapabilities {
    fn default() -> Self {
        // Physics formula: h = v²/(2g) using rise gravity
        let max_jump_height = JUMP_VELOCITY * JUMP_VELOCITY / (2.0 * GRAVITY_RISE);
        // Time to peak: t = v/g
        let time_to_peak = JUMP_VELOCITY / GRAVITY_RISE;
        // Total horizontal reach during jump (time up + time down at MOVE_SPEED)
        // Note: fall time is shorter due to higher GRAVITY_FALL
        let fall_time = (2.0 * max_jump_height / GRAVITY_FALL).sqrt();
        let max_jump_reach = MOVE_SPEED * (time_to_peak + fall_time);

        Self {
            max_jump_height,
            time_to_peak,
            max_jump_reach,
        }
    }
}

impl AiCapabilities {
    /// Can AI jump from current_y to target_y?
    pub fn can_reach_height(&self, current_y: f32, target_y: f32) -> bool {
        target_y - current_y <= self.max_jump_height
    }

    /// Calculate jump hold duration needed to reach a specific height.
    /// Returns a value between 0.1 (tap) and 1.0 (full hold).
    pub fn jump_hold_for_height(&self, height: f32) -> f32 {
        let ratio = height / self.max_jump_height;
        // Overshoot slightly for safety margin
        (ratio * 1.2).clamp(0.1, 1.0)
    }

    /// Check if there's ceiling clearance to jump at this position.
    /// Uses platform bounds to determine if a jump would be blocked.
    pub fn has_ceiling_clearance(&self, pos: Vec2, platforms: &[(Vec2, Vec2)]) -> bool {
        let head_y = pos.y + PLAYER_SIZE.y / 2.0;
        let peak_y = head_y + self.max_jump_height;
        let player_half_width = PLAYER_SIZE.x / 2.0;

        for (center, size) in platforms {
            let plat_left = center.x - size.x / 2.0;
            let plat_right = center.x + size.x / 2.0;
            let plat_bottom = center.y - size.y / 2.0;

            // Check horizontal overlap
            if pos.x + player_half_width < plat_left || pos.x - player_half_width > plat_right {
                continue;
            }

            // Check if platform bottom is in the vertical jump range
            if plat_bottom > head_y && plat_bottom < peak_y {
                return false; // Ceiling blocks jump
            }
        }

        true // No ceiling blocking
    }

    /// Find the nearest X position to escape from under a blocking platform.
    /// Returns the closest platform edge that allows jumping to target height.
    pub fn find_escape_x(
        &self,
        pos: Vec2,
        target_y: f32,
        platforms: &[(Vec2, Vec2)],
    ) -> Option<f32> {
        let head_y = pos.y + PLAYER_SIZE.y / 2.0;
        let player_half_width = PLAYER_SIZE.x / 2.0;

        for (center, size) in platforms {
            let plat_left = center.x - size.x / 2.0;
            let plat_right = center.x + size.x / 2.0;
            let plat_bottom = center.y - size.y / 2.0;
            let plat_top = center.y + size.y / 2.0;

            // Check horizontal overlap
            let overlaps_x = pos.x + player_half_width > plat_left
                && pos.x - player_half_width < plat_right;

            if !overlaps_x {
                continue;
            }

            // Check if platform is above head and blocks path to target
            let is_above_head = plat_bottom > head_y;
            let blocks_path_to_target = plat_top >= target_y - PLAYER_SIZE.y;

            if is_above_head && blocks_path_to_target {
                // Find closest edge to escape to
                let margin = PLAYER_SIZE.x;
                let left_escape = plat_left - margin;
                let right_escape = plat_right + margin;

                let dist_left = (pos.x - left_escape).abs();
                let dist_right = (pos.x - right_escape).abs();

                return Some(if dist_left < dist_right {
                    left_escape
                } else {
                    right_escape
                });
            }
        }

        None // No blocking platform found
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_jump_height_matches_constant() {
        let caps = AiCapabilities::default();
        // Should be approximately 215px based on v=650, g=980
        // h = 650² / (2*980) = 422500 / 1960 ≈ 215.56
        assert!(
            (caps.max_jump_height - 215.56).abs() < 1.0,
            "Expected ~215.56, got {}",
            caps.max_jump_height
        );
    }

    #[test]
    fn test_can_reach_height() {
        let caps = AiCapabilities::default();
        // Can reach platform 100px above
        assert!(caps.can_reach_height(0.0, 100.0));
        // Cannot reach platform 300px above
        assert!(!caps.can_reach_height(0.0, 300.0));
    }

    #[test]
    fn test_jump_hold_for_height() {
        let caps = AiCapabilities::default();
        // Low height = short hold
        assert!(caps.jump_hold_for_height(50.0) < 0.5);
        // High height = long hold
        assert!(caps.jump_hold_for_height(200.0) > 0.8);
    }

    #[test]
    fn test_ceiling_clearance() {
        let caps = AiCapabilities::default();
        let pos = Vec2::new(0.0, 0.0);

        // No platforms = clearance
        assert!(caps.has_ceiling_clearance(pos, &[]));

        // Platform directly above within jump height = no clearance
        let blocking = vec![(Vec2::new(0.0, 100.0), Vec2::new(100.0, 20.0))];
        assert!(!caps.has_ceiling_clearance(pos, &blocking));

        // Platform to the side = clearance
        let side_platform = vec![(Vec2::new(200.0, 100.0), Vec2::new(100.0, 20.0))];
        assert!(caps.has_ceiling_clearance(pos, &side_platform));
    }
}
