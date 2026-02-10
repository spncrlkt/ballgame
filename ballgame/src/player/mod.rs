//! Player module - components and physics systems

pub mod animation;
mod components;
mod physics;
pub mod spawn;

pub use animation::{
    animate_player_sprites, load_player_animations, update_player_animation_state,
    PlayerAnimClips, PlayerAnimState, PlayerAnimTimer, PlayerCurrentAnim, PlayerVisual,
};
pub use components::{
    BlockState, Buff, Character, ControlledBy, CoyoteTimer, Facing, Grounded, HoldingBall,
    HumanControlled, JumpState, Player, RemoteControlled, TargetBasket, Team, TurboGauge, Velocity,
};
pub use physics::*;
pub use spawn::{
    color_for_character, initial_facing, spawn_character, spawn_characters_for_mode,
    spawn_charge_gauge, spawn_position, spawn_position_for_level, target_basket_for_character,
    team_for_character, CharacterSpawnConfig,
};
