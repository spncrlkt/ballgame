//! Player module - components and physics systems

mod components;
mod physics;
pub mod spawn;

pub use components::*;
pub use physics::*;
pub use spawn::{
    CharacterSpawnConfig, color_for_character, initial_facing, spawn_character,
    spawn_charge_gauge, spawn_characters_for_mode, spawn_position, target_basket_for_character,
    team_for_character,
};
