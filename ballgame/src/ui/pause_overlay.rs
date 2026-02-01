//! Pause menu overlay - PAUSED title with menu options

use bevy::prelude::*;

use super::DebugMenuState;
use crate::scoring::{GamePaused, RestartRequested};

/// Font sizes
const TITLE_FONT_SIZE: f32 = 120.0;
const MENU_FONT_SIZE: f32 = 96.0; // 20% smaller than title

/// Vertical spacing
const TITLE_Y: f32 = 100.0;
const MENU_START_Y: f32 = -50.0;
const MENU_ITEM_SPACING: f32 = 80.0;

/// Shadow offset for text
const SHADOW_OFFSET: Vec3 = Vec3::new(3.0, -3.0, -0.5);

/// Menu options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PauseMenuOption {
    #[default]
    Continue,
    RestartLevel,
    Quit,
}

impl PauseMenuOption {
    pub fn label(&self) -> &'static str {
        match self {
            PauseMenuOption::Continue => "Continue",
            PauseMenuOption::RestartLevel => "Restart Level",
            PauseMenuOption::Quit => "Quit",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            PauseMenuOption::Continue => PauseMenuOption::RestartLevel,
            PauseMenuOption::RestartLevel => PauseMenuOption::Quit,
            PauseMenuOption::Quit => PauseMenuOption::Continue,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            PauseMenuOption::Continue => PauseMenuOption::Quit,
            PauseMenuOption::RestartLevel => PauseMenuOption::Continue,
            PauseMenuOption::Quit => PauseMenuOption::RestartLevel,
        }
    }
}

/// Current pause menu state
#[derive(Resource, Default)]
pub struct PauseMenuState {
    pub selected: PauseMenuOption,
    pub pulse_timer: f32,
}

/// Marker component for the pause overlay background
#[derive(Component)]
pub struct PauseBackground;

/// Marker component for the PAUSED title (both layers)
#[derive(Component)]
pub struct PauseTitle;

/// Marker for title foreground (white, pulses)
#[derive(Component)]
pub struct PauseTitleForeground;

/// Marker component for menu item text
#[derive(Component)]
pub struct PauseMenuItem {
    pub option: PauseMenuOption,
    pub is_shadow: bool,
}

