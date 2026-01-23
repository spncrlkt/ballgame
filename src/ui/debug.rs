//! Debug UI components and systems

use bevy::prelude::*;

use crate::levels::LevelDatabase;
use crate::scoring::CurrentLevel;
use crate::shooting::LastShotInfo;
use crate::steal::StealContest;
use crate::world::Basket;

/// Debug settings resource
#[derive(Resource)]
pub struct DebugSettings {
    pub visible: bool,
}

impl Default for DebugSettings {
    fn default() -> Self {
        Self { visible: true }
    }
}

/// Debug text component
#[derive(Component)]
pub struct DebugText;

/// Marker for the debug level style key UI
#[derive(Component)]
pub struct DebugStyleKey;

/// Toggle debug UI visibility
pub fn toggle_debug(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<DebugSettings>,
    mut text_query: Query<&mut Visibility, With<DebugText>>,
) {
    if keyboard.just_pressed(KeyCode::Tab) {
        settings.visible = !settings.visible;
        if let Ok(mut visibility) = text_query.single_mut() {
            *visibility = if settings.visible {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }
}

/// Update debug text display
pub fn update_debug_text(
    debug_settings: Res<DebugSettings>,
    shot_info: Res<LastShotInfo>,
    steal_contest: Res<StealContest>,
    mut text_query: Query<&mut Text2d, With<DebugText>>,
) {
    if !debug_settings.visible {
        return;
    }

    let Ok(mut text) = text_query.single_mut() else {
        return;
    };

    let steal_str = if steal_contest.active {
        format!(
            " | Steal: A:{} D:{} ({:.1}s)",
            steal_contest.attacker_presses, steal_contest.defender_presses, steal_contest.timer
        )
    } else {
        String::new()
    };

    // Show last shot info
    if shot_info.target.is_some() {
        let target_str = match shot_info.target {
            Some(Basket::Left) => "Left",
            Some(Basket::Right) => "Right",
            None => "?",
        };
        **text = format!(
            "Last Shot: {:.0}deg {:.0}u/s | Variance: base {:.0}% + air {:.0}% + move {:.0}% + dist {:.0}% = {:.0}% | Req speed: {:.0} | Target: {}{}",
            shot_info.angle_degrees,
            shot_info.speed,
            shot_info.base_variance * 100.0,
            shot_info.air_penalty * 100.0,
            shot_info.move_penalty * 100.0,
            shot_info.distance_variance * 100.0,
            shot_info.total_variance * 100.0,
            shot_info.required_speed,
            target_str,
            steal_str,
        );
    } else {
        **text = format!("No shots yet{}", steal_str);
    }
}

/// Show/hide the style key based on whether we're on the Debug level
pub fn update_style_key_visibility(
    current_level: Res<CurrentLevel>,
    level_db: Res<LevelDatabase>,
    mut query: Query<&mut Visibility, With<DebugStyleKey>>,
) {
    if !current_level.is_changed() {
        return;
    }

    let is_debug = level_db
        .get(current_level.0.saturating_sub(1) as usize)
        .map(|l| l.name == "Debug")
        .unwrap_or(false);

    for mut visibility in &mut query {
        *visibility = if is_debug {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}
