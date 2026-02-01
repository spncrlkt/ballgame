//! Lobby screen UI for server mode
//!
//! Displays slot status, configuration options, and start button
//! while waiting for clients to connect.

use bevy::prelude::*;

use crate::constants::{ARENA_HEIGHT, TEXT_PRIMARY};
use crate::countdown::MatchCountdown;
use crate::levels::LevelDatabase;
use crate::scoring::{CurrentLevel, GamePaused};
use crate::server::{LobbyRow, LobbyState, ServerBridge, SlotDisplay, TournamentConfig};
use crate::ai::AiProfileDatabase;

// UI Layout constants
const LOBBY_Z_LAYER: f32 = 920.0;
const BACKGROUND_ALPHA: f32 = 1.0;

const TITLE_FONT_SIZE: f32 = 48.0;
const SUBTITLE_FONT_SIZE: f32 = 24.0;
const SLOT_FONT_SIZE: f32 = 28.0;
const OPTION_FONT_SIZE: f32 = 32.0;
const START_FONT_SIZE: f32 = 40.0;

const TITLE_Y: f32 = ARENA_HEIGHT / 2.0 - 60.0;
const SUBTITLE_Y: f32 = TITLE_Y - 40.0;
const SLOTS_Y: f32 = TITLE_Y - 140.0;
const SLOT_SPACING_X: f32 = 180.0;
const OPTIONS_START_Y: f32 = SLOTS_Y - 180.0;
const OPTION_SPACING_Y: f32 = 50.0;
const START_BUTTON_Y: f32 = OPTIONS_START_Y - 180.0;

const SLOT_CARD_WIDTH: f32 = 150.0;
const SLOT_CARD_HEIGHT: f32 = 120.0;

const HIGHLIGHT_COLOR: Color = Color::srgb(1.0, 0.9, 0.2);
const SLOT_LOCAL_COLOR: Color = Color::srgb(0.2, 0.8, 0.3);
const SLOT_REMOTE_COLOR: Color = Color::srgb(0.3, 0.6, 1.0);
const SLOT_AI_COLOR: Color = Color::srgb(0.7, 0.5, 0.8);
const SLOT_EMPTY_COLOR: Color = Color::srgb(0.4, 0.4, 0.4);

/// Shared marker for all lobby UI elements (for visibility toggling)
#[derive(Component)]
pub struct LobbyElement;

/// Marker for lobby background
#[derive(Component)]
pub struct LobbyBackground;

/// Marker for lobby title
#[derive(Component)]
pub struct LobbyTitle;

/// Marker for server info subtitle
#[derive(Component)]
pub struct LobbyServerInfo;

/// Slot card background
#[derive(Component)]
pub struct LobbySlotCard {
    pub slot_id: u8,
}

/// Slot status text (LOCAL, REMOTE, AI, EMPTY)
#[derive(Component)]
pub struct LobbySlotStatus {
    pub slot_id: u8,
}

/// Slot AI profile text (shown for AI slots)
#[derive(Component)]
pub struct LobbySlotProfile {
    pub slot_id: u8,
}

/// Option row label
#[derive(Component)]
pub struct LobbyOptionLabel {
    pub row: LobbyRow,
}

/// Option row value
#[derive(Component)]
pub struct LobbyOptionValue {
    pub row: LobbyRow,
}

/// Start button text
#[derive(Component)]
pub struct LobbyStartButton;

