//! Ball Style Showcase Generator
//!
//! Generates a PNG grid showing all ball styles across the first 5 palettes.
//!
//! Run with: `cargo run --bin generate_showcase`

use ab_glyph::{FontRef, PxScale};
use image::{Rgba, RgbaImage};
use imageproc::drawing::draw_text_mut;
use std::fs;

const BALL_SIZE: u32 = 128;
const PADDING: u32 = 16;
const COVERAGE_COL_WIDTH: u32 = 70; // Width for coverage percentage column

// Representative palette indices for showcase variety:
// Aurora (0), Ocean Fire (2), Synthwave (8), Monochrome (9), Blood Moon (16)
const SHOWCASE_PALETTES: [usize; 5] = [0, 2, 8, 9, 16];

// Embed a simple font (using system font path for macOS)
const FONT_DATA: &[u8] = include_bytes!("/System/Library/Fonts/Helvetica.ttc");

/// Calculate color coverage percentage for a ball texture.
/// Returns (left_color_percent, right_color_percent) based on dominant colors.
fn calculate_coverage(ball: &RgbaImage) -> (f32, f32) {
    let mut color_counts: std::collections::HashMap<(u8, u8, u8), u32> =
        std::collections::HashMap::new();
    let mut total_opaque = 0u32;

    // Count pixels by color (ignoring transparency and black border)
    for pixel in ball.pixels() {
        if pixel[3] > 128 {
            // Skip very dark pixels (likely border)
            let brightness = (pixel[0] as u32 + pixel[1] as u32 + pixel[2] as u32) / 3;
            if brightness > 30 {
                let key = (pixel[0], pixel[1], pixel[2]);
                *color_counts.entry(key).or_insert(0) += 1;
                total_opaque += 1;
            }
        }
    }

    if total_opaque == 0 {
        return (50.0, 50.0);
    }

    // Find the two most common colors
    let mut counts: Vec<_> = color_counts.into_iter().collect();
    counts.sort_by(|a, b| b.1.cmp(&a.1));

    if counts.len() < 2 {
        return (100.0, 0.0);
    }

    // Cluster similar colors (within threshold)
    let threshold = 40u32;
    let mut cluster1_count = 0u32;
    let mut cluster2_count = 0u32;
    let color1 = counts[0].0;
    let color2 = counts[1].0;

    for (color, count) in counts {
        let dist1 = ((color.0 as i32 - color1.0 as i32).abs() as u32
            + (color.1 as i32 - color1.1 as i32).abs() as u32
            + (color.2 as i32 - color1.2 as i32).abs() as u32)
            / 3;
        let dist2 = ((color.0 as i32 - color2.0 as i32).abs() as u32
            + (color.1 as i32 - color2.1 as i32).abs() as u32
            + (color.2 as i32 - color2.2 as i32).abs() as u32)
            / 3;

        if dist1 <= threshold || dist1 < dist2 {
            cluster1_count += count;
        } else {
            cluster2_count += count;
        }
    }

    let total = cluster1_count + cluster2_count;
    if total == 0 {
        return (50.0, 50.0);
    }

    let pct1 = (cluster1_count as f32 / total as f32) * 100.0;
    let pct2 = (cluster2_count as f32 / total as f32) * 100.0;

    (pct1, pct2)
}

