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
//! Configuration is read from assets/ball_options.txt.
//! Run with: `cargo run --bin generate_ball`

use image::{Rgba, RgbaImage};
use std::collections::HashMap;
use std::f32::consts::PI;
use std::fs;

// Colors (matching constants.rs)
const WHITE: [u8; 4] = [245, 245, 240, 255]; // Off-white (cream)
const TURQUOISE: [u8; 4] = [64, 191, 204, 255]; // Team left (0.25, 0.75, 0.8)
const TERRACOTTA: [u8; 4] = [204, 115, 77, 255]; // Team right (0.8, 0.45, 0.3)
const BLACK: [u8; 4] = [20, 20, 20, 255]; // Outline

const OPTIONS_FILE: &str = "assets/ball_options.txt";

/// Global configuration from ball_options.txt
struct BallConfig {
    size: u32,
    border: f32,
    styles: Vec<StyleConfig>,
}

/// Configuration for a single style
#[derive(Clone)]
struct StyleConfig {
    name: String,
    pattern: String,
    params: HashMap<String, f32>,
}

#[derive(Clone, Copy)]
enum PossessionState {
    Neutral,
    Left,
    Right,
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
    let config = load_config();

    let states = [
        PossessionState::Neutral,
        PossessionState::Left,
        PossessionState::Right,
    ];

    println!("Generating ball textures from {}...", OPTIONS_FILE);
    println!("  Size: {}px, Border: {}px", config.size, config.border);
    println!("  Styles: {}", config.styles.len());

    for style in &config.styles {
        for state in &states {
            let filename = format!("assets/ball_{}_{}.png", style.name, state.name());
            generate_texture(&filename, &config, style, *state);
            println!("  Created: {}", filename);
        }
    }

    println!(
        "\nGenerated {} ball textures.",
        config.styles.len() * states.len()
    );
}

/// Load and parse ball_options.txt, panicking with helpful message on error
fn load_config() -> BallConfig {
    let content = fs::read_to_string(OPTIONS_FILE).unwrap_or_else(|e| {
        panic!(
            "\n\nERROR: Could not read ball options file '{}': {}\n\n\
             The ball generator requires this configuration file.\n\
             Please ensure assets/ball_options.txt exists with proper format:\n\n\
             size: 128\n\
             border: 4\n\n\
             style: stripe\n\
             pattern: stripe\n\
             param: height 0.20\n\n\
             ... (see CLAUDE.md for full example)\n",
            OPTIONS_FILE, e
        )
    });

    parse_config(&content)
}

/// Parse the configuration file content
fn parse_config(content: &str) -> BallConfig {
    let mut size: Option<u32> = None;
    let mut border: Option<f32> = None;
    let mut styles: Vec<StyleConfig> = Vec::new();
    let mut current_style: Option<StyleConfig> = None;

    for (line_num, line) in content.lines().enumerate() {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(val) = line.strip_prefix("size:") {
            size = Some(val.trim().parse().unwrap_or_else(|_| {
                panic!(
                    "\n\nERROR: Invalid 'size' value on line {}: '{}'\n\
                     Expected integer (e.g., 'size: 128')\n",
                    line_num + 1,
                    val.trim()
                )
            }));
        } else if let Some(val) = line.strip_prefix("border:") {
            border = Some(val.trim().parse().unwrap_or_else(|_| {
                panic!(
                    "\n\nERROR: Invalid 'border' value on line {}: '{}'\n\
                     Expected number (e.g., 'border: 4')\n",
                    line_num + 1,
                    val.trim()
                )
            }));
        } else if let Some(name) = line.strip_prefix("style:") {
            // Save previous style if exists
            if let Some(style) = current_style.take() {
                if style.pattern.is_empty() {
                    panic!(
                        "\n\nERROR: Style '{}' has no pattern defined.\n\
                         Each style must have 'pattern: <type>' after 'style: <name>'\n",
                        style.name
                    );
                }
                styles.push(style);
            }
            // Start new style
            current_style = Some(StyleConfig {
                name: name.trim().to_string(),
                pattern: String::new(),
                params: HashMap::new(),
            });
        } else if let Some(pattern) = line.strip_prefix("pattern:") {
            if let Some(style) = &mut current_style {
                let pattern = pattern.trim().to_string();
                // Validate pattern type
                if !["stripe", "wedges", "dot", "half", "ring", "solid"].contains(&pattern.as_str())
                {
                    panic!(
                        "\n\nERROR: Unknown pattern '{}' on line {}.\n\
                         Valid patterns: stripe, wedges, dot, half, ring, solid\n",
                        pattern,
                        line_num + 1
                    );
                }
                style.pattern = pattern;
            } else {
                panic!(
                    "\n\nERROR: 'pattern:' on line {} outside of style definition.\n\
                     First define a style with 'style: <name>'\n",
                    line_num + 1
                );
            }
        } else if let Some(param) = line.strip_prefix("param:") {
            if let Some(style) = &mut current_style {
                let parts: Vec<&str> = param.split_whitespace().collect();
                if parts.len() != 2 {
                    panic!(
                        "\n\nERROR: Invalid param on line {}: '{}'\n\
                         Expected format: 'param: <name> <value>' (e.g., 'param: height 0.20')\n",
                        line_num + 1,
                        param.trim()
                    );
                }
                let value: f32 = parts[1].parse().unwrap_or_else(|_| {
                    panic!(
                        "\n\nERROR: Invalid param value on line {}: '{}'\n\
                         Expected number (e.g., 'param: height 0.20')\n",
                        line_num + 1,
                        parts[1]
                    )
                });
                style.params.insert(parts[0].to_string(), value);
            } else {
                panic!(
                    "\n\nERROR: 'param:' on line {} outside of style definition.\n\
                     First define a style with 'style: <name>'\n",
                    line_num + 1
                );
            }
        } else {
            panic!(
                "\n\nERROR: Unknown directive on line {}: '{}'\n\
                 Valid directives: size, border, style, pattern, param\n",
                line_num + 1,
                line
            );
        }
    }

    // Don't forget the last style
    if let Some(style) = current_style {
        if style.pattern.is_empty() {
            panic!(
                "\n\nERROR: Style '{}' has no pattern defined.\n\
                 Each style must have 'pattern: <type>' after 'style: <name>'\n",
                style.name
            );
        }
        styles.push(style);
    }

    // Validate required fields
    let size = size.unwrap_or_else(|| {
        panic!(
            "\n\nERROR: Missing required 'size:' in ball_options.txt\n\
             Add 'size: 128' (or desired pixel size) near the top of the file.\n"
        )
    });

    let border = border.unwrap_or_else(|| {
        panic!(
            "\n\nERROR: Missing required 'border:' in ball_options.txt\n\
             Add 'border: 4' (or desired border thickness) near the top of the file.\n"
        )
    });

    if styles.is_empty() {
        panic!(
            "\n\nERROR: No styles defined in ball_options.txt\n\
             Add at least one style definition:\n\n\
             style: stripe\n\
             pattern: stripe\n\
             param: height 0.20\n"
        );
    }

    BallConfig {
        size,
        border,
        styles,
    }
}

