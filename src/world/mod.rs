//! World components for the arena, platforms, and baskets

use bevy::prelude::*;

use crate::constants::*;

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

// ============================================================================
// Arena spawning functions (shared between main game and test runner)
// ============================================================================

/// Spawn the arena floor
pub fn spawn_floor(commands: &mut Commands, color: Color) {
    commands.spawn((
        Sprite::from_color(color, Vec2::new(ARENA_WIDTH - WALL_THICKNESS * 2.0, 40.0)),
        Transform::from_xyz(0.0, ARENA_FLOOR_Y, 0.0),
        Platform,
    ));
}

/// Spawn arena walls (left and right)
pub fn spawn_walls(commands: &mut Commands, color: Color) {
    // Left wall
    commands.spawn((
        Sprite::from_color(color, Vec2::new(WALL_THICKNESS, 5000.0)),
        Transform::from_xyz(-ARENA_WIDTH / 2.0 + WALL_THICKNESS / 2.0, 2000.0, 0.0),
        Platform,
    ));
    // Right wall
    commands.spawn((
        Sprite::from_color(color, Vec2::new(WALL_THICKNESS, 5000.0)),
        Transform::from_xyz(ARENA_WIDTH / 2.0 - WALL_THICKNESS / 2.0, 2000.0, 0.0),
        Platform,
    ));
}

/// Spawn a single basket with rim children
///
/// - `side`: Which basket (Left or Right)
/// - `x`: X position of basket center
/// - `y`: Y position of basket center
/// - `basket_color`: Color of the basket body
/// - `rim_color`: Color of the rim platforms
pub fn spawn_basket_with_rims(
    commands: &mut Commands,
    side: Basket,
    x: f32,
    y: f32,
    basket_color: Color,
    rim_color: Color,
) {
    // Rim dimensions
    let rim_outer_height = BASKET_SIZE.y * 0.5; // 50% - wall side
    let rim_inner_height = BASKET_SIZE.y * 0.1; // 10% - center side
    let rim_outer_y = -BASKET_SIZE.y / 2.0 + rim_outer_height / 2.0;
    let rim_inner_y = -BASKET_SIZE.y / 2.0 + rim_inner_height / 2.0;
    let rim_bottom_width = BASKET_SIZE.x + RIM_THICKNESS;

    // Determine which side gets the tall rim (outer = wall side)
    let (left_rim_height, left_rim_y, right_rim_height, right_rim_y) = match side {
        Basket::Left => (rim_outer_height, rim_outer_y, rim_inner_height, rim_inner_y),
        Basket::Right => (rim_inner_height, rim_inner_y, rim_outer_height, rim_outer_y),
    };

    commands
        .spawn((
            Sprite::from_color(basket_color, BASKET_SIZE),
            Transform::from_xyz(x, y, -0.1),
            side,
        ))
        .with_children(|parent| {
            // Left rim
            parent.spawn((
                Sprite::from_color(rim_color, Vec2::new(RIM_THICKNESS, left_rim_height)),
                Transform::from_xyz(-BASKET_SIZE.x / 2.0, left_rim_y, 0.1),
                Platform,
                BasketRim,
            ));
            // Right rim
            parent.spawn((
                Sprite::from_color(rim_color, Vec2::new(RIM_THICKNESS, right_rim_height)),
                Transform::from_xyz(BASKET_SIZE.x / 2.0, right_rim_y, 0.1),
                Platform,
                BasketRim,
            ));
            // Bottom rim
            parent.spawn((
                Sprite::from_color(rim_color, Vec2::new(rim_bottom_width, RIM_THICKNESS)),
                Transform::from_xyz(0.0, -BASKET_SIZE.y / 2.0, 0.1),
                Platform,
                BasketRim,
            ));
        });
}

/// Spawn both baskets with rims at specified positions
pub fn spawn_baskets(
    commands: &mut Commands,
    basket_y: f32,
    basket_push_in: f32,
    left_basket_color: Color,
    right_basket_color: Color,
    left_rim_color: Color,
    right_rim_color: Color,
) {
    let wall_inner = ARENA_WIDTH / 2.0 - WALL_THICKNESS;
    let left_x = -wall_inner + basket_push_in;
    let right_x = wall_inner - basket_push_in;

    spawn_basket_with_rims(
        commands,
        Basket::Left,
        left_x,
        basket_y,
        left_basket_color,
        right_rim_color,
    );
    spawn_basket_with_rims(
        commands,
        Basket::Right,
        right_x,
        basket_y,
        right_basket_color,
        left_rim_color,
    );
}
