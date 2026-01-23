//! Ball module - components, physics, and interaction systems

mod components;
mod interaction;
mod physics;

pub use components::*;
pub use interaction::*;
pub use physics::*;

// Re-export Velocity from player since it's shared
pub use crate::player::Velocity;
