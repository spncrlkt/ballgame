//! Level platform spawning helpers

use bevy::prelude::*;

use crate::constants::*;
use crate::levels::database::{LevelDatabase, PlatformDef};
use crate::world::{CornerRamp, LevelPlatform, Platform};

/// Helper to spawn a platform mirrored on both sides (symmetric)
pub fn spawn_mirrored_platform(commands: &mut Commands, x: f32, y: f32, width: f32) {
    // Left side
    commands.spawn((
        Sprite::from_color(PLATFORM_COLOR, Vec2::new(width, 20.0)),
        Transform::from_xyz(-x, y, 0.0),
        Platform,
        LevelPlatform,
    ));
    // Right side (mirrored)
    commands.spawn((
        Sprite::from_color(PLATFORM_COLOR, Vec2::new(width, 20.0)),
        Transform::from_xyz(x, y, 0.0),
        Platform,
        LevelPlatform,
    ));
}

/// Helper to spawn a centered platform
pub fn spawn_center_platform(commands: &mut Commands, y: f32, width: f32) {
    commands.spawn((
        Sprite::from_color(PLATFORM_COLOR, Vec2::new(width, 20.0)),
        Transform::from_xyz(0.0, y, 0.0),
        Platform,
        LevelPlatform,
    ));
}

/// Spawn corner steps in the bottom corners
/// step_count of 0 means no steps
pub fn spawn_corner_ramps(commands: &mut Commands, step_count: usize, corner_height: f32, corner_width: f32) {
    if step_count == 0 {
        return;
    }

    // Wall inner edges (walls are 40 wide, centered at ±(ARENA_WIDTH/2 - 20))
    let left_wall_inner = -ARENA_WIDTH / 2.0 + 40.0;
    let right_wall_inner = ARENA_WIDTH / 2.0 - 40.0;

    // Step dimensions
    let step_height = corner_height / step_count as f32;
    let step_width = corner_width / step_count as f32;

    // Left steps: go from wall (high) toward center (low)
    // Step 0 is highest (closest to wall), step N-1 is lowest (closest to center)
    let floor_top = ARENA_FLOOR_Y + 20.0;
    for i in 0..step_count {
        let step_num = (step_count - 1 - i) as f32; // Reverse so 0 is lowest
        let y = floor_top + step_height * (step_num + 0.5);
        let x = left_wall_inner + step_width * (i as f32 + 0.5);

        commands.spawn((
            Sprite::from_color(FLOOR_COLOR, Vec2::new(step_width, CORNER_STEP_THICKNESS)),
            Transform::from_xyz(x, y, 0.0),
            Platform,
            CornerRamp,
        ));

        // Fill under the step
        let step_bottom = y - CORNER_STEP_THICKNESS / 2.0;
        let fill_height = step_bottom - floor_top;
        if fill_height > 0.0 {
            let fill_y = floor_top + fill_height / 2.0;
            commands.spawn((
                Sprite::from_color(FLOOR_COLOR, Vec2::new(step_width, fill_height)),
                Transform::from_xyz(x, fill_y, -0.1),
                CornerRamp,
            ));
        }
    }

    // Right steps: mirror of left (go from wall toward center)
    for i in 0..step_count {
        let step_num = (step_count - 1 - i) as f32;
        let y = floor_top + step_height * (step_num + 0.5);
        let x = right_wall_inner - step_width * (i as f32 + 0.5);

        commands.spawn((
            Sprite::from_color(FLOOR_COLOR, Vec2::new(step_width, CORNER_STEP_THICKNESS)),
            Transform::from_xyz(x, y, 0.0),
            Platform,
            CornerRamp,
        ));

        // Fill under the step
        let step_bottom = y - CORNER_STEP_THICKNESS / 2.0;
        let fill_height = step_bottom - floor_top;
        if fill_height > 0.0 {
            let fill_y = floor_top + fill_height / 2.0;
            commands.spawn((
                Sprite::from_color(FLOOR_COLOR, Vec2::new(step_width, fill_height)),
                Transform::from_xyz(x, fill_y, -0.1),
                CornerRamp,
            ));
        }
    }
}

/// Spawn platforms for a specific level
pub fn spawn_level_platforms(commands: &mut Commands, level_db: &LevelDatabase, level_index: usize) {
    let Some(level) = level_db.get(level_index) else {
        warn!("Level {} not found, spawning empty", level_index);
        return;
    };

    for platform in &level.platforms {
        match platform {
            PlatformDef::Mirror { x, y, width } => {
                spawn_mirrored_platform(commands, *x, ARENA_FLOOR_Y + y, *width);
            }
            PlatformDef::Center { y, width } => {
                spawn_center_platform(commands, ARENA_FLOOR_Y + y, *width);
            }
        }
    }
}