/// Spawn the pause overlay (hidden initially)
pub fn spawn_pause_overlay(mut commands: Commands) {
    // Semi-transparent dark background
    commands.spawn((
        Sprite {
            color: Color::srgba(0.0, 0.0, 0.0, 0.7),
            custom_size: Some(Vec2::new(2000.0, 2000.0)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 900.0),
        Visibility::Hidden,
        PauseBackground,
    ));

    // "PAUSED" title - shadow layer (red)
    commands.spawn((
        Text2d::new("PAUSED"),
        TextFont {
            font_size: TITLE_FONT_SIZE,
            ..default()
        },
        TextColor(Color::srgb(0.8, 0.0, 0.0)),
        Transform::from_xyz(SHADOW_OFFSET.x, TITLE_Y + SHADOW_OFFSET.y, 901.0),
        Visibility::Hidden,
        PauseTitle,
    ));

    // "PAUSED" title - foreground layer (white, pulses)
    commands.spawn((
        Text2d::new("PAUSED"),
        TextFont {
            font_size: TITLE_FONT_SIZE,
            ..default()
        },
        TextColor(Color::WHITE),
        Transform::from_xyz(0.0, TITLE_Y, 902.0),
        Visibility::Hidden,
        PauseTitle,
        PauseTitleForeground,
    ));

    // Menu items
    let options = [
        PauseMenuOption::Continue,
        PauseMenuOption::RestartLevel,
        PauseMenuOption::Quit,
    ];

    for (i, option) in options.iter().enumerate() {
        let y = MENU_START_Y - (i as f32 * MENU_ITEM_SPACING);

        // Shadow layer (red)
        commands.spawn((
            Text2d::new(option.label()),
            TextFont {
                font_size: MENU_FONT_SIZE,
                ..default()
            },
            TextColor(Color::srgb(0.8, 0.0, 0.0)),
            Transform::from_xyz(SHADOW_OFFSET.x, y + SHADOW_OFFSET.y, 903.0),
            Visibility::Hidden,
            PauseMenuItem {
                option: *option,
                is_shadow: true,
            },
        ));

        // Foreground layer (white)
        commands.spawn((
            Text2d::new(option.label()),
            TextFont {
                font_size: MENU_FONT_SIZE,
                ..default()
            },
            TextColor(Color::WHITE),
            Transform::from_xyz(0.0, y, 904.0),
            Visibility::Hidden,
            PauseMenuItem {
                option: *option,
                is_shadow: false,
            },
        ));
    }
}

/// Update pause overlay visibility and animations
pub fn update_pause_overlay(
    game_paused: Res<GamePaused>,
    debug_menu: Res<DebugMenuState>,
    time: Res<Time>,
    mut menu_state: ResMut<PauseMenuState>,
    mut bg_query: Query<&mut Visibility, With<PauseBackground>>,
    mut title_query: Query<
        (&mut Visibility, &mut Transform, Option<&PauseTitleForeground>),
        (With<PauseTitle>, Without<PauseMenuItem>, Without<PauseBackground>),
    >,
    mut menu_query: Query<
        (&mut Visibility, &mut Transform, &PauseMenuItem),
        (Without<PauseTitle>, Without<PauseBackground>),
    >,
) {
    // Hide pause overlay when debug menu is open (game stays paused but UI is hidden)
    let show_pause_ui = game_paused.0 && !debug_menu.open;
    let is_paused = show_pause_ui;

    // Update pulse timer
    if is_paused {
        menu_state.pulse_timer += time.delta_secs() * 3.0;
    }

    // Update background visibility
    for mut vis in &mut bg_query {
        *vis = if is_paused {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    // Update title visibility and animation
    for (mut vis, mut transform, is_foreground) in &mut title_query {
        *vis = if is_paused {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };

        // Pulse the foreground title
        if is_paused && is_foreground.is_some() {
            let scale_factor = 1.0 + 0.1 * menu_state.pulse_timer.sin();
            transform.scale = Vec3::splat(scale_factor);
        }
    }

    // Update menu items visibility and animation
    for (mut vis, mut transform, menu_item) in &mut menu_query {
        *vis = if is_paused {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };

        if is_paused {
            let is_selected = menu_item.option == menu_state.selected;

            if is_selected && !menu_item.is_shadow {
                // Pulse selected item (foreground only)
                let scale_factor = 1.0 + 0.1 * menu_state.pulse_timer.sin();
                transform.scale = Vec3::splat(scale_factor);
            } else {
                // Reset scale for unselected items
                transform.scale = Vec3::ONE;
            }
        }
    }
}

/// Handle pause menu navigation (D-pad or left stick)
pub fn pause_menu_navigation(
    game_paused: Res<GamePaused>,
    debug_menu: Res<DebugMenuState>,
    gamepads: Query<&Gamepad>,
    mut menu_state: ResMut<PauseMenuState>,
) {
    // Skip navigation when debug menu is open (it has its own controls)
    if !game_paused.0 || debug_menu.open {
        return;
    }

    // D-pad navigation
    let up_pressed = gamepads
        .iter()
        .any(|gp| gp.just_pressed(GamepadButton::DPadUp));
    let down_pressed = gamepads
        .iter()
        .any(|gp| gp.just_pressed(GamepadButton::DPadDown));

    if up_pressed {
        menu_state.selected = menu_state.selected.prev();
    } else if down_pressed {
        menu_state.selected = menu_state.selected.next();
    }
}

/// Handle pause menu confirmation (Start or face buttons)
pub fn pause_menu_confirm(
    mut game_paused: ResMut<GamePaused>,
    mut debug_menu: ResMut<DebugMenuState>,
    mut restart_requested: ResMut<RestartRequested>,
    mut menu_state: ResMut<PauseMenuState>,
    gamepads: Query<&Gamepad>,
    mut app_exit: MessageWriter<AppExit>,
) {
    // Skip confirmation when debug menu is open
    if !game_paused.0 || debug_menu.open {
        return;
    }

    // Skip confirmation on the frame pause was just enabled
    // (prevents Start press from immediately closing the menu)
    if game_paused.is_changed() {
        return;
    }

    // Select button switches to debug menu
    let select_pressed = gamepads
        .iter()
        .any(|gp| gp.just_pressed(GamepadButton::Select));

    if select_pressed {
        debug_menu.open = true;
        // Keep game paused - debug menu will show
        info!("Pause menu closed, debug menu opened");
        return;
    }

    // Check for confirm input (Start or any face button)
    let confirm_pressed = gamepads.iter().any(|gp| {
        gp.just_pressed(GamepadButton::Start)
            || gp.just_pressed(GamepadButton::South)
            || gp.just_pressed(GamepadButton::East)
            || gp.just_pressed(GamepadButton::West)
            || gp.just_pressed(GamepadButton::North)
    });

    if !confirm_pressed {
        return;
    }

    match menu_state.selected {
        PauseMenuOption::Continue => {
            game_paused.0 = false;
            info!("Game RESUMED");
        }
        PauseMenuOption::RestartLevel => {
            restart_requested.0 = true;
            game_paused.0 = false;
            info!("Level restart requested");
        }
        PauseMenuOption::Quit => {
            info!("Quit requested from pause menu");
            app_exit.write(AppExit::Success);
        }
    }

    // Reset selection for next time
    menu_state.selected = PauseMenuOption::default();
}
