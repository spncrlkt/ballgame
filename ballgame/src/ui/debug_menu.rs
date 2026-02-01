//! Debug menu - unified settings menu accessible via Tab/Select
//!
//! Replaces: tweak panel, cycle indicator, shot debug text

use bevy::prelude::*;

use crate::ai::{AiProfileDatabase, AiState};
use crate::ball::{Ball, BallStyle, BallTextures, CurrentPalette};
use crate::events::CharacterId;
use crate::levels::LevelDatabase;
use crate::palettes::PaletteDatabase;
use crate::player::{Character, HumanControlled, Player};
use crate::presets::{apply_composite_preset, CurrentPresets, PresetDatabase};
use crate::scoring::{CurrentLevel, GamePaused};
use crate::settings::CurrentSettings;
use crate::ui::{PauseMenuState, ViewportScale};

// =============================================================================
// CONSTANTS
// =============================================================================

/// Z-layers for debug menu
const DEBUG_MENU_BG_Z: f32 = 950.0;
const DEBUG_MENU_TEXT_Z: f32 = 951.0;

/// Font sizes
const MENU_FONT_SIZE: f32 = 24.0;

/// Spacing
const MENU_ROW_SPACING: f32 = 32.0;
const MENU_START_Y: f32 = 150.0;
const MENU_X: f32 = -300.0;

/// Colors
const MENU_BG_COLOR: Color = Color::srgba(0.0, 0.0, 0.0, 0.85);
const SELECTED_COLOR: Color = Color::srgb(1.0, 0.9, 0.2);
const UNSELECTED_COLOR: Color = Color::srgb(0.9, 0.9, 0.9);

// =============================================================================
// MENU OPTIONS
// =============================================================================

/// All available debug menu options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugMenuOption {
    Viewport,
    Character,
    Level,
    Palette,
    BallStyle,
    AiL0,
    AiL1,
    AiR0,
    AiR1,
    CompositePreset,
    MovementPreset,
    BallPreset,
    ShootingPreset,
}

impl DebugMenuOption {
    pub const ALL: [DebugMenuOption; 13] = [
        DebugMenuOption::Viewport,
        DebugMenuOption::Character,
        DebugMenuOption::Level,
        DebugMenuOption::Palette,
        DebugMenuOption::BallStyle,
        DebugMenuOption::AiL0,
        DebugMenuOption::AiL1,
        DebugMenuOption::AiR0,
        DebugMenuOption::AiR1,
        DebugMenuOption::CompositePreset,
        DebugMenuOption::MovementPreset,
        DebugMenuOption::BallPreset,
        DebugMenuOption::ShootingPreset,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            DebugMenuOption::Viewport => "Viewport",
            DebugMenuOption::Character => "Character",
            DebugMenuOption::Level => "Level",
            DebugMenuOption::Palette => "Palette",
            DebugMenuOption::BallStyle => "Ball Style",
            DebugMenuOption::AiL0 => "AI L0",
            DebugMenuOption::AiL1 => "AI L1",
            DebugMenuOption::AiR0 => "AI R0",
            DebugMenuOption::AiR1 => "AI R1",
            DebugMenuOption::CompositePreset => "Composite",
            DebugMenuOption::MovementPreset => "Movement",
            DebugMenuOption::BallPreset => "Ball",
            DebugMenuOption::ShootingPreset => "Shooting",
        }
    }
}

// =============================================================================
// RESOURCES AND COMPONENTS
// =============================================================================

/// Debug menu state
#[derive(Resource, Default)]
pub struct DebugMenuState {
    pub open: bool,
    pub selected_row: usize,
    /// Pending action: Some(true) = cycle forward, Some(false) = cycle backward
    pub pending_cycle: Option<bool>,
    /// Pending character cycle: Some(true) = forward, Some(false) = backward
    pub pending_character_cycle: Option<bool>,
}

/// Marker for debug menu background
#[derive(Component)]
pub struct DebugMenuBackground;

/// Marker for debug menu row
#[derive(Component)]
pub struct DebugMenuRow {
    pub index: usize,
}

/// Marker for value picker overlay background (unused in simplified version)
#[derive(Component)]
pub struct ValuePickerOverlay;

