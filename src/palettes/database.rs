//! Palette database - parsing, storage, and file generation

use bevy::prelude::*;
use std::fs;
use std::path::Path;

/// Full color palette including team colors and environment colors
#[derive(Clone, Debug)]
pub struct Palette {
    pub name: String,
    pub left: Color,
    pub left_dark: Color,
    pub right: Color,
    pub right_dark: Color,
    pub background: Color,
    pub floor: Color,
    pub platform: Color,
}

impl Palette {
    /// Create a palette with auto-darkened team variants
    pub fn new(
        name: &str,
        left: (f32, f32, f32),
        right: (f32, f32, f32),
        background: (f32, f32, f32),
        floor: (f32, f32, f32),
        platform: (f32, f32, f32),
    ) -> Self {
        Self {
            name: name.to_string(),
            left: Color::srgb(left.0, left.1, left.2),
            left_dark: Color::srgb(left.0 * 0.5, left.1 * 0.5, left.2 * 0.5),
            right: Color::srgb(right.0, right.1, right.2),
            right_dark: Color::srgb(right.0 * 0.5, right.1 * 0.5, right.2 * 0.5),
            background: Color::srgb(background.0, background.1, background.2),
            floor: Color::srgb(floor.0, floor.1, floor.2),
            platform: Color::srgb(platform.0, platform.1, platform.2),
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

/// Number of palettes (used by ball texture system)
pub const NUM_PALETTES: usize = 20;

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
                if db.palettes.len() != NUM_PALETTES {
                    warn!(
                        "Expected {} palettes, found {}. Using defaults.",
                        NUM_PALETTES,
                        db.palettes.len()
                    );
                    return Self::default_palettes();
                }
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
        content.push_str("#   right: <r> <g> <b>           Right team color (0.0-1.0)\n");
        content.push_str("#   background: <r> <g> <b>      Arena background color\n");
        content.push_str("#   floor: <r> <g> <b>           Floor and wall color\n");
        content.push_str("#   platform: <r> <g> <b>        Platform color\n");
        content.push_str("#\n");
        content.push_str("# Dark variants (left_dark, right_dark) are auto-generated at 50% brightness.\n");
        content.push_str("# Exactly 10 palettes are required (for ball texture system).\n");
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
                "right: {:.3} {:.3} {:.3}\n",
                Self::color_r(&palette.right),
                Self::color_g(&palette.right),
                Self::color_b(&palette.right)
            ));
            content.push_str(&format!(
                "background: {:.3} {:.3} {:.3}\n",
                Self::color_r(&palette.background),
                Self::color_g(&palette.background),
                Self::color_b(&palette.background)
            ));
            content.push_str(&format!(
                "floor: {:.3} {:.3} {:.3}\n",
                Self::color_r(&palette.floor),
                Self::color_g(&palette.floor),
                Self::color_b(&palette.floor)
            ));
            content.push_str(&format!(
                "platform: {:.3} {:.3} {:.3}\n",
                Self::color_r(&palette.platform),
                Self::color_g(&palette.platform),
                Self::color_b(&palette.platform)
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
                if let Some(rgb) = line.strip_prefix("left:") {
                    builder.left = Self::parse_rgb(rgb);
                } else if let Some(rgb) = line.strip_prefix("right:") {
                    builder.right = Self::parse_rgb(rgb);
                } else if let Some(rgb) = line.strip_prefix("background:") {
                    builder.background = Self::parse_rgb(rgb);
                } else if let Some(rgb) = line.strip_prefix("floor:") {
                    builder.floor = Self::parse_rgb(rgb);
                } else if let Some(rgb) = line.strip_prefix("platform:") {
                    builder.platform = Self::parse_rgb(rgb);
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

    /// Default palettes - 20 unique color schemes with good contrast
    pub fn default_palettes() -> Self {
        Self {
            palettes: vec![
                // 0: Ocean Fire - Blue vs Orange on warm stone
                Palette::new(
                    "Ocean Fire",
                    (0.118, 0.565, 1.0),
                    (1.0, 0.42, 0.208),
                    (0.35, 0.32, 0.28),
                    (0.15, 0.13, 0.12),
                    (0.22, 0.2, 0.17),
                ),
                // 1: Forest Crimson - Green vs Red on dark earth
                Palette::new(
                    "Forest Crimson",
                    (0.133, 0.545, 0.133),
                    (0.863, 0.078, 0.235),
                    (0.18, 0.15, 0.12),
                    (0.08, 0.07, 0.05),
                    (0.12, 0.1, 0.08),
                ),
                // 2: Electric Neon - Cyan vs Pink on deep black
                Palette::new(
                    "Electric Neon",
                    (0.0, 1.0, 0.784),
                    (1.0, 0.196, 0.588),
                    (0.06, 0.06, 0.1),
                    (0.02, 0.02, 0.04),
                    (0.08, 0.08, 0.14),
                ),
                // 3: Royal Gold - Blue vs Gold on deep purple
                Palette::new(
                    "Royal Gold",
                    (0.255, 0.412, 0.882),
                    (1.0, 0.843, 0.0),
                    (0.12, 0.08, 0.18),
                    (0.05, 0.03, 0.08),
                    (0.1, 0.06, 0.14),
                ),
                // 4: Sunset - Violet vs Orange on dusky rose
                Palette::new(
                    "Sunset",
                    (0.933, 0.51, 0.933),
                    (1.0, 0.647, 0.0),
                    (0.22, 0.14, 0.18),
                    (0.1, 0.06, 0.08),
                    (0.16, 0.1, 0.13),
                ),
                // 5: Arctic Ember - Sky vs Tomato on cool slate
                Palette::new(
                    "Arctic Ember",
                    (0.529, 0.808, 0.98),
                    (0.91, 0.298, 0.239),
                    (0.18, 0.22, 0.28),
                    (0.08, 0.1, 0.14),
                    (0.13, 0.16, 0.2),
                ),
                // 6: Toxic Slime - Lime vs Purple on murky swamp
                Palette::new(
                    "Toxic Slime",
                    (0.0, 1.0, 0.0),
                    (0.58, 0.0, 0.827),
                    (0.06, 0.1, 0.04),
                    (0.02, 0.05, 0.01),
                    (0.05, 0.1, 0.03),
                ),
                // 7: Bubblegum - Teal vs Pink on soft lavender
                Palette::new(
                    "Bubblegum",
                    (0.0, 0.753, 0.753),
                    (1.0, 0.412, 0.706),
                    (0.2, 0.16, 0.24),
                    (0.08, 0.06, 0.1),
                    (0.14, 0.11, 0.18),
                ),
                // 8: Desert Storm - Tan vs Brown on sandy dunes
                Palette::new(
                    "Desert Storm",
                    (0.824, 0.706, 0.549),
                    (0.545, 0.271, 0.075),
                    (0.38, 0.32, 0.24),
                    (0.18, 0.14, 0.1),
                    (0.26, 0.22, 0.16),
                ),
                // 9: Neon Noir - Cyan vs Magenta on near-black
                Palette::new(
                    "Neon Noir",
                    (0.0, 0.98, 0.98),
                    (0.98, 0.0, 0.471),
                    (0.05, 0.05, 0.07),
                    (0.02, 0.02, 0.03),
                    (0.07, 0.07, 0.1),
                ),
                // 10: Ice & Fire - White-blue vs Deep red on glacier
                Palette::new(
                    "Ice and Fire",
                    (0.7, 0.85, 1.0),
                    (0.8, 0.1, 0.1),
                    (0.15, 0.2, 0.28),
                    (0.06, 0.08, 0.12),
                    (0.1, 0.14, 0.2),
                ),
                // 11: Jungle Fever - Bright green vs Hot pink on dark jungle
                Palette::new(
                    "Jungle Fever",
                    (0.2, 0.9, 0.3),
                    (1.0, 0.2, 0.5),
                    (0.08, 0.12, 0.06),
                    (0.03, 0.06, 0.02),
                    (0.06, 0.1, 0.04),
                ),
                // 12: Copper Patina - Teal vs Copper on aged metal
                Palette::new(
                    "Copper Patina",
                    (0.2, 0.6, 0.55),
                    (0.85, 0.45, 0.2),
                    (0.2, 0.22, 0.2),
                    (0.08, 0.1, 0.08),
                    (0.14, 0.16, 0.14),
                ),
                // 13: Midnight Sun - Gold vs Deep blue on twilight
                Palette::new(
                    "Midnight Sun",
                    (1.0, 0.8, 0.2),
                    (0.1, 0.2, 0.6),
                    (0.12, 0.1, 0.2),
                    (0.05, 0.04, 0.1),
                    (0.08, 0.07, 0.16),
                ),
                // 14: Cherry Blossom - Pink vs Mint on soft cream
                Palette::new(
                    "Cherry Blossom",
                    (1.0, 0.6, 0.7),
                    (0.4, 0.8, 0.6),
                    (0.28, 0.25, 0.22),
                    (0.12, 0.1, 0.09),
                    (0.2, 0.18, 0.15),
                ),
                // 15: Volcanic - Orange vs Black on molten rock
                Palette::new(
                    "Volcanic",
                    (1.0, 0.5, 0.0),
                    (0.2, 0.2, 0.25),
                    (0.15, 0.08, 0.05),
                    (0.06, 0.03, 0.02),
                    (0.12, 0.06, 0.04),
                ),
                // 16: Deep Sea - Aqua vs Coral on ocean depths
                Palette::new(
                    "Deep Sea",
                    (0.0, 0.8, 0.9),
                    (1.0, 0.5, 0.45),
                    (0.05, 0.1, 0.15),
                    (0.02, 0.04, 0.07),
                    (0.04, 0.08, 0.12),
                ),
                // 17: Autumn Harvest - Orange vs Purple on warm brown
                Palette::new(
                    "Autumn Harvest",
                    (0.95, 0.6, 0.2),
                    (0.5, 0.2, 0.6),
                    (0.25, 0.18, 0.12),
                    (0.12, 0.08, 0.05),
                    (0.18, 0.13, 0.08),
                ),
                // 18: Synthwave - Hot pink vs Electric blue on deep purple
                Palette::new(
                    "Synthwave",
                    (1.0, 0.2, 0.6),
                    (0.2, 0.6, 1.0),
                    (0.1, 0.05, 0.15),
                    (0.04, 0.02, 0.07),
                    (0.08, 0.04, 0.12),
                ),
                // 19: Monochrome - White vs Gray on charcoal
                Palette::new(
                    "Monochrome",
                    (0.95, 0.95, 0.95),
                    (0.5, 0.5, 0.5),
                    (0.15, 0.15, 0.15),
                    (0.06, 0.06, 0.06),
                    (0.1, 0.1, 0.1),
                ),
            ],
        }
    }
}

/// Builder for parsing palettes from file
struct PaletteBuilder {
    name: String,
    left: Option<(f32, f32, f32)>,
    right: Option<(f32, f32, f32)>,
    background: Option<(f32, f32, f32)>,
    floor: Option<(f32, f32, f32)>,
    platform: Option<(f32, f32, f32)>,
}

impl PaletteBuilder {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            left: None,
            right: None,
            background: None,
            floor: None,
            platform: None,
        }
    }

    fn build(self) -> Option<Palette> {
        // All fields required
        let left = self.left?;
        let right = self.right?;
        let background = self.background?;
        let floor = self.floor?;
        let platform = self.platform?;

        Some(Palette::new(&self.name, left, right, background, floor, platform))
    }
}
