//! Update default profiles in constants.rs

use std::fs;
use std::path::Path;

/// Default profile constants that can be updated
pub struct DefaultProfiles {
    pub left: String,
    pub right: String,
}

/// Get current default profiles from constants.rs
pub fn get_current_defaults(constants_path: &Path) -> Option<DefaultProfiles> {
    let content = fs::read_to_string(constants_path).ok()?;

    let left = extract_const_string(&content, "DEFAULT_LEFT_PROFILE")?;
    let right = extract_const_string(&content, "DEFAULT_RIGHT_PROFILE")?;

    Some(DefaultProfiles { left, right })
}

/// Update default profiles in constants.rs
pub fn update_default_profiles(
    constants_path: &Path,
    new_left: &str,
    new_right: &str,
) -> Result<(String, String), String> {
    let content = fs::read_to_string(constants_path)
        .map_err(|e| format!("Failed to read constants.rs: {}", e))?;

    // Get current values for reporting
    let old_left = extract_const_string(&content, "DEFAULT_LEFT_PROFILE")
        .unwrap_or_else(|| "unknown".to_string());
    let old_right = extract_const_string(&content, "DEFAULT_RIGHT_PROFILE")
        .unwrap_or_else(|| "unknown".to_string());

    // Check if constants exist
    let has_left = content.contains("DEFAULT_LEFT_PROFILE");
    let has_right = content.contains("DEFAULT_RIGHT_PROFILE");

    let new_content = if has_left && has_right {
        // Update existing constants
        let updated = replace_const_string(&content, "DEFAULT_LEFT_PROFILE", new_left);
        replace_const_string(&updated, "DEFAULT_RIGHT_PROFILE", new_right)
    } else {
        // Add new constants section
        add_profile_constants(&content, new_left, new_right)
    };

    fs::write(constants_path, &new_content)
        .map_err(|e| format!("Failed to write constants.rs: {}", e))?;

    Ok((old_left, old_right))
}

/// Extract a string constant value
fn extract_const_string(content: &str, const_name: &str) -> Option<String> {
    // Look for: pub const NAME: &str = "value";
    let pattern = format!(r#"pub const {}: &str = ""#, const_name);

    for line in content.lines() {
        if line.contains(&pattern) || line.contains(&format!("pub const {}", const_name)) {
            // Extract the string value between quotes
            if let Some(start) = line.find('"') {
                if let Some(end) = line[start + 1..].find('"') {
                    return Some(line[start + 1..start + 1 + end].to_string());
                }
            }
        }
    }

    None
}

/// Replace a string constant value
fn replace_const_string(content: &str, const_name: &str, new_value: &str) -> String {
    let mut result = String::new();

    for line in content.lines() {
        if line.contains(&format!("pub const {}", const_name)) && line.contains("&str") {
            // Reconstruct the line with new value
            if let Some(eq_pos) = line.find('=') {
                let prefix = &line[..eq_pos + 1];
                result.push_str(&format!("{} \"{}\";\n", prefix, new_value));
            } else {
                result.push_str(line);
                result.push('\n');
            }
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }

    result
}

/// Add profile constants to content
fn add_profile_constants(content: &str, left: &str, right: &str) -> String {
    let section = format!(
        r#"
// =============================================================================
// DEFAULT AI PROFILES
// =============================================================================

/// Default AI profile for left player
pub const DEFAULT_LEFT_PROFILE: &str = "{}";
/// Default AI profile for right player
pub const DEFAULT_RIGHT_PROFILE: &str = "{}";
"#,
        left, right
    );

    // Find a good place to insert (after another section or at end)
    if content.contains("// =============") {
        // Insert before the last section marker we can find
        // Actually, just append to end
        format!("{}{}", content.trim_end(), section)
    } else {
        format!("{}{}", content.trim_end(), section)
    }
}

/// Format the update report
pub fn format_update_report(
    old_left: &str,
    old_right: &str,
    new_left: &str,
    new_right: &str,
) -> String {
    let mut output = String::new();
    output.push_str("\nDEFAULT PROFILES UPDATED:\n");

    if old_left != new_left {
        output.push_str(&format!("  Left player:  {} -> {}\n", old_left, new_left));
    } else {
        output.push_str(&format!("  Left player:  {} (unchanged)\n", new_left));
    }

    if old_right != new_right {
        output.push_str(&format!("  Right player: {} -> {}\n", old_right, new_right));
    } else {
        output.push_str(&format!("  Right player: {} (unchanged)\n", new_right));
    }

    output.push_str("  (Written to src/constants.rs)\n");
    output
}
