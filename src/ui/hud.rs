//! HUD components and systems (score/level display)

use bevy::prelude::*;

use crate::levels::LevelDatabase;
use crate::scoring::{CurrentLevel, Score};

/// Score and level text component
#[derive(Component)]
pub struct ScoreLevelText;

/// Update score and level display
pub fn update_score_level_text(
    score: Res<Score>,
    current_level: Res<CurrentLevel>,
    level_db: Res<LevelDatabase>,
    mut text_query: Query<&mut Text, With<ScoreLevelText>>,
) {
    let Ok(mut text) = text_query.single_mut() else {
        return;
    };

    let level_index = (current_level.0 - 1) as usize;
    let level_name = level_db
        .get(level_index)
        .map(|l| l.name.as_str())
        .unwrap_or("???");
    let num_levels = level_db.len();

    text.0 = format!(
        "Lv {}/{}: {}  |  {} - {}",
        current_level.0, num_levels, level_name, score.left, score.right,
    );
}
