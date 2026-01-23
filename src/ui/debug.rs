//! Debug UI components and systems

use bevy::prelude::*;

use crate::ai::{AiProfileDatabase, AiState};
use crate::ball::{Ball, BallStyle, BallTextures};
use crate::constants::{DEFAULT_VIEWPORT_INDEX, VIEWPORT_PRESETS};
use crate::levels::LevelDatabase;
use crate::palettes::PaletteDatabase;
use crate::player::{HumanControlled, Player, Team};
use crate::presets::{CurrentPresets, PresetDatabase, apply_composite_preset};
use crate::scoring::CurrentLevel;
use crate::shooting::LastShotInfo;
use crate::steal::StealContest;
use crate::ui::hud::ScoreLevelText;
use crate::world::{Basket, BasketRim, CornerRamp, LevelPlatform, Platform};

// =============================================================================
// CYCLE SYSTEM - D-pad direction-based cycling for debug/test options
// =============================================================================

/// Which D-pad direction is currently active for value cycling
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CycleDirection {
    #[default]
    Up,
    Down,
    Left,
    Right,
}

/// Options available for D-pad Down (presets)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DownOption {
    #[default]
    Composite,
    Movement,
    Ball,
    Shooting,
}

impl DownOption {
    pub fn next(&self) -> Self {
        match self {
            DownOption::Composite => DownOption::Movement,
            DownOption::Movement => DownOption::Ball,
            DownOption::Ball => DownOption::Shooting,
            DownOption::Shooting => DownOption::Composite,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            DownOption::Composite => "Composite",
            DownOption::Movement => "Movement",
            DownOption::Ball => "Ball",
            DownOption::Shooting => "Shooting",
        }
    }
}

/// Options available for D-pad Right (visual/level settings)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RightOption {
    #[default]
    Level,
    Palette,
    BallStyle,
}

impl RightOption {
    pub fn next(&self) -> Self {
        match self {
            RightOption::Level => RightOption::Palette,
            RightOption::Palette => RightOption::BallStyle,
            RightOption::BallStyle => RightOption::Level,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            RightOption::Level => "Level",
            RightOption::Palette => "Palette",
            RightOption::BallStyle => "BallStyle",
        }
    }
}

/// Tracks cycle state for the new D-pad direction model
#[derive(Resource)]
pub struct CycleSelection {
    pub active_direction: CycleDirection,
    pub down_option: DownOption,
    pub right_option: RightOption,
    pub ai_player_index: usize, // 0=Left, 1=Right for AI profile editing
    /// Whether the menu is enabled (disabled when player uses stick, re-enabled by D-pad)
    pub menu_enabled: bool,
}

impl Default for CycleSelection {
    fn default() -> Self {
        Self {
            active_direction: CycleDirection::Down,
            down_option: DownOption::Composite,
            right_option: RightOption::Level,
            ai_player_index: 0, // Start with Left player
            menu_enabled: false, // Start disabled until player explicitly activates
        }
    }
}

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

/// Current viewport scale preset index
#[derive(Resource)]
pub struct ViewportScale {
    pub preset_index: usize,
}

impl Default for ViewportScale {
    fn default() -> Self {
        Self {
            preset_index: DEFAULT_VIEWPORT_INDEX,
        }
    }
}

impl ViewportScale {
    /// Get current preset (width, height, label)
    pub fn current(&self) -> (f32, f32, &'static str) {
        VIEWPORT_PRESETS[self.preset_index]
    }

    /// Cycle to next preset
    pub fn cycle_next(&mut self) {
        self.preset_index = (self.preset_index + 1) % VIEWPORT_PRESETS.len();
    }

    /// Cycle to previous preset
    pub fn cycle_prev(&mut self) {
        self.preset_index =
            (self.preset_index + VIEWPORT_PRESETS.len() - 1) % VIEWPORT_PRESETS.len();
    }
}

