//! Debug UI components and systems

use bevy::prelude::*;

use crate::ai::{AiProfileDatabase, AiState};
use crate::ball::{Ball, BallStyle, BallTextures};
use crate::constants::{DEFAULT_VIEWPORT_INDEX, VIEWPORT_PRESETS};
use crate::levels::LevelDatabase;
use crate::palettes::PaletteDatabase;
use crate::player::{Player, Team};
use crate::presets::{CurrentPresets, PresetDatabase, apply_composite_preset};
use crate::scoring::CurrentLevel;
use crate::shooting::LastShotInfo;
use crate::steal::StealContest;
use crate::ui::hud::ScoreLevelText;
use crate::world::{Basket, BasketRim, CornerRamp, LevelPlatform, Platform};

// =============================================================================
// CYCLE SYSTEM - Unified controller cycling for debug/test options
// =============================================================================

/// What category is currently selected for cycling with RT/LT
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CycleTarget {
    #[default]
    CompositePreset, // Global - sets everything
    Level,
    AiProfile,
    Palette,
    BallStyle,
    Viewport,
    MovementPreset,
    BallPreset,
    ShootingPreset,
}

impl CycleTarget {
    /// Ordered by significance: Global first, then gameplay, visuals, testing, tuning
    pub const ALL: [CycleTarget; 9] = [
        CycleTarget::CompositePreset, // Global - sets all options
        CycleTarget::Level,           // Core gameplay
        CycleTarget::AiProfile,       // Affects gameplay
        CycleTarget::Palette,         // Visual - colors
        CycleTarget::BallStyle,       // Visual - ball appearance
        CycleTarget::Viewport,        // Testing different resolutions
        CycleTarget::MovementPreset,  // Physics tuning
        CycleTarget::BallPreset,      // Physics tuning
        CycleTarget::ShootingPreset,  // Physics tuning
    ];

