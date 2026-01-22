//! Target basket selection and marker systems

use bevy::prelude::*;

use crate::input::PlayerInput;
use crate::player::Player;
use crate::world::Basket;

/// Which basket player is aiming at
#[derive(Component)]
pub struct TargetBasket(pub Basket);

impl Default for TargetBasket {
    fn default() -> Self {
        Self(Basket::Right) // Default targeting right basket
    }
}

/// White marker shown in targeted basket
#[derive(Component)]
pub struct TargetMarker;

/// Cycle to next target basket
pub fn cycle_target(
    mut input: ResMut<PlayerInput>,
    mut player_query: Query<&mut TargetBasket, With<Player>>,
    baskets: Query<&Basket>,
) {
    if !input.cycle_target_pressed {
        return;
    }
    input.cycle_target_pressed = false; // Consume input

    // Collect unique basket types available
    let mut has_left = false;
    let mut has_right = false;
    for basket in &baskets {
        match basket {
            Basket::Left => has_left = true,
            Basket::Right => has_right = true,
        }
    }

    // Cycle to next available target
    for mut target in &mut player_query {
        target.0 = match target.0 {
            Basket::Left => {
                if has_right {
                    Basket::Right
                } else {
                    Basket::Left
                }
            }
            Basket::Right => {
                if has_left {
                    Basket::Left
                } else {
                    Basket::Right
                }
            }
        };
    }
}

/// Update target marker position
pub fn update_target_marker(
    player_query: Query<(&Transform, &TargetBasket), With<Player>>,
    baskets: Query<(&Transform, &Basket), Without<TargetMarker>>,
    mut marker_query: Query<&mut Transform, (With<TargetMarker>, Without<Player>)>,
) {
    let Ok((player_transform, target)) = player_query.single() else {
        return;
    };

    let player_pos = player_transform.translation.truncate();

    // Find the closest basket matching the target type
    let target_pos = baskets
        .iter()
        .filter(|(_, basket)| **basket == target.0)
        .min_by(|(a, _), (b, _)| {
            let dist_a = player_pos.distance_squared(a.translation.truncate());
            let dist_b = player_pos.distance_squared(b.translation.truncate());
            dist_a.partial_cmp(&dist_b).unwrap()
        })
        .map(|(transform, _)| transform.translation);

    let Some(basket_pos) = target_pos else {
        return;
    };

    // Move marker to target basket
    for mut marker_transform in &mut marker_query {
        marker_transform.translation.x = basket_pos.x;
        marker_transform.translation.y = basket_pos.y;
    }
}
