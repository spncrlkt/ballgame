//! Ball texture generator
//!
//! Generates 6 ball styles × 3 possession states = 18 PNG textures.
//!
//! Styles:
//! - stripe: White ball with horizontal stripe
//! - wedges: 4 large wedges (beach ball style)
//! - dot: White ball with large center circle
//! - half: Ball split vertically down middle
//! - ring: White ball with colored ring
//! - solid: Solid team color with black outline
//!
//! States:
//! - neutral: Both team colors (Free ball)
//! - left: Turquoise (held by left team)
//! - right: Terracotta (held by right team)
//!
//! Run with: `cargo run --bin generate_ball`

use image::{Rgba, RgbaImage};
use std::f32::consts::PI;

// Colors
const WHITE: [u8; 4] = [245, 245, 240, 255]; // Off-white (cream)
const TURQUOISE: [u8; 4] = [64, 191, 204, 255]; // Team left (0.25, 0.75, 0.8)
const TERRACOTTA: [u8; 4] = [204, 115, 77, 255]; // Team right (0.8, 0.45, 0.3)
const BLACK: [u8; 4] = [20, 20, 20, 255]; // Outline

const SIZE: u32 = 128;
const CENTER: f32 = SIZE as f32 / 2.0;
const RADIUS: f32 = CENTER - 4.0; // Leave room for border
const BORDER_THICKNESS: f32 = 4.0;

#[derive(Clone, Copy)]
enum BallStyle {
    Stripe,
    Wedges,
    Dot,
    Half,
    Ring,
    Solid,
}

#[derive(Clone, Copy)]
enum PossessionState {
    Neutral,
    Left,
    Right,
}

impl BallStyle {
    fn name(&self) -> &'static str {
        match self {
            BallStyle::Stripe => "stripe",
            BallStyle::Wedges => "wedges",
            BallStyle::Dot => "dot",
            BallStyle::Half => "half",
            BallStyle::Ring => "ring",
            BallStyle::Solid => "solid",
        }
    }
}

impl PossessionState {
    fn name(&self) -> &'static str {
        match self {
            PossessionState::Neutral => "neutral",
            PossessionState::Left => "left",
            PossessionState::Right => "right",
        }
    }
}

fn main() {
    let styles = [
        BallStyle::Stripe,
        BallStyle::Wedges,
        BallStyle::Dot,
        BallStyle::Half,
        BallStyle::Ring,
        BallStyle::Solid,
    ];
    let states = [
        PossessionState::Neutral,
        PossessionState::Left,
        PossessionState::Right,
    ];

    println!("Generating ball textures...");

    for style in &styles {
        for state in &states {
            let filename = format!("assets/ball_{}_{}.png", style.name(), state.name());
            generate_texture(&filename, *style, *state);
            println!("  Created: {}", filename);
        }
    }

    println!("\nGenerated {} ball textures.", styles.len() * states.len());
}

fn generate_texture(path: &str, style: BallStyle, state: PossessionState) {
    let mut img = RgbaImage::new(SIZE, SIZE);

    // Fill with transparent
    for pixel in img.pixels_mut() {
        *pixel = Rgba([0, 0, 0, 0]);
    }

    // Draw the ball interior
    for y in 0..SIZE {
        for x in 0..SIZE {
            let fx = x as f32 - CENTER;
            let fy = y as f32 - CENTER;
            let dist = (fx * fx + fy * fy).sqrt();

            if dist <= RADIUS {
                let color = get_pixel_color(fx, fy, dist, style, state);

                // Edge anti-aliasing
                let edge_dist = RADIUS - dist;
                let alpha = if edge_dist < 2.0 {
                    ((edge_dist / 2.0) * 255.0) as u8
                } else {
                    255
                };

                img.put_pixel(x, y, Rgba([color[0], color[1], color[2], alpha]));
            }
        }
    }

    // Add thick black border
    for y in 0..SIZE {
        for x in 0..SIZE {
            let fx = x as f32 - CENTER;
            let fy = y as f32 - CENTER;
            let dist = (fx * fx + fy * fy).sqrt();

            if dist > RADIUS - BORDER_THICKNESS && dist <= RADIUS {
                let current = img.get_pixel(x, y);
                if current[3] > 0 {
                    let border_strength =
                        ((dist - (RADIUS - BORDER_THICKNESS)) / BORDER_THICKNESS).min(1.0);
                    let blend = border_strength * 0.9;
                    let r = lerp_u8(current[0], BLACK[0], blend);
                    let g = lerp_u8(current[1], BLACK[1], blend);
                    let b = lerp_u8(current[2], BLACK[2], blend);
                    img.put_pixel(x, y, Rgba([r, g, b, current[3]]));
                }
            }
        }
    }

    img.save(path).expect("Failed to save ball texture");
}

