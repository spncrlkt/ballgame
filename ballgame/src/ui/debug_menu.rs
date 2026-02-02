//! Debug menu - unified settings menu accessible via Tab/Select
//!
//! Replaces: tweak panel, cycle indicator, shot debug text

use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::ball::{Ball, BallStyle, BallTextures, CurrentPalette};
use crate::levels::LevelDatabase;
use crate::palettes::PaletteDatabase;
use crate::presets::{apply_composite_preset, CurrentPresets, PresetDatabase};
use crate::scoring::{CurrentLevel, GamePaused};
use crate::settings::CurrentSettings;
use crate::ui::ViewportScale;

// =============================================================================
// CONSTANTS
// =============================================================================

/// Z-layers for debug menu (above countdown z=100, above pause z=900)
const DEBUG_MENU_BORDER_Z: f32 = 949.0;
const DEBUG_MENU_BG_Z: f32 = 950.0;
const DEBUG_MENU_TEXT_Z: f32 = 951.0;

/// Font sizes
const MENU_FONT_SIZE: f32 = 24.0;

/// Spacing
const MENU_ROW_SPACING: f32 = 32.0;
const MENU_START_Y: f32 = 140.0;

/// Menu dimensions (8 options × 32px spacing + padding)
const MENU_WIDTH: f32 = 810.0;
const MENU_HEIGHT: f32 = 400.0;
const BORDER_THICKNESS: f32 = 4.0;
const LABEL_VALUE_GAP: f32 = 15.0; // Gap between labels and values
/// Max characters for value text to stay 20px from border
const MAX_VALUE_CHARS: usize = 22;

/// Colors
const MENU_BG_COLOR: Color = Color::srgb(0.0, 0.0, 0.0);
const MENU_BORDER_COLOR: Color = Color::srgb(1.0, 1.0, 1.0);
const SELECTED_COLOR: Color = Color::srgb(1.0, 0.9, 0.2);
const UNSELECTED_COLOR: Color = Color::srgb(0.9, 0.9, 0.9);

// =============================================================================
// MENU OPTIONS
// =============================================================================

/// All available debug menu options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugMenuOption {
    Viewport,
    Level,
    Palette,
    BallStyle,
    CompositePreset,
    MovementPreset,
    BallPreset,
    ShootingPreset,
}

impl DebugMenuOption {
    pub const ALL: [DebugMenuOption; 8] = [
        DebugMenuOption::Viewport,
        DebugMenuOption::Level,
        DebugMenuOption::Palette,
        DebugMenuOption::BallStyle,
        DebugMenuOption::CompositePreset,
        DebugMenuOption::MovementPreset,
        DebugMenuOption::BallPreset,
        DebugMenuOption::ShootingPreset,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            DebugMenuOption::Viewport => "Viewport",
            DebugMenuOption::Level => "Level",
            DebugMenuOption::Palette => "Palette",
            DebugMenuOption::BallStyle => "Ball Style",
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
    /// Set when opened from pause menu, cleared after one frame (prevents immediate re-toggle)
    pub skip_next_select: bool,
}

/// Marker for debug menu border
#[derive(Component)]
pub struct DebugMenuBorder;

/// Marker for debug menu background
#[derive(Component)]
pub struct DebugMenuBackground;

/// Marker for debug menu row label (left-aligned)
#[derive(Component)]
pub struct DebugMenuRow {
    pub index: usize,
}

/// Marker for debug menu row value (right-aligned)
#[derive(Component)]
pub struct DebugMenuValue {
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
    // White border (slightly larger than background)
    commands.spawn((
        Sprite {
            color: MENU_BORDER_COLOR,
            custom_size: Some(Vec2::new(
                MENU_WIDTH + BORDER_THICKNESS * 2.0,
                MENU_HEIGHT + BORDER_THICKNESS * 2.0,
            )),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, DEBUG_MENU_BORDER_Z),
        Visibility::Hidden,
        DebugMenuBorder,
    ));

