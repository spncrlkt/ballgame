//! World components for the arena, platforms, and baskets

use bevy::prelude::*;

/// Marker for collidable entities
#[derive(Component, Default)]
pub struct Collider;

/// Platform component - collidable surface
#[derive(Component)]
#[require(Collider)]
pub struct Platform;

/// Marks platforms that belong to current level (despawned on level change)
#[derive(Component)]
pub struct LevelPlatform;

/// Marks rim platforms attached to baskets (for collision filtering)
#[derive(Component)]
pub struct BasketRim;

/// Angled wall extensions in bottom corners
#[derive(Component)]
pub struct CornerRamp;

/// Basket scoring zone
#[derive(Component, Clone, Copy, PartialEq)]
pub enum Basket {
    Left,
    Right,
}