/// Marker for value picker item (unused in simplified version)
#[derive(Component)]
pub struct ValuePickerItem {
    pub index: usize,
}

// =============================================================================
// SPAWN SYSTEM
// =============================================================================

/// Spawn the debug menu UI (hidden initially)
pub fn spawn_debug_menu(mut commands: Commands) {
    // Semi-transparent dark background
    commands.spawn((
        Sprite {
            color: MENU_BG_COLOR,
            custom_size: Some(Vec2::new(400.0, 500.0)),
            ..default()
        },
        Transform::from_xyz(MENU_X + 150.0, 0.0, DEBUG_MENU_BG_Z),
        Visibility::Hidden,
        DebugMenuBackground,
    ));

    // Menu rows (one per option)
    for (i, option) in DebugMenuOption::ALL.iter().enumerate() {
        let y = MENU_START_Y - (i as f32 * MENU_ROW_SPACING);

        commands.spawn((
            Text2d::new(format!("  {}: ---", option.label())),
            TextFont {
                font_size: MENU_FONT_SIZE,
                ..default()
            },
            TextLayout::new_with_justify(Justify::Left),
            TextColor(UNSELECTED_COLOR),
            Transform::from_xyz(MENU_X, y, DEBUG_MENU_TEXT_Z),
            Visibility::Hidden,
            DebugMenuRow { index: i },
        ));
    }
}

// =============================================================================
// TOGGLE SYSTEM
// =============================================================================

/// Toggle debug menu open/close with Tab or Select button
pub fn toggle_debug_menu(
    keyboard: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut menu_state: ResMut<DebugMenuState>,
    mut game_paused: ResMut<GamePaused>,
    pause_menu: Res<PauseMenuState>,
) {
    // Don't toggle if pause menu is open (they're mutually exclusive)
    if game_paused.0 && !menu_state.open {
        return;
    }

    let toggle_pressed = keyboard.just_pressed(KeyCode::Tab)
        || gamepads
            .iter()
            .any(|gp| gp.just_pressed(GamepadButton::Select));

    if toggle_pressed {
        menu_state.open = !menu_state.open;

        // Pause game when menu opens, unpause when it closes
        // But only if pause menu isn't active
        if !pause_menu.selected.label().is_empty() || menu_state.open {
            game_paused.0 = menu_state.open;
        }

        if menu_state.open {
            info!("Debug menu opened");
        } else {
            info!("Debug menu closed");
        }
    }
}

// =============================================================================
// NAVIGATION SYSTEM (simplified - row navigation only)
// =============================================================================

/// Handle menu row navigation (Up/Down) and cycle triggers (Left/Right)
pub fn debug_menu_navigation(
    gamepads: Query<&Gamepad>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut menu_state: ResMut<DebugMenuState>,
) {
    if !menu_state.open {
        return;
    }

    // Up/Down navigation
    let up_pressed = keyboard.just_pressed(KeyCode::ArrowUp)
        || gamepads
            .iter()
            .any(|gp| gp.just_pressed(GamepadButton::DPadUp));
    let down_pressed = keyboard.just_pressed(KeyCode::ArrowDown)
        || gamepads
            .iter()
            .any(|gp| gp.just_pressed(GamepadButton::DPadDown));

    let num_options = DebugMenuOption::ALL.len();
    if up_pressed {
        menu_state.selected_row = (menu_state.selected_row + num_options - 1) % num_options;
    } else if down_pressed {
        menu_state.selected_row = (menu_state.selected_row + 1) % num_options;
    }

    // Left/Right sets pending cycle action
    let left_pressed = keyboard.just_pressed(KeyCode::ArrowLeft)
        || gamepads
            .iter()
            .any(|gp| gp.just_pressed(GamepadButton::DPadLeft));
    let right_pressed = keyboard.just_pressed(KeyCode::ArrowRight)
        || gamepads
            .iter()
            .any(|gp| gp.just_pressed(GamepadButton::DPadRight));

    if left_pressed {
        menu_state.pending_cycle = Some(false);
    } else if right_pressed {
        menu_state.pending_cycle = Some(true);
    }
}

