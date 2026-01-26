//! Level Showcase Generator
//!
//! Combines level screenshots into a grid PNG with level names.
//! Expects screenshots in level_screenshots/ directory from the shell script.
//!
//! Run with: `cargo run --bin generate_level_showcase`

use ab_glyph::{FontRef, PxScale};
use image::{imageops, Rgba, RgbaImage};
use imageproc::drawing::draw_text_mut;
use std::fs;

// Layout parameters
const COLS: u32 = 4;
const SCALE: f32 = 0.25; // Scale down screenshots to 25%
const PADDING: u32 = 20;
const LABEL_HEIGHT: u32 = 40;

// Embed system font
const FONT_DATA: &[u8] = include_bytes!("/System/Library/Fonts/Helvetica.ttc");

fn main() {
    let screenshot_dir = "level_screenshots";
    let output_path = "showcase/level_showcase.png";

    // Find all level screenshots
    let mut screenshots: Vec<(String, String)> = Vec::new();

    if let Ok(entries) = fs::read_dir(screenshot_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "png").unwrap_or(false) {
                let filename = path.file_name().unwrap().to_string_lossy().to_string();
                // Extract level name from filename: level_XX_Name.png -> Name
                if let Some(name) = extract_level_name(&filename) {
                    screenshots.push((path.to_string_lossy().to_string(), name));
                }
            }
        }
    }

    screenshots.sort_by(|a, b| a.0.cmp(&b.0)); // Sort by path (which includes level number)

    if screenshots.is_empty() {
        eprintln!("No screenshots found in {}/", screenshot_dir);
        eprintln!("Run ./scripts/generate_level_showcase.sh first to capture screenshots");
        std::process::exit(1);
    }

    println!("Found {} level screenshots", screenshots.len());

    // Load first image to get dimensions
    let first_img = image::open(&screenshots[0].0).expect("Failed to load first screenshot");
    let orig_width = first_img.width();
    let orig_height = first_img.height();

    let scaled_width = (orig_width as f32 * SCALE) as u32;
    let scaled_height = (orig_height as f32 * SCALE) as u32;

    println!("Original size: {}x{}, scaled to: {}x{}",
             orig_width, orig_height, scaled_width, scaled_height);

    // Calculate output dimensions
    let num_levels = screenshots.len() as u32;
    let rows = (num_levels + COLS - 1) / COLS;

    let cell_width = scaled_width + PADDING * 2;
    let cell_height = scaled_height + LABEL_HEIGHT + PADDING * 2;

    let output_width = cell_width * COLS;
    let output_height = cell_height * rows;

    println!("Output size: {}x{} ({} cols x {} rows)", output_width, output_height, COLS, rows);

    // Create output image with dark background
    let mut showcase = RgbaImage::new(output_width, output_height);
    for pixel in showcase.pixels_mut() {
        *pixel = Rgba([30, 30, 35, 255]);
    }

    // Load font
    let font = FontRef::try_from_slice(FONT_DATA).expect("Failed to load font");
    let scale = PxScale::from(24.0);
    let text_color = Rgba([220u8, 220u8, 220u8, 255u8]);

    // Place each screenshot
    for (idx, (path, name)) in screenshots.iter().enumerate() {
        let col = (idx as u32) % COLS;
        let row = (idx as u32) / COLS;

        let cell_x = col * cell_width;
        let cell_y = row * cell_height;

        // Load and resize screenshot
        match image::open(path) {
            Ok(img) => {
                let rgba = img.to_rgba8();
                let resized = imageops::resize(&rgba, scaled_width, scaled_height, imageops::FilterType::Lanczos3);

                // Copy resized image to showcase
                let img_x = cell_x + PADDING;
                let img_y = cell_y + PADDING;

                for (px, py, pixel) in resized.enumerate_pixels() {
                    let dst_x = img_x + px;
                    let dst_y = img_y + py;
                    if dst_x < output_width && dst_y < output_height {
                        showcase.put_pixel(dst_x, dst_y, *pixel);
                    }
                }

                // Draw level name below screenshot
                let text_x = cell_x + PADDING + 10;
                let text_y = cell_y + PADDING + scaled_height + 8;
                draw_text_mut(&mut showcase, text_color, text_x as i32, text_y as i32, scale, &font, name);

                println!("  Added: {}", name);
            }
            Err(e) => {
                eprintln!("Warning: Failed to load {}: {}", path, e);
            }
        }
    }

    // Draw grid lines between cells
    let line_color = Rgba([50, 50, 55, 255]);

    // Vertical lines
    for col in 1..COLS {
        let x = col * cell_width;
        for y in 0..output_height {
            showcase.put_pixel(x, y, line_color);
        }
    }

    // Horizontal lines
    for row in 1..rows {
        let y = row * cell_height;
        for x in 0..output_width {
            showcase.put_pixel(x, y, line_color);
        }
    }

    // Save output
    showcase.save(output_path).expect("Failed to save showcase");
    println!("\nShowcase saved to: {}", output_path);
}

fn extract_level_name(filename: &str) -> Option<String> {
    // level_03_Islands.png -> Islands
    // level_12_Twin_Towers.png -> Twin Towers
    let without_ext = filename.strip_suffix(".png")?;
    let parts: Vec<&str> = without_ext.splitn(3, '_').collect();
    if parts.len() >= 3 {
        Some(parts[2].replace('_', " "))
    } else {
        None
    }
}
