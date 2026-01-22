//! Ball module - components, physics, and interaction systems

mod components;
mod physics;
mod interaction;

pub use components::*;
pub use physics::*;
pub use interaction::*;

// Re-export Velocity from player since it's shared
pub use crate::player::Velocity;