/// Spawn the lobby UI (called at startup in server mode)
pub fn spawn_lobby_ui(mut commands: Commands, bridge: Res<ServerBridge>) {
    // Semi-transparent dark background
    commands.spawn((
        Sprite {
            color: Color::srgba(0.0, 0.0, 0.0, BACKGROUND_ALPHA),
            custom_size: Some(Vec2::new(2000.0, 2000.0)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, LOBBY_Z_LAYER),
        Visibility::Visible,
        LobbyElement,
        LobbyBackground,
    ));

    // Title
    commands.spawn((
        Text2d::new("SERVER LOBBY"),
        TextFont {
            font_size: TITLE_FONT_SIZE,
            ..default()
        },
        TextColor(TEXT_PRIMARY),
        Transform::from_xyz(0.0, TITLE_Y, LOBBY_Z_LAYER + 1.0),
        Visibility::Visible,
        LobbyElement,
        LobbyTitle,
    ));

    // Server info subtitle
    let port = bridge.port();
    commands.spawn((
        Text2d::new(format!("Port {} - 0 clients connected", port)),
        TextFont {
            font_size: SUBTITLE_FONT_SIZE,
            ..default()
        },
        TextColor(Color::srgb(0.7, 0.7, 0.7)),
        Transform::from_xyz(0.0, SUBTITLE_Y, LOBBY_Z_LAYER + 1.0),
        Visibility::Visible,
        LobbyElement,
        LobbyServerInfo,
    ));

    // Slot cards (4 slots across)
    for slot_id in 0..4u8 {
        let x = (slot_id as f32 - 1.5) * SLOT_SPACING_X;

        // Slot card background
        commands.spawn((
            Sprite {
                color: SLOT_EMPTY_COLOR,
                custom_size: Some(Vec2::new(SLOT_CARD_WIDTH, SLOT_CARD_HEIGHT)),
                ..default()
            },
            Transform::from_xyz(x, SLOTS_Y, LOBBY_Z_LAYER + 1.0),
            Visibility::Visible,
            LobbyElement,
            LobbySlotCard { slot_id },
        ));

        // Slot number label
        commands.spawn((
            Text2d::new(format!("Slot {}", slot_id)),
            TextFont {
                font_size: 18.0,
                ..default()
            },
            TextColor(Color::WHITE),
            Transform::from_xyz(x, SLOTS_Y + 40.0, LOBBY_Z_LAYER + 2.0),
            Visibility::Visible,
            LobbyElement,
        ));

        // Slot status text
        commands.spawn((
            Text2d::new("EMPTY"),
            TextFont {
                font_size: SLOT_FONT_SIZE,
                ..default()
            },
            TextColor(Color::WHITE),
            Transform::from_xyz(x, SLOTS_Y + 10.0, LOBBY_Z_LAYER + 2.0),
            Visibility::Visible,
            LobbyElement,
            LobbySlotStatus { slot_id },
        ));

        // Slot AI profile text (initially hidden)
        commands.spawn((
            Text2d::new(""),
            TextFont {
                font_size: 20.0,
                ..default()
            },
            TextColor(Color::srgb(0.8, 0.8, 0.8)),
            Transform::from_xyz(x, SLOTS_Y - 25.0, LOBBY_Z_LAYER + 2.0),
            Visibility::Visible,
            LobbyElement,
            LobbySlotProfile { slot_id },
        ));
    }

    // Option rows
    let options = [
        (LobbyRow::Level, "Level"),
        (LobbyRow::ScoreLimit, "Score Limit"),
        (LobbyRow::TimeLimit, "Time Limit"),
    ];

    for (i, (row, label)) in options.iter().enumerate() {
        let y = OPTIONS_START_Y - (i as f32) * OPTION_SPACING_Y;

        // Label
        commands.spawn((
            Text2d::new(*label),
            TextFont {
                font_size: OPTION_FONT_SIZE,
                ..default()
            },
            TextColor(TEXT_PRIMARY),
            TextLayout::new_with_justify(bevy::text::Justify::Right),
            Transform::from_xyz(-100.0, y, LOBBY_Z_LAYER + 1.0),
            Visibility::Visible,
            LobbyElement,
            LobbyOptionLabel { row: *row },
        ));

        // Value
        commands.spawn((
            Text2d::new("---"),
            TextFont {
                font_size: OPTION_FONT_SIZE,
                ..default()
            },
            TextColor(HIGHLIGHT_COLOR),
            TextLayout::new_with_justify(bevy::text::Justify::Left),
            Transform::from_xyz(100.0, y, LOBBY_Z_LAYER + 1.0),
            Visibility::Visible,
            LobbyElement,
            LobbyOptionValue { row: *row },
        ));
    }

    // Start button
    commands.spawn((
        Text2d::new(">>> START GAME <<<"),
        TextFont {
            font_size: START_FONT_SIZE,
            ..default()
        },
        TextColor(HIGHLIGHT_COLOR),
        Transform::from_xyz(0.0, START_BUTTON_Y, LOBBY_Z_LAYER + 1.0),
        Visibility::Visible,
        LobbyElement,
        LobbyStartButton,
    ));

    // Instructions
    commands.spawn((
        Text2d::new("Up/Down: Navigate   Left/Right: Adjust   Start/Enter: Begin Match"),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::srgb(0.6, 0.6, 0.6)),
        Transform::from_xyz(0.0, START_BUTTON_Y - 50.0, LOBBY_Z_LAYER + 1.0),
        Visibility::Visible,
        LobbyElement,
    ));
}

