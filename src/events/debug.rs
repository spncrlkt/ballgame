//! Debug sampling for manual reachability capture.

use bevy::prelude::*;

use crate::ai::{AiNavState, InputState};
use crate::events::types::PlayerId;
use crate::player::{CoyoteTimer, Facing, Grounded, JumpState, Player, Team, Velocity};

pub const DEBUG_TICK_MS: u32 = 50;

#[derive(Debug, Clone)]
pub struct DebugSample {
    pub time_ms: u32,
    pub tick_frame: u64,
    pub player: PlayerId,
    pub pos_x: f32,
    pub pos_y: f32,
    pub vel_x: f32,
    pub vel_y: f32,
    pub input_move_x: f32,
    pub input_jump: bool,
    pub grounded: bool,
    pub is_jumping: bool,
    pub coyote_timer: f32,
    pub jump_buffer_timer: f32,
    pub facing: f32,
    pub nav_active: bool,
    pub nav_path_index: i64,
    pub nav_action: Option<String>,
    pub level_id: String,
    pub human_controlled: bool,
}

#[derive(Resource, Default)]
pub struct DebugSampleBuffer {
    pub samples: Vec<DebugSample>,
}

pub fn tick_frame_from_time(time_ms: u32) -> u64 {
    (time_ms / DEBUG_TICK_MS) as u64
}

pub fn push_debug_samples(
    buffer: &mut DebugSampleBuffer,
    time_ms: u32,
    tick_frame: u64,
    level_id: &str,
    query: &Query<
        (
            &Team,
            &Transform,
            &Velocity,
            &InputState,
            &Grounded,
            &JumpState,
            &CoyoteTimer,
            &Facing,
            Option<&AiNavState>,
            Option<&crate::player::HumanControlled>,
        ),
        With<Player>,
    >,
) {
    for (
        team,
        transform,
        velocity,
        input,
        grounded,
        jump_state,
        coyote_timer,
        facing,
        nav_state,
        human_controlled,
    ) in query.iter()
    {
        let player_id = match team {
            Team::Left => PlayerId::L,
            Team::Right => PlayerId::R,
        };
        let nav_action = nav_state
            .and_then(|nav| nav.current_action())
            .map(|action| format!("{:?}", action));
        let nav_active = nav_state.map(|nav| nav.active).unwrap_or(false);
        let nav_path_index = nav_state.map(|nav| nav.path_index as i64).unwrap_or(-1);

        buffer.samples.push(DebugSample {
            time_ms,
            tick_frame,
            player: player_id,
            pos_x: transform.translation.x,
            pos_y: transform.translation.y,
            vel_x: velocity.0.x,
            vel_y: velocity.0.y,
            input_move_x: input.move_x,
            input_jump: input.jump_held,
            grounded: grounded.0,
            is_jumping: jump_state.is_jumping,
            coyote_timer: coyote_timer.0,
            jump_buffer_timer: input.jump_buffer_timer,
            facing: facing.0,
            nav_active,
            nav_path_index,
            nav_action,
            level_id: level_id.to_string(),
            human_controlled: human_controlled.is_some(),
        });
    }
}
