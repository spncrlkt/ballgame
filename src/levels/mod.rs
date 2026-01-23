//! Levels module - database, spawning, and hot reload

mod database;
mod spawning;

pub use database::*;
pub use spawning::*;

use bevy::prelude::*;

use crate::ball::CurrentPalette;
use crate::constants::LEVELS_FILE;
use crate::helpers::basket_x_from_offset;
use crate::palettes::PaletteDatabase;
use crate::scoring::CurrentLevel;
use crate::world::{Basket, CornerRamp, LevelPlatform};

/// Hot reload levels from file (F2)
pub fn reload_levels(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut level_db: ResMut<LevelDatabase>,
    palette_db: Res<PaletteDatabase>,
    current_level: Res<CurrentLevel>,
    current_palette: Res<CurrentPalette>,
    level_platforms: Query<Entity, With<LevelPlatform>>,
    corner_ramps: Query<Entity, With<CornerRamp>>,
    mut baskets: Query<(&mut Transform, &Basket)>,
) {
    if !keyboard.just_pressed(KeyCode::F2) {
        return;
    }

    // Reload level database from file
    *level_db = LevelDatabase::load_from_file(LEVELS_FILE);
    info!("Reloaded levels from {}", LEVELS_FILE);

    let level_index = (current_level.0 - 1) as usize;
    let palette = palette_db
        .get(current_palette.0)
        .expect("Palette index out of bounds");

    // Despawn old level platforms
    for entity in &level_platforms {
        commands.entity(entity).despawn();
    }

    // Despawn old corner ramps
    for entity in &corner_ramps {
        commands.entity(entity).despawn();
    }

    // Spawn new level geometry
    spawn_level_platforms(&mut commands, &level_db, level_index, palette.platforms);

    // Update basket positions and spawn corner ramps
    if let Some(level) = level_db.get(level_index) {
        let basket_y = crate::constants::ARENA_FLOOR_Y + level.basket_height;
        let (left_x, right_x) = basket_x_from_offset(level.basket_push_in);
        for (mut basket_transform, basket) in &mut baskets {
            basket_transform.translation.y = basket_y;
            basket_transform.translation.x = match basket {
                Basket::Left => left_x,
                Basket::Right => right_x,
            };
        }

        spawn_corner_ramps(
            &mut commands,
            level.step_count,
            level.corner_height,
            level.corner_width,
            level.step_push_in,
            palette.platforms,
        );
    }
}
