//! Palette database - parsing, storage, and file generation

use bevy::prelude::*;
use std::fs;
use std::path::Path;

/// Full color palette including team colors and environment colors
#[derive(Clone, Debug)]
pub struct Palette {
    pub name: String,
    pub left: Color,
    pub left_rim: Color,
    pub right: Color,
    pub right_rim: Color,
    pub background: Color,
    pub platforms: Color,
}

impl Palette {
    /// Create a palette with explicit colors
    pub fn new(
        name: &str,
        left: (f32, f32, f32),
        left_rim: (f32, f32, f32),
        right: (f32, f32, f32),
        right_rim: (f32, f32, f32),
        background: (f32, f32, f32),
        platforms: (f32, f32, f32),
    ) -> Self {
        Self {
            name: name.to_string(),
            left: Color::srgb(left.0, left.1, left.2),
            left_rim: Color::srgb(left_rim.0, left_rim.1, left_rim.2),
            right: Color::srgb(right.0, right.1, right.2),
            right_rim: Color::srgb(right_rim.0, right_rim.1, right_rim.2),
            background: Color::srgb(background.0, background.1, background.2),
            platforms: Color::srgb(platforms.0, platforms.1, platforms.2),
        }
    }
}

/// Database of all loaded palettes
#[derive(Resource)]
pub struct PaletteDatabase {
    pub palettes: Vec<Palette>,
}

impl Default for PaletteDatabase {
    fn default() -> Self {
        Self::default_palettes()
    }
}

/// Path to palettes file
pub const PALETTES_FILE: &str = "assets/palettes.txt";

impl PaletteDatabase {
    /// Load palettes from file, creating default file if it doesn't exist
    pub fn load_or_create(path: &str) -> Self {
        // If file doesn't exist, create it with defaults
        if !Path::new(path).exists() {
            info!("Palettes file not found, creating default: {}", path);
            let defaults = Self::default_palettes();
            if let Err(e) = defaults.write_to_file(path) {
                warn!("Failed to write default palettes file: {}", e);
            }
            return defaults;
        }

        // Load from file
        match fs::read_to_string(path) {
            Ok(content) => {
                let db = Self::parse(&content);
                if db.palettes.is_empty() {
                    warn!("No palettes loaded from file, using defaults");
                    return Self::default_palettes();
                }
                info!("Loaded {} palettes from {}", db.palettes.len(), path);
                db
            }
            Err(e) => {
                warn!("Failed to load palettes from {}: {}, using defaults", path, e);
                Self::default_palettes()
            }
        }
    }

    /// Write palettes to file
    pub fn write_to_file(&self, path: &str) -> std::io::Result<()> {
        let mut content = String::new();
        content.push_str("# Ballgame Color Palettes\n");
        content.push_str("# =======================\n");
        content.push_str("#\n");
        content.push_str("# Format:\n");
        content.push_str("#   palette: <name>              Start a new palette\n");
        content.push_str("#   left: <r> <g> <b>            Left team color (0.0-1.0)\n");
        content.push_str("#   left_rim: <r> <g> <b>        Left basket rim color\n");
        content.push_str("#   right: <r> <g> <b>           Right team color\n");
        content.push_str("#   right_rim: <r> <g> <b>       Right basket rim color\n");
        content.push_str("#   background: <r> <g> <b>      Arena background color\n");
        content.push_str("#   platforms: <r> <g> <b>       Floor, walls, steps, and floating platforms\n");
        content.push_str("#\n");
        content.push_str("# Exactly 20 palettes are required (for ball texture system).\n");
        content.push_str("#\n");
        content.push_str("# Blank lines and # comments are ignored.\n");
        content.push_str("\n");

        for palette in &self.palettes {
            content.push_str(&format!("palette: {}\n", palette.name));
            content.push_str(&format!(
                "left: {:.3} {:.3} {:.3}\n",
                Self::color_r(&palette.left),
                Self::color_g(&palette.left),
                Self::color_b(&palette.left)
            ));
            content.push_str(&format!(
                "left_rim: {:.3} {:.3} {:.3}\n",
                Self::color_r(&palette.left_rim),
                Self::color_g(&palette.left_rim),
                Self::color_b(&palette.left_rim)
            ));
            content.push_str(&format!(
                "right: {:.3} {:.3} {:.3}\n",
                Self::color_r(&palette.right),
                Self::color_g(&palette.right),
                Self::color_b(&palette.right)
            ));
            content.push_str(&format!(
                "right_rim: {:.3} {:.3} {:.3}\n",
                Self::color_r(&palette.right_rim),
                Self::color_g(&palette.right_rim),
                Self::color_b(&palette.right_rim)
            ));
            content.push_str(&format!(
                "background: {:.3} {:.3} {:.3}\n",
                Self::color_r(&palette.background),
                Self::color_g(&palette.background),
                Self::color_b(&palette.background)
            ));
            content.push_str(&format!(
                "platforms: {:.3} {:.3} {:.3}\n",
                Self::color_r(&palette.platforms),
                Self::color_g(&palette.platforms),
                Self::color_b(&palette.platforms)
            ));
            content.push_str("\n");
        }

        fs::write(path, content)
    }

