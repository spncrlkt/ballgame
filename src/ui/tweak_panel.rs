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

/// Toggle tweak panel visibility and handle input
pub fn toggle_tweak_panel(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut tweaks: ResMut<PhysicsTweaks>,
    mut panel_state: ResMut<TweakPanelState>,
    mut panel_query: Query<&mut Visibility, With<TweakPanel>>,
) {
    // F1 toggles panel visibility
    if keyboard.just_pressed(KeyCode::F1) {
        panel_state.panel_visible = !panel_state.panel_visible;
        if let Ok(mut visibility) = panel_query.single_mut() {
            *visibility = if panel_state.panel_visible {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }

    // Only process input when panel is visible
    if !panel_state.panel_visible {
        return;
    }

    let num_params = PhysicsTweaks::LABELS.len();

    // Up/Down to select parameter
    if keyboard.just_pressed(KeyCode::ArrowUp) {
        panel_state.selected_index = (panel_state.selected_index + num_params - 1) % num_params;
    }
    if keyboard.just_pressed(KeyCode::ArrowDown) {
        panel_state.selected_index = (panel_state.selected_index + 1) % num_params;
    }

    // Left/Right to adjust value (10% increments)
    let idx = panel_state.selected_index;
    let step = tweaks.get_step(idx);
    if keyboard.just_pressed(KeyCode::ArrowLeft) {
        let current = tweaks.get_value(idx);
        tweaks.set_value(idx, (current - step).max(0.01));
    }
    if keyboard.just_pressed(KeyCode::ArrowRight) {
        let current = tweaks.get_value(idx);
        tweaks.set_value(idx, current + step);
    }

    // R to reset selected parameter to default
    if keyboard.just_pressed(KeyCode::KeyR) {
        if keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight) {
            // Shift+R resets ALL parameters
            tweaks.reset_all();
        } else {
            // R resets just the selected parameter
            tweaks.reset_value(idx);
        }
    }
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
