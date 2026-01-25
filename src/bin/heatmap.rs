//! Heatmap generator for shot trajectories
//!
//! Generates heatmaps for shot analysis:
//! - **speed** (default): Shot angle (arrow direction) and required speed (color)
//! - **score**: Scoring percentage via Monte Carlo simulation with rim physics
//!
//! Usage:
//!   cargo run --bin heatmap              # Default: speed heatmap
//!   cargo run --bin heatmap -- speed     # Explicit: speed heatmap
//!   cargo run --bin heatmap -- score     # Scoring percentage heatmap

use ballgame::{
    ARENA_FLOOR_Y, ARENA_HEIGHT, ARENA_WIDTH, BALL_BOUNCE, BALL_GRAVITY, BASKET_PUSH_IN,
    RIM_THICKNESS, SHOT_DISTANCE_VARIANCE, SHOT_MIN_VARIANCE, WALL_THICKNESS,
    calculate_shot_trajectory,
};

// Basket dimensions (matching ballgame constants)
const BASKET_SIZE_X: f32 = 60.0;
const BASKET_SIZE_Y: f32 = 80.0;
const BALL_RADIUS: f32 = 12.0;

// Derived basket position (wall_inner - basket_push_in)
const RIGHT_BASKET_X: f32 = ARENA_WIDTH / 2.0 - WALL_THICKNESS - BASKET_PUSH_IN;
use image::{Rgb, RgbImage};
use rand::Rng;

// Grid settings
const CELL_SIZE: u32 = 20; // pixels per cell
const GRID_WIDTH: u32 = (ARENA_WIDTH as u32) / CELL_SIZE; // 80 cells
const GRID_HEIGHT: u32 = (ARENA_HEIGHT as u32) / CELL_SIZE; // 45 cells

// Level 1 (Open Floor) basket height
const BASKET_HEIGHT: f32 = 600.0;

// Speed range for color mapping
const SPEED_MIN: f32 = 300.0; // Green
const SPEED_MAX: f32 = 1400.0; // Red

// Monte Carlo settings
const MONTE_CARLO_TRIALS: u32 = 100;

// =============================================================================
// SIMULATION TYPE
// =============================================================================

#[derive(Debug, Clone, Copy)]
enum SimType {
    Speed,
    Score,
}

fn parse_args() -> SimType {
    match std::env::args().nth(1).as_deref() {
        Some("score") => SimType::Score,
        _ => SimType::Speed, // default
    }
}

// =============================================================================
// RIM GEOMETRY
// =============================================================================