/// Debug text component
#[derive(Component)]
pub struct DebugText;

/// Toggle debug UI visibility (Tab only) - CycleIndicator is always visible
pub fn toggle_debug(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<DebugSettings>,
    mut text_query: Query<&mut Visibility, With<DebugText>>,
) {
    if keyboard.just_pressed(KeyCode::Tab) {
        settings.visible = !settings.visible;
        let new_visibility = if settings.visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if let Ok(mut visibility) = text_query.single_mut() {
            *visibility = new_visibility;
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

    let steal_str = if steal_contest.last_attempt_failed {
        format!(" | Steal: FAILED ({:.2}s)", steal_contest.fail_flash_timer)
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

/// Cycle through viewport scale presets (V key only - controller uses unified cycle)
pub fn cycle_viewport(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut viewport_scale: ResMut<ViewportScale>,
    mut window_query: Query<&mut Window>,
) {
    if keyboard.just_pressed(KeyCode::KeyV) {
        viewport_scale.cycle_next();
        apply_viewport(&viewport_scale, &mut window_query);
    }
}

/// Apply current viewport scale to window (camera uses FixedVertical scaling mode)
fn apply_viewport(viewport_scale: &ViewportScale, window_query: &mut Query<&mut Window>) {
    let (width, height, label) = viewport_scale.current();

    // Change window size - use scale_factor_override 1.0 for consistent HiDPI behavior
    // Camera uses FixedVertical scaling mode so it automatically shows full arena height
    if let Ok(mut window) = window_query.single_mut() {
        window.resolution = bevy::window::WindowResolution::new(width as u32, height as u32)
            .with_scale_factor_override(1.0);
    }

    info!("Viewport: {}", label);
}

/// Marker for cycle indicator text (one per line, index 0-3 for Up/Down/Left/Right)
#[derive(Component)]
pub struct CycleIndicator(pub usize);

/// Deadzone for detecting active stick usage (disables menu when playing)
const STICK_ACTIVE_DEADZONE: f32 = 0.2;

/// Unified cycle system - D-pad directions select/cycle options, RT/LT cycle values
/// Disabled when player is actively using the control stick (playing the game)
#[allow(clippy::too_many_arguments)]
pub fn unified_cycle_system(
    gamepads: Query<&Gamepad>,
    mut cycle_selection: ResMut<CycleSelection>,
    mut current_level: ResMut<CurrentLevel>,
    mut current_palette: ResMut<crate::ball::CurrentPalette>,
    mut viewport_scale: ResMut<ViewportScale>,
    mut current_presets: ResMut<CurrentPresets>,
    level_db: Res<LevelDatabase>,
    palette_db: Res<PaletteDatabase>,
    profile_db: Res<AiProfileDatabase>,
    preset_db: Res<PresetDatabase>,
    ball_textures: Res<BallTextures>,
    mut window_query: Query<&mut Window>,
    mut ball_query: Query<(&mut BallStyle, &mut Sprite), With<Ball>>,
    mut ai_query: Query<(&mut AiState, &Team), With<Player>>,
) {
    // Check if player is actively using the control stick (playing the game)
    // If so, disable the menu entirely until a D-pad button re-enables it
    let stick_active = gamepads.iter().any(|gp| {
        let left_x = gp.get(GamepadAxis::LeftStickX).unwrap_or(0.0).abs();
        let left_y = gp.get(GamepadAxis::LeftStickY).unwrap_or(0.0).abs();
        left_x > STICK_ACTIVE_DEADZONE || left_y > STICK_ACTIVE_DEADZONE
    });

    // Disable menu when stick is active
    if stick_active {
        cycle_selection.menu_enabled = false;
    }

    // Check D-pad directions
    let dpad_up = gamepads
        .iter()
        .any(|gp| gp.just_pressed(GamepadButton::DPadUp));
    let dpad_down = gamepads
        .iter()
        .any(|gp| gp.just_pressed(GamepadButton::DPadDown));
    let dpad_left = gamepads
        .iter()
        .any(|gp| gp.just_pressed(GamepadButton::DPadLeft));
    let dpad_right = gamepads
        .iter()
        .any(|gp| gp.just_pressed(GamepadButton::DPadRight));

    let any_dpad = dpad_up || dpad_down || dpad_left || dpad_right;

    // D-pad handling: first press selects direction, second press on same direction cycles
    if any_dpad {
        let pressed_direction = if dpad_up {
            CycleDirection::Up
        } else if dpad_down {
            CycleDirection::Down
        } else if dpad_left {
            CycleDirection::Left
        } else {
            CycleDirection::Right
        };

        let same_direction = cycle_selection.menu_enabled
            && cycle_selection.active_direction == pressed_direction;

        if !cycle_selection.menu_enabled || !same_direction {
            // First press or different direction - just select (no cycling)
            cycle_selection.menu_enabled = true;
            cycle_selection.active_direction = pressed_direction;
            info!("Selected: {:?}", pressed_direction);
        } else {
            // Second press on same direction - cycle options
            match pressed_direction {
                CycleDirection::Up => {
                    // Up only has Viewport, no cycling needed
                    info!("Cycle: Up (Viewport - single option)");
                }
                CycleDirection::Down => {
                    cycle_selection.down_option = cycle_selection.down_option.next();
                    info!("Cycle: Down ({})", cycle_selection.down_option.name());
                }
                CycleDirection::Left => {
                    // Left only has AI, no cycling needed
                    info!("Cycle: Left (AI - single option)");
                }
                CycleDirection::Right => {
                    cycle_selection.right_option = cycle_selection.right_option.next();
                    info!("Cycle: Right ({})", cycle_selection.right_option.name());
                }
            }
        }
    }

    // Skip LT/RT value cycling if menu is disabled
    if !cycle_selection.menu_enabled {
        return;
    }

    // RT cycles forward, LT cycles backward
    let cycle_next = gamepads
        .iter()
        .any(|gp| gp.just_pressed(GamepadButton::RightTrigger2));
    let cycle_prev = gamepads
        .iter()
        .any(|gp| gp.just_pressed(GamepadButton::LeftTrigger2));

    if !cycle_next && !cycle_prev {
        return;
    }

    // Handle value cycling based on active direction
    match cycle_selection.active_direction {
        CycleDirection::Up => {
            // Viewport only
            if cycle_next {
                viewport_scale.cycle_next();
            } else if cycle_prev {
                viewport_scale.cycle_prev();
            }
            apply_viewport(&viewport_scale, &mut window_query);
        }
        CycleDirection::Down => {
            // Presets: Composite, Movement, Ball, Shooting
            match cycle_selection.down_option {
                DownOption::Composite => {
                    let num = preset_db.composite_len();
                    if cycle_next {
                        current_presets.composite = (current_presets.composite + 1) % num;
                    } else if cycle_prev {
                        current_presets.composite = (current_presets.composite + num - 1) % num;
                    }
                    let idx = current_presets.composite;
                    apply_composite_preset(&mut current_presets, &preset_db, idx);

                    // Apply additional global settings from composite preset
                    if let Some(p) = preset_db.get_composite(idx) {
                        if let Some(level) = p.level {
                            current_level.0 = level;
                        }
                        if let Some(palette) = p.palette {
                            current_palette.0 = palette;
                        }
                        if let Some(ref style_name) = p.ball_style {
                            if let Some(style_textures) = ball_textures.get(style_name) {
                                for (mut style, mut sprite) in &mut ball_query {
                                    *style = BallStyle::new(style_name);
                                    if let Some(handle) =
                                        style_textures.textures.get(current_palette.0)
                                    {
                                        sprite.image = handle.clone();
                                    }
                                }
                            }
                        }
                        info!("Composite: {}", p.name);
                    }
                }
                DownOption::Movement => {
                    let num = preset_db.movement_len();
                    if cycle_next {
                        current_presets.movement = (current_presets.movement + 1) % num;
                    } else if cycle_prev {
                        current_presets.movement = (current_presets.movement + num - 1) % num;
                    }
                    current_presets.mark_apply();
                    if let Some(p) = preset_db.get_movement(current_presets.movement) {
                        info!("Movement: {}", p.name);
                    }
                }
                DownOption::Ball => {
                    let num = preset_db.ball_len();
                    if cycle_next {
                        current_presets.ball = (current_presets.ball + 1) % num;
                    } else if cycle_prev {
                        current_presets.ball = (current_presets.ball + num - 1) % num;
                    }
                    current_presets.mark_apply();
                    if let Some(p) = preset_db.get_ball(current_presets.ball) {
                        info!("Ball Preset: {}", p.name);
                    }
                }
                DownOption::Shooting => {
                    let num = preset_db.shooting_len();
                    if cycle_next {
                        current_presets.shooting = (current_presets.shooting + 1) % num;
                    } else if cycle_prev {
                        current_presets.shooting = (current_presets.shooting + num - 1) % num;
                    }
                    current_presets.mark_apply();
                    if let Some(p) = preset_db.get_shooting(current_presets.shooting) {
                        info!("Shooting: {}", p.name);
                    }
                }
            }
        }
        CycleDirection::Left => {
            // AI: LT cycles player selection, RT cycles profile
            if cycle_prev {
                // Toggle selected player
                cycle_selection.ai_player_index = (cycle_selection.ai_player_index + 1) % 2;
                let player_name = if cycle_selection.ai_player_index == 0 {
                    "Left"
                } else {
                    "Right"
                };
                info!("AI: editing {} player", player_name);
            } else if cycle_next {
                // Cycle profile for selected player
                let target_team = if cycle_selection.ai_player_index == 0 {
                    Team::Left
                } else {
                    Team::Right
                };
                let num_profiles = profile_db.len();
                for (mut ai_state, team) in &mut ai_query {
                    if *team == target_team {
                        ai_state.profile_index = (ai_state.profile_index + 1) % num_profiles;
                        let profile = profile_db.get(ai_state.profile_index);
                        info!(
                            "AI {}: {}",
                            if *team == Team::Left { "L" } else { "R" },
                            profile.name
                        );
                    }
                }
            }
        }
        CycleDirection::Right => {
            // Level, Palette, BallStyle
            match cycle_selection.right_option {
                RightOption::Level => {
                    let num_levels = level_db.len() as u32;
                    if cycle_next {
                        current_level.0 = if current_level.0 >= num_levels {
                            1
                        } else {
                            current_level.0 + 1
                        };
                    } else if cycle_prev {
                        current_level.0 = if current_level.0 <= 1 {
                            num_levels
                        } else {
                            current_level.0 - 1
                        };
                    }
                    info!("Level: {}", current_level.0);
                }
                RightOption::Palette => {
                    let num_palettes = palette_db.len();
                    if cycle_next {
                        current_palette.0 = (current_palette.0 + 1) % num_palettes;
                    } else if cycle_prev {
                        current_palette.0 = (current_palette.0 + num_palettes - 1) % num_palettes;
                    }
                    info!("Palette: {}", current_palette.0);
                }
                RightOption::BallStyle => {
                    for (mut style, mut sprite) in &mut ball_query {
                        let new_style_name = if cycle_next {
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
                    }

                    if let Some((style, _)) = ball_query.iter().next() {
                        info!("BallStyle: {}", style.name());
                    }
                }
            }
        }
    }
}

/// Font sizes for cycle indicator
const CYCLE_FONT_SIZE_NORMAL: f32 = 14.0;
const CYCLE_FONT_SIZE_SELECTED: f32 = 18.0;

/// Update cycle indicator display - 4 separate lines with different sizes when selected
#[allow(clippy::too_many_arguments)]
pub fn update_cycle_indicator(
    cycle_selection: Res<CycleSelection>,
    current_level: Res<CurrentLevel>,
    current_palette: Res<crate::ball::CurrentPalette>,
    viewport_scale: Res<ViewportScale>,
    current_presets: Res<CurrentPresets>,
    level_db: Res<LevelDatabase>,
    profile_db: Res<AiProfileDatabase>,
    preset_db: Res<PresetDatabase>,
    ball_query: Query<&BallStyle, With<Ball>>,
    ai_query: Query<(&AiState, &Team, Option<&HumanControlled>), With<Player>>,
    mut query: Query<(&CycleIndicator, &mut Text2d, &mut TextFont)>,
) {
    let enabled = cycle_selection.menu_enabled;

    // Build content for each line
    let viewport_label = viewport_scale.current().2;

    let down_value = match cycle_selection.down_option {
        DownOption::Composite => preset_db
            .get_composite(current_presets.composite)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "?".to_string()),
        DownOption::Movement => preset_db
            .get_movement(current_presets.movement)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "?".to_string()),
        DownOption::Ball => preset_db
            .get_ball(current_presets.ball)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "?".to_string()),
        DownOption::Shooting => preset_db
            .get_shooting(current_presets.shooting)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "?".to_string()),
    };

    // AI profiles
    let mut left_profile = "?".to_string();
    let mut right_profile = "?".to_string();
    let mut left_human = false;
    let mut right_human = false;
    for (ai_state, team, human) in &ai_query {
        let profile_name = profile_db.get(ai_state.profile_index).name.clone();
        match team {
            Team::Left => {
                left_profile = profile_name;
                left_human = human.is_some();
            }
            Team::Right => {
                right_profile = profile_name;
                right_human = human.is_some();
            }
        }
    }
    let left_marker_human = if left_human { "*" } else { "" };
    let right_marker_human = if right_human { "*" } else { "" };
    let ai_str = if cycle_selection.ai_player_index == 0 {
        format!(
            "[L{} {}] R{} {}",
            left_marker_human, left_profile, right_marker_human, right_profile
        )
    } else {
        format!(
            "L{} {} [R{} {}]",
            left_marker_human, left_profile, right_marker_human, right_profile
        )
    };

    let right_value = match cycle_selection.right_option {
        RightOption::Level => format!("{}/{}", current_level.0, level_db.len()),
        RightOption::Palette => format!("{}", current_palette.0),
        RightOption::BallStyle => ball_query
            .iter()
            .next()
            .map(|s| s.name().to_string())
            .unwrap_or_else(|| "?".to_string()),
    };

    // Direction to index mapping (N/W/E/S order: Up, Left, Right, Down)
    let active_index = match cycle_selection.active_direction {
        CycleDirection::Up => 0,    // N
        CycleDirection::Left => 1,  // W
        CycleDirection::Right => 2, // E
        CycleDirection::Down => 3,  // S
    };

    // Update each line
    for (indicator, mut text, mut font) in &mut query {
        let line_index = indicator.0;
        let is_selected = enabled && line_index == active_index;

        // Marker: ">" when selected, " " otherwise (no marker when disabled)
        let marker = if is_selected {
            ">"
        } else {
            " "
        };

        // Font size: larger when selected
        font.font_size = if is_selected {
            CYCLE_FONT_SIZE_SELECTED
        } else {
            CYCLE_FONT_SIZE_NORMAL
        };

        // Set text content based on line (N/W/E/S order)
        // Build label and value for each line
        let (label, value) = match line_index {
            0 => ("Viewport", viewport_label.to_string()),
            1 => ("AI", ai_str.clone()),
            2 => (cycle_selection.right_option.name(), right_value.clone()),
            3 => (cycle_selection.down_option.name(), down_value.clone()),
            _ => ("", String::new()),
        };
        **text = format!("{}{}: {}", marker, label, value);
    }
}

/// Apply palette colors when CurrentPalette changes
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
pub fn apply_palette_colors(
    current_palette: Res<crate::ball::CurrentPalette>,
    palette_db: Res<PaletteDatabase>,
    ball_textures: Res<BallTextures>,
    mut clear_color: ResMut<ClearColor>,
    mut player_query: Query<(&mut Sprite, &Team), (With<Player>, Without<Ball>, Without<Basket>)>,
    mut basket_query: Query<
        (&mut Sprite, &Basket, Option<&Children>),
        (Without<Player>, Without<Ball>),
    >,
    mut rim_query: Query<
        &mut Sprite,
        (
            With<BasketRim>,
            Without<Player>,
            Without<Ball>,
            Without<Basket>,
        ),
    >,
    mut floor_query: Query<
        &mut Sprite,
        (
            With<Platform>,
            Without<LevelPlatform>,
            Without<CornerRamp>,
            Without<Player>,
            Without<Ball>,
            Without<Basket>,
            Without<BasketRim>,
        ),
    >,
    mut level_platform_query: Query<
        &mut Sprite,
        (
            With<LevelPlatform>,
            Without<CornerRamp>,
            Without<Player>,
            Without<Ball>,
            Without<Basket>,
            Without<BasketRim>,
        ),
    >,
    mut corner_ramp_query: Query<
        &mut Sprite,
        (
            With<CornerRamp>,
            Without<LevelPlatform>,
            Without<Player>,
            Without<Ball>,
            Without<Basket>,
            Without<BasketRim>,
        ),
    >,
    mut ball_query: Query<(&BallStyle, &mut Sprite), With<Ball>>,
    mut score_text_query: Query<&mut TextColor, (With<ScoreLevelText>, Without<CycleIndicator>)>,
    mut cycle_text_query: Query<&mut TextColor, (With<CycleIndicator>, Without<ScoreLevelText>)>,
) {
    // Only run when palette actually changes
    if !current_palette.is_changed() {
        return;
    }

    let palette = palette_db
        .get(current_palette.0)
        .expect("Palette index out of bounds");

    // Background
    clear_color.0 = palette.background;

    // Players
    for (mut sprite, team) in &mut player_query {
        sprite.color = match team {
            Team::Left => palette.left,
            Team::Right => palette.right,
        };
    }

    // Baskets and rims
    for (mut sprite, basket, children) in &mut basket_query {
        sprite.color = match basket {
            Basket::Left => palette.left,
            Basket::Right => palette.right,
        };

        // Update rim colors (children)
        if let Some(children) = children {
            for child in children.iter() {
                if let Ok(mut rim_sprite) = rim_query.get_mut(child) {
                    rim_sprite.color = match basket {
                        Basket::Left => palette.right_rim,
                        Basket::Right => palette.left_rim,
                    };
                }
            }
        }
    }

    // Floor and walls
    for mut sprite in &mut floor_query {
        sprite.color = palette.platforms;
    }

    // Level platforms (same color as floor)
    for mut sprite in &mut level_platform_query {
        sprite.color = palette.platforms;
    }

    // Corner ramps
    for mut sprite in &mut corner_ramp_query {
        sprite.color = palette.platforms;
    }

    // Ball textures
    for (style, mut sprite) in &mut ball_query {
        if let Some(textures) = ball_textures.get(style.name()) {
            if let Some(texture) = textures.textures.get(current_palette.0) {
                sprite.image = texture.clone();
            }
        }
    }

    // Text colors (score/level text)
    for mut text_color in &mut score_text_query {
        text_color.0 = palette.text;
    }

    // Text colors (cycle indicator)
    for mut text_color in &mut cycle_text_query {
        text_color.0 = palette.text_accent;
    }
}
