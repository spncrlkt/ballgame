//! Match countdown system - displays 3-2-1 before play begins
//!
//! Triggered at game start and after each score.

use bevy::prelude::*;

/// Resource tracking the countdown state
#[derive(Resource)]
pub struct MatchCountdown {
    /// Time remaining in countdown (starts at 3.0)
    pub timer: f32,
    /// Whether countdown is currently active
    pub active: bool,
}

impl Default for MatchCountdown {
    fn default() -> Self {
        Self {
            timer: 3.0,
            active: true, // Start active for game start
        }
    }
}

impl MatchCountdown {
    /// Start a new countdown
    pub fn start(&mut self) {
        self.timer = 3.0;
        self.active = true;
    }

    /// Check if countdown is finished
    pub fn is_finished(&self) -> bool {
        !self.active
    }

    /// Get the current number to display (3, 2, 1, or 0 for "GO!")
    pub fn display_number(&self) -> u32 {
        if self.timer > 2.0 {
            3
        } else if self.timer > 1.0 {
            2
        } else if self.timer > 0.0 {
            1
        } else {
            0
        }
    }
}

/// Marker for the countdown text entity
#[derive(Component)]
pub struct CountdownText;

/// System to update the countdown timer and text display
pub fn update_countdown(
    time: Res<Time>,
    mut countdown: ResMut<MatchCountdown>,
    mut text_query: Query<(&mut Text2d, &mut Visibility, &mut TextColor), With<CountdownText>>,
) {
    if !countdown.active {
        // Hide text when not counting down
        for (_, mut visibility, _) in &mut text_query {
            *visibility = Visibility::Hidden;
        }
        return;
    }

    // Update timer
    countdown.timer -= time.delta_secs();

    // Update text display
    for (mut text, mut visibility, mut color) in &mut text_query {
        *visibility = Visibility::Visible;

        let display = countdown.display_number();
        if display > 0 {
            text.0 = display.to_string();
            // Pulse effect: scale color intensity with timer phase
            let phase = countdown.timer.fract();
            let intensity = 0.7 + 0.3 * (phase * std::f32::consts::PI).sin();
            *color = TextColor(Color::srgba(1.0, intensity, 0.2, 1.0));
        } else {
            text.0 = "GO!".to_string();
            *color = TextColor(Color::srgb(0.2, 1.0, 0.2));
        }
    }

    // End countdown after showing "GO!" briefly
    if countdown.timer < -0.3 {
        countdown.active = false;
    }
}

/// Run condition: game is NOT in countdown
pub fn not_in_countdown(countdown: Res<MatchCountdown>) -> bool {
    !countdown.active
}

/// Run condition: game IS in countdown
pub fn in_countdown(countdown: Res<MatchCountdown>) -> bool {
    countdown.active
}

/// System to trigger countdown when level changes
/// Only runs when MatchCountdown resource exists (not in training mode)
pub fn trigger_countdown_on_level_change(
    current_level: Res<crate::scoring::CurrentLevel>,
    mut countdown: ResMut<MatchCountdown>,
) {
    // Trigger countdown when level changes (level resource is marked changed)
    if current_level.is_changed() {
        countdown.start();
    }
}

/// Spawn the countdown text entity (called from setup)
pub fn spawn_countdown_text(commands: &mut Commands) {
    commands.spawn((
        Text2d::new("3"),
        TextFont {
            font_size: 200.0,
            ..default()
        },
        TextLayout::new_with_justify(bevy::text::Justify::Center),
        TextColor(Color::srgb(1.0, 0.8, 0.2)),
        // Center of screen, high z to render on top
        Transform::from_xyz(0.0, 0.0, 100.0),
        Visibility::Visible,
        CountdownText,
    ));
}
