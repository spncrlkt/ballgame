//! Shot quality evaluation for AI decision making
//!
//! Based on Monte Carlo heatmap analysis of shot success rates.
//! Higher quality = higher probability of scoring.

use bevy::prelude::*;

use crate::constants::ARENA_FLOOR_Y;

/// Evaluate the quality of a shot from a given position to a target basket.
/// Returns a value from 0.0 (terrible shot) to 1.0 (excellent shot).
///
/// Based on heatmap analysis of actual shot success rates:
/// - Above basket level: 60-90% success (quality 0.6-0.9)
/// - Same level as basket: 50-70% success (quality 0.5-0.7)
/// - Below basket (floor shots): 30-50% success (quality 0.35-0.5)
/// - Behind basket: 20-40% success (quality 0.2-0.4)
/// - Directly under basket: 15-30% success (quality 0.15-0.3)
pub fn evaluate_shot_quality(shooter_pos: Vec2, basket_pos: Vec2) -> f32 {
    // Calculate relative position
    let dx = shooter_pos.x - basket_pos.x;
    let dy = shooter_pos.y - basket_pos.y;
    let horizontal_dist = dx.abs();

    // Determine which side of basket we're on (in front = good, behind = bad)
    // "In front" means the open side of the basket (center of arena)
    let basket_is_left = basket_pos.x < 0.0;
    let in_front = if basket_is_left {
        dx > 0.0 // Shooter is to the right of left basket (toward center)
    } else {
        dx < 0.0 // Shooter is to the left of right basket (toward center)
    };

    // Start with base quality that allows floor shots
    let mut quality = 0.45;

    // === Height factor ===
    // Above basket is best, but below basket is still viable (floor shots are common)
    if dy > 0.0 {
        // Above basket - excellent position (arc drops into basket)
        // Peak bonus around 100-300 pixels above
        let height_bonus = (dy / 250.0).min(1.0) * 0.4;
        quality += height_bonus;
    } else {
        // Below basket - gradual penalty, but floor shots are still valid
        // Even 600+ pixels below should still be ~0.35 quality (playable)
        let depth_ratio = (-dy / 800.0).min(1.0);
        let height_penalty = depth_ratio * 0.15;
        quality -= height_penalty;
    }

    // === Horizontal position factor ===
    if in_front {
        // In front of basket - good angle for scoring
        // Sweet spot: 150-400 pixels from basket horizontally
        let dist_bonus = if horizontal_dist < 100.0 {
            // Too close - harder to arc over rim
            horizontal_dist / 100.0 * 0.1
        } else if horizontal_dist < 400.0 {
            // Good range
            0.15
        } else if horizontal_dist < 600.0 {
            // Getting far but still reasonable
            0.15 - ((horizontal_dist - 400.0) / 200.0) * 0.1
        } else {
            // Very far - low bonus
            0.05
        };
        quality += dist_bonus;
    } else {
        // Behind basket - significant penalty (shooting through backboard area)
        let behind_penalty = (horizontal_dist / 150.0).min(1.0) * 0.25;
        quality -= behind_penalty;
    }

    // === Directly under penalty ===
    // Shooting straight up is very difficult (ball goes straight up and down)
    if dy < 0.0 && horizontal_dist < 60.0 {
        let under_penalty = (1.0 - horizontal_dist / 60.0) * 0.2;
        quality -= under_penalty;
    }

    // === Absolute floor level penalty ===
    // Shots from near floor level (Y < -350) are empirically much worse
    // Tournament data shows: floor shots (Y ~ -350) have 3-10% success vs
    // elevated shots (Y ~ -200) which have 20-35% success
    // This penalty encourages AI to seek platforms before shooting
    let floor_threshold = ARENA_FLOOR_Y + 100.0; // About -350
    if shooter_pos.y < floor_threshold {
        // Gradual penalty: max 0.15 at floor level, tapering to 0 at threshold
        let depth_below_threshold = floor_threshold - shooter_pos.y;
        let floor_penalty = (depth_below_threshold / 100.0).min(1.0) * 0.15;
        quality -= floor_penalty;
    }

    // Clamp to valid range
    quality.clamp(0.1, 1.0)
}

/// Minimum shot quality thresholds for different AI behaviors
pub const SHOT_QUALITY_EXCELLENT: f32 = 0.75;
pub const SHOT_QUALITY_GOOD: f32 = 0.55;
pub const SHOT_QUALITY_ACCEPTABLE: f32 = 0.40;
pub const SHOT_QUALITY_DESPERATE: f32 = 0.25;

