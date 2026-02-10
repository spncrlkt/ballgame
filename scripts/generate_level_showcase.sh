#!/bin/bash
# Generate a showcase of all non-debug levels
# Creates a grid image with 4 levels per row, with level names
#
# Parses config/levels.txt to find playable levels (skips debug/regression)
# Uses level IDs for stable references
#
# Usage: ./scripts/generate_level_showcase.sh

set -e

# Change to project root
cd "$(dirname "$0")/.."

# Output directory for individual level screenshots
LEVEL_DIR="level_screenshots"
LEVELS_FILE="config/levels.txt"

# Clean up old screenshots
rm -rf "$LEVEL_DIR"
mkdir -p "$LEVEL_DIR"

# Build once
echo "Building..."
cargo build --quiet

# Parse levels.txt to extract non-debug levels
# Format: level name, then id on next line, skip if debug: true or regression: true
declare -a LEVEL_IDS=()
declare -a LEVEL_NAMES=()

current_name=""
current_id=""
is_debug=false

while IFS= read -r line || [[ -n "$line" ]]; do
    # Skip comments and empty lines
    [[ "$line" =~ ^[[:space:]]*# ]] && continue
    [[ -z "${line// }" ]] && continue

    # Check for level start
    if [[ "$line" =~ ^level:[[:space:]]*(.+)$ ]]; then
        # Save previous level if valid
        if [[ -n "$current_id" && "$is_debug" == false ]]; then
            LEVEL_IDS+=("$current_id")
            LEVEL_NAMES+=("$current_name")
        fi
        # Start new level
        current_name="${BASH_REMATCH[1]}"
        current_id=""
        is_debug=false
    elif [[ "$line" =~ ^id:[[:space:]]*(.+)$ ]]; then
        current_id="${BASH_REMATCH[1]}"
    elif [[ "$line" =~ ^debug:[[:space:]]*true ]] || [[ "$line" =~ ^regression:[[:space:]]*true ]]; then
        is_debug=true
    fi
done < "$LEVELS_FILE"

# Don't forget the last level
if [[ -n "$current_id" && "$is_debug" == false ]]; then
    LEVEL_IDS+=("$current_id")
    LEVEL_NAMES+=("$current_name")
fi

echo "Found ${#LEVEL_IDS[@]} playable levels"
echo "Capturing screenshots..."

# Capture screenshot for each level
for i in "${!LEVEL_IDS[@]}"; do
    id="${LEVEL_IDS[$i]}"
    name="${LEVEL_NAMES[$i]}"

    echo "  $name ($id)"

    # Clear snapshots
    rm -rf showcase/snapshots/

    # Run game with level ID, capture screenshot, quit immediately
    # --freeze-countdown keeps physics frozen so players stay visible at spawn positions
    cargo run --quiet -- --level "$id" --screenshot-and-quit --freeze-countdown 2>/dev/null || true

    # Find the screenshot and copy with nice name
    SCREENSHOT=$(ls -t showcase/snapshots/*startup*.png 2>/dev/null | head -1)
    if [ -n "$SCREENSHOT" ]; then
        # Use index for sorting, name for display
        cp "$SCREENSHOT" "$LEVEL_DIR/level_$(printf '%02d' $i)_${name// /_}.png"
    else
        echo "    Warning: No screenshot captured for $name"
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
