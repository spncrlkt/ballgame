//! AI navigation system - graph-based platform navigation for AI players
//!
//! Precomputes a navigation graph on level load where each platform becomes a node,
//! with edges representing possible jumps/drops between them.

use bevy::prelude::*;

use crate::constants::*;
use crate::levels::LevelDatabase;
use crate::scoring::CurrentLevel;
use crate::world::{BasketRim, CornerRamp, LevelPlatform, Platform};

/// A node in the navigation graph representing a walkable surface
#[derive(Clone, Debug)]
pub struct NavNode {
    /// Unique identifier for this node
    pub id: usize,
    /// Center position of the platform
    pub center: Vec2,
    /// Left edge X coordinate
    pub left_x: f32,
    /// Right edge X coordinate
    pub right_x: f32,
    /// Top Y coordinate (surface the player walks on)
    pub top_y: f32,
    /// Entity this node represents (None for floor)
    pub platform_entity: Option<Entity>,
    /// Whether this is the main floor
    pub is_floor: bool,
}

impl NavNode {
    /// Check if a position is on this platform (within horizontal bounds)
    pub fn contains_x(&self, x: f32) -> bool {
        x >= self.left_x && x <= self.right_x
    }

    /// Get the closest X position on this platform to a given X
    pub fn clamp_x(&self, x: f32) -> f32 {
        // Ensure min <= max (handles edge cases with tiny platforms)
        let min_x = self.left_x.min(self.right_x);
        let max_x = self.left_x.max(self.right_x);
        x.clamp(min_x, max_x)
    }
}

/// Type of edge connecting two navigation nodes
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EdgeType {
    /// Walk along the same platform or connected surfaces
    Walk,
    /// Jump up to reach a higher platform
    Jump,
    /// Drop down to a lower platform
    Drop,
}

/// An edge in the navigation graph connecting two nodes
#[derive(Clone, Debug)]
pub struct NavEdge {
    /// Target node index
    pub to_node: usize,
    /// Type of traversal
    pub edge_type: EdgeType,
    /// Cost of this edge (for pathfinding)
    pub cost: f32,
    /// X position to start the jump/drop from
    pub jump_from_x: f32,
    /// X position expected to land on
    pub land_on_x: f32,
    /// For jumps: how long to hold jump button (0.0 = tap, 1.0 = full hold)
    pub jump_hold_duration: f32,
}

/// Resource containing the navigation graph for the current level
#[derive(Resource, Default)]
pub struct NavGraph {
    /// All navigation nodes
    pub nodes: Vec<NavNode>,
    /// Adjacency list of edges (edges[i] = edges from node i)
    pub edges: Vec<Vec<NavEdge>>,
    /// Whether the graph needs rebuilding
    pub dirty: bool,
    /// Level index this graph was built for
    pub built_for_level: usize,
}

impl NavGraph {
    /// Find which node a position is standing on (if any)
    pub fn find_node_at(&self, pos: Vec2, tolerance: f32) -> Option<usize> {
        // Check if position is on any platform
        for (i, node) in self.nodes.iter().enumerate() {
            // Must be within horizontal bounds
            if pos.x < node.left_x - tolerance || pos.x > node.right_x + tolerance {
                continue;
            }
            // Must be close to the top surface (standing on it)
            let y_diff = (pos.y - node.top_y).abs();
            // Player half-height + some tolerance
            if y_diff < PLAYER_SIZE.y / 2.0 + tolerance {
                return Some(i);
            }
        }
        None
    }

    /// Find the closest node to a target position
    pub fn find_closest_node(&self, target: Vec2) -> Option<usize> {
        self.nodes
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                let dist_a = a.center.distance_squared(target);
                let dist_b = b.center.distance_squared(target);
                dist_a.partial_cmp(&dist_b).unwrap()
            })
            .map(|(i, _)| i)
    }

    /// Find the best node from which to shoot at a target basket position
    pub fn find_shooting_node(&self, target: Vec2, shoot_range: f32) -> Option<usize> {
        // Find all nodes within shooting range of target
        let mut candidates: Vec<(usize, f32)> = self
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(i, node)| {
                let dist = node.center.distance(target);
                if dist <= shoot_range {
                    Some((i, dist))
                } else {
                    None
                }
            })
            .collect();

        // Sort by distance (prefer closer positions for better accuracy)
        candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        if let Some((i, _)) = candidates.first() {
            return Some(*i);
        }

        // No platform within shoot_range - find the closest one anyway
        // This ensures AI navigates to the best possible position
        self.nodes
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                let dist_a = a.center.distance(target);
                let dist_b = b.center.distance(target);
                dist_a.partial_cmp(&dist_b).unwrap()
            })
            .map(|(i, _)| i)
    }
}