fn main() {
    // Load style names from ball_options.txt
    let style_names = load_style_names();
    let palette_names = load_palette_names();

    println!(
        "Creating showcase with {} styles x {} palettes",
        style_names.len(),
        SHOWCASE_PALETTES.len()
    );

    // Calculate image dimensions
    let cols = SHOWCASE_PALETTES.len() as u32;
    let rows = style_names.len() as u32;

    let style_label_width: u32 = 120; // Left column for style names
    let palette_label_height: u32 = 40; // Top row for palette names

    let width = style_label_width + cols * (BALL_SIZE + PADDING) + PADDING + COVERAGE_COL_WIDTH;
    let height = palette_label_height + rows * (BALL_SIZE + PADDING) + PADDING;

    let mut showcase = RgbaImage::new(width, height);

    // Fill with dark background
    for pixel in showcase.pixels_mut() {
        *pixel = Rgba([30, 30, 35, 255]);
    }

    // Load font
    let font = FontRef::try_from_slice(FONT_DATA).expect("Failed to load font");
    let scale = PxScale::from(18.0);
    let small_scale = PxScale::from(14.0);
    let text_color = Rgba([220u8, 220u8, 220u8, 255u8]);

    // Draw palette names in header
    for (col, &palette_idx) in SHOWCASE_PALETTES.iter().enumerate() {
        let palette_name = palette_names
            .get(palette_idx)
            .map(|s| s.as_str())
            .unwrap_or("?");
        let x = style_label_width + PADDING + (col as u32) * (BALL_SIZE + PADDING) + 10;
        let y = 12;
        draw_text_mut(
            &mut showcase,
            text_color,
            x as i32,
            y,
            small_scale,
            &font,
            palette_name,
        );
    }

    // Draw coverage header
    let coverage_x = style_label_width + PADDING + (cols) * (BALL_SIZE + PADDING) + 5;
    draw_text_mut(
        &mut showcase,
        text_color,
        coverage_x as i32,
        12,
        small_scale,
        &font,
        "L% / R%",
    );

    // Load and place each ball texture
    for (row, style_name) in style_names.iter().enumerate() {
        // Draw style name label
        let label_y =
            palette_label_height + PADDING + (row as u32) * (BALL_SIZE + PADDING) + BALL_SIZE / 2
                - 9;
        draw_text_mut(
            &mut showcase,
            text_color,
            8,
            label_y as i32,
            scale,
            &font,
            style_name,
        );

        let mut row_coverage: Option<(f32, f32)> = None;

        for (col, &palette_idx) in SHOWCASE_PALETTES.iter().enumerate() {
            let filename = format!(
                "assets/textures/balls/ball_{}_{}.png",
                style_name, palette_idx
            );

            match image::open(&filename) {
                Ok(ball_img) => {
                    let ball = ball_img.to_rgba8();

                    // Calculate coverage on first palette (they should be similar)
                    if row_coverage.is_none() {
                        row_coverage = Some(calculate_coverage(&ball));
                    }

                    // Calculate position
                    let x = style_label_width + PADDING + (col as u32) * (BALL_SIZE + PADDING);
                    let y = palette_label_height + PADDING + (row as u32) * (BALL_SIZE + PADDING);

                    // Copy ball texture to showcase with alpha blending
                    for (bx, by, pixel) in ball.enumerate_pixels() {
                        if pixel[3] > 0 {
                            let dst_x = x + bx;
                            let dst_y = y + by;
                            if dst_x < width && dst_y < height {
                                let dst = showcase.get_pixel(dst_x, dst_y);
                                let alpha = pixel[3] as f32 / 255.0;
                                let inv_alpha = 1.0 - alpha;
                                let blended = Rgba([
                                    (pixel[0] as f32 * alpha + dst[0] as f32 * inv_alpha) as u8,
                                    (pixel[1] as f32 * alpha + dst[1] as f32 * inv_alpha) as u8,
                                    (pixel[2] as f32 * alpha + dst[2] as f32 * inv_alpha) as u8,
                                    255,
                                ]);
                                showcase.put_pixel(dst_x, dst_y, blended);
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Could not load {}: {}", filename, e);
                }
            }
        }

        // Draw coverage percentage for this row
        if let Some((left_pct, right_pct)) = row_coverage {
            let coverage_text = format!("{:.0}/{:.0}", left_pct, right_pct);
            let cov_y = palette_label_height
                + PADDING
                + (row as u32) * (BALL_SIZE + PADDING)
                + BALL_SIZE / 2
                - 9;
            draw_text_mut(
                &mut showcase,
                text_color,
                coverage_x as i32,
                cov_y as i32,
                scale,
                &font,
                &coverage_text,
            );
        }
    }

    // Draw subtle grid lines
    let line_color = Rgba([50, 50, 55, 255]);

    // Horizontal line under header
    let header_line_y = palette_label_height;
    for x in 0..width {
        showcase.put_pixel(x, header_line_y, line_color);
    }

    // Vertical line after labels
    for y in 0..height {
        showcase.put_pixel(style_label_width, y, line_color);
    }

    // Save the showcase
    let output_path = "showcase/ball_styles_showcase.png";
    showcase.save(output_path).expect("Failed to save showcase");

    println!("\nShowcase saved to: {}", output_path);
    println!("\nStyles: {:?}", style_names);
    let showcase_palette_names: Vec<_> = SHOWCASE_PALETTES
        .iter()
        .map(|&i| palette_names.get(i).map(|s| s.as_str()).unwrap_or("?"))
        .collect();
    println!("Palettes: {:?}", showcase_palette_names);
}

fn load_style_names() -> Vec<String> {
    let content =
        fs::read_to_string("config/ball_options.txt").expect("Could not read ball_options.txt");

    let mut styles = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if let Some(name) = line.strip_prefix("style:") {
            styles.push(name.trim().to_string());
        }
    }

    styles
}

fn load_palette_names() -> Vec<String> {
    let content = fs::read_to_string("config/palettes.txt").expect("Could not read palettes.txt");

    let mut palettes = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if let Some(name) = line.strip_prefix("palette:") {
            palettes.push(name.trim().to_string());
        }
    }

    palettes
}
