//! Lobby screen UI for server mode
//!
//! Two-panel layout:
//! - Left panel: Connected inputs (gamepads, remote clients)
//! - Right panel: Character assignments (L0, L1, R0, R1)
//!
//! Press A on a character to open source picker overlay.
//! Note: Keyboard is not supported as a gameplay controller.

use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::events::CharacterId;

use crate::ai::AiProfileDatabase;
use crate::constants::{ARENA_HEIGHT, ARENA_WIDTH, TEXT_PRIMARY};
use crate::countdown::MatchCountdown;
use crate::levels::LevelDatabase;
use crate::scoring::{CurrentLevel, GamePaused};
use crate::server::{
    CharacterAssignment, CharacterAssignments, ConnectedInputs, ConnectedInputType,
    LobbyRow, LobbyState, ServerBridge, SourceOption, TournamentConfig,
};

// UI Layout constants
const LOBBY_Z_LAYER: f32 = 920.0;
const PICKER_Z_LAYER: f32 = 950.0;

const TITLE_FONT_SIZE: f32 = 40.0;
const SECTION_FONT_SIZE: f32 = 24.0;
const ROW_FONT_SIZE: f32 = 22.0;
const OPTION_FONT_SIZE: f32 = 28.0;
const START_FONT_SIZE: f32 = 36.0;

const TITLE_Y: f32 = ARENA_HEIGHT / 2.0 - 40.0;
const SUBTITLE_Y: f32 = TITLE_Y - 30.0;

// Two-panel layout
const LEFT_PANEL_X: f32 = -ARENA_WIDTH / 4.0 - 20.0;
const RIGHT_PANEL_X: f32 = ARENA_WIDTH / 4.0 + 20.0;
const PANEL_TOP_Y: f32 = TITLE_Y - 90.0;
const ROW_HEIGHT: f32 = 32.0;

// Options and start button
const OPTIONS_Y: f32 = -80.0;
const START_Y: f32 = -ARENA_HEIGHT / 2.0 + 80.0;

// Colors
const HIGHLIGHT_COLOR: Color = Color::srgb(1.0, 0.9, 0.2);
const LOCAL_COLOR: Color = Color::srgb(0.2, 0.8, 0.3);
const REMOTE_COLOR: Color = Color::srgb(0.3, 0.6, 1.0);
const REMOTE_FAILING_COLOR: Color = Color::srgb(0.9, 0.3, 0.3);
const AI_COLOR: Color = Color::srgb(0.7, 0.5, 0.8);
const EMPTY_COLOR: Color = Color::srgb(0.5, 0.5, 0.5);
const PICKER_BG_COLOR: Color = Color::srgb(0.1, 0.1, 0.15);

/// Truncate a name to fit in the UI, adding ".." if needed
fn truncate_name(name: &str, max_len: usize) -> String {
    if name.chars().count() <= max_len {
        name.to_string()
    } else {
        let truncated: String = name.chars().take(max_len - 2).collect();
        format!("{}..", truncated)
    }
}

/// Shared marker for all lobby UI elements
#[derive(Component)]
pub struct LobbyElement;

/// Marker for lobby background
#[derive(Component)]
pub struct LobbyBackground;

/// Marker for server info subtitle
#[derive(Component)]
pub struct LobbyServerInfo;

/// Connected input row in left panel
#[derive(Component)]
pub struct ConnectedInputRow {
    pub index: usize,
}

/// Character assignment row in right panel
#[derive(Component)]
pub struct CharacterRow {
    pub character: CharacterId,
}