/// Actions the AI can take to navigate between nodes
#[derive(Clone, Debug)]
pub enum NavAction {
    /// Walk to a specific X position on current platform
    WalkTo { x: f32 },
    /// Jump at a specific X position with given hold duration
    JumpAt { x: f32, hold_duration: f32 },
    /// Drop from a specific X position (walk off edge)
    DropFrom { x: f32 },
    /// Walk off edge in a direction to fall
    WalkOffEdge { direction: f32 },
}

/// Component tracking AI navigation state
#[derive(Component, Default)]
pub struct AiNavState {
    /// Current path being followed (sequence of actions)
    pub current_path: Vec<NavAction>,
    /// Index of current action in path
    pub path_index: usize,
    /// Whether the current action has been started
    pub action_started: bool,
    /// Timer for held jumps
    pub jump_timer: f32,
    /// Target position we're navigating to
    pub nav_target: Option<Vec2>,
    /// Whether navigation is active
    pub active: bool,
}

impl AiNavState {
    /// Get the current action (if any)
    pub fn current_action(&self) -> Option<&NavAction> {
        self.current_path.get(self.path_index)
    }

    /// Advance to the next action
    pub fn advance(&mut self) {
        self.path_index += 1;
        self.action_started = false;
        self.jump_timer = 0.0;
    }

    /// Check if path is complete
    pub fn path_complete(&self) -> bool {
        self.path_index >= self.current_path.len()
    }

    /// Clear the current path
    pub fn clear(&mut self) {
        self.current_path.clear();
        self.path_index = 0;
        self.action_started = false;
        self.jump_timer = 0.0;
        self.nav_target = None;
        self.active = false;
    }

    /// Set a new path
    pub fn set_path(&mut self, path: Vec<NavAction>, target: Vec2) {
        self.current_path = path;
        self.path_index = 0;
        self.action_started = false;
        self.jump_timer = 0.0;
        self.nav_target = Some(target);
        self.active = true;
    }
}