fn get_pixel_color(
    fx: f32,
    fy: f32,
    dist: f32,
    style: BallStyle,
    state: PossessionState,
) -> [u8; 4] {
    let normalized_dist = dist / RADIUS;
    let angle = fy.atan2(fx);

    match style {
        BallStyle::Stripe => draw_stripe(fx, fy, state),
        BallStyle::Wedges => draw_wedges(angle, state),
        BallStyle::Dot => draw_dot(normalized_dist, fx, state),
        BallStyle::Half => draw_half(fx, state),
        BallStyle::Ring => draw_ring(normalized_dist, fx, state),
        BallStyle::Solid => draw_solid(fx, state),
    }
}

/// Stripe: White ball with horizontal stripe through middle
fn draw_stripe(fx: f32, fy: f32, state: PossessionState) -> [u8; 4] {
    let stripe_half_height = RADIUS * 0.2; // 20% of radius on each side of center

    if fy.abs() < stripe_half_height {
        // Inside stripe
        match state {
            PossessionState::Neutral => {
                if fx < 0.0 {
                    TURQUOISE
                } else {
                    TERRACOTTA
                }
            }
            PossessionState::Left => TURQUOISE,
            PossessionState::Right => TERRACOTTA,
        }
    } else {
        WHITE
    }
}

/// Wedges: 4 equal 90-degree sections (beach ball style)
fn draw_wedges(angle: f32, state: PossessionState) -> [u8; 4] {
    // Normalize angle to 0..2PI
    let angle = if angle < 0.0 { angle + 2.0 * PI } else { angle };
    let sector = ((angle / (PI / 2.0)) as i32) % 4;

    match state {
        PossessionState::Neutral => {
            // Alternating: turquoise, white, terracotta, white
            match sector {
                0 => TURQUOISE,
                1 => WHITE,
                2 => TERRACOTTA,
                3 => WHITE,
                _ => WHITE,
            }
        }
        PossessionState::Left => {
            // Alternating: turquoise, white
            if sector % 2 == 0 { TURQUOISE } else { WHITE }
        }
        PossessionState::Right => {
            // Alternating: terracotta, white
            if sector % 2 == 0 { TERRACOTTA } else { WHITE }
        }
    }
}

/// Dot: White ball with large center circle
fn draw_dot(normalized_dist: f32, fx: f32, state: PossessionState) -> [u8; 4] {
    let dot_radius = 0.45; // 45% of ball radius

    if normalized_dist < dot_radius {
        match state {
            PossessionState::Neutral => {
                if fx < 0.0 {
                    TURQUOISE
                } else {
                    TERRACOTTA
                }
            }
            PossessionState::Left => TURQUOISE,
            PossessionState::Right => TERRACOTTA,
        }
    } else {
        WHITE
    }
}

/// Half: Ball split vertically down the middle
fn draw_half(fx: f32, state: PossessionState) -> [u8; 4] {
    match state {
        PossessionState::Neutral => {
            if fx < 0.0 {
                TURQUOISE
            } else {
                TERRACOTTA
            }
        }
        PossessionState::Left => TURQUOISE,
        PossessionState::Right => TERRACOTTA,
    }
}

/// Ring: White ball with colored ring around middle
fn draw_ring(normalized_dist: f32, fx: f32, state: PossessionState) -> [u8; 4] {
    let ring_inner = 0.45;
    let ring_outer = 0.70;

    if normalized_dist > ring_inner && normalized_dist < ring_outer {
        match state {
            PossessionState::Neutral => {
                if fx < 0.0 {
                    TURQUOISE
                } else {
                    TERRACOTTA
                }
            }
            PossessionState::Left => TURQUOISE,
            PossessionState::Right => TERRACOTTA,
        }
    } else {
        WHITE
    }
}

/// Solid: Entire ball is team color
fn draw_solid(fx: f32, state: PossessionState) -> [u8; 4] {
    match state {
        PossessionState::Neutral => {
            if fx < 0.0 {
                TURQUOISE
            } else {
                TERRACOTTA
            }
        }
        PossessionState::Left => TURQUOISE,
        PossessionState::Right => TERRACOTTA,
    }
}

/// Linear interpolation for u8
fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    ((a as f32) * (1.0 - t) + (b as f32) * t) as u8
}