// =============================================================================
// VALUE CYCLE SYSTEM (processes pending cycles)
// =============================================================================

/// Apply pending value cycles (separate system to reduce parameter count)
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
pub fn debug_menu_apply_cycle(
    mut menu_state: ResMut<DebugMenuState>,
    mut viewport_scale: ResMut<ViewportScale>,
    mut current_presets: ResMut<CurrentPresets>,
    mut current_level: ResMut<CurrentLevel>,
    mut current_palette: ResMut<CurrentPalette>,
    mut current_settings: ResMut<CurrentSettings>,
    mut window_query: Query<&mut Window>,
    level_db: Res<LevelDatabase>,
    palette_db: Res<PaletteDatabase>,
    profile_db: Res<AiProfileDatabase>,
    preset_db: Res<PresetDatabase>,
    ball_textures: Res<BallTextures>,
    mut ball_query: Query<(&mut BallStyle, &mut Sprite), With<Ball>>,
    mut ai_query: Query<(&mut AiState, Option<&Character>), With<Player>>,
) {
    let Some(forward) = menu_state.pending_cycle.take() else {
        return;
    };

    let option = DebugMenuOption::ALL[menu_state.selected_row];

    match option {
        DebugMenuOption::Viewport => {
            if forward {
                viewport_scale.cycle_next();
            } else {
                viewport_scale.cycle_prev();
            }
            apply_viewport(&viewport_scale, &mut window_query);
            current_settings.settings.viewport_index = viewport_scale.preset_index;
            current_settings.mark_dirty();
        }
        DebugMenuOption::CompositePreset => {
            let num = preset_db.composite_len();
            if forward {
                current_presets.composite = (current_presets.composite + 1) % num;
            } else {
                current_presets.composite = (current_presets.composite + num - 1) % num;
            }
            let idx = current_presets.composite;
            apply_composite_preset(&mut current_presets, &preset_db, idx);
            if let Some(p) = preset_db.get_composite(idx) {
                apply_composite_extras(
                    p,
                    &mut current_level,
                    &mut current_palette,
                    &level_db,
                    &ball_textures,
                    &mut ball_query,
                );
                info!("Composite: {}", p.name);
            }
        }
        DebugMenuOption::MovementPreset => {
            let num = preset_db.movement_len();
            if forward {
                current_presets.movement = (current_presets.movement + 1) % num;
            } else {
                current_presets.movement = (current_presets.movement + num - 1) % num;
            }
            current_presets.mark_apply();
            if let Some(p) = preset_db.get_movement(current_presets.movement) {
                info!("Movement: {}", p.name);
            }
        }
        DebugMenuOption::BallPreset => {
            let num = preset_db.ball_len();
            if forward {
                current_presets.ball = (current_presets.ball + 1) % num;
            } else {
                current_presets.ball = (current_presets.ball + num - 1) % num;
            }
            current_presets.mark_apply();
            if let Some(p) = preset_db.get_ball(current_presets.ball) {
                info!("Ball Preset: {}", p.name);
            }
        }
        DebugMenuOption::ShootingPreset => {
            let num = preset_db.shooting_len();
            if forward {
                current_presets.shooting = (current_presets.shooting + 1) % num;
            } else {
                current_presets.shooting = (current_presets.shooting + num - 1) % num;
            }
            current_presets.mark_apply();
            if let Some(p) = preset_db.get_shooting(current_presets.shooting) {
                info!("Shooting: {}", p.name);
            }
        }
        DebugMenuOption::AiL0 => {
            cycle_ai_profile_for_character(
                CharacterId::L0,
                forward,
                &profile_db,
                &mut current_settings,
                &mut ai_query,
            );
        }
        DebugMenuOption::AiL1 => {
            cycle_ai_profile_for_character(
                CharacterId::L1,
                forward,
                &profile_db,
                &mut current_settings,
                &mut ai_query,
            );
        }
        DebugMenuOption::AiR0 => {
            cycle_ai_profile_for_character(
                CharacterId::R0,
                forward,
                &profile_db,
                &mut current_settings,
                &mut ai_query,
            );
        }
        DebugMenuOption::AiR1 => {
            cycle_ai_profile_for_character(
                CharacterId::R1,
                forward,
                &profile_db,
                &mut current_settings,
                &mut ai_query,
            );
        }
        DebugMenuOption::Character => {
            // Cycle through: L0 → L1 → R0 → R1 → Observer → L0
            // Handled by separate system to reduce parameter count
            menu_state.pending_character_cycle = Some(forward);
        }
        DebugMenuOption::Level => {
            let level_ids: Vec<String> = level_db.all().iter().map(|l| l.id.clone()).collect();
            let num_levels = level_ids.len();
            let current_idx = level_ids
                .iter()
                .position(|id| *id == current_level.0)
                .unwrap_or(0);
            let new_idx = if forward {
                (current_idx + 1) % num_levels
            } else {
                (current_idx + num_levels - 1) % num_levels
            };
            current_level.0 = level_ids[new_idx].clone();
            current_settings.settings.level = current_level.0.clone();
            current_settings.mark_dirty();
            let level_name = level_db
                .get_by_id(&current_level.0)
                .map(|l| l.name.as_str())
                .unwrap_or("?");
            info!("Level: {}/{} {}", new_idx + 1, num_levels, level_name);
        }
        DebugMenuOption::Palette => {
            let num_palettes = palette_db.len();
            if forward {
                current_palette.0 = (current_palette.0 + 1) % num_palettes;
            } else {
                current_palette.0 = (current_palette.0 + num_palettes - 1) % num_palettes;
            }
            current_settings.settings.palette_index = current_palette.0;
            current_settings.mark_dirty();
            info!("Palette: {}", current_palette.0);
        }
        DebugMenuOption::BallStyle => {
            for (mut style, mut sprite) in ball_query.iter_mut() {
                let new_style_name = if forward {
                    ball_textures.next_style(style.name())
                } else {
                    ball_textures.prev_style(style.name())
                };

                style.0 = new_style_name.to_string();

                if let Some(textures) = ball_textures.get(style.name()) {
                    if let Some(texture) = textures.textures.get(current_palette.0) {
                        sprite.image = texture.clone();
                    }
                }

                current_settings.settings.ball_style = style.name().to_string();
                current_settings.mark_dirty();
                info!("BallStyle: {}", style.name());
                break;
            }
        }
    }
}