/// System to rebuild the navigation graph when level changes
pub fn rebuild_nav_graph(
    mut nav_graph: ResMut<NavGraph>,
    current_level: Res<CurrentLevel>,
    _level_db: Res<LevelDatabase>,
    platform_query: Query<
        (Entity, &GlobalTransform, &Sprite, Option<&CornerRamp>),
        (With<Platform>, Without<BasketRim>),
    >,
    level_platform_query: Query<Entity, With<LevelPlatform>>,
    corner_ramp_query: Query<Entity, With<CornerRamp>>,
) {
    // Check if we need to rebuild
    // CurrentLevel uses 1-based numbering, convert to 0-based index
    let level_idx = (current_level.0.saturating_sub(1)) as usize;
    if !nav_graph.dirty && nav_graph.built_for_level == level_idx && !nav_graph.nodes.is_empty() {
        return;
    }

    info!("Rebuilding nav graph for level {}", current_level.0);

    nav_graph.nodes.clear();
    nav_graph.edges.clear();

    // Create floor node
    let floor_left = -ARENA_WIDTH / 2.0 + WALL_THICKNESS;
    let floor_right = ARENA_WIDTH / 2.0 - WALL_THICKNESS;
    let floor_y = ARENA_FLOOR_Y + 20.0; // Floor surface is 20 units above ARENA_FLOOR_Y

    nav_graph.nodes.push(NavNode {
        id: 0,
        center: Vec2::new(0.0, floor_y),
        left_x: floor_left,
        right_x: floor_right,
        top_y: floor_y,
        platform_entity: None,
        is_floor: true,
    });

    // Collect level platforms and corner ramps
    let level_platforms: Vec<Entity> = level_platform_query.iter().collect();
    let corner_ramps: Vec<Entity> = corner_ramp_query.iter().collect();

    // Add platform nodes (including corner ramp steps)
    for (entity, transform, sprite, is_corner_ramp) in platform_query.iter() {
        let is_level_platform = level_platforms.contains(&entity);
        let is_ramp = corner_ramps.contains(&entity) || is_corner_ramp.is_some();

        // Include level platforms and corner ramps
        if !is_level_platform && !is_ramp {
            // Check if this is the floor (already added)
            let pos = transform.translation();
            if (pos.y - ARENA_FLOOR_Y).abs() < 5.0 {
                continue; // Skip floor, already added
            }
            // Skip walls (very tall sprites)
            if sprite.custom_size.is_some_and(|size| size.y > 500.0) {
                continue;
            }
            continue; // Skip other non-level platforms
        }

        let pos = transform.translation();
        let size = sprite.custom_size.unwrap_or(Vec2::new(100.0, 20.0));

        let half_width = size.x.abs() / 2.0;
        let half_height = size.y.abs() / 2.0;

        // Ensure left_x < right_x
        let left_x = pos.x - half_width;
        let right_x = pos.x + half_width;

        let node = NavNode {
            id: nav_graph.nodes.len(),
            center: Vec2::new(pos.x, pos.y + half_height),
            left_x: left_x.min(right_x),
            right_x: left_x.max(right_x),
            top_y: pos.y + half_height,
            platform_entity: Some(entity),
            is_floor: false,
        };

        nav_graph.nodes.push(node);
    }

    // Build edges between nodes
    let node_count = nav_graph.nodes.len();
    nav_graph.edges = vec![Vec::new(); node_count];

    for i in 0..node_count {
        for j in 0..node_count {
            if i == j {
                continue;
            }

            let from = &nav_graph.nodes[i];
            let to = &nav_graph.nodes[j];

            // Check if we can reach node j from node i
            if let Some(edge) = calculate_edge(from, to) {
                nav_graph.edges[i].push(edge);
            }
        }
    }

    nav_graph.dirty = false;
    nav_graph.built_for_level = level_idx;

    info!(
        "Nav graph built: {} nodes, {} total edges",
        nav_graph.nodes.len(),
        nav_graph.edges.iter().map(|e| e.len()).sum::<usize>()
    );
}