    /// Extract red component from Color
    fn color_r(c: &Color) -> f32 {
        match c {
            Color::Srgba(srgba) => srgba.red,
            _ => 0.0,
        }
    }

    /// Extract green component from Color
    fn color_g(c: &Color) -> f32 {
        match c {
            Color::Srgba(srgba) => srgba.green,
            _ => 0.0,
        }
    }

    /// Extract blue component from Color
    fn color_b(c: &Color) -> f32 {
        match c {
            Color::Srgba(srgba) => srgba.blue,
            _ => 0.0,
        }
    }

    /// Parse palette data from string
    pub fn parse(content: &str) -> Self {
        let mut palettes = Vec::new();
        let mut current: Option<PaletteBuilder> = None;

        for line in content.lines() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some(name) = line.strip_prefix("palette:") {
                // Save previous palette if complete
                if let Some(builder) = current.take() {
                    if let Some(palette) = builder.build() {
                        palettes.push(palette);
                    }
                }
                // Start new palette
                current = Some(PaletteBuilder::new(name.trim()));
            } else if let Some(builder) = &mut current {
                if let Some(rgb) = line.strip_prefix("left_rim:") {
                    builder.left_rim = Self::parse_rgb(rgb);
                } else if let Some(rgb) = line.strip_prefix("left:") {
                    builder.left = Self::parse_rgb(rgb);
                } else if let Some(rgb) = line.strip_prefix("right_rim:") {
                    builder.right_rim = Self::parse_rgb(rgb);
                } else if let Some(rgb) = line.strip_prefix("right:") {
                    builder.right = Self::parse_rgb(rgb);
                } else if let Some(rgb) = line.strip_prefix("background:") {
                    builder.background = Self::parse_rgb(rgb);
                } else if let Some(rgb) = line.strip_prefix("platforms:") {
                    builder.platforms = Self::parse_rgb(rgb);
                } else if let Some(rgb) = line.strip_prefix("floor:") {
                    // Legacy support: treat 'floor:' as 'platforms:'
                    builder.platforms = Self::parse_rgb(rgb);
                }
            }
        }

        // Don't forget the last palette
        if let Some(builder) = current {
            if let Some(palette) = builder.build() {
                palettes.push(palette);
            }
        }

        if palettes.is_empty() {
            warn!("No palettes parsed, using defaults");
            return Self::default_palettes();
        }

        info!("Loaded {} palettes from file", palettes.len());
        Self { palettes }
    }

    /// Parse RGB values from "r g b" string
    fn parse_rgb(s: &str) -> Option<(f32, f32, f32)> {
        let parts: Vec<&str> = s.trim().split_whitespace().collect();
        if parts.len() >= 3 {
            if let (Ok(r), Ok(g), Ok(b)) = (
                parts[0].parse::<f32>(),
                parts[1].parse::<f32>(),
                parts[2].parse::<f32>(),
            ) {
                return Some((r, g, b));
            }
        }
        None
    }

    /// Get palette by index
    pub fn get(&self, index: usize) -> Option<&Palette> {
        self.palettes.get(index)
    }

    /// Get number of palettes
    pub fn len(&self) -> usize {
        self.palettes.len()
    }

    /// Minimal fallback palettes - used only if assets/palettes.txt fails to load
    /// The real palettes should be defined in assets/palettes.txt (single source of truth)
    pub fn default_palettes() -> Self {
        Self {
            palettes: vec![
                // Just 3 essential fallbacks - the real palettes are in assets/palettes.txt
                Palette::new("Neon", (0.0, 1.0, 0.8), (0.0, 0.75, 0.6), (1.0, 0.2, 0.6), (0.75, 0.15, 0.45), (0.06, 0.06, 0.1), (0.18, 0.18, 0.25)),
                Palette::new("Classic", (0.1, 0.5, 1.0), (0.05, 0.25, 0.5), (1.0, 0.4, 0.2), (0.5, 0.2, 0.1), (0.35, 0.32, 0.28), (0.15, 0.13, 0.12)),
                Palette::new("Mono", (0.95, 0.95, 0.95), (0.7, 0.7, 0.7), (0.5, 0.5, 0.5), (0.38, 0.38, 0.38), (0.15, 0.15, 0.15), (0.35, 0.35, 0.35)),
            ],
        }
    }
}

/// Builder for parsing palettes from file
struct PaletteBuilder {
    name: String,
    left: Option<(f32, f32, f32)>,
    left_rim: Option<(f32, f32, f32)>,
    right: Option<(f32, f32, f32)>,
    right_rim: Option<(f32, f32, f32)>,
    background: Option<(f32, f32, f32)>,
    platforms: Option<(f32, f32, f32)>,
}

impl PaletteBuilder {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            left: None,
            left_rim: None,
            right: None,
            right_rim: None,
            background: None,
            platforms: None,
        }
    }

    fn build(self) -> Option<Palette> {
        // All fields required
        let left = self.left?;
        let left_rim = self.left_rim?;
        let right = self.right?;
        let right_rim = self.right_rim?;
        let background = self.background?;
        let platforms = self.platforms?;

        Some(Palette::new(&self.name, left, left_rim, right, right_rim, background, platforms))
    }
}
