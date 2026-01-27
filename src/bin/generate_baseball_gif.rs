//! Baseball rotation GIF generator
//!
//! Properly renders a 3D baseball with the seam curve,
//! handling front/back faces correctly.

use image::{Rgba, RgbaImage};
use std::f32::consts::PI;

const SIZE: u32 = 256;
const FRAMES: u32 = 120; // More frames for smoother dual-axis rotation

const COLOR_A: [u8; 3] = [0, 255, 128];
const COLOR_B: [u8; 3] = [255, 0, 128];
const SEAM_COLOR: [u8; 3] = [15, 15, 15]; // Black seam line
const BORDER_COLOR: [u8; 3] = [20, 20, 20];
const BG_COLOR: [u8; 3] = [30, 30, 35];

fn main() {
    println!(
        "Generating baseball frames ({} frames at {}x{})...",
        FRAMES, SIZE, SIZE
    );

    std::fs::create_dir_all("assets/baseball_frames").ok();

    for frame in 0..FRAMES {
        let t = frame as f32 / FRAMES as f32;
        // Primary rotation around Y axis (full rotation)
        let y_angle = t * 2.0 * PI;
        // Secondary slow tilt on X axis (wobble to show top/bottom)
        let x_angle = (t * 2.0 * PI).sin() * 0.4; // ±0.4 radians tilt

        let img = render_baseball(y_angle, x_angle);
        img.save(format!("assets/baseball_frames/frame_{:03}.png", frame))
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
            "assets/baseball_frames/frame_%03d.png",
            "-vf",
            "split[s0][s1];[s0]palettegen=max_colors=128[p];[s1][p]paletteuse",
            "-loop",
            "0",
            "assets/baseball_rotation.gif",
        ])
        .status();

    println!("Done! assets/baseball_rotation.gif");
}