// =============================================================================
// CHARACTER CYCLE SYSTEM (separate to stay under 16-param limit)
// =============================================================================

/// Apply pending character cycle
pub fn debug_menu_character_cycle(
    mut commands: Commands,
    mut menu_state: ResMut<DebugMenuState>,
    player_query: Query<(Entity, Option<&Character>), With<Player>>,
    human_query: Query<(Entity, Option<&Character>), (With<Player>, With<HumanControlled>)>,
) {
    let Some(forward) = menu_state.pending_character_cycle.take() else {
        return;
    };

    cycle_character_selection(forward, &mut commands, &player_query, &human_query);
}

// =============================================================================
// DISPLAY UPDATE SYSTEM
// =============================================================================

/// Update debug menu visibility and text content
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
pub fn update_debug_menu_display(
    menu_state: Res<DebugMenuState>,
    viewport_scale: Res<ViewportScale>,
    current_presets: Res<CurrentPresets>,
    current_level: Res<CurrentLevel>,
    current_palette: Res<CurrentPalette>,
    level_db: Res<LevelDatabase>,
    _palette_db: Res<PaletteDatabase>,
    profile_db: Res<AiProfileDatabase>,
    preset_db: Res<PresetDatabase>,
    _ball_textures: Res<BallTextures>,
    ball_query: Query<&BallStyle, With<Ball>>,
    ai_query: Query<(&AiState, Option<&Character>, Option<&HumanControlled>), With<Player>>,
    mut bg_query: Query<
        &mut Visibility,
        (With<DebugMenuBackground>, Without<DebugMenuRow>),
    >,
    mut row_query: Query<
        (&mut Visibility, &mut Text2d, &mut TextColor, &DebugMenuRow),
        Without<DebugMenuBackground>,
    >,
) {
    let is_open = menu_state.open;

    // Update background visibility
    for mut vis in &mut bg_query {
        *vis = if is_open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    // Update menu rows
    for (mut vis, mut text, mut color, row) in &mut row_query {
        *vis = if is_open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };

        if is_open {
            let is_selected = row.index == menu_state.selected_row;
            let option = DebugMenuOption::ALL[row.index];

            // Get current value for this option
            let value_str = get_current_value_str(
                option,
                &viewport_scale,
                &current_presets,
                &current_level,
                &current_palette,
                &level_db,
                &profile_db,
                &preset_db,
                &ball_query,
                &ai_query,
            );

            let marker = if is_selected { ">" } else { " " };
            **text = format!("{} {}: {}", marker, option.label(), value_str);
            color.0 = if is_selected {
                SELECTED_COLOR
            } else {
                UNSELECTED_COLOR
            };
        }
    }
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Get AI profile name for a specific character
fn get_ai_profile_for_character(
    target_character: CharacterId,
    profile_db: &AiProfileDatabase,
    ai_query: &Query<(&AiState, Option<&Character>, Option<&HumanControlled>), With<Player>>,
) -> String {
    for (ai_state, character, _) in ai_query.iter() {
        let char_id = character.map(|c| c.0).unwrap_or(CharacterId::L0);
        if char_id == target_character {
            return profile_db
                .get_by_id(&ai_state.profile_id)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| "?".to_string());
        }
    }
    "?".to_string()
}

