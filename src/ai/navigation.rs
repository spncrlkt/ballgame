//! AI navigation system - graph-based platform navigation for AI players
//!
//! Precomputes a navigation graph on level load where each platform becomes a node,
//! with edges representing possible jumps/drops between them.

use bevy::prelude::*;

use crate::ai::capabilities::AiCapabilities;
use crate::ai::shot_quality::evaluate_shot_quality;
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
    /// Pre-computed shot quality for shooting at left basket (0.0-1.0)
    pub shot_quality_left: f32,
    /// Pre-computed shot quality for shooting at right basket (0.0-1.0)
    pub shot_quality_right: f32,
    /// Classification of this platform's role
    pub platform_role: PlatformRole,
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

/// Classification of a platform's role for AI decision-making
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum PlatformRole {
    /// Main arena floor
    #[default]
    Floor,
    /// Corner ramp steps
    Ramp,
    /// Good position for shooting (quality >= 0.55)
    ShotPosition,
    /// Poor position - avoid (under basket, quality < 0.25)
    DeadZone,
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
    /// Frames to wait before rebuilding (allows platform spawning)
    pub rebuild_delay: u8,
    /// Maximum achievable shot quality on this level (for scaling AI thresholds)
    pub level_max_shot_quality: f32,
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

    /// Find the best node from which to shoot at a target basket position.
    /// Takes `min_shot_quality` to filter out positions where shot quality is too low
    /// (e.g., directly under the basket where shots are nearly impossible).
    /// Uses pre-computed shot quality values for efficiency.
    pub fn find_shooting_node(
        &self,
        target: Vec2,
        shoot_range: f32,
        min_shot_quality: f32,
    ) -> Option<usize> {
        // Determine which basket we're shooting at based on target position
        let shooting_at_left_basket = target.x < 0.0;

        // Find all nodes within shooting range of target that meet quality threshold
        // Skip DeadZone nodes
        let mut candidates: Vec<(usize, f32, f32)> = self
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(i, node)| {
                // Skip dead zones
                if node.platform_role == PlatformRole::DeadZone {
                    return None;
                }

                let dist = node.center.distance(target);
                if dist <= shoot_range {
                    // Use pre-computed shot quality
                    let quality = if shooting_at_left_basket {
                        node.shot_quality_left
                    } else {
                        node.shot_quality_right
                    };
                    if quality >= min_shot_quality {
                        Some((i, dist, quality))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        // Sort by quality (higher = better), then by distance (closer = better)
        candidates.sort_by(|a, b| {
            b.2.partial_cmp(&a.2) // Quality descending
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.1.partial_cmp(&b.1).unwrap()) // Distance ascending
        });

        if let Some((i, _, _)) = candidates.first() {
            return Some(*i);
        }

        // No platform within shoot_range meeting quality threshold - find the best quality one
        // even if outside range, so AI navigates toward a usable shooting position
        // Skip DeadZone nodes
        self.nodes
            .iter()
            .enumerate()
            .filter_map(|(i, node)| {
                // Skip dead zones
                if node.platform_role == PlatformRole::DeadZone {
                    return None;
                }

                // Use pre-computed shot quality
                let quality = if shooting_at_left_basket {
                    node.shot_quality_left
                } else {
                    node.shot_quality_right
                };
                if quality >= min_shot_quality {
                    Some((i, quality))
                } else {
                    None
                }
            })
            .max_by(|(_, q1), (_, q2)| q1.partial_cmp(q2).unwrap())
            .map(|(i, _)| i)
    }

    /// Find the node with the best shot quality for a given target basket.
    /// Excludes DeadZone nodes.
    pub fn find_best_shot_position(&self, target: Vec2) -> Option<usize> {
        let shooting_at_left = target.x < 0.0;
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node.platform_role != PlatformRole::DeadZone)
            .max_by(|(_, a), (_, b)| {
                let qa = if shooting_at_left {
                    a.shot_quality_left
                } else {
                    a.shot_quality_right
                };
                let qb = if shooting_at_left {
                    b.shot_quality_left
                } else {
                    b.shot_quality_right
                };
                qa.partial_cmp(&qb).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
    }

    /// Get the shot quality for a specific node shooting at a target basket.
    pub fn get_shot_quality(&self, node_idx: usize, target: Vec2) -> f32 {
        if node_idx >= self.nodes.len() {
            return 0.0;
        }
        let shooting_at_left = target.x < 0.0;
        if shooting_at_left {
            self.nodes[node_idx].shot_quality_left
        } else {
            self.nodes[node_idx].shot_quality_right
        }
    }

    /// Estimate cost to reach a node from current position.
    /// Weights: horizontal + going up (moderate cost) + going down (cheap).
    /// Going up requires jumping, going down is just dropping.
    pub fn estimate_path_cost(&self, from: Vec2, to_node: usize) -> f32 {
        if to_node >= self.nodes.len() {
            return f32::MAX;
        }
        let target = self.nodes[to_node].center;
        let dx = (from.x - target.x).abs();
        let dy = target.y - from.y; // Positive = going up, negative = going down
        let dy_up = dy.max(0.0);
        let dy_down = (-dy).max(0.0);
        // Going up costs more (jumping), going down is cheap (just drop)
        dx + dy_up * 1.2 + dy_down * 0.3
    }

    /// Find a platform suitable for defending against an elevated opponent.
    /// Returns a platform that:
    /// 1. Is at or above the opponent's height (within tolerance)
    /// 2. Is horizontally positioned to intercept between opponent and basket
    /// 3. Is reachable (has edges from floor)
    pub fn find_defensive_platform(
        &self,
        opponent_pos: Vec2,
        basket_pos: Vec2,
        min_height: f32,
    ) -> Option<usize> {
        // Determine which side we want to defend from
        let defend_left = basket_pos.x < opponent_pos.x;
        let mut best_node = None;
        let mut best_score = f32::MAX;

        for (idx, node) in self.nodes.iter().enumerate() {
            // Skip floor - we want elevated positions
            if node.is_floor {
                continue;
            }

            // Platform must be at or above min_height (opponent's position minus some tolerance)
            if node.top_y < min_height - PLAYER_SIZE.y {
                continue;
            }

            // Check if platform is on the correct side (between opponent and basket)
            let on_correct_side = if defend_left {
                node.center.x < opponent_pos.x
            } else {
                node.center.x > opponent_pos.x
            };

            let x_dist = (node.center.x - opponent_pos.x).abs();
            let height_diff = (node.top_y - opponent_pos.y).abs();

            // Scoring: prefer correct side, moderate distance, similar height
            // Large penalty for being on wrong side
            let side_penalty = if on_correct_side { 0.0 } else { 500.0 };
            // Penalize being too close (< 50px) to avoid clustering
            let dist_penalty = if x_dist < 50.0 { 100.0 } else { x_dist };
            // Prefer similar height to opponent
            let height_penalty = height_diff * 0.5;

            let score = side_penalty + dist_penalty + height_penalty;

            if score < best_score {
                best_score = score;
                best_node = Some(idx);
            }
        }

        best_node
    }

    /// Find the floor node (main arena floor).
    pub fn find_floor_node(&self) -> Option<usize> {
        self.nodes.iter().position(|n| n.is_floor)
    }

    /// Find the best elevated platform for the AI to navigate to when no good
    /// shooting position is found. Returns the highest reachable platform with
    /// decent shot quality.
    pub fn find_elevated_platform(
        &self,
        target: Vec2,
        min_shot_quality: f32,
    ) -> Option<usize> {
        let shooting_at_left_basket = target.x < 0.0;

        // Find elevated platforms (not floor) with decent shot quality
        // Prefer higher platforms
        self.nodes
            .iter()
            .enumerate()
            .filter_map(|(i, node)| {
                // Skip floor and dead zones
                if node.is_floor || node.platform_role == PlatformRole::DeadZone {
                    return None;
                }

                let quality = if shooting_at_left_basket {
                    node.shot_quality_left
                } else {
                    node.shot_quality_right
                };

                // Accept platforms with decent quality (lower threshold than shooting)
                if quality >= min_shot_quality * 0.7 {
                    Some((i, node.top_y, quality))
                } else {
                    None
                }
            })
            // Sort by height (higher = better), then quality
            .max_by(|(_, h1, q1), (_, h2, q2)| {
                h1.partial_cmp(h2)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| q1.partial_cmp(q2).unwrap())
            })
            .map(|(i, _, _)| i)
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

    /// Check and auto-clear navigation when path completes
    /// Call this at the end of navigation execution each frame
    pub fn update_completion(&mut self) {
        if self.active && self.path_complete() {
            self.clear();
        }
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
        (Entity, &Transform, &Sprite, Option<&CornerRamp>),
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

    // Wait for rebuild delay (allows platforms to spawn/despawn after level change)
    if nav_graph.rebuild_delay > 0 {
        nav_graph.rebuild_delay -= 1;
        return;
    }

    // Check if level platforms have spawned yet
    // Levels 2+ should have platforms; if none found, wait for next frame
    let level_platform_count = level_platform_query.iter().count();
    if level_idx >= 1 && level_platform_count == 0 {
        // Level platforms haven't spawned yet - keep graph dirty and wait
        nav_graph.dirty = true;
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
        shot_quality_left: 0.0,  // Will be computed after all nodes are added
        shot_quality_right: 0.0,
        platform_role: PlatformRole::Floor,
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
            let pos = transform.translation;
            if (pos.y - ARENA_FLOOR_Y).abs() < 5.0 {
                continue; // Skip floor, already added
            }
            // Skip walls (very tall sprites)
            if sprite.custom_size.is_some_and(|size| size.y > 500.0) {
                continue;
            }
            continue; // Skip other non-level platforms
        }

        let pos = transform.translation;
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
            shot_quality_left: 0.0,  // Will be computed after all nodes are added
            shot_quality_right: 0.0,
            platform_role: if is_ramp { PlatformRole::Ramp } else { PlatformRole::ShotPosition },
        };

        nav_graph.nodes.push(node);
    }

    // Pre-compute shot qualities for all nodes
    // Basket positions are at ±BASKET_PUSH_IN from arena edges
    let basket_x_offset = ARENA_WIDTH / 2.0 - WALL_THICKNESS - BASKET_PUSH_IN;
    let basket_y = ARENA_FLOOR_Y + BASKET_SIZE.y / 2.0 + 200.0; // Approximate basket center height
    let left_basket = Vec2::new(-basket_x_offset, basket_y);
    let right_basket = Vec2::new(basket_x_offset, basket_y);

    for node in &mut nav_graph.nodes {
        node.shot_quality_left = evaluate_shot_quality(node.center, left_basket);
        node.shot_quality_right = evaluate_shot_quality(node.center, right_basket);
        node.platform_role = classify_platform_role(node);
    }

    // Calculate level's max achievable shot quality (for AI threshold scaling)
    nav_graph.level_max_shot_quality = nav_graph
        .nodes
        .iter()
        .map(|n| n.shot_quality_left.max(n.shot_quality_right))
        .fold(0.3_f32, |acc, q| acc.max(q));

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
            if let Some(edge) = calculate_edge(from, to, &nav_graph.nodes) {
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

    // Debug: log nav graph structure
    debug!("=== Nav Graph Debug ===");
    for node in &nav_graph.nodes {
        let role_str = match node.platform_role {
            PlatformRole::Floor => "Floor",
            PlatformRole::Ramp => "Ramp",
            PlatformRole::ShotPosition => "Shot",
            PlatformRole::DeadZone => "Dead",
        };
        let edges = &nav_graph.edges[node.id];
        let edge_summary: Vec<String> = edges
            .iter()
            .map(|e| {
                let edge_type = match e.edge_type {
                    EdgeType::Walk => "W",
                    EdgeType::Jump => "J",
                    EdgeType::Drop => "D",
                };
                format!("{}->{}({})", node.id, e.to_node, edge_type)
            })
            .collect();
        debug!(
            "  Node {}: {:?} @ ({:.0}, {:.0}) x:[{:.0}, {:.0}] role={} edges=[{}]",
            node.id,
            if node.is_floor { "FLOOR" } else { "PLAT" },
            node.center.x,
            node.top_y,
            node.left_x,
            node.right_x,
            role_str,
            edge_summary.join(", ")
        );
    }
    debug!("=== End Nav Graph ===");
}

/// Check if any platform would block the trajectory between two nodes
fn is_trajectory_blocked(from: &NavNode, to: &NavNode, all_nodes: &[NavNode]) -> bool {
    let min_y = from.top_y.min(to.top_y);
    let max_y = from.top_y.max(to.top_y);

    // Determine horizontal span of trajectory
    let traj_left = from.center.x.min(to.center.x);
    let traj_right = from.center.x.max(to.center.x);

    for node in all_nodes {
        // Skip the from and to nodes
        if node.id == from.id || node.id == to.id {
            continue;
        }

        // Skip floor (can always jump from floor)
        if node.is_floor {
            continue;
        }

        // Check if this platform is in the vertical range of the trajectory
        // Platform top must be between from and to heights (with margin for player height)
        let platform_in_y_range = node.top_y > min_y + PLAYER_SIZE.y / 2.0
            && node.top_y < max_y - PLAYER_SIZE.y / 2.0;

        if !platform_in_y_range {
            continue;
        }

        // Check if platform overlaps horizontally with trajectory
        let platform_overlaps_x = node.right_x > traj_left - PLAYER_SIZE.x / 2.0
            && node.left_x < traj_right + PLAYER_SIZE.x / 2.0;

        if platform_overlaps_x {
            return true; // Blocked!
        }
    }

    false
}

/// Calculate if an edge exists between two nodes and what type
fn calculate_edge(from: &NavNode, to: &NavNode, all_nodes: &[NavNode]) -> Option<NavEdge> {
    // Check if any platform blocks the jump/drop trajectory
    if is_trajectory_blocked(from, to, all_nodes) {
        return None;
    }

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
        // Use larger margin for landing to ensure AI clears platform edge and lands safely
        let edge_margin = PLAYER_SIZE.x / 2.0 + NAV_JUMP_TOLERANCE; // ~24px from edge
        let (jump_from_x, land_on_x) = if to.left_x > from.right_x {
            // Target is to the right - jump from our right edge, land on their left + margin
            (from.right_x - NAV_JUMP_TOLERANCE, to.left_x + edge_margin)
        } else if from.left_x > to.right_x {
            // Target is to the left - jump from our left edge, land on their right - margin
            (from.left_x + NAV_JUMP_TOLERANCE, to.right_x - edge_margin)
        } else {
            // Platforms overlap - jump from OUTSIDE the overlap, not center
            // This ensures we arc over the platform rather than jumping straight up into it
            let overlap_left = from.left_x.max(to.left_x);
            let overlap_right = from.right_x.min(to.right_x);

            if from.center.x < to.center.x {
                // Target is to our right - jump from left edge of overlap (outside it)
                let jump_x = overlap_left - PLAYER_SIZE.x / 2.0 - NAV_JUMP_TOLERANCE;
                (jump_x.max(from.left_x), overlap_left + edge_margin)
            } else {
                // Target is to our left - jump from right edge of overlap (outside it)
                let jump_x = overlap_right + PLAYER_SIZE.x / 2.0 + NAV_JUMP_TOLERANCE;
                (jump_x.min(from.right_x), overlap_right - edge_margin)
            }
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

/// Classify a platform's role based on its shot quality
fn classify_platform_role(node: &NavNode) -> PlatformRole {
    // Preserve floor and ramp designations
    if node.is_floor {
        return PlatformRole::Floor;
    }

    let best_quality = node.shot_quality_left.max(node.shot_quality_right);
    let worst_quality = node.shot_quality_left.min(node.shot_quality_right);

    if worst_quality < 0.25 {
        // Bad for at least one basket - mark as dead zone
        PlatformRole::DeadZone
    } else if best_quality >= 0.55 {
        PlatformRole::ShotPosition
    } else {
        // Keep existing role (Ramp for corner steps, or default)
        PlatformRole::Ramp
    }
}

/// Mark nav graph as dirty when level changes
pub fn mark_nav_dirty_on_level_change(
    current_level: Res<CurrentLevel>,
    mut nav_graph: ResMut<NavGraph>,
) {
    if current_level.is_changed() {
        nav_graph.dirty = true;
        // Add delay to allow platforms to spawn/despawn
        nav_graph.rebuild_delay = 3;
    }
}

/// Check if there's a platform directly above a position that would block a jump.
/// Returns true if jumping from this position would bonk the AI's head on a ceiling.
/// Uses AiCapabilities for physics calculations.
pub fn has_ceiling_above(pos: Vec2, capabilities: &AiCapabilities, nav_graph: &NavGraph) -> bool {
    let platforms = crate::ai::world_model::extract_platforms_from_nav(&nav_graph.nodes);
    !capabilities.has_ceiling_clearance(pos, &platforms)
}

/// Find the nearest X position to escape from under a blocking platform.
/// Returns the closest platform edge (left or right) that the AI can move to
/// in order to clear the ceiling and then jump.
/// Uses AiCapabilities for physics calculations.
pub fn find_escape_x(pos: Vec2, target_y: f32, capabilities: &AiCapabilities, nav_graph: &NavGraph) -> Option<f32> {
    let platforms = crate::ai::world_model::extract_platforms_from_nav(&nav_graph.nodes);
    capabilities.find_escape_x(pos, target_y, &platforms)
}