/// Update slot card displays based on current slot state
pub fn update_lobby_slots(
    bridge: Res<ServerBridge>,
    mut card_query: Query<(&LobbySlotCard, &mut Sprite)>,
    mut status_query: Query<(&LobbySlotStatus, &mut Text2d)>,
    mut profile_query: Query<(&LobbySlotProfile, &mut Text2d, &mut Visibility), Without<LobbySlotStatus>>,
) {
    // Get slot displays from the bridge
    let displays = bridge.runtime.block_on(bridge.slots.get_all_slot_displays());

    // Update slot cards
    for (card, mut sprite) in &mut card_query {
        let display = &displays[card.slot_id as usize];
        sprite.color = match display {
            SlotDisplay::Local => SLOT_LOCAL_COLOR,
            SlotDisplay::Remote { .. } => SLOT_REMOTE_COLOR,
            SlotDisplay::ServerAi { .. } => SLOT_AI_COLOR,
            SlotDisplay::Empty => SLOT_EMPTY_COLOR,
        };
    }

    // Update status text
    for (status, mut text) in &mut status_query {
        let display = &displays[status.slot_id as usize];
        text.0 = match display {
            SlotDisplay::Local => "LOCAL".to_string(),
            SlotDisplay::Remote { name } => name.clone(),
            SlotDisplay::ServerAi { .. } => "AI".to_string(),
            SlotDisplay::Empty => "EMPTY".to_string(),
        };
    }

    // Update profile text
    for (profile, mut text, mut vis) in &mut profile_query {
        let display = &displays[profile.slot_id as usize];
        match display {
            SlotDisplay::ServerAi { profile: p } => {
                text.0 = format!("[{}]", p);
                *vis = Visibility::Visible;
            }
            SlotDisplay::Empty => {
                text.0 = "[click to set AI]".to_string();
                *vis = Visibility::Visible;
            }
            _ => {
                text.0.clear();
                *vis = Visibility::Hidden;
            }
        }
    }
}

/// Update server info subtitle with client count
pub fn update_lobby_server_info(
    bridge: Res<ServerBridge>,
    mut info_query: Query<&mut Text2d, With<LobbyServerInfo>>,
) {
    let client_count = bridge.runtime.block_on(bridge.broadcaster.client_count());
    let port = bridge.port();

    for mut text in &mut info_query {
        text.0 = format!("Port {} - {} client{} connected",
            port,
            client_count,
            if client_count == 1 { "" } else { "s" }
        );
    }
}

/// Update option value displays
pub fn update_lobby_options(
    current_level: Res<CurrentLevel>,
    level_db: Res<LevelDatabase>,
    tournament_config: Res<TournamentConfig>,
    mut value_query: Query<(&LobbyOptionValue, &mut Text2d)>,
) {
    for (option, mut text) in &mut value_query {
        text.0 = match option.row {
            LobbyRow::Level => {
                level_db
                    .get_by_id(&current_level.0)
                    .map(|l| l.name.clone())
                    .unwrap_or_else(|| "Unknown".to_string())
            }
            LobbyRow::ScoreLimit => {
                tournament_config
                    .score_limit
                    .map(|s| format!("First to {}", s))
                    .unwrap_or_else(|| "Unlimited".to_string())
            }
            LobbyRow::TimeLimit => {
                tournament_config
                    .time_limit_secs
                    .map(|t| format!("{:.0}s", t))
                    .unwrap_or_else(|| "Unlimited".to_string())
            }
            _ => continue,
        };
    }
}