/// Get current value string for an option
fn get_current_value_str(
    option: DebugMenuOption,
    viewport_scale: &ViewportScale,
    current_presets: &CurrentPresets,
    current_level: &CurrentLevel,
    current_palette: &CurrentPalette,
    level_db: &LevelDatabase,
    profile_db: &AiProfileDatabase,
    preset_db: &PresetDatabase,
    ball_query: &Query<&BallStyle, With<Ball>>,
    ai_query: &Query<(&AiState, Option<&Character>, Option<&HumanControlled>), With<Player>>,
) -> String {
    match option {
        DebugMenuOption::Viewport => viewport_scale.current().2.to_string(),
        DebugMenuOption::CompositePreset => preset_db
            .get_composite(current_presets.composite)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "?".to_string()),
        DebugMenuOption::MovementPreset => preset_db
            .get_movement(current_presets.movement)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "?".to_string()),
        DebugMenuOption::BallPreset => preset_db
            .get_ball(current_presets.ball)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "?".to_string()),
        DebugMenuOption::ShootingPreset => preset_db
            .get_shooting(current_presets.shooting)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "?".to_string()),
        DebugMenuOption::AiL0 => get_ai_profile_for_character(CharacterId::L0, profile_db, ai_query),
        DebugMenuOption::AiL1 => get_ai_profile_for_character(CharacterId::L1, profile_db, ai_query),
        DebugMenuOption::AiR0 => get_ai_profile_for_character(CharacterId::R0, profile_db, ai_query),
        DebugMenuOption::AiR1 => get_ai_profile_for_character(CharacterId::R1, profile_db, ai_query),
        DebugMenuOption::Character => {
            for (_, character, human) in ai_query.iter() {
                if human.is_some() {
                    if let Some(c) = character {
                        return format!("{:?}", c.0);
                    }
                }
            }
            "Observer".to_string()
        }
        DebugMenuOption::Level => {
            let level_ids: Vec<&str> = level_db.all().iter().map(|l| l.id.as_str()).collect();
            let display_num = level_ids
                .iter()
                .position(|id| *id == current_level.0)
                .map(|i| i + 1)
                .unwrap_or(0);
            let level_name = level_db
                .get_by_id(&current_level.0)
                .map(|l| l.name.as_str())
                .unwrap_or("?");
            format!("{}/{} {}", display_num, level_db.len(), level_name)
        }
        DebugMenuOption::Palette => format!("Palette {}", current_palette.0),
        DebugMenuOption::BallStyle => ball_query
            .iter()
            .next()
            .map(|s| s.name().to_string())
            .unwrap_or_else(|| "?".to_string()),
    }
}

