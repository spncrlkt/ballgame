//! Replay UI components and systems

use bevy::prelude::*;

use crate::constants::*;
use crate::events::{GameEvent, PlayerId};
use crate::player::Team;

use super::ReplayData;
use super::state::ReplayState;

/// Timeline bar at the bottom of screen
#[derive(Component)]
pub struct ReplayTimeline;

/// Timeline progress indicator
#[derive(Component)]
pub struct ReplayTimelineProgress;

/// Time display (current / total)
#[derive(Component)]
pub struct ReplayTimeDisplay;

/// Speed display
#[derive(Component)]
pub struct ReplaySpeedDisplay;

/// Event marker on the timeline
#[derive(Component)]
pub struct ReplayEventMarker {
    pub time_ms: u32,
    pub event_type: EventMarkerType,
}

/// Type of event marker for coloring
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EventMarkerType {
    Goal,
    Steal,
    Pickup,
    AiGoal,
}

impl EventMarkerType {
    pub fn color(&self) -> Color {
        match self {
            EventMarkerType::Goal => Color::srgb(1.0, 0.84, 0.0), // Gold
            EventMarkerType::Steal => Color::srgb(0.9, 0.2, 0.2), // Red
            EventMarkerType::Pickup => Color::srgb(0.9, 0.9, 0.9), // White
            EventMarkerType::AiGoal => Color::srgb(0.3, 0.7, 0.9), // Light blue
        }
    }
}

/// AI goal label above a player
#[derive(Component)]
pub struct PlayerGoalLabel(pub Team);

/// Controls help text
#[derive(Component)]
pub struct ReplayControlsText;

/// Setup the replay UI (called once when replay starts)
pub fn setup_replay_ui(mut commands: Commands, replay_data: Res<ReplayData>) {
    let timeline_y = ARENA_FLOOR_Y - 60.0;
    let timeline_width = ARENA_WIDTH - 100.0;
    let timeline_height = 8.0;

    // Timeline background (dark bar)
    commands.spawn((
        Sprite {
            color: Color::srgba(0.1, 0.1, 0.1, 0.8),
            custom_size: Some(Vec2::new(timeline_width, timeline_height)),
            ..default()
        },
        Transform::from_xyz(0.0, timeline_y, 10.0),
        ReplayTimeline,
    ));

    // Timeline progress (colored, width scales with time)
    commands.spawn((
        Sprite {
            color: Color::srgb(0.3, 0.7, 0.9),
            custom_size: Some(Vec2::new(0.0, timeline_height - 2.0)),
            ..default()
        },
        Transform::from_xyz(-timeline_width / 2.0, timeline_y, 11.0),
        ReplayTimelineProgress,
    ));

    // Event markers on timeline
    let duration = replay_data.duration_ms as f32;
    if duration > 0.0 {
        for event in &replay_data.events {
            let marker_type = match &event.event {
                GameEvent::Goal { .. } => Some(EventMarkerType::Goal),
                GameEvent::StealSuccess { .. } | GameEvent::StealFail { .. } => {
                    Some(EventMarkerType::Steal)
                }
                GameEvent::Pickup { .. } => Some(EventMarkerType::Pickup),
                GameEvent::AiGoal { .. } => Some(EventMarkerType::AiGoal),
                _ => None,
            };

            if let Some(marker_type) = marker_type {
                let x_offset =
                    (event.time_ms as f32 / duration) * timeline_width - timeline_width / 2.0;
                commands.spawn((
                    Sprite {
                        color: marker_type.color(),
                        custom_size: Some(Vec2::new(3.0, timeline_height + 4.0)),
                        ..default()
                    },
                    Transform::from_xyz(x_offset, timeline_y, 12.0),
                    ReplayEventMarker {
                        time_ms: event.time_ms,
                        event_type: marker_type,
                    },
                ));
            }
        }
    }

    // Time display (top-right)
    commands.spawn((
        Text2d::new("0.0s / 0.0s"),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(TEXT_PRIMARY),
        Transform::from_xyz(ARENA_WIDTH / 2.0 - 80.0, ARENA_HEIGHT / 2.0 - 30.0, 10.0),
        ReplayTimeDisplay,
    ));

    // Speed display
    commands.spawn((
        Text2d::new("1.0x"),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(TEXT_ACCENT),
        Transform::from_xyz(ARENA_WIDTH / 2.0 - 80.0, ARENA_HEIGHT / 2.0 - 55.0, 10.0),
        ReplaySpeedDisplay,
    ));

    // Controls help text
    commands.spawn((
        Text2d::new("SPACE: pause | </>: speed | ,/.: step | Home/End: jump"),
        TextFont {
            font_size: 12.0,
            ..default()
        },
        TextColor(TEXT_SECONDARY),
        Transform::from_xyz(0.0, timeline_y - 20.0, 10.0),
        ReplayControlsText,
    ));

    // AI goal labels above each player spawn position
    // Left player label
    commands.spawn((
        Text2d::new(""),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgb(0.2, 0.6, 0.9)),
        Transform::from_xyz(-200.0, PLAYER_SPAWN_LEFT.y + PLAYER_SIZE.y + 20.0, 10.0),
        PlayerGoalLabel(Team::Left),
    ));

    // Right player label
    commands.spawn((
        Text2d::new(""),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgb(0.9, 0.3, 0.2)),
        Transform::from_xyz(200.0, PLAYER_SPAWN_RIGHT.y + PLAYER_SIZE.y + 20.0, 10.0),
        PlayerGoalLabel(Team::Right),
    ));

    // Match info display (top-left)
    let info_text = format!(
        "{} vs {} on {} (seed: {})",
        replay_data.match_info.left_profile,
        replay_data.match_info.right_profile,
        replay_data.match_info.level_name,
        replay_data.match_info.seed
    );
    commands.spawn((
        Text2d::new(info_text),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextLayout::new_with_justify(Justify::Left),
        TextColor(TEXT_PRIMARY),
        Transform::from_xyz(-ARENA_WIDTH / 2.0 + 100.0, ARENA_HEIGHT / 2.0 - 30.0, 10.0),
    ));
}

