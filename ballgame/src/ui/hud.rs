//! HUD components and systems (score/level display, character indicators)

use bevy::prelude::*;

use crate::constants::*;
use crate::events::CharacterId;
use crate::palettes::PaletteDatabase;
use crate::player::{Character, HumanControlled, Player, Team};
use crate::scoring::Score;
use crate::CurrentPalette;

/// Score and level text component
#[derive(Component)]
pub struct ScoreLevelText;

/// Match timer text component
#[derive(Component)]
pub struct MatchTimerText;

/// Character indicator label (shows ID above player)
#[derive(Component)]
pub struct CharacterIndicator {
    /// The character this indicator follows
    pub character_id: CharacterId,
}

/// Vertical offset for character indicator above player
const INDICATOR_Y_OFFSET: f32 = 45.0;

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

/// Update match timer display (only in server mode with tournament config)
pub fn update_match_timer_text(
    tournament_config: Option<Res<crate::server::TournamentConfig>>,
    mut text_query: Query<(&mut Text2d, &mut Visibility), With<MatchTimerText>>,
) {
    let Ok((mut text, mut vis)) = text_query.single_mut() else {
        return;
    };

    if let Some(config) = tournament_config {
        if config.match_active {
            **text = config.format_time_display();
            *vis = Visibility::Visible;
        } else {
            *vis = Visibility::Hidden;
        }
    } else {
        *vis = Visibility::Hidden;
    }
}

/// Spawn character indicators for all players
pub fn spawn_character_indicators(
    mut commands: Commands,
    players: Query<(Entity, &Character, &Team, Option<&HumanControlled>), Added<Character>>,
    palette_db: Res<PaletteDatabase>,
    current_palette: Res<CurrentPalette>,
) {
    let palette = palette_db
        .get(current_palette.0)
        .unwrap_or_else(|| palette_db.get(0).expect("No palettes"));

    for (_entity, character, team, human) in &players {
        let char_id = character.0;

        // Build indicator text: ID + human marker
        let human_marker = if human.is_some() { "*" } else { "" };
        let label = format!("{}{}", char_id, human_marker);

        // Team color for indicator
        let color = match team {
            Team::Left => palette.left,
            Team::Right => palette.right,
        };

        commands.spawn((
            Text2d::new(label),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextColor(color),
            Transform::from_xyz(0.0, 0.0, 10.0), // Will be positioned by update system
            CharacterIndicator { character_id: char_id },
        ));
    }
}

/// Update character indicator positions to follow players
pub fn update_character_indicators(
    players: Query<(&Transform, &Character, Option<&HumanControlled>), With<Player>>,
    mut indicators: Query<(&mut Transform, &mut Text2d, &CharacterIndicator), Without<Player>>,
) {
    for (mut indicator_transform, mut text, indicator) in &mut indicators {
        // Find the player with matching character ID
        for (player_transform, character, human) in &players {
            if character.0 == indicator.character_id {
                // Position indicator above player
                indicator_transform.translation.x = player_transform.translation.x;
                indicator_transform.translation.y =
                    player_transform.translation.y + PLAYER_SIZE.y / 2.0 + INDICATOR_Y_OFFSET;

                // Update human marker if control changed
                let human_marker = if human.is_some() { "*" } else { "" };
                let label = format!("{}{}", indicator.character_id, human_marker);
                **text = label;

                break;
            }
        }
    }
}

/// Update indicator colors when palette changes
pub fn update_indicator_colors(
    palette_db: Res<PaletteDatabase>,
    current_palette: Res<CurrentPalette>,
    players: Query<(&Character, &Team), With<Player>>,
    mut indicators: Query<(&CharacterIndicator, &mut TextColor)>,
) {
    if !current_palette.is_changed() {
        return;
    }

    let palette = palette_db
        .get(current_palette.0)
        .unwrap_or_else(|| palette_db.get(0).expect("No palettes"));

    for (indicator, mut color) in &mut indicators {
        // Find player's team for this indicator
        for (character, team) in &players {
            if character.0 == indicator.character_id {
                *color = TextColor(match team {
                    Team::Left => palette.left,
                    Team::Right => palette.right,
                });
                break;
            }
        }
    }
}