/// Apply viewport scale to window
fn apply_viewport(viewport_scale: &ViewportScale, window_query: &mut Query<&mut Window>) {
    let (width, height, label) = viewport_scale.current();

    if let Ok(mut window) = window_query.single_mut() {
        window.resolution = bevy::window::WindowResolution::new(width as u32, height as u32)
            .with_scale_factor_override(1.0);
    }

    info!("Viewport: {}", label);
}

/// Cycle AI profile for a specific character
fn cycle_ai_profile_for_character(
    target_character: CharacterId,
    forward: bool,
    profile_db: &AiProfileDatabase,
    current_settings: &mut CurrentSettings,
    ai_query: &mut Query<(&mut AiState, Option<&Character>), With<Player>>,
) {
    let profile_ids: Vec<String> = profile_db.profiles().iter().map(|p| p.id.clone()).collect();
    let num_profiles = profile_ids.len();

    for (mut ai_state, character) in ai_query.iter_mut() {
        let char_id = character.map(|c| c.0).unwrap_or(CharacterId::L0);
        if char_id == target_character {
            let current_idx = profile_ids
                .iter()
                .position(|id| *id == ai_state.profile_id)
                .unwrap_or(0);
            let next_idx = if forward {
                (current_idx + 1) % num_profiles
            } else {
                (current_idx + num_profiles - 1) % num_profiles
            };
            ai_state.profile_id = profile_ids[next_idx].clone();
            let profile = profile_db
                .get_by_id(&ai_state.profile_id)
                .unwrap_or_else(|| profile_db.default_profile());
            current_settings.mark_dirty();
            info!("AI {:?}: {}", target_character, profile.name);
            return;
        }
    }
}

/// Cycle character selection: L0 → L1 → R0 → R1 → Observer → L0
fn cycle_character_selection(
    forward: bool,
    commands: &mut Commands,
    player_query: &Query<(Entity, Option<&Character>), With<Player>>,
    human_query: &Query<(Entity, Option<&Character>), (With<Player>, With<HumanControlled>)>,
) {
    // Define the cycle order
    const ORDER: [Option<CharacterId>; 5] = [
        Some(CharacterId::L0),
        Some(CharacterId::L1),
        Some(CharacterId::R0),
        Some(CharacterId::R1),
        None, // Observer
    ];

    // Find current position in cycle
    let current_char = human_query.iter().next().and_then(|(_, c)| c.map(|c| c.0));
    let current_idx = ORDER.iter().position(|&c| c == current_char).unwrap_or(0);

    // Calculate next position
    let next_idx = if forward {
        (current_idx + 1) % ORDER.len()
    } else {
        (current_idx + ORDER.len() - 1) % ORDER.len()
    };
    let next_char = ORDER[next_idx];

    // Remove HumanControlled from current
    for (entity, _) in human_query.iter() {
        commands.entity(entity).remove::<HumanControlled>();
    }

    // Add HumanControlled to next (if not Observer)
    if let Some(target_id) = next_char {
        for (entity, character) in player_query.iter() {
            if let Some(c) = character {
                if c.0 == target_id {
                    commands.entity(entity).insert(HumanControlled);
                    info!("Character: {:?}", target_id);
                    return;
                }
            }
        }
    }

    info!("Character: Observer");
}

/// Apply additional settings from composite preset (level, palette, ball style)
fn apply_composite_extras(
    preset: &crate::presets::CompositePreset,
    current_level: &mut CurrentLevel,
    current_palette: &mut CurrentPalette,
    level_db: &LevelDatabase,
    ball_textures: &BallTextures,
    ball_query: &mut Query<(&mut BallStyle, &mut Sprite), With<Ball>>,
) {
    if let Some(level_num) = preset.level {
        if let Some(level_data) = level_db.all().get((level_num as usize).saturating_sub(1)) {
            current_level.0 = level_data.id.clone();
        }
    }
    if let Some(palette) = preset.palette {
        current_palette.0 = palette;
    }
    if let Some(ref style_name) = preset.ball_style {
        if let Some(style_textures) = ball_textures.get(style_name) {
            for (mut style, mut sprite) in ball_query.iter_mut() {
                *style = BallStyle::new(style_name);
                if let Some(handle) = style_textures.textures.get(current_palette.0) {
                    sprite.image = handle.clone();
                }
            }
        }
    }
}
