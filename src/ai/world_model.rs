//! World model for AI - extracts platform data from game state.
//!
//! This module provides utilities to extract platform information from the ECS
//! in a format that AiCapabilities can use for physics calculations.

use bevy::prelude::*;

use crate::constants::PLAYER_SIZE;
use crate::world::{BasketRim, Platform};

/// Bounds of a platform in world coordinates
#[derive(Clone, Copy, Debug)]
pub struct PlatformBounds {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
    pub center: Vec2,
    pub size: Vec2,
}

impl PlatformBounds {
    /// Create from center position and size
    pub fn from_center_size(center: Vec2, size: Vec2) -> Self {
        Self {
            left: center.x - size.x / 2.0,
            right: center.x + size.x / 2.0,
            top: center.y + size.y / 2.0,
            bottom: center.y - size.y / 2.0,
            center,
            size,
        }
    }

    /// Check if a position (with player width) horizontally overlaps this platform
    pub fn overlaps_x(&self, x: f32) -> bool {
        let half_width = PLAYER_SIZE.x / 2.0;
        x + half_width > self.left && x - half_width < self.right
    }

    /// Check if this platform is in a vertical range
    pub fn in_vertical_range(&self, bottom_y: f32, top_y: f32) -> bool {
        self.bottom > bottom_y && self.bottom < top_y
    }
}

/// Extract platform data from ECS queries for use with AiCapabilities.
/// Returns a Vec of (center, size) tuples for ceiling/escape calculations.
pub fn extract_platform_data(
    platform_query: &Query<(&Transform, &Sprite), (With<Platform>, Without<BasketRim>)>,
) -> Vec<(Vec2, Vec2)> {
    platform_query
        .iter()
        .filter_map(|(transform, sprite)| {
            // Skip floor (very wide platforms at bottom)
            let size = sprite.custom_size.unwrap_or(Vec2::new(100.0, 20.0));
            let pos = transform.translation.truncate();

            // Skip floor and walls
            if size.y > 100.0 || size.x > 1400.0 {
                return None;
            }

            Some((pos, size))
        })
        .collect()
}

/// Extract platform data from NavGraph nodes instead of direct queries.
/// This is more efficient when NavGraph is already built.
pub fn extract_platforms_from_nav(nav_nodes: &[crate::ai::NavNode]) -> Vec<(Vec2, Vec2)> {
    nav_nodes
        .iter()
        .filter(|node| !node.is_floor)
        .map(|node| {
            let width = node.right_x - node.left_x;
            // Use actual node data to compute center and approximate size
            // The node stores top_y, and platforms are typically 20px tall
            let center = Vec2::new(node.center.x, node.top_y - 10.0);
            let size = Vec2::new(width, 20.0);
            (center, size)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_bounds_overlap() {
        let bounds =
            PlatformBounds::from_center_size(Vec2::new(100.0, 50.0), Vec2::new(80.0, 20.0));

        // Position at center should overlap
        assert!(bounds.overlaps_x(100.0));

        // Position at edge should overlap (player width extends)
        assert!(bounds.overlaps_x(140.0 + PLAYER_SIZE.x / 2.0 - 1.0));

        // Position far away should not overlap
        assert!(!bounds.overlaps_x(200.0));
    }

    #[test]
    fn test_platform_bounds_vertical_range() {
        let bounds = PlatformBounds::from_center_size(Vec2::new(0.0, 100.0), Vec2::new(80.0, 20.0));
        // bottom = 90, top = 110

        // Range that includes the bottom
        assert!(bounds.in_vertical_range(50.0, 95.0));

        // Range below platform
        assert!(!bounds.in_vertical_range(0.0, 50.0));

        // Range above platform
        assert!(!bounds.in_vertical_range(120.0, 200.0));
    }
}
