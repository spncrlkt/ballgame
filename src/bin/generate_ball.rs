//! Ball texture generator
//!
//! Generates ball textures for all styles × all color palettes.
//! Styles and palettes are read from config files (single source of truth).
//!
//! Run with: `cargo run --bin generate_ball`

use image::{Rgba, RgbaImage};
use std::collections::HashMap;
use std::f32::consts::PI;
use std::fs;

// Colors
const WHITE: [u8; 4] = [245, 245, 240, 255]; // Off-white (cream)
const BLACK: [u8; 4] = [20, 20, 20, 255]; // Outline

const PALETTES_FILE: &str = "assets/palettes.txt";
const OPTIONS_FILE: &str = "assets/ball_options.txt";

/// Color palette with left and right team colors (RGB 0-255)
#[derive(Clone)]
struct Palette {
    name: String,
    left: [u8; 4],
    right: [u8; 4],
}

/// Load palettes from assets/palettes.txt
fn load_palettes() -> Vec<Palette> {
    let content = fs::read_to_string(PALETTES_FILE).unwrap_or_else(|e| {
        panic!("\n\nERROR: Could not read palettes file '{}': {}\n", PALETTES_FILE, e)
    });

    let mut palettes = Vec::new();
    let mut current_name: Option<String> = None;
    let mut current_left: Option<[u8; 4]> = None;
    let mut current_right: Option<[u8; 4]> = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(name) = line.strip_prefix("palette:") {
            if let (Some(name), Some(left), Some(right)) =
                (current_name.take(), current_left.take(), current_right.take())
            {
                palettes.push(Palette { name, left, right });
            }
            current_name = Some(name.trim().to_string());
            current_left = None;
            current_right = None;
        } else if let Some(rgb) = line.strip_prefix("left:") {
            current_left = parse_rgb_to_u8(rgb);
        } else if let Some(rgb) = line.strip_prefix("right:") {
            current_right = parse_rgb_to_u8(rgb);
        }
    }

    if let (Some(name), Some(left), Some(right)) = (current_name, current_left, current_right) {
        palettes.push(Palette { name, left, right });
    }

    println!("Loaded {} palettes from {}", palettes.len(), PALETTES_FILE);
    palettes
}

fn parse_rgb_to_u8(s: &str) -> Option<[u8; 4]> {
    let parts: Vec<&str> = s.trim().split_whitespace().collect();
    if parts.len() >= 3 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            parts[0].parse::<f32>(),
            parts[1].parse::<f32>(),
            parts[2].parse::<f32>(),
        ) {
            return Some([
                (r * 255.0).round() as u8,
                (g * 255.0).round() as u8,
                (b * 255.0).round() as u8,
                255,
            ]);
        }
    }
    None
}

/// Style configuration
#[derive(Clone)]
struct StyleConfig {
    name: String,
    pattern: String,
    params: HashMap<String, f32>,
}

/// Global configuration
struct BallConfig {
    size: u32,
    border: f32,
    styles: Vec<StyleConfig>,
    palettes: Vec<Palette>,
}

fn main() {
    let palettes = load_palettes();
    let config = load_config(palettes);

    println!("\nGenerating ball textures...");
    println!("  Size: {}px, Border: {}px", config.size, config.border);
    println!("  Styles: {}", config.styles.len());
    println!("  Palettes: {}", config.palettes.len());

    for style in &config.styles {
        for (palette_idx, palette) in config.palettes.iter().enumerate() {
            let filename = format!("assets/ball_{}_{}.png", style.name, palette_idx);
            generate_texture(&filename, &config, style, palette);
            println!("  Created: {} ({})", filename, palette.name);
        }
    }

    println!(
        "\nGenerated {} ball textures.",
        config.styles.len() * config.palettes.len()
    );
}

fn load_config(palettes: Vec<Palette>) -> BallConfig {
    let content = fs::read_to_string(OPTIONS_FILE).unwrap_or_else(|e| {
        panic!("\n\nERROR: Could not read '{}': {}\n", OPTIONS_FILE, e)
    });
    parse_config(&content, palettes)
}

fn parse_config(content: &str, palettes: Vec<Palette>) -> BallConfig {
    let mut size: Option<u32> = None;
    let mut border: Option<f32> = None;
    let mut styles: Vec<StyleConfig> = Vec::new();
    let mut current_style: Option<StyleConfig> = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(val) = line.strip_prefix("size:") {
            size = Some(val.trim().parse().expect("Invalid size"));
        } else if let Some(val) = line.strip_prefix("border:") {
            border = Some(val.trim().parse().expect("Invalid border"));
        } else if let Some(name) = line.strip_prefix("style:") {
            if let Some(style) = current_style.take() {
                styles.push(style);
            }
            current_style = Some(StyleConfig {
                name: name.trim().to_string(),
                pattern: String::new(),
                params: HashMap::new(),
            });
        } else if let Some(pattern) = line.strip_prefix("pattern:") {
            if let Some(style) = &mut current_style {
                style.pattern = pattern.trim().to_string();
            }
        } else if let Some(param) = line.strip_prefix("param:") {
            if let Some(style) = &mut current_style {
                let parts: Vec<&str> = param.split_whitespace().collect();
                if parts.len() == 2 {
                    if let Ok(value) = parts[1].parse::<f32>() {
                        style.params.insert(parts[0].to_string(), value);
                    }
                }
            }
        }
    }

    if let Some(style) = current_style {
        styles.push(style);
    }

    BallConfig {
        size: size.expect("Missing size"),
        border: border.expect("Missing border"),
        styles,
        palettes,
    }
}