/// Baseball seam point at parameter t (0 to 4π for full curve)
fn seam_point(t: f32) -> [f32; 3] {
    let a = 0.4;
    let theta = PI / 2.0 - (PI / 2.0 - a) * t.cos();
    let phi = t / 2.0 + a * (2.0 * t).sin();
    [
        theta.sin() * phi.cos(),
        theta.sin() * phi.sin(),
        theta.cos(),
    ]
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

/// Distance from a point to the nearest seam point
fn distance_to_seam(point: [f32; 3]) -> f32 {
    let num_samples = 200;
    let mut min_dist = f32::MAX;

    for i in 0..num_samples {
        let t = (i as f32 / num_samples as f32) * 4.0 * PI;
        let s = seam_point(t);

        // Euclidean distance on sphere surface (chord length)
        let dx = point[0] - s[0];
        let dy = point[1] - s[1];
        let dz = point[2] - s[2];
        let dist = (dx * dx + dy * dy + dz * dz).sqrt();

        if dist < min_dist {
            min_dist = dist;
        }
    }

    min_dist
}

/// Determine which region a 3D point on the unit sphere belongs to.
/// We check which side of the seam curve the point is on by computing
/// a winding-number-like sum around the seam.
fn which_region(point: [f32; 3]) -> bool {
    // Use the spherical angle method:
    // For each segment of the seam, compute the signed solid angle
    // contribution. The sign tells us which side we're on.

    let num_samples = 400;
    let mut total = 0.0f32;

    for i in 0..num_samples {
        let t1 = (i as f32 / num_samples as f32) * 4.0 * PI;
        let t2 = ((i + 1) as f32 / num_samples as f32) * 4.0 * PI;

        let s1 = seam_point(t1);
        let s2 = seam_point(t2);

        // Compute spherical excess (solid angle) of triangle formed by
        // point, s1, s2 on the unit sphere
        total += spherical_angle(point, s1, s2);
    }

    // If total is ~2π, point is inside; if ~-2π, outside (or vice versa)
    total > 0.0
}

/// Compute the signed angle at vertex A in the spherical triangle ABC
fn spherical_angle(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> f32 {
    // Vectors from A to B and A to C, projected onto tangent plane at A
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];

    // Cross product gives signed area
    let cross = [
        ab[1] * ac[2] - ab[2] * ac[1],
        ab[2] * ac[0] - ab[0] * ac[2],
        ab[0] * ac[1] - ab[1] * ac[0],
    ];

    // Dot with point (normal at point on sphere) gives signed volume
    let dot = cross[0] * a[0] + cross[1] * a[1] + cross[2] * a[2];

    // atan2 of the signed area
    let ab_len = (ab[0] * ab[0] + ab[1] * ab[1] + ab[2] * ab[2]).sqrt();
    let ac_len = (ac[0] * ac[0] + ac[1] * ac[1] + ac[2] * ac[2]).sqrt();

    if ab_len < 1e-10 || ac_len < 1e-10 {
        return 0.0;
    }

    let cos_angle = (ab[0] * ac[0] + ab[1] * ac[1] + ab[2] * ac[2]) / (ab_len * ac_len);
    let sin_sign = if dot > 0.0 { 1.0 } else { -1.0 };

    sin_sign * cos_angle.clamp(-1.0, 1.0).acos()
}

fn render_baseball(y_rotation: f32, x_rotation: f32) -> RgbaImage {
    let mut img = RgbaImage::from_pixel(
        SIZE,
        SIZE,
        Rgba([BG_COLOR[0], BG_COLOR[1], BG_COLOR[2], 255]),
    );

    let center = SIZE as f32 / 2.0;
    let radius = center - 20.0;
    let seam_width = 0.08; // Width of seam line on sphere surface

    for py in 0..SIZE {
        for px in 0..SIZE {
            let x = (px as f32 - center) / radius;
            let y = (py as f32 - center) / radius;
            let r2 = x * x + y * y;

            if r2 > 1.0 {
                continue;
            }

            // Point on front of sphere (z > 0)
            let z = (1.0 - r2).sqrt();

            // This is the point we see. Rotate it BACKWARDS to get the
            // corresponding point in the seam's reference frame.
            let world_point = [x, -y, z]; // flip y for screen coords
            let local_point = rotate_yx(world_point, -y_rotation, -x_rotation);

            // Check distance to seam
            let seam_dist = distance_to_seam(local_point);

            // Determine color
            let base = if seam_dist < seam_width {
                // On the seam - draw black line
                let seam_blend = (seam_dist / seam_width).clamp(0.0, 1.0);
                let region = which_region(local_point);
                let panel_color = if region { COLOR_A } else { COLOR_B };
                blend(panel_color, SEAM_COLOR, seam_blend)
            } else {
                // Normal panel color
                let region = which_region(local_point);
                if region { COLOR_A } else { COLOR_B }
            };

            // Anti-aliasing at edge
            let dist = r2.sqrt();
            let aa = ((1.0 - dist) * radius / 2.0).clamp(0.0, 1.0);
            let color = blend(base, BG_COLOR, aa);

            img.put_pixel(px, py, Rgba([color[0], color[1], color[2], 255]));
        }
    }

    // Draw border
    for py in 0..SIZE {
        for px in 0..SIZE {
            let x = (px as f32 - center) / radius;
            let y = (py as f32 - center) / radius;
            let r = (x * x + y * y).sqrt();

            if r > 0.95 && r < 1.08 {
                let t = if r < 1.0 {
                    (r - 0.95) / 0.05
                } else {
                    1.0 - (r - 1.0) / 0.08
                };
                let t = t.clamp(0.0, 1.0);

                let curr = img.get_pixel(px, py);
                let curr_rgb = [curr[0], curr[1], curr[2]];
                let color = blend(BORDER_COLOR, curr_rgb, t);
                img.put_pixel(px, py, Rgba([color[0], color[1], color[2], 255]));
            }
        }
    }

    img
}

fn blend(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    [
        (a[0] as f32 * t + b[0] as f32 * (1.0 - t)) as u8,
        (a[1] as f32 * t + b[1] as f32 * (1.0 - t)) as u8,
        (a[2] as f32 * t + b[2] as f32 * (1.0 - t)) as u8,
    ]
}
