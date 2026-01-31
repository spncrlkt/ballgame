//! Physics tweak panel UI components and systems

use bevy::prelude::*;

use crate::tuning::PhysicsTweaks;

/// UI state for the tweak panel (selection/visibility only)
#[derive(Resource, Default)]
pub struct TweakPanelState {
    pub selected_index: usize,
    pub panel_visible: bool,
}

/// Tweak panel container component
#[derive(Component)]
pub struct TweakPanel;

/// Tweak row component with index
#[derive(Component)]
pub struct TweakRow(pub usize);

/// Toggle tweak panel visibility and handle input (dev tool - no keyboard shortcuts)
pub fn toggle_tweak_panel(
    _keyboard: Res<ButtonInput<KeyCode>>,
    _tweaks: ResMut<PhysicsTweaks>,
    _panel_state: ResMut<TweakPanelState>,
    _panel_query: Query<&mut Visibility, With<TweakPanel>>,
) {
    // Keyboard shortcuts removed - tweak panel disabled for controller-only gameplay
}

/// Update tweak panel display
pub fn update_tweak_panel(
    tweaks: Res<PhysicsTweaks>,
    panel_state: Res<TweakPanelState>,
    mut row_query: Query<(&mut Text, &mut TextColor, &TweakRow)>,
) {
    if !panel_state.panel_visible {
        return;
    }

    for (mut text, mut color, row) in &mut row_query {
        let value = tweaks.get_value(row.0);
        let label = PhysicsTweaks::LABELS[row.0];
        let is_modified = tweaks.is_modified(row.0);

        // Format based on value type:
        // - Indices 5, 7, 9: decel/bounce values (0-1 range) → 2 decimals
        // - Indices 10, 11: friction values (small decimals) → 4 decimals
        // - Index 13: charge time → 1 decimal with "s" suffix
        // - Others: velocities/accelerations → 0 decimals
        let value_str = match row.0 {
            5 | 7 | 9 => format!("{:.2}", value), // Decel/bounce (0-1)
            10 | 11 => format!("{:.4}", value),   // Friction (small)
            13 => format!("{:.1}s", value),       // Charge time
            _ => format!("{:.0}", value),         // Velocities
        };

        text.0 = format!("{}: {}", label, value_str);

        // Color priority: selected (yellow) > modified (red) > default (white)
        if row.0 == panel_state.selected_index {
            color.0 = Color::srgb(1.0, 1.0, 0.0); // Yellow for selected
        } else if is_modified {
            color.0 = Color::srgb(1.0, 0.4, 0.4); // Red for modified
        } else {
            color.0 = Color::WHITE;
        }
    }
}
