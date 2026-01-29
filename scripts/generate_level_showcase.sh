#!/bin/bash
# Generate a showcase of all non-debug levels
# Creates a grid image with 4 levels per row, with level names
#
# Usage: ./scripts/generate_level_showcase.sh

set -e

# Change to project root
cd "$(dirname "$0")/.."

# Output directory for individual level screenshots
LEVEL_DIR="level_screenshots"
OUTPUT="assets/level_showcase.png"

# Clean up old screenshots
rm -rf "$LEVEL_DIR"
mkdir -p "$LEVEL_DIR"

# Build once
echo "Building..."
cargo build --quiet

# Level names and indices (1-indexed, matching levels.txt)
# Skipping debug levels (1=Debug, 2=Open Floor have debug: true)
declare -a LEVELS=(
    "3:Islands"
    "4:Slopes"
    "5:Tower"
    "6:Arena"
    "7:Skyway"
    "8:Terraces"
    "9:Catwalk"
    "10:Bunker"
    "11:Pit"
    "12:Twin Towers"
)

echo "Capturing ${#LEVELS[@]} levels..."

# Capture screenshot for each level
for level_info in "${LEVELS[@]}"; do
    idx="${level_info%%:*}"
    name="${level_info#*:}"

    echo "  Level $idx: $name"

    # Clear snapshots
    rm -rf showcase/snapshots/

    # Run game with level override, capture screenshot, quit immediately
    cargo run --quiet -- --level "$idx" --screenshot-and-quit 2>/dev/null || true

    # Find the screenshot and copy with nice name
    SCREENSHOT=$(ls -t showcase/snapshots/*startup*.png 2>/dev/null | head -1)
    if [ -n "$SCREENSHOT" ]; then
        cp "$SCREENSHOT" "$LEVEL_DIR/level_$(printf '%02d' $idx)_${name// /_}.png"
    else
        echo "    Warning: No screenshot captured for level $idx"
    fi
done

echo ""
echo "Combining into showcase..."

# Run the Rust binary to combine screenshots
cargo run --quiet --bin generate levels

# Clean up temp directories
rm -rf "$LEVEL_DIR"
rm -rf showcase/snapshots/

echo "Done!"
