//! Wedge ball rotation GIF generator
//!
//! Renders a 3D sphere with wedge sections (beach ball style),
//! with dual-axis rotation for a dynamic view.

use image::{Rgba, RgbaImage};
use std::f32::consts::PI;

const SIZE: u32 = 256;
const FRAMES: u32 = 120;
const SECTIONS: u32 = 6; // Number of wedge sections

const COLOR_A: [u8; 3] = [0, 255, 128]; // Green
const COLOR_B: [u8; 3] = [255, 0, 128]; // Pink
const BORDER_COLOR: [u8; 3] = [20, 20, 20];
const BG_COLOR: [u8; 3] = [30, 30, 35];

fn main() {
    println!(
        "Generating wedge_{} frames ({} frames at {}x{})...",
        SECTIONS, FRAMES, SIZE, SIZE
    );

    std::fs::create_dir_all("assets/wedge_frames").ok();

    for frame in 0..FRAMES {
        let t = frame as f32 / FRAMES as f32;
        // Primary rotation around Y axis (full rotation)
        let y_angle = t * 2.0 * PI;
        // Secondary slow tilt on X axis (wobble to show top/bottom)
        let x_angle = (t * 2.0 * PI).sin() * 0.4; // ±0.4 radians tilt

        let img = render_wedge(y_angle, x_angle);
        img.save(format!("assets/wedge_frames/frame_{:03}.png", frame))
            .unwrap();
        print!("\r  Frame {}/{}", frame + 1, FRAMES);
    }

    println!("\n\nCreating GIF...");
    let _ = std::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-framerate",
            "30",
            "-i",
            "assets/wedge_frames/frame_%03d.png",
            "-vf",
            "split[s0][s1];[s0]palettegen=max_colors=128[p];[s1][p]paletteuse",
            "-loop",
            "0",
            "assets/wedge_rotation.gif",
        ])
        .status();

    println!("Done! assets/wedge_rotation.gif");
}

/// Rotate point around Y axis
fn rotate_y(p: [f32; 3], angle: f32) -> [f32; 3] {
    let c = angle.cos();
    let s = angle.sin();
    [p[0] * c + p[2] * s, p[1], -p[0] * s + p[2] * c]
}

/// Rotate point around X axis
fn rotate_x(p: [f32; 3], angle: f32) -> [f32; 3] {
    let c = angle.cos();
    let s = angle.sin();
    [p[0], p[1] * c - p[2] * s, p[1] * s + p[2] * c]
}

/// Combined rotation: first Y, then X
fn rotate_yx(p: [f32; 3], y_angle: f32, x_angle: f32) -> [f32; 3] {
    rotate_x(rotate_y(p, y_angle), x_angle)
}

/// Determine which wedge section a 3D point belongs to.
/// Returns the section index (0 to SECTIONS-1)
fn which_section(point: [f32; 3]) -> u32 {
    // Calculate longitude angle (around Y axis)
    // Using atan2 to get angle in range [-π, π]
    let longitude = point[2].atan2(point[0]);

    // Normalize to [0, 2π]
    let normalized = if longitude < 0.0 {
        longitude + 2.0 * PI
    } else {
        longitude
    };

    // Determine section
    let section_angle = 2.0 * PI / SECTIONS as f32;
    ((normalized / section_angle) as u32) % SECTIONS
}

/// Check if a point is near a section boundary (for anti-aliasing)
fn near_boundary(point: [f32; 3], threshold: f32) -> bool {
    let longitude = point[2].atan2(point[0]);
    let normalized = if longitude < 0.0 {
        longitude + 2.0 * PI
    } else {
        longitude
    };

    let section_angle = 2.0 * PI / SECTIONS as f32;
    let within_section = normalized % section_angle;

    // Check if close to either edge of the section
    within_section < threshold || within_section > (section_angle - threshold)
}

fn render_wedge(y_angle: f32, x_angle: f32) -> RgbaImage {
    let mut img = RgbaImage::new(SIZE, SIZE);
    let radius = (SIZE as f32 / 2.0) - 2.0;
    let center = SIZE as f32 / 2.0;
    let border = 8.0;

    for y in 0..SIZE {
        for x in 0..SIZE {
            let fx = x as f32 - center;
            let fy = y as f32 - center;
            let dist = (fx * fx + fy * fy).sqrt();

            if dist > radius + 1.0 {
                // Outside sphere
                img.put_pixel(x, y, Rgba([BG_COLOR[0], BG_COLOR[1], BG_COLOR[2], 255]));
            } else if dist > radius - border {
                // Border region
                let alpha = ((radius + 1.0 - dist) / 2.0).min(1.0).max(0.0);
                let bg_blend = 1.0 - alpha;
                img.put_pixel(
                    x,
                    y,
                    Rgba([
                        (BORDER_COLOR[0] as f32 * alpha + BG_COLOR[0] as f32 * bg_blend) as u8,
                        (BORDER_COLOR[1] as f32 * alpha + BG_COLOR[1] as f32 * bg_blend) as u8,
                        (BORDER_COLOR[2] as f32 * alpha + BG_COLOR[2] as f32 * bg_blend) as u8,
                        255,
                    ]),
                );
            } else {
                // Inside sphere - project to 3D surface
                let px = fx / radius;
                let py = -fy / radius; // Flip y for screen coords
                let pz = (1.0 - px * px - py * py).sqrt();

                let world_point = [px, py, pz];

                // Rotate backwards to get local coordinates
                let local = rotate_yx(world_point, -y_angle, -x_angle);

                // Get section
                let section = which_section(local);

                // Choose color based on section (alternating)
                let (r, g, b) = if section % 2 == 0 {
                    (COLOR_A[0], COLOR_A[1], COLOR_A[2])
                } else {
                    (COLOR_B[0], COLOR_B[1], COLOR_B[2])
                };

                // Add subtle shading based on angle to light
                let light_dir = [0.3f32, 0.5, 0.8];
                let light_len = (light_dir[0] * light_dir[0]
                    + light_dir[1] * light_dir[1]
                    + light_dir[2] * light_dir[2])
                    .sqrt();
                let nl = [
                    light_dir[0] / light_len,
                    light_dir[1] / light_len,
                    light_dir[2] / light_len,
                ];

                let dot = world_point[0] * nl[0] + world_point[1] * nl[1] + world_point[2] * nl[2];
                let shade = 0.6 + 0.4 * dot.max(0.0);

                // Draw section boundary lines
                let boundary_width = 0.08;
                if near_boundary(local, boundary_width) {
                    // Draw dark line at boundary
                    let line_shade = 0.3;
                    img.put_pixel(
                        x,
                        y,
                        Rgba([
                            (r as f32 * line_shade) as u8,
                            (g as f32 * line_shade) as u8,
                            (b as f32 * line_shade) as u8,
                            255,
                        ]),
                    );
                } else {
                    img.put_pixel(
                        x,
                        y,
                        Rgba([
                            (r as f32 * shade) as u8,
                            (g as f32 * shade) as u8,
                            (b as f32 * shade) as u8,
                            255,
                        ]),
                    );
                }
            }
        }
    }

    img
}