struct Rect {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

/// Build rim geometry for collision detection
/// The basket opening is BASKET_SIZE_X wide, with rims on sides and bottom
fn build_rim_geometry(basket_x: f32, basket_y: f32) -> Vec<Rect> {
    let half_opening = BASKET_SIZE_X / 2.0;

    vec![
        // Outer rim (wall side) - 50% of basket height
        Rect {
            x: basket_x + half_opening,
            y: basket_y,
            width: RIM_THICKNESS,
            height: BASKET_SIZE_Y * 0.5,
        },
        // Inner rim (center side) - 10% of basket height
        Rect {
            x: basket_x - half_opening - RIM_THICKNESS,
            y: basket_y,
            width: RIM_THICKNESS,
            height: BASKET_SIZE_Y * 0.1,
        },
        // Bottom rim
        Rect {
            x: basket_x - half_opening,
            y: basket_y - BASKET_SIZE_Y / 2.0 - RIM_THICKNESS,
            width: BASKET_SIZE_X,
            height: RIM_THICKNESS,
        },
    ]
}

/// Check collision between circle and rectangle, return normal if colliding
fn check_circle_rect_collision(cx: f32, cy: f32, radius: f32, rect: &Rect) -> Option<(f32, f32)> {
    // Find closest point on rectangle to circle center
    let closest_x = cx.clamp(rect.x, rect.x + rect.width);
    let closest_y = cy.clamp(rect.y - rect.height, rect.y);

    let dx = cx - closest_x;
    let dy = cy - closest_y;
    let dist_sq = dx * dx + dy * dy;

    if dist_sq < radius * radius && dist_sq > 0.0 {
        let dist = dist_sq.sqrt();
        Some((dx / dist, dy / dist)) // Normal pointing away from rect
    } else {
        None
    }
}

// =============================================================================
// BALL FLIGHT SIMULATION
// =============================================================================

/// Simulate ball flight with rim physics, returns true if ball scores
fn simulate_ball_flight(
    start_x: f32,
    start_y: f32,
    angle: f32,
    speed: f32,
    basket_x: f32,
    basket_y: f32,
) -> bool {
    const DT: f32 = 0.001; // 1ms timestep
    const MAX_TIME: f32 = 5.0;

    let mut x = start_x;
    let mut y = start_y;
    let mut vx = angle.cos() * speed;
    let mut vy = angle.sin() * speed;
    let mut t = 0.0;

    let ball_radius = BALL_RADIUS;
    let rims = build_rim_geometry(basket_x, basket_y);

    // Scoring zone (inside basket)
    let score_left = basket_x - BASKET_SIZE_X / 2.0 + ball_radius;
    let score_right = basket_x + BASKET_SIZE_X / 2.0 - ball_radius;
    let score_top = basket_y;
    let score_bottom = basket_y - BASKET_SIZE_Y / 2.0;

    while t < MAX_TIME {
        // Apply gravity
        vy -= BALL_GRAVITY * DT;
        x += vx * DT;
        y += vy * DT;
        t += DT;

        // Check rim collisions
        for rim in &rims {
            if let Some((nx, ny)) = check_circle_rect_collision(x, y, ball_radius, rim) {
                // Reflect velocity
                let dot = vx * nx + vy * ny;
                vx = (vx - 2.0 * dot * nx) * BALL_BOUNCE;
                vy = (vy - 2.0 * dot * ny) * BALL_BOUNCE;
                // Push out of collision
                x += nx * 2.0;
                y += ny * 2.0;
            }
        }

        // Check if scored (ball center in basket bounds)
        if x > score_left && x < score_right && y < score_top && y > score_bottom {
            return true;
        }

        // Ball fell below floor - miss
        if y < ARENA_FLOOR_Y - 50.0 {
            return false;
        }
    }
    false
}

// =============================================================================
// MONTE CARLO SCORING SIMULATION
// =============================================================================

/// Simulate scoring percentage from a position using Monte Carlo
///
/// This simulates "ideal" fully-charged stationary shots to represent the AI
/// decision quality baseline. The heatmap now includes factors that affect
/// actual shots in throw.rs:
/// - Speed randomness (±10%)
/// - Distance multiplier (1.0→1.05 linear)
/// - Angle variance (based on SHOT_MIN_VARIANCE for ideal shots)
fn simulate_scoring(shooter_x: f32, shooter_y: f32, basket_x: f32, basket_y: f32) -> f32 {
    let mut rng = rand::thread_rng();

    let Some(traj) =
        calculate_shot_trajectory(shooter_x, shooter_y, basket_x, basket_y, BALL_GRAVITY)
    else {
        return 0.0;
    };

    let mut makes = 0;

    for _ in 0..MONTE_CARLO_TRIALS {
        // Apply variance: base + distance penalty (matching throw.rs)
        let distance = ((basket_x - shooter_x).powi(2) + (basket_y - shooter_y).powi(2)).sqrt();
        let distance_variance = distance * SHOT_DISTANCE_VARIANCE;
        let total_variance = SHOT_MIN_VARIANCE + distance_variance;

        // Random angle offset within variance range
        let angle_offset = rng.gen_range(-total_variance..total_variance) * 30f32.to_radians();
        let final_angle = traj.angle + angle_offset;

        // Speed randomness (±10%) - matching throw.rs line 173
        let speed_randomness = rng.gen_range(0.9..1.1);

        // Distance-based speed multiplier - matching throw.rs lines 163-167
        // Simple linear: 1.0 at close range (dx=200), 1.05 at far range (dx=800)
        let dx = (basket_x - shooter_x).abs();
        let t = ((dx - 200.0) / 600.0).clamp(0.0, 1.0);
        let distance_multiplier = 1.0 + 0.05 * t;

        // Final speed with both factors applied
        let final_speed = traj.required_speed * distance_multiplier * speed_randomness;

        if simulate_ball_flight(
            shooter_x,
            shooter_y,
            final_angle,
            final_speed,
            basket_x,
            basket_y,
        ) {
            makes += 1;
        }
    }

    makes as f32 / MONTE_CARLO_TRIALS as f32
}

// =============================================================================
// MAIN
// =============================================================================

fn main() {
    let sim_type = parse_args();

    let (type_name, output_path) = match sim_type {
        SimType::Speed => ("speed", "heatmap_speed.png"),
        SimType::Score => ("score", "heatmap_score.png"),
    };

    println!(
        "Generating {} heatmap: {}x{} cells ({} pixels)",
        type_name, GRID_WIDTH, GRID_HEIGHT, CELL_SIZE
    );

    // Create image (multiply by cell size for actual pixels)
    let img_width = GRID_WIDTH * CELL_SIZE;
    let img_height = GRID_HEIGHT * CELL_SIZE;
    let mut img = RgbImage::new(img_width, img_height);

    // Background color (light gray, like the game)
    let bg_color = Rgb([230, 230, 230]);
    for pixel in img.pixels_mut() {
        *pixel = bg_color;
    }

    // Basket position (right basket, level 1)
    let basket_y = ARENA_FLOOR_Y + BASKET_HEIGHT;

    // Track progress for score simulation (slower)
    let total_cells = GRID_WIDTH * GRID_HEIGHT;
    let mut processed = 0;

    // Process each cell
    for cy in 0..GRID_HEIGHT {
        for cx in 0..GRID_WIDTH {
            // Convert cell to world coordinates (center of cell)
            // Image Y is top-down, world Y is bottom-up
            let world_x = (cx as f32 + 0.5) * CELL_SIZE as f32 - ARENA_WIDTH / 2.0;
            let world_y = ARENA_HEIGHT / 2.0 - (cy as f32 + 0.5) * CELL_SIZE as f32;

            match sim_type {
                SimType::Speed => {
                    // Calculate trajectory to right basket
                    if let Some(traj) = calculate_shot_trajectory(
                        world_x,
                        world_y,
                        RIGHT_BASKET_X,
                        basket_y,
                        BALL_GRAVITY,
                    ) {
                        // Clamp speed to range
                        let speed = traj.required_speed.clamp(SPEED_MIN, SPEED_MAX);
                        let t = (speed - SPEED_MIN) / (SPEED_MAX - SPEED_MIN);

                        // Color gradient: green (low) -> yellow -> red (high)
                        let color = speed_to_color(t);

                        // Fill cell with color
                        fill_cell(&mut img, cx, cy, color);

                        // Draw arrow indicating angle
                        draw_arrow(&mut img, cx, cy, traj.angle, color);
                    } else {
                        // Can't reach - fill with dark gray
                        fill_cell(&mut img, cx, cy, Rgb([80, 80, 80]));
                    }
                }
                SimType::Score => {
                    // Monte Carlo scoring simulation
                    let score_pct = simulate_scoring(world_x, world_y, RIGHT_BASKET_X, basket_y);
                    let color = score_to_color(score_pct);
                    fill_cell(&mut img, cx, cy, color);
                }
            }

            // Progress indicator for score mode (it's slow)
            if matches!(sim_type, SimType::Score) {
                processed += 1;
                if processed % 100 == 0 {
                    print!(
                        "\rProgress: {:.1}%",
                        (processed as f32 / total_cells as f32) * 100.0
                    );
                    use std::io::Write;
                    std::io::stdout().flush().ok();
                }
            }
        }
    }

    if matches!(sim_type, SimType::Score) {
        println!(); // Newline after progress
    }

    // Draw basket position marker
    draw_basket_marker(&mut img, basket_y);

    // Draw floor line
    draw_floor_line(&mut img);

    // Save image
    img.save(output_path).expect("Failed to save image");
    println!("Saved to {}", output_path);

    match sim_type {
        SimType::Speed => {
            println!(
                "Speed range: {} (green) to {} (red) pixels/sec",
                SPEED_MIN, SPEED_MAX
            );
        }
        SimType::Score => {
            println!(
                "Score range: 0% (red) to 100% (green), {} trials per cell",
                MONTE_CARLO_TRIALS
            );
        }
    }
}

/// Convert normalized speed (0-1) to RGB color
/// Low speed = green, high speed = red
fn speed_to_color(t: f32) -> Rgb<u8> {
    // Green -> Yellow -> Red gradient
    let r = (t * 2.0).min(1.0);
    let g = ((1.0 - t) * 2.0).min(1.0);
    Rgb([(r * 255.0) as u8, (g * 255.0) as u8, 50])
}

/// Convert score percentage (0-1) to RGB color
/// Low score = red, high score = green
fn score_to_color(pct: f32) -> Rgb<u8> {
    // Red -> Yellow -> Green gradient (opposite of speed)
    let r = ((1.0 - pct) * 2.0).min(1.0);
    let g = (pct * 2.0).min(1.0);
    Rgb([(r * 255.0) as u8, (g * 255.0) as u8, 50])
}

/// Fill a cell with a solid color
fn fill_cell(img: &mut RgbImage, cx: u32, cy: u32, color: Rgb<u8>) {
    let x_start = cx * CELL_SIZE;
    let y_start = cy * CELL_SIZE;

    for dy in 0..CELL_SIZE {
        for dx in 0..CELL_SIZE {
            img.put_pixel(x_start + dx, y_start + dy, color);
        }
    }
}

/// Draw a small arrow in a cell indicating shot angle
fn draw_arrow(img: &mut RgbImage, cx: u32, cy: u32, angle: f32, base_color: Rgb<u8>) {
    let center_x = (cx * CELL_SIZE + CELL_SIZE / 2) as f32;
    let center_y = (cy * CELL_SIZE + CELL_SIZE / 2) as f32;

    // Arrow length (most of half cell size)
    let len = (CELL_SIZE as f32) * 0.7;

    // Note: image Y is inverted from world Y, so negate sin
    let dx = angle.cos() * len;
    let dy = -angle.sin() * len; // Negative because image Y is down

    // Arrow color (darker version of base)
    let arrow_color = Rgb([
        (base_color.0[0] as f32 * 0.5) as u8,
        (base_color.0[1] as f32 * 0.5) as u8,
        (base_color.0[2] as f32 * 0.5) as u8,
    ]);

    // Draw line from center toward direction
    draw_line(
        img,
        center_x as i32,
        center_y as i32,
        (center_x + dx) as i32,
        (center_y + dy) as i32,
        arrow_color,
    );
}

/// Draw a line using Bresenham's algorithm
fn draw_line(img: &mut RgbImage, x0: i32, y0: i32, x1: i32, y1: i32, color: Rgb<u8>) {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut x = x0;
    let mut y = y0;

    loop {
        if x >= 0 && x < img.width() as i32 && y >= 0 && y < img.height() as i32 {
            img.put_pixel(x as u32, y as u32, color);
        }

        if x == x1 && y == y1 {
            break;
        }

        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

/// Draw a marker for the basket position
fn draw_basket_marker(img: &mut RgbImage, basket_y: f32) {
    // Convert world coords to image coords
    let img_x = ((RIGHT_BASKET_X + ARENA_WIDTH / 2.0) / CELL_SIZE as f32 * CELL_SIZE as f32) as u32;
    let img_y = ((ARENA_HEIGHT / 2.0 - basket_y) / CELL_SIZE as f32 * CELL_SIZE as f32) as u32;

    let basket_color = Rgb([200, 50, 50]); // Red

    // Draw a small cross at basket position
    let size = CELL_SIZE as i32;
    for i in -size..=size {
        let px = (img_x as i32 + i) as u32;
        let py = img_y;
        if px < img.width() && py < img.height() {
            img.put_pixel(px, py, basket_color);
        }
        let px = img_x;
        let py = (img_y as i32 + i) as u32;
        if px < img.width() && py < img.height() {
            img.put_pixel(px, py, basket_color);
        }
    }
}

/// Draw the floor line
fn draw_floor_line(img: &mut RgbImage) {
    // Convert ARENA_FLOOR_Y to image Y
    let floor_img_y = ((ARENA_HEIGHT / 2.0 - ARENA_FLOOR_Y) / 1.0) as u32;

    if floor_img_y < img.height() {
        let floor_color = Rgb([50, 50, 50]); // Dark gray
        for x in 0..img.width() {
            img.put_pixel(x, floor_img_y, floor_color);
        }
    }
}