fn generate_texture(path: &str, config: &BallConfig, style: &StyleConfig, state: PossessionState) {
    let size = config.size;
    let center = size as f32 / 2.0;
    let radius = center - config.border; // Leave room for border

    let mut img = RgbaImage::new(size, size);

    // Fill with transparent
    for pixel in img.pixels_mut() {
        *pixel = Rgba([0, 0, 0, 0]);
    }

    // Draw the ball interior
    for y in 0..size {
        for x in 0..size {
            let fx = x as f32 - center;
            let fy = y as f32 - center;
            let dist = (fx * fx + fy * fy).sqrt();

            if dist <= radius {
                let color = get_pixel_color(fx, fy, dist, radius, style, state);

                // Edge anti-aliasing
                let edge_dist = radius - dist;
                let alpha = if edge_dist < 2.0 {
                    ((edge_dist / 2.0) * 255.0) as u8
                } else {
                    255
                };

                img.put_pixel(x, y, Rgba([color[0], color[1], color[2], alpha]));
            }
        }
    }

    // Draw solid black border ring (from interior edge to ball edge)
    let outer_radius = center - 1.0; // Leave 1px for anti-aliasing
    for y in 0..size {
        for x in 0..size {
            let fx = x as f32 - center;
            let fy = y as f32 - center;
            let dist = (fx * fx + fy * fy).sqrt();

            // Draw in the border ring: between interior radius and outer edge
            if dist > radius && dist <= outer_radius {
                let edge_dist = outer_radius - dist;
                let alpha = if edge_dist < 1.5 {
                    ((edge_dist / 1.5) * 255.0) as u8
                } else {
                    255
                };
                img.put_pixel(x, y, Rgba([BLACK[0], BLACK[1], BLACK[2], alpha]));
            }
        }
    }

    img.save(path).expect("Failed to save ball texture");
}

fn get_pixel_color(
    fx: f32,
    fy: f32,
    dist: f32,
    radius: f32,
    style: &StyleConfig,
    state: PossessionState,
) -> [u8; 4] {
    let normalized_dist = dist / radius;
    let angle = fy.atan2(fx);

    match style.pattern.as_str() {
        "stripe" => draw_stripe(fx, fy, radius, style, state),
        "wedges" => draw_wedges(angle, state),
        "dot" => draw_dot(normalized_dist, fx, style, state),
        "half" => draw_half(fx, state),
        "ring" => draw_ring(normalized_dist, fx, style, state),
        "solid" => draw_solid(fx, state),
        _ => WHITE, // Fallback (shouldn't happen due to validation)
    }
}

/// Stripe: White ball with horizontal stripe through middle
fn draw_stripe(
    fx: f32,
    fy: f32,
    radius: f32,
    style: &StyleConfig,
    state: PossessionState,
) -> [u8; 4] {
    let height = style.params.get("height").copied().unwrap_or(0.20);
    let stripe_half_height = radius * height;

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
            if sector % 2 == 0 {
                TURQUOISE
            } else {
                WHITE
            }
        }
        PossessionState::Right => {
            // Alternating: terracotta, white
            if sector % 2 == 0 {
                TERRACOTTA
            } else {
                WHITE
            }
        }
    }
}

/// Dot: White ball with large center circle
fn draw_dot(
    normalized_dist: f32,
    fx: f32,
    style: &StyleConfig,
    state: PossessionState,
) -> [u8; 4] {
    let dot_radius = style.params.get("radius").copied().unwrap_or(0.45);

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
fn draw_ring(
    normalized_dist: f32,
    fx: f32,
    style: &StyleConfig,
    state: PossessionState,
) -> [u8; 4] {
    let ring_inner = style.params.get("inner").copied().unwrap_or(0.45);
    let ring_outer = style.params.get("outer").copied().unwrap_or(0.70);

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