/// Update the replay UI each frame
pub fn update_replay_ui(
    state: Res<ReplayState>,
    replay_data: Res<ReplayData>,
    mut time_display: Query<&mut Text2d, (With<ReplayTimeDisplay>, Without<ReplaySpeedDisplay>)>,
    mut speed_display: Query<&mut Text2d, (With<ReplaySpeedDisplay>, Without<ReplayTimeDisplay>)>,
    mut progress: Query<(&mut Transform, &mut Sprite), With<ReplayTimelineProgress>>,
    mut goal_labels: Query<
        (&mut Text2d, &mut Transform, &PlayerGoalLabel),
        (
            Without<ReplayTimeDisplay>,
            Without<ReplaySpeedDisplay>,
            Without<ReplayTimelineProgress>,
        ),
    >,
    players: Query<
        (&Transform, &Team),
        (
            With<crate::player::Player>,
            Without<ReplayTimelineProgress>,
            Without<PlayerGoalLabel>,
        ),
    >,
) {
    // Update time display
    for mut text in &mut time_display {
        **text = state.time_string(replay_data.duration_ms);
    }

    // Update speed display
    for mut text in &mut speed_display {
        **text = state.speed_string();
    }

    // Update progress bar
    let timeline_width = ARENA_WIDTH - 100.0;
    let progress_ratio = if replay_data.duration_ms > 0 {
        state.current_time_ms as f32 / replay_data.duration_ms as f32
    } else {
        0.0
    };

    for (mut transform, mut sprite) in &mut progress {
        let new_width = progress_ratio * timeline_width;
        sprite.custom_size = Some(Vec2::new(new_width, 6.0));
        // Anchor at left edge
        transform.translation.x = -timeline_width / 2.0 + new_width / 2.0;
    }

    // Update AI goal labels - position above players and show current goal
    for (mut text, mut transform, label) in &mut goal_labels {
        // Find the player of this team
        for (player_transform, team) in &players {
            if *team == label.0 {
                // Position above player
                transform.translation.x = player_transform.translation.x;
                transform.translation.y =
                    player_transform.translation.y + PLAYER_SIZE.y / 2.0 + 25.0;

                // Get current AI goal
                let player_id = match team {
                    Team::Left => PlayerId::L,
                    Team::Right => PlayerId::R,
                };
                if let Some(goal) = replay_data.current_ai_goal(state.current_time_ms, player_id) {
                    **text = goal.to_string();
                } else {
                    **text = "---".to_string();
                }
                break;
            }
        }
    }
}