fn generate_texture(path: &str, config: &BallConfig, style: &StyleConfig, palette: &Palette) {
    let size = config.size;
    let center = size as f32 / 2.0;
    let radius = center - config.border;

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
                let color = get_pixel_color(fx, fy, dist, radius, style, palette);

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

    // Draw border ring
    let outer_radius = center - 1.0;
    for y in 0..size {
        for x in 0..size {
            let fx = x as f32 - center;
            let fy = y as f32 - center;
            let dist = (fx * fx + fy * fy).sqrt();

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
    palette: &Palette,
) -> [u8; 4] {
    let normalized_dist = dist / radius;
    let angle = fy.atan2(fx);

    match style.pattern.as_str() {
        "wedges" => draw_wedges(angle, style, palette),
        "half" => draw_half(fx, palette),
        "spiral" => draw_spiral(fx, fy, normalized_dist, angle, style, palette),
        "checker" => draw_checker(fx, fy, radius, style, palette),
        "star" => draw_star(angle, normalized_dist, style, palette),
        "swirl" => draw_swirl(angle, normalized_dist, style, palette),
        "plasma" => draw_plasma(fx, fy, radius, style, palette),
        "shatter" => draw_shatter(fx, fy, radius, style, palette),
        "wave" => draw_wave(fx, fy, radius, style, palette),
        "atoms" => draw_atoms(fx, fy, normalized_dist, angle, style, palette),
        _ => WHITE,
    }
}

/// Wedges: N equal sections (beach ball style)
fn draw_wedges(angle: f32, style: &StyleConfig, palette: &Palette) -> [u8; 4] {
    let sections = style.params.get("sections").copied().unwrap_or(4.0) as i32;
    let angle = if angle < 0.0 { angle + 2.0 * PI } else { angle };
    let sector = ((angle / (2.0 * PI / sections as f32)) as i32) % sections;

    // Alternating colors
    if sector % 2 == 0 {
        palette.left
    } else {
        palette.right
    }
}

/// Half: Split vertically down the middle
fn draw_half(fx: f32, palette: &Palette) -> [u8; 4] {
    if fx < 0.0 { palette.left } else { palette.right }
}

/// Spiral: Spiral arms from center
fn draw_spiral(_fx: f32, _fy: f32, norm_dist: f32, angle: f32, style: &StyleConfig, palette: &Palette) -> [u8; 4] {
    let arms = style.params.get("arms").copied().unwrap_or(3.0);
    let tightness = style.params.get("tightness").copied().unwrap_or(2.5);

    // Spiral equation: angle offset based on distance
    let spiral_angle = angle + norm_dist * tightness * PI;
    let spiral_sector = (spiral_angle * arms / (2.0 * PI)).floor() as i32;

    if spiral_sector % 2 == 0 {
        palette.left
    } else {
        palette.right
    }
}

/// Checker: Checkerboard pattern
fn draw_checker(fx: f32, fy: f32, radius: f32, style: &StyleConfig, palette: &Palette) -> [u8; 4] {
    let squares = style.params.get("squares").copied().unwrap_or(6.0);
    let cell_size = (radius * 2.0) / squares;

    let cx = ((fx + radius) / cell_size).floor() as i32;
    let cy = ((fy + radius) / cell_size).floor() as i32;

    if (cx + cy) % 2 == 0 {
        palette.left
    } else {
        palette.right
    }
}

/// Star: N-pointed star shape
fn draw_star(angle: f32, norm_dist: f32, style: &StyleConfig, palette: &Palette) -> [u8; 4] {
    let points = style.params.get("points").copied().unwrap_or(5.0);
    let inner_radius = style.params.get("inner_radius").copied().unwrap_or(0.35);

    // Calculate star edge at this angle
    let angle = if angle < 0.0 { angle + 2.0 * PI } else { angle };
    let sector_angle = 2.0 * PI / points;
    let angle_in_sector = angle % sector_angle;

    // Triangle wave for star points
    let half_sector = sector_angle / 2.0;
    let star_dist = if angle_in_sector < half_sector {
        inner_radius + (1.0 - inner_radius) * (1.0 - angle_in_sector / half_sector)
    } else {
        inner_radius + (1.0 - inner_radius) * ((angle_in_sector - half_sector) / half_sector)
    };

    if norm_dist < star_dist {
        // Inside star - split by angle
        let point_index = (angle / sector_angle).floor() as i32;
        if point_index % 2 == 0 { palette.left } else { palette.right }
    } else {
        WHITE
    }
}

/// Swirl: Pinwheel pattern
fn draw_swirl(angle: f32, norm_dist: f32, style: &StyleConfig, palette: &Palette) -> [u8; 4] {
    let blades = style.params.get("blades").copied().unwrap_or(6.0);
    let twist = style.params.get("twist").copied().unwrap_or(1.5);

    // Twist increases with distance
    let twisted_angle = angle + norm_dist * twist * PI;
    let blade_angle = 2.0 * PI / blades;
    let sector = (twisted_angle / blade_angle).floor() as i32;

    if sector % 2 == 0 {
        palette.left
    } else {
        palette.right
    }
}

/// Plasma: Organic plasma blobs using noise-like function
fn draw_plasma(fx: f32, fy: f32, radius: f32, style: &StyleConfig, palette: &Palette) -> [u8; 4] {
    let scale = style.params.get("scale").copied().unwrap_or(3.0);
    let threshold = style.params.get("threshold").copied().unwrap_or(0.5);

    // Normalize coordinates
    let nx = fx / radius;
    let ny = fy / radius;

    // Simple plasma function using sin waves
    let v1 = (nx * scale * PI).sin();
    let v2 = (ny * scale * PI).sin();
    let v3 = ((nx + ny) * scale * 0.7 * PI).sin();
    let v4 = ((nx * nx + ny * ny).sqrt() * scale * PI).sin();

    let value = (v1 + v2 + v3 + v4) / 4.0;

    if value > threshold - 0.5 {
        palette.left
    } else {
        palette.right
    }
}

/// Shatter: Broken glass fragments using voronoi-like pattern
fn draw_shatter(fx: f32, fy: f32, radius: f32, style: &StyleConfig, palette: &Palette) -> [u8; 4] {
    let pieces = style.params.get("pieces").copied().unwrap_or(8.0) as usize;
    let chaos = style.params.get("chaos").copied().unwrap_or(0.6);

    // Generate deterministic "random" points based on piece count
    let mut min_dist = f32::MAX;
    let mut closest_idx = 0;

    for i in 0..pieces {
        // Pseudo-random point positions using golden ratio
        let golden = 1.618033988749;
        let angle = i as f32 * golden * 2.0 * PI;
        let r = (i as f32 / pieces as f32).sqrt() * radius * chaos + radius * (1.0 - chaos) * 0.3;
        let px = angle.cos() * r;
        let py = angle.sin() * r;

        let dist = ((fx - px).powi(2) + (fy - py).powi(2)).sqrt();
        if dist < min_dist {
            min_dist = dist;
            closest_idx = i;
        }
    }

    // Add center point
    let center_dist = (fx * fx + fy * fy).sqrt();
    if center_dist < min_dist {
        closest_idx = pieces; // Center gets its own index
    }

    if closest_idx % 2 == 0 {
        palette.left
    } else {
        palette.right
    }
}

/// Wave: Wavy horizontal bands
fn draw_wave(fx: f32, fy: f32, radius: f32, style: &StyleConfig, palette: &Palette) -> [u8; 4] {
    let frequency = style.params.get("frequency").copied().unwrap_or(3.0);
    let amplitude = style.params.get("amplitude").copied().unwrap_or(0.2);

    // Normalize
    let nx = fx / radius;
    let ny = fy / radius;

    // Wave offset based on x position
    let wave_offset = (nx * frequency * PI).sin() * amplitude;
    let adjusted_y = ny - wave_offset;

    // Create bands
    let band = (adjusted_y * frequency).floor() as i32;

    if band % 2 == 0 {
        palette.left
    } else {
        palette.right
    }
}

/// Atoms: Orbital rings pattern
fn draw_atoms(fx: f32, fy: f32, norm_dist: f32, _angle: f32, style: &StyleConfig, palette: &Palette) -> [u8; 4] {
    let rings = style.params.get("rings").copied().unwrap_or(3.0) as i32;
    let thickness = style.params.get("thickness").copied().unwrap_or(0.12);

    // Check each orbital ring at different angles
    for i in 0..rings {
        let ring_angle = PI * i as f32 / rings as f32;

        // Rotate point to check against horizontal ellipse
        let cos_a = ring_angle.cos();
        let sin_a = ring_angle.sin();
        let rx = fx * cos_a + fy * sin_a;
        let ry = -fx * sin_a + fy * cos_a;

        // Ellipse with high eccentricity
        let ellipse_dist = (rx * rx + (ry * 3.0).powi(2)).sqrt();
        let target_dist = norm_dist * ((fx * fx + fy * fy).sqrt() / ellipse_dist.max(0.001));

        // Check if on ring
        let ring_center = 0.7; // Rings at 70% radius
        if (target_dist - ring_center).abs() < thickness {
            return if i % 2 == 0 { palette.left } else { palette.right };
        }
    }

    // Nucleus in center
    if norm_dist < 0.2 {
        // Split nucleus
        if fx < 0.0 { palette.left } else { palette.right }
    } else {
        WHITE
    }
}