/// Character assignment text
#[derive(Component)]
pub struct CharacterAssignmentText {
    pub character: CharacterId,
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

/// Start button
#[derive(Component)]
pub struct LobbyStartButton;

/// Source picker overlay background
#[derive(Component)]
pub struct SourcePickerOverlay;

/// Source picker option row
#[derive(Component)]
pub struct SourcePickerRow {
    pub index: usize,
}

/// Spawn the lobby UI
pub fn spawn_lobby_ui(mut commands: Commands, bridge: Res<ServerBridge>) {
    // Dark background
    commands.spawn((
        Sprite {
            color: Color::srgba(0.05, 0.05, 0.08, 1.0),
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
    ));

    // Server info subtitle
    let port = bridge.port();
    commands.spawn((
        Text2d::new(format!("Port {} - 0 clients connected", port)),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::srgb(0.6, 0.6, 0.6)),
        Transform::from_xyz(0.0, SUBTITLE_Y, LOBBY_Z_LAYER + 1.0),
        Visibility::Visible,
        LobbyElement,
        LobbyServerInfo,
    ));

    // Left panel header: CONNECTED INPUTS
    commands.spawn((
        Text2d::new("CONNECTED INPUTS"),
        TextFont {
            font_size: SECTION_FONT_SIZE,
            ..default()
        },
        TextColor(Color::srgb(0.8, 0.8, 0.8)),
        Transform::from_xyz(LEFT_PANEL_X, PANEL_TOP_Y, LOBBY_Z_LAYER + 1.0),
        Visibility::Visible,
        LobbyElement,
    ));

    // Right panel header: CHARACTER ASSIGNMENTS
    commands.spawn((
        Text2d::new("CHARACTER ASSIGNMENTS"),
        TextFont {
            font_size: SECTION_FONT_SIZE,
            ..default()
        },
        TextColor(Color::srgb(0.8, 0.8, 0.8)),
        Transform::from_xyz(RIGHT_PANEL_X, PANEL_TOP_Y, LOBBY_Z_LAYER + 1.0),
        Visibility::Visible,
        LobbyElement,
    ));

    // Connected inputs rows (initially empty, updated by system)
    for i in 0..8 {
        let y = PANEL_TOP_Y - 40.0 - (i as f32 * ROW_HEIGHT);
        commands.spawn((
            Text2d::new(""),
            TextFont {
                font_size: ROW_FONT_SIZE,
                ..default()
            },
            TextColor(Color::srgb(0.7, 0.7, 0.7)),
            Transform::from_xyz(LEFT_PANEL_X, y, LOBBY_Z_LAYER + 1.0),
            Visibility::Visible,
            LobbyElement,
            ConnectedInputRow { index: i },
        ));
    }

    // Character assignment rows
    let characters = [CharacterId::L0, CharacterId::L1, CharacterId::R0, CharacterId::R1];
    for (i, &character) in characters.iter().enumerate() {
        let y = PANEL_TOP_Y - 40.0 - (i as f32 * ROW_HEIGHT * 1.5);
        let team_label = match character {
            CharacterId::L0 | CharacterId::L1 => "(Team Left)",
            CharacterId::R0 | CharacterId::R1 => "(Team Right)",
        };

        // Character label
        commands.spawn((
            Text2d::new(format!("[{}]", character)),
            TextFont {
                font_size: ROW_FONT_SIZE,
                ..default()
            },
            TextColor(TEXT_PRIMARY),
            Transform::from_xyz(RIGHT_PANEL_X - 120.0, y, LOBBY_Z_LAYER + 1.0),
            Visibility::Visible,
            LobbyElement,
            CharacterRow { character },
        ));

        // Assignment text
        commands.spawn((
            Text2d::new("EMPTY - press A"),
            TextFont {
                font_size: ROW_FONT_SIZE,
                ..default()
            },
            TextColor(EMPTY_COLOR),
            Transform::from_xyz(RIGHT_PANEL_X, y, LOBBY_Z_LAYER + 1.0),
            Visibility::Visible,
            LobbyElement,
            CharacterAssignmentText { character },
        ));

        // Team label
        commands.spawn((
            Text2d::new(team_label),
            TextFont {
                font_size: 16.0,
                ..default()
            },
            TextColor(Color::srgb(0.5, 0.5, 0.5)),
            Transform::from_xyz(RIGHT_PANEL_X + 140.0, y, LOBBY_Z_LAYER + 1.0),
            Visibility::Visible,
            LobbyElement,
        ));
    }

    // Option rows
    let options = [
        (LobbyRow::Level, "Level"),
        (LobbyRow::ScoreLimit, "Score Limit"),
        (LobbyRow::TimeLimit, "Time Limit"),
    ];

    for (i, (row, label)) in options.iter().enumerate() {
        let y = OPTIONS_Y - (i as f32 * 40.0);

        commands.spawn((
            Text2d::new(*label),
            TextFont {
                font_size: OPTION_FONT_SIZE,
                ..default()
            },
            TextColor(TEXT_PRIMARY),
            TextLayout::new_with_justify(bevy::text::Justify::Right),
            Anchor::CENTER_RIGHT,
            Transform::from_xyz(-10.0, y, LOBBY_Z_LAYER + 1.0),
            Visibility::Visible,
            LobbyElement,
            LobbyOptionLabel { row: *row },
        ));

        commands.spawn((
            Text2d::new("---"),
            TextFont {
                font_size: OPTION_FONT_SIZE,
                ..default()
            },
            TextColor(HIGHLIGHT_COLOR),
            TextLayout::new_with_justify(bevy::text::Justify::Left),
            Anchor::CENTER_LEFT,
            Transform::from_xyz(10.0, y, LOBBY_Z_LAYER + 1.0),
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
        TextColor(Color::srgb(0.6, 0.6, 0.6)),
        Transform::from_xyz(0.0, START_Y, LOBBY_Z_LAYER + 1.0),
        Visibility::Visible,
        LobbyElement,
        LobbyStartButton,
    ));

    // Instructions
    commands.spawn((
        Text2d::new("Up/Down: Navigate | A: Assign | Left/Right: Adjust | Start: Begin Match"),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::srgb(0.5, 0.5, 0.5)),
        Transform::from_xyz(0.0, START_Y - 40.0, LOBBY_Z_LAYER + 1.0),
        Visibility::Visible,
        LobbyElement,
    ));

    // Source picker overlay (initially hidden)
    spawn_source_picker(&mut commands);
}

/// Spawn the source picker overlay (initially hidden)
fn spawn_source_picker(commands: &mut Commands) {
    // Overlay background
    commands.spawn((
        Sprite {
            color: PICKER_BG_COLOR,
            custom_size: Some(Vec2::new(350.0, 400.0)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, PICKER_Z_LAYER),
        Visibility::Hidden,
        LobbyElement,
        SourcePickerOverlay,
    ));

    // Picker rows (up to 10 options)
    for i in 0..10 {
        let y = 150.0 - (i as f32 * 35.0);
        commands.spawn((
            Text2d::new(""),
            TextFont {
                font_size: 22.0,
                ..default()
            },
            TextColor(Color::WHITE),
            Transform::from_xyz(0.0, y, PICKER_Z_LAYER + 1.0),
            Visibility::Hidden,
            LobbyElement,
            SourcePickerRow { index: i },
        ));
    }
}

/// Update server info subtitle
pub fn update_lobby_server_info(
    bridge: Res<ServerBridge>,
    mut info_query: Query<&mut Text2d, With<LobbyServerInfo>>,
) {
    let client_count = bridge.runtime.block_on(bridge.broadcaster.client_count());
    let port = bridge.port();

    for mut text in &mut info_query {
        text.0 = format!(
            "Port {} - {} client{} connected",
            port,
            client_count,
            if client_count == 1 { "" } else { "s" }
        );
    }
}

/// Update connected inputs display
pub fn update_connected_inputs_display(
    connected: Res<ConnectedInputs>,
    mut row_query: Query<(&ConnectedInputRow, &mut Text2d, &mut TextColor, &mut Visibility)>,
) {
    for (row, mut text, mut color, mut vis) in &mut row_query {
        if let Some(input) = connected.inputs.get(row.index) {
            let assignment_str = match &input.assigned_to {
                Some(char_id) => format!(" -> {}", char_id),
                None => " (free)".to_string(),
            };
            text.0 = format!("{}{}", input.display_name, assignment_str);

            // Set color based on input type and connection health
            *color = TextColor(match &input.input_type {
                ConnectedInputType::Gamepad { .. } => LOCAL_COLOR,
                ConnectedInputType::RemoteClient { .. } => {
                    if input.health.is_failing() {
                        REMOTE_FAILING_COLOR
                    } else {
                        REMOTE_COLOR
                    }
                }
            });

            *vis = Visibility::Visible;
        } else {
            text.0.clear();
            *vis = Visibility::Hidden;
        }
    }
}

/// Update character assignment display
pub fn update_character_assignments_display(
    assignments: Res<CharacterAssignments>,
    mut text_query: Query<(&CharacterAssignmentText, &mut Text2d, &mut TextColor)>,
) {
    for (char_text, mut text, mut color) in &mut text_query {
        let assignment = assignments.get(char_text.character);
        text.0 = assignment.display_name();
        *color = TextColor(match assignment {
            CharacterAssignment::Unassigned => EMPTY_COLOR,
            CharacterAssignment::Local { .. } => LOCAL_COLOR,
            CharacterAssignment::Remote { .. } => REMOTE_COLOR,
            CharacterAssignment::ServerAi { .. } => AI_COLOR,
        });
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
            LobbyRow::Level => level_db
                .get_by_id(&current_level.0)
                .map(|l| l.name.clone())
                .unwrap_or_else(|| "Unknown".to_string()),
            LobbyRow::ScoreLimit => tournament_config
                .score_limit
                .map(|s| format!("First to {}", s))
                .unwrap_or_else(|| "Unlimited".to_string()),
            LobbyRow::TimeLimit => tournament_config
                .time_limit_secs
                .map(|t| format!("{:.0}s", t))
                .unwrap_or_else(|| "Unlimited".to_string()),
            _ => continue,
        };
    }
}

/// Handle navigation input in lobby (main menu, not picker)
pub fn lobby_navigation(
    mut lobby_state: ResMut<LobbyState>,
    gamepads: Query<&Gamepad>,
) {
    // Don't navigate if picker is open
    if lobby_state.in_picker_mode() {
        return;
    }

    let up = gamepads
        .iter()
        .any(|gp| gp.just_pressed(GamepadButton::DPadUp));

    let down = gamepads
        .iter()
        .any(|gp| gp.just_pressed(GamepadButton::DPadDown));

    if up {
        lobby_state.selected_row = lobby_state.selected_row.prev();
    } else if down {
        lobby_state.selected_row = lobby_state.selected_row.next();
    }
}

/// Handle character selection (open picker)
pub fn lobby_open_picker(
    mut lobby_state: ResMut<LobbyState>,
    connected: Res<ConnectedInputs>,
    profile_db: Res<AiProfileDatabase>,
    gamepads: Query<&Gamepad>,
) {
    // Only open picker on character rows
    if !lobby_state.selected_row.is_character() || lobby_state.in_picker_mode() {
        return;
    }

    let select = gamepads
        .iter()
        .any(|gp| gp.just_pressed(GamepadButton::South));

    if !select {
        return;
    }

    if let Some(character) = lobby_state.selected_row.character_id() {
        // Build options list
        let mut options = Vec::new();

        // Unassigned option
        options.push(SourceOption::Unassigned);

        // Local inputs (gamepads only - keyboard not supported)
        for input in &connected.inputs {
            match &input.input_type {
                ConnectedInputType::Gamepad { source_id, .. } => {
                    options.push(SourceOption::Gamepad {
                        source_id: *source_id,
                        name: input.display_name.clone(),
                    });
                }
                ConnectedInputType::RemoteClient { client_id } => {
                    options.push(SourceOption::Remote {
                        client_id: *client_id,
                        name: input.display_name.clone(),
                    });
                }
            }
        }

        // Single AI option (L/R cycles through profiles when selected)
        let default_profile = profile_db.default_profile().name.clone();
        options.push(SourceOption::Ai {
            profile_name: default_profile,
        });

        lobby_state.source_picker.open_for(character, options);
    }
}

/// Pending remote reassignment that needs async processing
#[derive(Resource, Default)]
pub struct PendingRemoteReassignment {
    pub pending: Option<(u64, crate::events::CharacterId, String)>, // (client_id, target_character, name)
}

/// Handle picker navigation and selection
pub fn lobby_picker_input(
    mut lobby_state: ResMut<LobbyState>,
    mut assignments: ResMut<CharacterAssignments>,
    mut pending_reassignment: ResMut<PendingRemoteReassignment>,
    profile_db: Res<AiProfileDatabase>,
    gamepads: Query<&Gamepad>,
) {
    if !lobby_state.in_picker_mode() {
        return;
    }

    // Skip selection on the frame the picker opens (same A press that opened it)
    let just_opened = lobby_state.source_picker.just_opened;
    lobby_state.source_picker.just_opened = false;

    let up = gamepads
        .iter()
        .any(|gp| gp.just_pressed(GamepadButton::DPadUp));

    let down = gamepads
        .iter()
        .any(|gp| gp.just_pressed(GamepadButton::DPadDown));

    let left = gamepads
        .iter()
        .any(|gp| gp.just_pressed(GamepadButton::DPadLeft));

    let right = gamepads
        .iter()
        .any(|gp| gp.just_pressed(GamepadButton::DPadRight));

    let select = gamepads
        .iter()
        .any(|gp| gp.just_pressed(GamepadButton::South));

    let cancel = gamepads
        .iter()
        .any(|gp| gp.just_pressed(GamepadButton::East));

    if up {
        lobby_state.source_picker.select_prev();
    } else if down {
        lobby_state.source_picker.select_next();
    } else if left || right {
        // Cycle AI profile when an AI option is selected
        let picker = &mut lobby_state.source_picker;
        if let Some(SourceOption::Ai { profile_name }) = picker.selected_option_mut() {
            let profiles = profile_db.profiles();
            if !profiles.is_empty() {
                let current_idx = profiles.iter().position(|p| p.name == *profile_name).unwrap_or(0);
                let delta = if right { 1i32 } else { -1i32 };
                let new_idx = (current_idx as i32 + delta).rem_euclid(profiles.len() as i32) as usize;
                *profile_name = profiles[new_idx].name.clone();
            }
        }
    } else if cancel {
        lobby_state.source_picker.close();
    } else if select && !just_opened {
        // Apply the selection (skip if picker just opened this frame)
        if let Some(character) = lobby_state.source_picker.target_character {
            if let Some(option) = lobby_state.source_picker.selected_option().cloned() {
                match option {
                    SourceOption::Unassigned => {
                        assignments.unassign(character);
                    }
                    SourceOption::Gamepad { source_id, name } => {
                        assignments.assign_local(character, source_id, name);
                    }
                    SourceOption::Remote { client_id, name } => {
                        // Queue the remote reassignment for async processing
                        pending_reassignment.pending = Some((client_id, character, name));
                    }
                    SourceOption::Ai { profile_name } => {
                        assignments.assign_ai(character, profile_name);
                    }
                }
            }
        }
        lobby_state.source_picker.close();
    }
}

/// Process pending remote client reassignments
/// This system handles the async slot manager and broadcaster updates
pub fn process_remote_reassignments(
    mut pending: ResMut<PendingRemoteReassignment>,
    mut assignments: ResMut<CharacterAssignments>,
    bridge: Res<ServerBridge>,
) {
    if let Some((client_id, target_character, name)) = pending.pending.take() {
        let new_slot = target_character.to_slot_index();

        // Perform the assignment/reassignment
        bridge.runtime.block_on(async {
            // Check if client is waiting (not yet in any slot)
            let is_waiting = bridge.slots.check_waiting_assignment(client_id).await.is_none()
                && bridge.slots.find_by_client_id(client_id).await.is_none();

            if is_waiting {
                // Client is waiting - assign them to the slot
                if bridge.slots.assign_waiting_to_slot(client_id, new_slot).await {
                    info!("Assigned waiting client {} to slot {}", client_id, new_slot);
                    // Session's periodic assignment check will detect this and send SlotAssigned
                } else {
                    warn!("Failed to assign waiting client {} to slot {}", client_id, new_slot);
                }
            } else {
                // Client already has a slot - reassign them
                if let Some(old_slot) = bridge.slots.reassign_remote(client_id, new_slot).await {
                    // Reassign the broadcaster channel
                    bridge.broadcaster.reassign_channel(old_slot, new_slot).await;

                    // Send SlotAssigned message to the client
                    let tick = bridge.current_tick();
                    let protocol_char = ballgame_protocol::CharacterId::from_slot_index(new_slot);
                    if let Some(character) = protocol_char {
                        bridge.broadcaster.send_to(
                            new_slot,
                            tick,
                            ballgame_protocol::ServerPayload::SlotAssigned { character },
                        ).await;
                    }

                    info!("Reassigned client {} from slot {} to slot {}", client_id, old_slot, new_slot);
                } else {
                    warn!("Failed to reassign client {} to slot {}", client_id, new_slot);
                }
            }
        });

        // Update assignments resource
        assignments.assign_remote(target_character, client_id, name);
    }
}

/// Handle value adjustment in lobby (Level, ScoreLimit, TimeLimit)
pub fn lobby_adjust_value(
    lobby_state: Res<LobbyState>,
    level_db: Res<LevelDatabase>,
    mut current_level: ResMut<CurrentLevel>,
    mut tournament_config: ResMut<TournamentConfig>,
    gamepads: Query<&Gamepad>,
) {
    if lobby_state.in_picker_mode() {
        return;
    }

    let left = gamepads
        .iter()
        .any(|gp| gp.just_pressed(GamepadButton::DPadLeft));

    let right = gamepads
        .iter()
        .any(|gp| gp.just_pressed(GamepadButton::DPadRight));

    if !left && !right {
        return;
    }

    let delta = if right { 1i32 } else { -1i32 };

    match lobby_state.selected_row {
        LobbyRow::Level => {
            let levels = level_db.all();
            if let Some(current_idx) = levels.iter().position(|l| l.id == current_level.0) {
                let new_idx =
                    (current_idx as i32 + delta).rem_euclid(levels.len() as i32) as usize;
                current_level.0 = levels[new_idx].id.clone();
            }
        }
        LobbyRow::ScoreLimit => {
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
        _ => {}
    }
}

/// Handle start game action
pub fn lobby_start_game(
    mut lobby_state: ResMut<LobbyState>,
    mut assignments: ResMut<CharacterAssignments>,
    mut countdown: ResMut<MatchCountdown>,
    mut tournament_config: ResMut<TournamentConfig>,
    mut game_paused: ResMut<GamePaused>,
    bridge: Res<ServerBridge>,
    current_level: Res<CurrentLevel>,
    gamepads: Query<&Gamepad>,
) {
    if lobby_state.selected_row != LobbyRow::StartGame || lobby_state.in_picker_mode() {
        return;
    }

    let start_pressed = gamepads.iter().any(|gp| {
        gp.just_pressed(GamepadButton::Start) || gp.just_pressed(GamepadButton::South)
    });

    if !start_pressed || !lobby_state.can_start {
        return;
    }

    info!("Starting game from lobby");

    // Fill unassigned characters with Dummy AI (stands still)
    let dummy_profile = "Dummy".to_string();
    assignments.fill_with_ai(&dummy_profile);

    // Prepare match (timer starts when countdown ends)
    tournament_config.prepare_match();

    // Fill slots from assignments (sync with SlotManager)
    bridge.runtime.block_on(bridge.slots.fill_empty_with_ai(&dummy_profile));

    // Broadcast MatchStarting
    let level_id = current_level.0.clone();
    bridge.runtime.block_on(async {
        let tick = bridge.current_tick();
        bridge
            .broadcaster
            .broadcast(
                tick,
                ballgame_protocol::ServerPayload::MatchStarting {
                    level_id,
                    countdown_secs: 3.0,
                },
            )
            .await;
    });

    lobby_state.active = false;
    game_paused.0 = false;
    countdown.start();
}

/// Update visual highlighting
pub fn update_lobby_highlights(
    time: Res<Time>,
    mut lobby_state: ResMut<LobbyState>,
    mut char_row_query: Query<(&CharacterRow, &mut TextColor), (Without<LobbyOptionLabel>, Without<LobbyStartButton>)>,
    mut label_query: Query<(&LobbyOptionLabel, &mut TextColor), (Without<CharacterRow>, Without<LobbyStartButton>)>,
    mut start_query: Query<(&mut Transform, &mut TextColor), (With<LobbyStartButton>, Without<CharacterRow>, Without<LobbyOptionLabel>)>,
) {
    lobby_state.pulse_timer += time.delta_secs() * 3.0;
    let pulse = 1.0 + 0.1 * lobby_state.pulse_timer.sin();

    // Highlight character rows
    for (char_row, mut color) in &mut char_row_query {
        let is_selected = lobby_state.selected_row.character_id() == Some(char_row.character);
        *color = if is_selected {
            TextColor(HIGHLIGHT_COLOR)
        } else {
            TextColor(TEXT_PRIMARY)
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

    // Highlight start button
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

/// Update source picker overlay visibility and content
pub fn update_source_picker(
    lobby_state: Res<LobbyState>,
    mut overlay_query: Query<&mut Visibility, With<SourcePickerOverlay>>,
    mut row_query: Query<
        (&SourcePickerRow, &mut Text2d, &mut TextColor, &mut Visibility),
        Without<SourcePickerOverlay>,
    >,
) {
    let picker = &lobby_state.source_picker;
    let picker_visible = picker.open;

    // Update overlay visibility
    for mut vis in &mut overlay_query {
        *vis = if picker_visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    // Update picker rows
    for (row, mut text, mut color, mut vis) in &mut row_query {
        if picker_visible && row.index < picker.options.len() {
            let option = &picker.options[row.index];
            text.0 = match option {
                SourceOption::Unassigned => "[ Clear ]".to_string(),
                SourceOption::Gamepad { name, .. } => truncate_name(name, 20),
                SourceOption::Remote { name, .. } => format!("{} (remote)", truncate_name(name, 14)),
                SourceOption::Ai { profile_name } => format!("AI: {}", truncate_name(profile_name, 16)),
            };

            let is_selected = row.index == picker.selected_index;
            *color = if is_selected {
                TextColor(HIGHLIGHT_COLOR)
            } else {
                TextColor(Color::WHITE)
            };
            *vis = Visibility::Visible;
        } else {
            text.0.clear();
            *vis = Visibility::Hidden;
        }
    }
}

/// Update lobby UI visibility
/// Note: Excludes picker elements - those are controlled by update_source_picker
pub fn update_lobby_visibility(
    lobby_state: Res<LobbyState>,
    mut query: Query<
        &mut Visibility,
        (With<LobbyElement>, Without<SourcePickerOverlay>, Without<SourcePickerRow>),
    >,
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
pub fn sync_lobby_pause(lobby_state: Res<LobbyState>, mut game_paused: ResMut<GamePaused>) {
    if lobby_state.active && !game_paused.0 {
        game_paused.0 = true;
    }
}

// Keep old update_lobby_slots for backward compatibility (now simplified)
pub fn update_lobby_slots(
    _bridge: Res<ServerBridge>,
    _card_query: Query<()>,
    _status_query: Query<()>,
    _profile_query: Query<()>,
) {
    // This is now handled by update_character_assignments_display
}