    pub fn name(&self) -> &'static str {
        match self {
            CycleTarget::CompositePreset => "Global",
            CycleTarget::Level => "Level",
            CycleTarget::AiProfile => "AI Profile",
            CycleTarget::Palette => "Palette",
            CycleTarget::BallStyle => "Ball Style",
            CycleTarget::Viewport => "Viewport",
            CycleTarget::MovementPreset => "Movement",
            CycleTarget::BallPreset => "Ball",
            CycleTarget::ShootingPreset => "Shooting",
        }
    }

    pub fn next(&self) -> Self {
        let idx = Self::ALL.iter().position(|t| t == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    pub fn prev(&self) -> Self {
        let idx = Self::ALL.iter().position(|t| t == self).unwrap_or(0);
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

/// Tracks which cycle target is selected for controller cycling
#[derive(Resource)]
pub struct CycleSelection {
    pub target: CycleTarget,
    pub ai_profile_player: Team, // Which player's AI profile to edit (Left or Right)
}

impl Default for CycleSelection {
    fn default() -> Self {
        Self {
            target: CycleTarget::CompositePreset, // Global - first option
            ai_profile_player: Team::Left,
        }
    }
}

impl CycleSelection {
    pub fn select_next(&mut self) {
        self.target = self.target.next();
    }

    pub fn select_prev(&mut self) {
        self.target = self.target.prev();
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

/// Toggle debug UI visibility (Tab only)
pub fn toggle_debug(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<DebugSettings>,
    mut text_query: Query<&mut Visibility, (With<DebugText>, Without<CycleIndicator>)>,
    mut cycle_query: Query<&mut Visibility, (With<CycleIndicator>, Without<DebugText>)>,
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
        if let Ok(mut visibility) = cycle_query.single_mut() {
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

/// Marker for cycle indicator text
#[derive(Component)]
pub struct CycleIndicator;

/// Unified cycle system - D-pad Down selects target, RT/LT cycle values
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
    // D-pad Down/Up selects next/prev cycle target
    let select_next = gamepads
        .iter()
        .any(|gp| gp.just_pressed(GamepadButton::DPadDown));
    let select_prev = gamepads
        .iter()
        .any(|gp| gp.just_pressed(GamepadButton::DPadUp));

    if select_next {
        cycle_selection.select_next();
        info!("Cycle target: {}", cycle_selection.target.name());
    } else if select_prev {
        cycle_selection.select_prev();
        info!("Cycle target: {}", cycle_selection.target.name());
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

    match cycle_selection.target {
        CycleTarget::Level => {
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
        CycleTarget::Viewport => {
            if cycle_next {
                viewport_scale.cycle_next();
            } else if cycle_prev {
                viewport_scale.cycle_prev();
            }
            apply_viewport(&viewport_scale, &mut window_query);
        }
        CycleTarget::Palette => {
            // Just change the index - apply_palette_colors system handles the visuals
            let num_palettes = palette_db.len();
            if cycle_next {
                current_palette.0 = (current_palette.0 + 1) % num_palettes;
            } else if cycle_prev {
                current_palette.0 = (current_palette.0 + num_palettes - 1) % num_palettes;
            }
            info!("Palette: {}", current_palette.0);
        }
        CycleTarget::BallStyle => {
            // Cycle all balls to the next/prev style
            for (mut style, mut sprite) in &mut ball_query {
                let new_style_name = if cycle_next {
                    ball_textures.next_style(style.name())
                } else {
                    ball_textures.prev_style(style.name())
                };

                style.0 = new_style_name.to_string();

                // Update sprite texture
                if let Some(textures) = ball_textures.get(style.name()) {
                    if let Some(texture) = textures.textures.get(current_palette.0) {
                        sprite.image = texture.clone();
                    }
                }
            }

            // Log the new style (use first ball's style)
            if let Some((style, _)) = ball_query.iter().next() {
                info!("Ball Style: {}", style.name());
            }
        }
        CycleTarget::AiProfile => {
            // LT toggles which player to edit, RT cycles their profile
            if cycle_prev {
                // Toggle selected player
                cycle_selection.ai_profile_player = match cycle_selection.ai_profile_player {
                    Team::Left => Team::Right,
                    Team::Right => Team::Left,
                };
                info!(
                    "AI Profile: editing {} player",
                    match cycle_selection.ai_profile_player {
                        Team::Left => "Left",
                        Team::Right => "Right",
                    }
                );
            } else if cycle_next {
                // Cycle profile for selected player
                let num_profiles = profile_db.len();
                for (mut ai_state, team) in &mut ai_query {
                    if *team == cycle_selection.ai_profile_player {
                        ai_state.profile_index = (ai_state.profile_index + 1) % num_profiles;
                        let profile = profile_db.get(ai_state.profile_index);
                        info!(
                            "{} AI Profile: {}",
                            match team {
                                Team::Left => "Left",
                                Team::Right => "Right",
                            },
                            profile.name
                        );
                    }
                }
            }
        }
        CycleTarget::MovementPreset => {
            let num = preset_db.movement_len();
            if cycle_next {
                current_presets.movement = (current_presets.movement + 1) % num;
            } else if cycle_prev {
                current_presets.movement = (current_presets.movement + num - 1) % num;
            }
            current_presets.mark_apply();
            if let Some(p) = preset_db.get_movement(current_presets.movement) {
                info!("Movement Preset: {}", p.name);
            }
        }
        CycleTarget::BallPreset => {
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
        CycleTarget::ShootingPreset => {
            let num = preset_db.shooting_len();
            if cycle_next {
                current_presets.shooting = (current_presets.shooting + 1) % num;
            } else if cycle_prev {
                current_presets.shooting = (current_presets.shooting + num - 1) % num;
            }
            current_presets.mark_apply();
            if let Some(p) = preset_db.get_shooting(current_presets.shooting) {
                info!("Shooting Preset: {}", p.name);
            }
        }
        CycleTarget::CompositePreset => {
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
                // Apply level if specified
                if let Some(level) = p.level {
                    current_level.0 = level;
                }
                // Apply palette if specified
                if let Some(palette) = p.palette {
                    current_palette.0 = palette;
                }
                // Apply ball style if specified
                if let Some(ref style_name) = p.ball_style {
                    if let Some(style_textures) = ball_textures.get(style_name) {
                        for (mut style, mut sprite) in &mut ball_query {
                            *style = BallStyle::new(style_name);
                            if let Some(handle) = style_textures.textures.get(current_palette.0) {
                                sprite.image = handle.clone();
                            }
                        }
                    }
                }
                info!("Global Preset: {}", p.name);
            }
        }
    }
}

/// Update cycle indicator display
#[allow(clippy::too_many_arguments)]
pub fn update_cycle_indicator(
    cycle_selection: Res<CycleSelection>,
    current_level: Res<CurrentLevel>,
    current_palette: Res<crate::ball::CurrentPalette>,
    viewport_scale: Res<ViewportScale>,
    current_presets: Res<CurrentPresets>,
    level_db: Res<LevelDatabase>,
    palette_db: Res<PaletteDatabase>,
    profile_db: Res<AiProfileDatabase>,
    preset_db: Res<PresetDatabase>,
    ball_query: Query<&BallStyle, With<Ball>>,
    ai_query: Query<(&AiState, &Team), With<Player>>,
    mut query: Query<(&mut Text2d, &mut Visibility), With<CycleIndicator>>,
) {
    let Ok((mut text, _visibility)) = query.single_mut() else {
        return;
    };

    // Show current target and its value
    let value_str = match cycle_selection.target {
        CycleTarget::Level => format!("{}/{}", current_level.0, level_db.len()),
        CycleTarget::Viewport => {
            let (_, _, label) = viewport_scale.current();
            label.to_string()
        }
        CycleTarget::Palette => format!("{}/{}", current_palette.0 + 1, palette_db.len()),
        CycleTarget::BallStyle => {
            // Get style from first ball
            ball_query
                .iter()
                .next()
                .map(|s| s.name().to_string())
                .unwrap_or_else(|| "None".to_string())
        }
        CycleTarget::AiProfile => {
            // Show both players' profiles, highlight selected with brackets
            let mut left_profile = "?".to_string();
            let mut right_profile = "?".to_string();
            for (ai_state, team) in &ai_query {
                let profile_name = profile_db.get(ai_state.profile_index).name.clone();
                match team {
                    Team::Left => left_profile = profile_name,
                    Team::Right => right_profile = profile_name,
                }
            }
            // Highlight selected player with brackets
            match cycle_selection.ai_profile_player {
                Team::Left => format!("[L:{}] R:{}", left_profile, right_profile),
                Team::Right => format!("L:{} [R:{}]", left_profile, right_profile),
            }
        }
        CycleTarget::MovementPreset => preset_db
            .get_movement(current_presets.movement)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "None".to_string()),
        CycleTarget::BallPreset => preset_db
            .get_ball(current_presets.ball)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "None".to_string()),
        CycleTarget::ShootingPreset => preset_db
            .get_shooting(current_presets.shooting)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "None".to_string()),
        CycleTarget::CompositePreset => preset_db
            .get_composite(current_presets.composite)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "None".to_string()),
    };

    **text = format!("[{}] {}", cycle_selection.target.name(), value_str);
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