/// Calculate if an edge exists between two nodes and what type
fn calculate_edge(from: &NavNode, to: &NavNode) -> Option<NavEdge> {
    let height_diff = to.top_y - from.top_y;

    // Check horizontal overlap/reachability
    let horizontal_gap = if to.left_x > from.right_x {
        to.left_x - from.right_x // Gap between platforms
    } else if from.left_x > to.right_x {
        from.left_x - to.right_x
    } else {
        0.0 // Platforms overlap horizontally
    };

    // Calculate jump physics
    // Max jump height: v^2 / (2g) = 650^2 / (2*980) = ~215px
    // Using rise gravity for jump arc calculation
    let max_jump_height = JUMP_VELOCITY * JUMP_VELOCITY / (2.0 * GRAVITY_RISE);

    if height_diff > 0.0 {
        // Need to jump UP to reach target
        if height_diff > NAV_MAX_JUMP_HEIGHT {
            return None; // Too high to reach
        }

        // Calculate time to reach height
        // Using kinematic: h = v*t - 0.5*g*t^2
        // Solving for t when at height h: t = (v - sqrt(v^2 - 2*g*h)) / g
        let v = JUMP_VELOCITY;
        let g = GRAVITY_RISE;
        let discriminant = v * v - 2.0 * g * height_diff;
        if discriminant < 0.0 {
            return None; // Can't reach this height
        }

        let time_to_height = (v - discriminant.sqrt()) / g;
        let horizontal_reach = MOVE_SPEED * time_to_height;

        // Check if we can reach horizontally
        if horizontal_gap > horizontal_reach + NAV_POSITION_TOLERANCE {
            return None; // Too far horizontally
        }

        // Calculate jump point and landing point
        let (jump_from_x, land_on_x) = if to.left_x > from.right_x {
            // Target is to the right
            (from.right_x - NAV_JUMP_TOLERANCE, to.left_x + NAV_JUMP_TOLERANCE)
        } else if from.left_x > to.right_x {
            // Target is to the left
            (from.left_x + NAV_JUMP_TOLERANCE, to.right_x - NAV_JUMP_TOLERANCE)
        } else {
            // Platforms overlap - jump from closest point
            let overlap_center = ((from.left_x.max(to.left_x)) + (from.right_x.min(to.right_x))) / 2.0;
            (overlap_center, overlap_center)
        };

        // Calculate hold duration (how much of max jump needed)
        // Full hold = max height, tap = ~40% height (due to cut multiplier)
        let height_ratio = height_diff / max_jump_height;
        let hold_duration = (height_ratio * 1.2).clamp(0.1, 1.0); // Overshoot a bit for safety

        let cost = height_diff + horizontal_gap * 0.5; // Prefer shorter jumps

        Some(NavEdge {
            to_node: to.id,
            edge_type: EdgeType::Jump,
            cost,
            jump_from_x,
            land_on_x,
            jump_hold_duration: hold_duration,
        })
    } else if height_diff < -PLAYER_SIZE.y {
        // Need to DROP down to reach target (significant height difference)

        // Calculate fall time and horizontal reach while falling
        let fall_height = -height_diff;
        // Using fall gravity: h = 0.5*g*t^2, so t = sqrt(2h/g)
        let fall_time = (2.0 * fall_height / GRAVITY_FALL).sqrt();
        let horizontal_reach = MOVE_SPEED * fall_time;

        // Check if we can reach horizontally
        if horizontal_gap > horizontal_reach + NAV_POSITION_TOLERANCE {
            return None;
        }

        // Calculate drop point and landing point
        let (drop_from_x, land_on_x) = if to.center.x > from.center.x {
            // Target is to the right - drop from right edge
            (from.right_x, to.clamp_x(from.right_x + horizontal_reach * 0.5))
        } else {
            // Target is to the left - drop from left edge
            (from.left_x, to.clamp_x(from.left_x - horizontal_reach * 0.5))
        };

        let cost = fall_height * 0.3 + horizontal_gap * 0.5; // Drops are cheaper than jumps

        Some(NavEdge {
            to_node: to.id,
            edge_type: EdgeType::Drop,
            cost,
            jump_from_x: drop_from_x,
            land_on_x,
            jump_hold_duration: 0.0,
        })
    } else {
        // Platforms are at similar height - check if we can walk between them
        // This handles overlapping platforms or platforms with a small step

        if horizontal_gap > NAV_POSITION_TOLERANCE {
            // Need a small hop to cross gap
            if horizontal_gap > MOVE_SPEED * 0.5 {
                return None; // Gap too wide for simple hop
            }

            let (jump_from_x, land_on_x) = if to.left_x > from.right_x {
                (from.right_x - NAV_JUMP_TOLERANCE, to.left_x + NAV_JUMP_TOLERANCE)
            } else {
                (from.left_x + NAV_JUMP_TOLERANCE, to.right_x - NAV_JUMP_TOLERANCE)
            };

            Some(NavEdge {
                to_node: to.id,
                edge_type: EdgeType::Jump,
                cost: horizontal_gap,
                jump_from_x,
                land_on_x,
                jump_hold_duration: 0.1, // Short hop
            })
        } else {
            // Platforms overlap or touch - can walk
            let walk_to_x = if to.center.x > from.center.x {
                to.left_x + NAV_POSITION_TOLERANCE
            } else {
                to.right_x - NAV_POSITION_TOLERANCE
            };

            Some(NavEdge {
                to_node: to.id,
                edge_type: EdgeType::Walk,
                cost: (from.center.x - to.center.x).abs() * 0.1,
                jump_from_x: walk_to_x,
                land_on_x: walk_to_x,
                jump_hold_duration: 0.0,
            })
        }
    }
}

/// Mark nav graph as dirty when level changes
pub fn mark_nav_dirty_on_level_change(
    current_level: Res<CurrentLevel>,
    mut nav_graph: ResMut<NavGraph>,
) {
    if current_level.is_changed() {
        nav_graph.dirty = true;
    }
}