/// Handle navigation input in lobby
pub fn lobby_navigation(
    mut lobby_state: ResMut<LobbyState>,
    keyboard: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
) {
    let up = keyboard.just_pressed(KeyCode::ArrowUp)
        || keyboard.just_pressed(KeyCode::KeyW)
        || gamepads.iter().any(|gp| gp.just_pressed(GamepadButton::DPadUp));

    let down = keyboard.just_pressed(KeyCode::ArrowDown)
        || keyboard.just_pressed(KeyCode::KeyS)
        || gamepads.iter().any(|gp| gp.just_pressed(GamepadButton::DPadDown));

    if up {
        lobby_state.selected_row = lobby_state.selected_row.prev();
    } else if down {
        lobby_state.selected_row = lobby_state.selected_row.next();
    }
}

/// Handle value adjustment in lobby
pub fn lobby_adjust_value(
    lobby_state: Res<LobbyState>,
    bridge: Res<ServerBridge>,
    level_db: Res<LevelDatabase>,
    profile_db: Res<AiProfileDatabase>,
    mut current_level: ResMut<CurrentLevel>,
    mut tournament_config: ResMut<TournamentConfig>,
    keyboard: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
) {
    let left = keyboard.just_pressed(KeyCode::ArrowLeft)
        || keyboard.just_pressed(KeyCode::KeyA)
        || gamepads.iter().any(|gp| gp.just_pressed(GamepadButton::DPadLeft));

    let right = keyboard.just_pressed(KeyCode::ArrowRight)
        || keyboard.just_pressed(KeyCode::KeyD)
        || gamepads.iter().any(|gp| gp.just_pressed(GamepadButton::DPadRight));

    if !left && !right {
        return;
    }

    let delta = if right { 1i32 } else { -1i32 };

    match lobby_state.selected_row {
        LobbyRow::Slot0 | LobbyRow::Slot1 | LobbyRow::Slot2 | LobbyRow::Slot3 => {
            // Cycle AI profile for this slot
            if let Some(slot_id) = lobby_state.selected_row.slot_index() {
                let profiles = profile_db.profiles();
                if !profiles.is_empty() {
                    // Get current profile and find next
                    let display = bridge.runtime.block_on(bridge.slots.get_slot_display(slot_id));
                    let current_profile = match display {
                        SlotDisplay::ServerAi { profile } => profile,
                        SlotDisplay::Empty => profiles[0].name.clone(),
                        _ => return, // Can't change AI for Local/Remote slots
                    };

                    let current_idx = profiles
                        .iter()
                        .position(|p| p.name == current_profile)
                        .unwrap_or(0);

                    let new_idx = (current_idx as i32 + delta).rem_euclid(profiles.len() as i32) as usize;
                    let new_profile = profiles[new_idx].name.clone();

                    bridge.runtime.block_on(bridge.slots.set_ai_profile(slot_id, new_profile));
                }
            }
        }
        LobbyRow::Level => {
            // Cycle through levels
            let levels = level_db.all();
            if let Some(current_idx) = levels.iter().position(|l| l.id == current_level.0) {
                let new_idx = (current_idx as i32 + delta).rem_euclid(levels.len() as i32) as usize;
                current_level.0 = levels[new_idx].id.clone();
            }
        }
        LobbyRow::ScoreLimit => {
            // Cycle through score limits: None, 3, 5, 7, 10
            let limits = [None, Some(3), Some(5), Some(7), Some(10)];
            let current_idx = limits
                .iter()
                .position(|l| *l == tournament_config.score_limit)
                .unwrap_or(0);
            let new_idx = (current_idx as i32 + delta).rem_euclid(limits.len() as i32) as usize;
            tournament_config.score_limit = limits[new_idx];
            tournament_config.enabled = tournament_config.score_limit.is_some()
                || tournament_config.time_limit_secs.is_some();
        }
        LobbyRow::TimeLimit => {
            // Cycle through time limits: None, 60, 120, 180, 300
            let limits = [None, Some(60.0), Some(120.0), Some(180.0), Some(300.0)];
            let current_idx = limits
                .iter()
                .position(|l| *l == tournament_config.time_limit_secs)
                .unwrap_or(0);
            let new_idx = (current_idx as i32 + delta).rem_euclid(limits.len() as i32) as usize;
            tournament_config.time_limit_secs = limits[new_idx];
            tournament_config.enabled = tournament_config.score_limit.is_some()
                || tournament_config.time_limit_secs.is_some();
        }
        LobbyRow::StartGame => {
            // No adjustment for start button
        }
    }
}