    // Opaque black background (centered)
    commands.spawn((
        Sprite {
            color: MENU_BG_COLOR,
            custom_size: Some(Vec2::new(MENU_WIDTH, MENU_HEIGHT)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, DEBUG_MENU_BG_Z),
        Visibility::Hidden,
        DebugMenuBackground,
    ));

    // Menu rows - label (right-anchored) and value (left-anchored) centered
    for (i, option) in DebugMenuOption::ALL.iter().enumerate() {
        let y = MENU_START_Y - (i as f32 * MENU_ROW_SPACING);

        // Label (right-anchored, to the left of center)
        commands.spawn((
            Text2d::new(format!("  {}", option.label())),
            TextFont {
                font_size: MENU_FONT_SIZE,
                ..default()
            },
            TextLayout::new_with_justify(Justify::Right),
            Anchor::CENTER_RIGHT,
            TextColor(UNSELECTED_COLOR),
            Transform::from_xyz(-LABEL_VALUE_GAP, y, DEBUG_MENU_TEXT_Z),
            Visibility::Hidden,
            DebugMenuRow { index: i },
        ));

        // Value (left-anchored, to the right of center)
        commands.spawn((
            Text2d::new("---"),
            TextFont {
                font_size: MENU_FONT_SIZE,
                ..default()
            },
            TextLayout::new_with_justify(Justify::Left),
            Anchor::CENTER_LEFT,
            TextColor(UNSELECTED_COLOR),
            Transform::from_xyz(LABEL_VALUE_GAP, y, DEBUG_MENU_TEXT_Z),
            Visibility::Hidden,
            DebugMenuValue { index: i },
        ));
    }
}

// =============================================================================
// TOGGLE SYSTEM
// =============================================================================

/// Toggle debug menu open/close with Select button
pub fn toggle_debug_menu(
    gamepads: Query<&Gamepad>,
    mut menu_state: ResMut<DebugMenuState>,
    mut game_paused: ResMut<GamePaused>,
) {
    // When debug menu is open, Start switches to pause menu
    if menu_state.open {
        let start_pressed = gamepads
            .iter()
            .any(|gp| gp.just_pressed(GamepadButton::Start));

        if start_pressed {
            menu_state.open = false;
            // Keep game paused - pause menu will show
            info!("Debug menu closed, pause menu opened");
            return;
        }
    }

    // Don't toggle if pause menu is open (Select in pause menu handled by pause_menu_confirm)
    if game_paused.0 && !menu_state.open {
        return;
    }

    // Skip one Select press after being opened from pause menu (prevents immediate re-toggle)
    if menu_state.skip_next_select {
        menu_state.skip_next_select = false;
        return;
    }

    let toggle_pressed = gamepads
        .iter()
        .any(|gp| gp.just_pressed(GamepadButton::Select));

    if toggle_pressed {
        menu_state.open = !menu_state.open;

        // Pause game when menu opens, unpause when it closes
        game_paused.0 = menu_state.open;

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
pub fn debug_menu_navigation(gamepads: Query<&Gamepad>, mut menu_state: ResMut<DebugMenuState>) {
    if !menu_state.open {
        return;
    }

    // Up/Down navigation
    let up_pressed = gamepads
        .iter()
        .any(|gp| gp.just_pressed(GamepadButton::DPadUp));
    let down_pressed = gamepads
        .iter()
        .any(|gp| gp.just_pressed(GamepadButton::DPadDown));

    let num_options = DebugMenuOption::ALL.len();
    if up_pressed {
        menu_state.selected_row = (menu_state.selected_row + num_options - 1) % num_options;
    } else if down_pressed {
        menu_state.selected_row = (menu_state.selected_row + 1) % num_options;
    }

    // Left/Right sets pending cycle action
    let left_pressed = gamepads
        .iter()
        .any(|gp| gp.just_pressed(GamepadButton::DPadLeft));
    let right_pressed = gamepads
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
    preset_db: Res<PresetDatabase>,
    ball_textures: Res<BallTextures>,
    mut ball_query: Query<(&mut BallStyle, &mut Sprite), With<Ball>>,
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
    preset_db: Res<PresetDatabase>,
    ball_query: Query<&BallStyle, With<Ball>>,
    mut border_query: Query<
        &mut Visibility,
        (
            With<DebugMenuBorder>,
            Without<DebugMenuBackground>,
            Without<DebugMenuRow>,
            Without<DebugMenuValue>,
        ),
    >,
    mut bg_query: Query<
        &mut Visibility,
        (
            With<DebugMenuBackground>,
            Without<DebugMenuBorder>,
            Without<DebugMenuRow>,
            Without<DebugMenuValue>,
        ),
    >,
    mut row_query: Query<
        (&mut Visibility, &mut Text2d, &mut TextColor, &DebugMenuRow),
        (
            Without<DebugMenuBackground>,
            Without<DebugMenuBorder>,
            Without<DebugMenuValue>,
        ),
    >,
    mut value_query: Query<
        (
            &mut Visibility,
            &mut Text2d,
            &mut TextColor,
            &DebugMenuValue,
        ),
        (
            Without<DebugMenuBackground>,
            Without<DebugMenuBorder>,
            Without<DebugMenuRow>,
        ),
    >,
) {
    let is_open = menu_state.open;

    // Update border visibility
    for mut vis in &mut border_query {
        *vis = if is_open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    // Update background visibility
    for mut vis in &mut bg_query {
        *vis = if is_open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    // Update menu row labels (left-aligned)
    for (mut vis, mut text, mut color, row) in &mut row_query {
        *vis = if is_open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };

        if is_open {
            let is_selected = row.index == menu_state.selected_row;
            let option = DebugMenuOption::ALL[row.index];

            let marker = if is_selected { ">" } else { " " };
            **text = format!("{} {}", marker, option.label());
            color.0 = if is_selected {
                SELECTED_COLOR
            } else {
                UNSELECTED_COLOR
            };
        }
    }

    // Update menu row values (right-aligned)
    for (mut vis, mut text, mut color, value) in &mut value_query {
        *vis = if is_open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };

        if is_open {
            let is_selected = value.index == menu_state.selected_row;
            let option = DebugMenuOption::ALL[value.index];

            // Get current value for this option (truncated to fit)
            let value_str = get_current_value_str(
                option,
                &viewport_scale,
                &current_presets,
                &current_level,
                &current_palette,
                &level_db,
                &preset_db,
                &ball_query,
            );

            **text = truncate_value(&value_str);
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

/// Truncate a string to fit within the value column
fn truncate_value(s: &str) -> String {
    if s.chars().count() <= MAX_VALUE_CHARS {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(MAX_VALUE_CHARS - 2).collect();
        format!("{}..", truncated)
    }
}

/// Get current value string for an option
fn get_current_value_str(
    option: DebugMenuOption,
    viewport_scale: &ViewportScale,
    current_presets: &CurrentPresets,
    current_level: &CurrentLevel,
    current_palette: &CurrentPalette,
    level_db: &LevelDatabase,
    preset_db: &PresetDatabase,
    ball_query: &Query<&BallStyle, With<Ball>>,
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
