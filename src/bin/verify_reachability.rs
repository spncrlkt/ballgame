//! Quick verification that reachability heatmaps are loaded and integrated
//!
//! Run with: cargo run --bin verify_reachability

use std::fs;
use std::path::Path;

fn main() {
    println!("Verifying reachability heatmap integration\n");

    let heatmap_dir = Path::new("showcase/heatmaps");
    if !heatmap_dir.exists() {
        eprintln!("ERROR: Heatmap directory not found: {}", heatmap_dir.display());
        std::process::exit(1);
    }

    // Find all reachability heatmap files
    let mut levels_checked = 0;
    let mut levels_with_varied_data = 0;

    for entry in fs::read_dir(heatmap_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let name = path.file_name().unwrap().to_string_lossy();

        // Look for reachability txt files
        if name.starts_with("heatmap_reachability_") && name.ends_with(".txt") {
            levels_checked += 1;
            let level_name = name
                .strip_prefix("heatmap_reachability_")
                .unwrap()
                .split('_')
                .take_while(|s| !s.chars().all(|c| c.is_ascii_hexdigit()))
                .collect::<Vec<_>>()
                .join("_");

            // Read and parse the heatmap (CSV format: x,y,value)
            let content = fs::read_to_string(&path).unwrap();
            let values: Vec<f32> = content
                .lines()
                .skip(1) // Skip header
                .filter_map(|line| {
                    let parts: Vec<&str> = line.split(',').collect();
                    if parts.len() >= 3 {
                        parts[2].parse::<f32>().ok()
                    } else {
                        None
                    }
                })
                .collect();

            if values.is_empty() {
                println!("  {} - ERROR: No values found", level_name);
                continue;
            }

            let min = values.iter().cloned().fold(f32::INFINITY, f32::min);
            let max = values.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            let sum: f32 = values.iter().sum();
            let avg = sum / values.len() as f32;
            let varied = values.iter().filter(|&&v| (v - 0.5).abs() > 0.01).count();
            let varied_pct = (varied as f32 / values.len() as f32) * 100.0;

            let status = if varied_pct > 10.0 {
                levels_with_varied_data += 1;
                "OK"
            } else {
                "WARN: low variation"
            };

            println!(
                "  {:20} min={:.2} max={:.2} avg={:.2} varied={:4}/{:4} ({:5.1}%) [{}]",
                level_name,
                min,
                max,
                avg,
                varied,
                values.len(),
                varied_pct,
                status
            );
        }
    }

    println!("\n----------------------------------------");
    println!(
        "Checked {} levels, {} have varied reachability data",
        levels_checked, levels_with_varied_data
    );

    if levels_with_varied_data == levels_checked && levels_checked > 0 {
        println!("\nRESULT: PASS - All levels have reachability heatmaps with real data");
        std::process::exit(0);
    } else if levels_checked == 0 {
        eprintln!("\nRESULT: FAIL - No reachability heatmaps found");
        std::process::exit(1);
    } else {
        eprintln!(
            "\nRESULT: WARN - {} of {} levels may have default/empty data",
            levels_checked - levels_with_varied_data,
            levels_checked
        );
        std::process::exit(0);
    }
}