/// Handle start game action
pub fn lobby_start_game(
    mut lobby_state: ResMut<LobbyState>,
    mut countdown: ResMut<MatchCountdown>,
    mut tournament_config: ResMut<TournamentConfig>,
    mut game_paused: ResMut<GamePaused>,
    time: Res<Time>,
    bridge: Res<ServerBridge>,
    current_level: Res<CurrentLevel>,
    profile_db: Res<AiProfileDatabase>,
    keyboard: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
) {
    // Only respond when on the StartGame row
    if lobby_state.selected_row != LobbyRow::StartGame {
        return;
    }

    let start_pressed = keyboard.just_pressed(KeyCode::Enter)
        || keyboard.just_pressed(KeyCode::Space)
        || gamepads.iter().any(|gp| {
            gp.just_pressed(GamepadButton::Start)
                || gp.just_pressed(GamepadButton::South)
        });

    if !start_pressed || !lobby_state.can_start {
        return;
    }

    info!("Starting game from lobby");

    // Record match start time for time limit tracking
    tournament_config.start_match(time.elapsed_secs());

    // Fill empty slots with AI before starting
    let default_profile = profile_db.default_profile().name.clone();
    bridge.runtime.block_on(bridge.slots.fill_empty_with_ai(&default_profile));

    // Broadcast MatchStarting to clients
    let level_id = current_level.0.clone();
    bridge.runtime.block_on(async {
        let tick = bridge.current_tick();
        bridge.broadcaster.broadcast(
            tick,
            ballgame_protocol::ServerPayload::MatchStarting {
                level_id,
                countdown_secs: 3.0,
            },
        ).await;
    });

    // Deactivate lobby and unpause game
    lobby_state.active = false;
    game_paused.0 = false;

    // Start countdown
    countdown.start();
}

/// Update visual highlighting based on selection
pub fn update_lobby_highlights(
    time: Res<Time>,
    mut lobby_state: ResMut<LobbyState>,
    mut card_query: Query<(&LobbySlotCard, &mut Transform), Without<LobbyStartButton>>,
    mut label_query: Query<(&LobbyOptionLabel, &mut TextColor), Without<LobbyStartButton>>,
    mut start_query: Query<(&mut Transform, &mut TextColor), With<LobbyStartButton>>,
) {
    // Update pulse timer
    lobby_state.pulse_timer += time.delta_secs() * 3.0;
    let pulse = 1.0 + 0.1 * lobby_state.pulse_timer.sin();

    // Highlight slot cards
    for (card, mut transform) in &mut card_query {
        let is_selected = lobby_state.selected_row.slot_index() == Some(card.slot_id);
        transform.scale = if is_selected {
            Vec3::splat(pulse)
        } else {
            Vec3::ONE
        };
    }

    // Highlight option labels
    for (label, mut color) in &mut label_query {
        let is_selected = lobby_state.selected_row == label.row;
        *color = if is_selected {
            TextColor(HIGHLIGHT_COLOR)
        } else {
            TextColor(TEXT_PRIMARY)
        };
    }

    // Highlight and pulse start button
    for (mut transform, mut color) in &mut start_query {
        let is_selected = lobby_state.selected_row == LobbyRow::StartGame;
        if is_selected {
            transform.scale = Vec3::splat(pulse);
            *color = TextColor(HIGHLIGHT_COLOR);
        } else {
            transform.scale = Vec3::ONE;
            *color = TextColor(Color::srgb(0.6, 0.6, 0.6));
        }
    }
}

/// Update lobby UI visibility based on lobby state
pub fn update_lobby_visibility(
    lobby_state: Res<LobbyState>,
    mut query: Query<&mut Visibility, With<LobbyElement>>,
) {
    let vis = if lobby_state.active {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };

    for mut v in &mut query {
        *v = vis;
    }
}

/// Keep game paused while lobby is active
pub fn sync_lobby_pause(
    lobby_state: Res<LobbyState>,
    mut game_paused: ResMut<GamePaused>,
) {
    // Ensure game stays paused while in lobby
    if lobby_state.active && !game_paused.0 {
        game_paused.0 = true;
    }
}
