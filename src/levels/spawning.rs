//! Level platform spawning helpers

use bevy::prelude::*;

use crate::constants::*;
use crate::helpers::basket_x_from_offset;
use crate::levels::database::{LevelDatabase, PlatformDef};
use crate::world::{CornerRamp, LevelPlatform, Platform};

/// Helper to spawn a platform mirrored on both sides (symmetric)
pub fn spawn_mirrored_platform(commands: &mut Commands, x: f32, y: f32, width: f32, color: Color) {
    // Left side
    commands.spawn((
        Sprite::from_color(color, Vec2::new(width, 20.0)),
        Transform::from_xyz(-x, y, 0.0),
        Platform,
        LevelPlatform,
    ));
    // Right side (mirrored)
    commands.spawn((
        Sprite::from_color(color, Vec2::new(width, 20.0)),
        Transform::from_xyz(x, y, 0.0),
        Platform,
        LevelPlatform,
    ));
}

/// Helper to spawn a centered platform
pub fn spawn_center_platform(commands: &mut Commands, y: f32, width: f32, color: Color) {
    commands.spawn((
        Sprite::from_color(color, Vec2::new(width, 20.0)),
        Transform::from_xyz(0.0, y, 0.0),
        Platform,
        LevelPlatform,
    ));
}

/// Spawn corner steps in the bottom corners
/// step_count of 0 means no steps
/// step_push_in is the distance from wall where stairs start (top step extends to wall)
pub fn spawn_corner_ramps(
    commands: &mut Commands,
    step_count: usize,
    corner_height: f32,
    corner_width: f32,
    step_push_in: f32,
    floor_color: Color,
) {
    if step_count == 0 {
        return;
    }

    // Wall inner edges
    let left_wall_inner = -ARENA_WIDTH / 2.0 + WALL_THICKNESS;
    let right_wall_inner = ARENA_WIDTH / 2.0 - WALL_THICKNESS;

    // Step dimensions
    let step_height = corner_height / step_count as f32;
    let step_width = corner_width / step_count as f32;

    // Left steps: go from wall (high) toward center (low)
    // Step 0 is highest (closest to wall), step N-1 is lowest (closest to center)
    let floor_top = ARENA_FLOOR_Y + 20.0;
    for i in 0..step_count {
        let step_num = (step_count - 1 - i) as f32; // Reverse so 0 is lowest
        let y = floor_top + step_height * (step_num + 0.5);

        // Top step (i=0) extends from wall to step_push_in + step_width
        // Other steps start at step_push_in offset
        let (x, width) = if i == 0 {
            // Top step extends from wall to end of first step position
            let right_edge = left_wall_inner + step_push_in + step_width;
            let center = (left_wall_inner + right_edge) / 2.0;
            let full_width = right_edge - left_wall_inner;
            (center, full_width)
        } else {
            (
                left_wall_inner + step_push_in + step_width * (i as f32 + 0.5),
                step_width,
            )
        };

        commands.spawn((
            Sprite::from_color(floor_color, Vec2::new(width, CORNER_STEP_THICKNESS)),
            Transform::from_xyz(x, y, 0.0),
            Platform,
            CornerRamp,
        ));

        // Fill under the step (visual only, but needs CornerRamp for despawning)
        let step_bottom = y - CORNER_STEP_THICKNESS / 2.0;
        let fill_height = step_bottom - floor_top;
        if fill_height > 0.0 {
            let fill_y = floor_top + fill_height / 2.0;
            commands.spawn((
                Sprite::from_color(floor_color, Vec2::new(width, fill_height)),
                Transform::from_xyz(x, fill_y, -0.1),
                CornerRamp, // Needed so it gets despawned on level change
            ));
        }
    }

    // Right steps: mirror of left (go from wall toward center)
    for i in 0..step_count {
        let step_num = (step_count - 1 - i) as f32;
        let y = floor_top + step_height * (step_num + 0.5);

        // Top step (i=0) extends from wall to step_push_in + step_width
        let (x, width) = if i == 0 {
            let left_edge = right_wall_inner - step_push_in - step_width;
            let center = (right_wall_inner + left_edge) / 2.0;
            let full_width = right_wall_inner - left_edge;
            (center, full_width)
        } else {
            (
                right_wall_inner - step_push_in - step_width * (i as f32 + 0.5),
                step_width,
            )
        };

        commands.spawn((
            Sprite::from_color(floor_color, Vec2::new(width, CORNER_STEP_THICKNESS)),
            Transform::from_xyz(x, y, 0.0),
            Platform,
            CornerRamp,
        ));

        // Fill under the step (visual only, but needs CornerRamp for despawning)
        let step_bottom = y - CORNER_STEP_THICKNESS / 2.0;
        let fill_height = step_bottom - floor_top;
        if fill_height > 0.0 {
            let fill_y = floor_top + fill_height / 2.0;
            commands.spawn((
                Sprite::from_color(floor_color, Vec2::new(width, fill_height)),
                Transform::from_xyz(x, fill_y, -0.1),
                CornerRamp, // Needed so it gets despawned on level change
            ));
        }
    }
}

/// Spawn platforms for a specific level
pub fn spawn_level_platforms(
    commands: &mut Commands,
    level_db: &LevelDatabase,
    level_index: usize,
    platform_color: Color,
) {
    let Some(level) = level_db.get(level_index) else {
        warn!("Level {} not found, spawning empty", level_index);
        return;
    };

    for platform in &level.platforms {
        match platform {
            PlatformDef::Mirror { x, y, width } => {
                spawn_mirrored_platform(commands, *x, ARENA_FLOOR_Y + y, *width, platform_color);
            }
            PlatformDef::Center { y, width } => {
                spawn_center_platform(commands, ARENA_FLOOR_Y + y, *width, platform_color);
            }
        }
    }
}

/// Reload all level geometry (platforms and corner ramps).
/// Despawns existing geometry and spawns new geometry for the specified level.
/// Returns the (left_basket_x, right_basket_x, basket_y) positions if level exists.
pub fn reload_level_geometry(
    commands: &mut Commands,
    level_db: &LevelDatabase,
    level_index: usize,
    platform_color: Color,
    platforms_to_despawn: impl IntoIterator<Item = Entity>,
    ramps_to_despawn: impl IntoIterator<Item = Entity>,
) -> Option<(f32, f32, f32)> {
    // Despawn old level platforms
    for entity in platforms_to_despawn {
        commands.entity(entity).despawn();
    }

    // Despawn old corner ramps
    for entity in ramps_to_despawn {
        commands.entity(entity).despawn();
    }

    // Spawn new level platforms
    spawn_level_platforms(commands, level_db, level_index, platform_color);

    // Spawn corner ramps and return basket positions
    let level = level_db.get(level_index)?;

    spawn_corner_ramps(
        commands,
        level.step_count,
        level.corner_height,
        level.corner_width,
        level.step_push_in,
        platform_color,
    );

    let basket_y = ARENA_FLOOR_Y + level.basket_height;
    let (left_x, right_x) = basket_x_from_offset(level.basket_push_in);

    Some((left_x, right_x, basket_y))
}
