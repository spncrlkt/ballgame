//! Pause menu overlay - PAUSED title with menu options

use bevy::prelude::*;

use super::DebugMenuState;
use crate::scoring::{GamePaused, RestartRequested};
use crate::server::LobbyState;

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
    Characters,
    RestartLevel,
    Lobby,
    Quit,
}

impl PauseMenuOption {
    pub fn label(&self) -> &'static str {
        match self {
            PauseMenuOption::Continue => "Continue",
            PauseMenuOption::Characters => "Characters",
            PauseMenuOption::RestartLevel => "Restart Level",
            PauseMenuOption::Lobby => "Lobby",
            PauseMenuOption::Quit => "Quit",
        }
    }

    pub fn next(&self, has_lobby: bool) -> Self {
        match self {
            PauseMenuOption::Continue => PauseMenuOption::Characters,
            PauseMenuOption::Characters => PauseMenuOption::RestartLevel,
            PauseMenuOption::RestartLevel => {
                if has_lobby {
                    PauseMenuOption::Lobby
                } else {
                    PauseMenuOption::Quit
                }
            }
            PauseMenuOption::Lobby => PauseMenuOption::Quit,
            PauseMenuOption::Quit => PauseMenuOption::Continue,
        }
    }

    pub fn prev(&self, has_lobby: bool) -> Self {
        match self {
            PauseMenuOption::Continue => PauseMenuOption::Quit,
            PauseMenuOption::Characters => PauseMenuOption::Continue,
            PauseMenuOption::RestartLevel => PauseMenuOption::Characters,
            PauseMenuOption::Lobby => PauseMenuOption::RestartLevel,
            PauseMenuOption::Quit => {
                if has_lobby {
                    PauseMenuOption::Lobby
                } else {
                    PauseMenuOption::RestartLevel
                }
            }
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

    // Menu items (Lobby and Characters will be hidden when not in server mode)
    let options = [
        PauseMenuOption::Continue,
        PauseMenuOption::Characters,
        PauseMenuOption::RestartLevel,
        PauseMenuOption::Lobby,
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
    lobby_state: Option<Res<LobbyState>>,
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
    // Hide pause overlay when debug menu is open or lobby is active (game stays paused but UI is hidden)
    let lobby_active = lobby_state.as_ref().map(|l| l.active).unwrap_or(false);
    let show_pause_ui = game_paused.0 && !debug_menu.open && !lobby_active;
    let is_paused = show_pause_ui;
    let has_lobby = lobby_state.is_some();

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
        // Hide Lobby and Characters options when not in server mode
        let should_show = is_paused
            && (menu_item.option != PauseMenuOption::Lobby || has_lobby)
            && (menu_item.option != PauseMenuOption::Characters || has_lobby);

        *vis = if should_show {
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
    lobby_state: Option<Res<LobbyState>>,
    gamepads: Query<&Gamepad>,
    mut menu_state: ResMut<PauseMenuState>,
) {
    // Skip navigation when debug menu is open (it has its own controls)
    if !game_paused.0 || debug_menu.open {
        return;
    }

    // Skip navigation when lobby is active (lobby has its own input handling)
    if lobby_state.as_ref().map(|l| l.active).unwrap_or(false) {
        return;
    }

    let has_lobby = lobby_state.is_some();

    // D-pad navigation
    let up_pressed = gamepads
        .iter()
        .any(|gp| gp.just_pressed(GamepadButton::DPadUp));
    let down_pressed = gamepads
        .iter()
        .any(|gp| gp.just_pressed(GamepadButton::DPadDown));

    if up_pressed {
        menu_state.selected = menu_state.selected.prev(has_lobby);
    } else if down_pressed {
        menu_state.selected = menu_state.selected.next(has_lobby);
    }
}

/// Handle pause menu confirmation (Start or face buttons)
pub fn pause_menu_confirm(
    mut game_paused: ResMut<GamePaused>,
    mut debug_menu: ResMut<DebugMenuState>,
    mut restart_requested: ResMut<RestartRequested>,
    mut menu_state: ResMut<PauseMenuState>,
    mut lobby_state: Option<ResMut<LobbyState>>,
    mut score: ResMut<crate::Score>,
    gamepads: Query<&Gamepad>,
    mut app_exit: MessageWriter<AppExit>,
    server_bridge: Option<Res<crate::server::ServerBridge>>,
) {
    // Skip confirmation when debug menu is open
    if !game_paused.0 || debug_menu.open {
        return;
    }

    // Skip confirmation when lobby is active (lobby has its own input handling)
    if lobby_state.as_ref().map(|l| l.active).unwrap_or(false) {
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
        debug_menu.skip_next_select = true; // Prevent toggle_debug_menu from immediately closing
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
        PauseMenuOption::Characters => {
            // Open lobby to reassign characters, but keep game state
            if let Some(ref mut lobby) = lobby_state {
                lobby.active = true;
                // Don't reset score - just allowing reassignment mid-game
                info!("Opening character assignment from pause menu");
            }
        }
        PauseMenuOption::RestartLevel => {
            restart_requested.0 = true;
            game_paused.0 = false;
            info!("Level restart requested");
        }
        PauseMenuOption::Lobby => {
            if let Some(ref mut lobby) = lobby_state {
                lobby.active = true;
                score.left = 0;
                score.right = 0;
                // Clear ServerAi slots so remote clients can connect
                if let Some(ref bridge) = server_bridge {
                    bridge.runtime.block_on(bridge.slots.clear_server_ai_slots());
                }
                // Keep game paused - lobby will control pause state
                info!("Returning to lobby from pause menu");
            }
        }
        PauseMenuOption::Quit => {
            info!("Quit requested from pause menu");
            app_exit.write(AppExit::Success);
        }
    }

    // Reset selection for next time
    menu_state.selected = PauseMenuOption::default();
}
