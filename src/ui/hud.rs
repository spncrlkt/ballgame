//! HUD components and systems (score/level display)

use bevy::prelude::*;

use crate::scoring::Score;

/// Score and level text component
#[derive(Component)]
pub struct ScoreLevelText;

/// Update score display
pub fn update_score_level_text(
    score: Res<Score>,
    mut text_query: Query<&mut Text2d, With<ScoreLevelText>>,
) {
    let Ok(mut text) = text_query.single_mut() else {
        return;
    };

    **text = format!("{} - {}", score.left, score.right);
}
