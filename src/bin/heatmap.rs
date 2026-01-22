//! Heatmap generator for shot trajectories
//!
//! Generates a PNG showing shot angle (arrow direction) and required speed (color)
//! for every position on the court targeting the right basket.
//!
//! Usage: cargo run --bin heatmap && open heatmap.png

use ballgame::{
    calculate_shot_trajectory, ARENA_FLOOR_Y, ARENA_HEIGHT, ARENA_WIDTH, BALL_GRAVITY,
    RIGHT_BASKET_X,
};
use image::{Rgb, RgbImage};

// Grid settings
const CELL_SIZE: u32 = 20; // pixels per cell
const GRID_WIDTH: u32 = (ARENA_WIDTH as u32) / CELL_SIZE; // 80 cells
const GRID_HEIGHT: u32 = (ARENA_HEIGHT as u32) / CELL_SIZE; // 45 cells

// Level 1 (Open Floor) basket height
const BASKET_HEIGHT: f32 = 600.0;

// Speed range for color mapping
const SPEED_MIN: f32 = 300.0; // Green
const SPEED_MAX: f32 = 1400.0; // Red

fn main() {
    println!(
        "Generating heatmap: {}x{} cells ({} pixels)",
        GRID_WIDTH,
        GRID_HEIGHT,
        CELL_SIZE
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

    // Process each cell
    for cy in 0..GRID_HEIGHT {
        for cx in 0..GRID_WIDTH {
            // Convert cell to world coordinates (center of cell)
            // Image Y is top-down, world Y is bottom-up
            let world_x = (cx as f32 + 0.5) * CELL_SIZE as f32 - ARENA_WIDTH / 2.0;
            let world_y = ARENA_HEIGHT / 2.0 - (cy as f32 + 0.5) * CELL_SIZE as f32;

            // Calculate trajectory to right basket
            if let Some(traj) =
                calculate_shot_trajectory(world_x, world_y, RIGHT_BASKET_X, basket_y, BALL_GRAVITY)
            {
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
    }

    // Draw basket position marker
    draw_basket_marker(&mut img, basket_y);

    // Draw floor line
    draw_floor_line(&mut img);

    // Save image
    let output_path = "heatmap.png";
    img.save(output_path).expect("Failed to save image");
    println!("Saved to {}", output_path);
    println!(
        "Speed range: {} (green) to {} (red) pixels/sec",
        SPEED_MIN, SPEED_MAX
    );
}

/// Convert normalized speed (0-1) to RGB color
fn speed_to_color(t: f32) -> Rgb<u8> {
    // Green -> Yellow -> Red gradient
    let r = (t * 2.0).min(1.0);
    let g = ((1.0 - t) * 2.0).min(1.0);
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