/// Reference max quality (achievable with good platform elevation)
/// Used for scaling min_shot_quality on flat levels
pub const SHOT_QUALITY_REFERENCE_MAX: f32 = 0.85;

/// Calculate the maximum achievable shot quality for a level given nav graph node positions.
/// Returns the best quality achievable from any platform position to either basket.
pub fn calculate_level_max_quality(node_positions: &[Vec2], basket_positions: &[Vec2]) -> f32 {
    let mut max_quality = 0.3; // Minimum floor quality baseline

    for &node_pos in node_positions {
        for &basket_pos in basket_positions {
            let quality = evaluate_shot_quality(node_pos, basket_pos);
            if quality > max_quality {
                max_quality = quality;
            }
        }
    }

    max_quality
}

/// Scale a profile's min_shot_quality based on what's achievable on the current level.
/// On flat levels (max ~0.50), this lowers the threshold so AI still shoots.
/// On elevated levels (max ~0.85), threshold stays close to profile value.
pub fn scale_min_quality_for_level(profile_min_quality: f32, level_max_quality: f32) -> f32 {
    // Scale relative to reference max (0.85)
    // If level max is lower, proportionally reduce the threshold
    let scale_factor = (level_max_quality / SHOT_QUALITY_REFERENCE_MAX).min(1.0);
    profile_min_quality * scale_factor
}

/// Get a descriptive label for a shot quality value
pub fn quality_label(quality: f32) -> &'static str {
    if quality >= SHOT_QUALITY_EXCELLENT {
        "excellent"
    } else if quality >= SHOT_QUALITY_GOOD {
        "good"
    } else if quality >= SHOT_QUALITY_ACCEPTABLE {
        "acceptable"
    } else if quality >= SHOT_QUALITY_DESPERATE {
        "desperate"
    } else {
        "terrible"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_above_basket_high_quality() {
        // Basket at left side, shooter above and in front
        let basket = Vec2::new(-600.0, 0.0);
        let shooter = Vec2::new(-400.0, 150.0); // Above and to the right (in front)
        let quality = evaluate_shot_quality(shooter, basket);
        assert!(
            quality >= SHOT_QUALITY_GOOD,
            "Above+front should be good: {}",
            quality
        );
    }

    #[test]
    fn test_floor_shot_acceptable() {
        // Realistic floor shot: player on floor (y=-418), basket elevated (y=210)
        // This is the most common shot scenario
        let basket = Vec2::new(-644.0, 210.0); // Left basket, elevated
        let shooter = Vec2::new(-200.0, -418.0); // Player on floor, center-ish
        let quality = evaluate_shot_quality(shooter, basket);
        assert!(
            quality >= 0.35,
            "Floor shot should be at least 0.35 quality (got {})",
            quality
        );
        assert!(
            quality <= 0.6,
            "Floor shot should be at most 0.6 quality (got {})",
            quality
        );
    }

    #[test]
    fn test_directly_under_penalty() {
        // Shooting from directly below basket
        let basket = Vec2::new(-600.0, 200.0);
        let shooter = Vec2::new(-600.0, -100.0); // Directly below
        let quality = evaluate_shot_quality(shooter, basket);
        assert!(
            quality < SHOT_QUALITY_ACCEPTABLE,
            "Directly under should be low: {}",
            quality
        );
    }

    #[test]
    fn test_behind_basket_penalty() {
        // Behind left basket (further left)
        let basket = Vec2::new(-600.0, 0.0);
        let shooter = Vec2::new(-750.0, 0.0); // Behind the basket
        let quality = evaluate_shot_quality(shooter, basket);
        assert!(
            quality < SHOT_QUALITY_GOOD,
            "Behind basket should be penalized: {}",
            quality
        );
    }

    #[test]
    fn test_optimal_position_high_quality() {
        // Optimal position: slightly above basket, good horizontal distance
        let basket = Vec2::new(-600.0, 200.0);
        let shooter = Vec2::new(-300.0, 350.0); // Above basket, in front
        let quality = evaluate_shot_quality(shooter, basket);
        assert!(
            quality >= SHOT_QUALITY_EXCELLENT,
            "Optimal position should be excellent: {}",
            quality
        );
    }
}
