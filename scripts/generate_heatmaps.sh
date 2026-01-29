#!/bin/bash
# Heatmap Generation Script
# =========================
#
# Generates all heatmap types for analysis, including reachability-dependent maps.
# Now that reachability loads from config/gameplay_tuning.json, all types work.
#
# Usage:
#   ./scripts/generate_heatmaps.sh                    # Generate all heatmap types for all levels
#   ./scripts/generate_heatmaps.sh --full             # Full bundle per level (all types combined)
#   ./scripts/generate_heatmaps.sh --type reachability  # Single type only
#   ./scripts/generate_heatmaps.sh --level "Islands"  # Single level only
#   ./scripts/generate_heatmaps.sh --check            # Only regenerate changed levels
#   ./scripts/generate_heatmaps.sh --refresh          # Clear cache and regenerate all
#   ./scripts/generate_heatmaps.sh --release          # Use release build (faster)
#   ./scripts/generate_heatmaps.sh --stats            # Show stats summary at end
#
# Heatmap Types:
#   speed          - Shot angle and required speed (default)
#   score          - Scoring percentage via Monte Carlo
#   reachability   - Player reachability from ground (requires physics config)
#   path_cost      - Distance from floor (depends on reachability)
#   escape_routes  - Number of reachable neighbors (depends on reachability)
#   landing_safety - Platform landing margin safety
#   line_of_sight  - Clear shot to basket (left/right)
#   elevation      - Height relative to basket
#
# Output:
#   showcase/heatmaps/heatmap_<type>_<level>_<uuid>.png
#   showcase/heatmaps/heatmap_<type>_<level>_<uuid>.txt
#   showcase/heatmap_<type>_all.png (combined sheet)
#   showcase/heatmaps/heatmap_stats.txt (statistics log)

set -e

# Change to project root
cd "$(dirname "$0")/.."

# Configuration
OUTPUT_DIR="showcase/heatmaps"
STATS_FILE="$OUTPUT_DIR/heatmap_stats.txt"
BUILD_MODE="--quiet"
RELEASE_FLAG=""

# All heatmap types
ALL_TYPES=(speed score reachability path_cost escape_routes landing_safety line_of_sight elevation)

# Reachability-dependent types (require physics config)
REACHABILITY_DEPENDENT=(reachability path_cost escape_routes)

# Parse arguments
TYPES=()
LEVELS=()
FULL_MODE=""
CHECK_MODE=""
REFRESH_MODE=""
SHOW_STATS=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --full)
            FULL_MODE="1"
            shift
            ;;
        --type)
            TYPES+=("$2")
            shift 2
            ;;
        --level)
            LEVELS+=("$2")
            shift 2
            ;;
        --check)
            CHECK_MODE="--check"
            shift
            ;;
        --refresh)
            REFRESH_MODE="--refresh"
            shift
            ;;
        --release)
            RELEASE_FLAG="--release"
            shift
            ;;
        --stats)
            SHOW_STATS="1"
            shift
            ;;
        --help|-h)
            awk '/^# Heatmap Generation Script/,/^set -e/{if(/^set -e/)exit; print}' "$0" | sed 's/^# //' | sed 's/^#//'
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Build level arguments
LEVEL_ARGS=""
for level in "${LEVELS[@]}"; do
    LEVEL_ARGS="$LEVEL_ARGS --level \"$level\""
done

# Ensure output directory exists
mkdir -p "$OUTPUT_DIR"

echo "========================================"
echo "Heatmap Generation"
echo "========================================"
echo ""

# Check that gameplay tuning exists (required for reachability)
if [ ! -f "config/gameplay_tuning.json" ]; then
    echo "WARNING: config/gameplay_tuning.json not found"
    echo "Reachability-dependent heatmaps will use default physics values"
    echo ""
fi

# Build first
echo "Building heatmap binary..."
cargo build $BUILD_MODE $RELEASE_FLAG --bin heatmap
echo ""

# Determine run mode
if [[ -n "$FULL_MODE" ]]; then
    # Full bundle mode: all types per level
    echo "Mode: Full bundle (all types per level)"
    echo ""

    CMD="cargo run $BUILD_MODE $RELEASE_FLAG --bin heatmap -- --full $CHECK_MODE $REFRESH_MODE"
    for level in "${LEVELS[@]}"; do
        CMD="$CMD --level \"$level\""
    done

    echo "Running: $CMD"
    echo ""
    eval "$CMD"

elif [[ ${#TYPES[@]} -gt 0 ]]; then
    # Specific types mode
    echo "Mode: Specific types (${TYPES[*]})"
    echo ""

    for type in "${TYPES[@]}"; do
        echo "----------------------------------------"
        echo "Generating: $type"
        echo "----------------------------------------"

        CMD="cargo run $BUILD_MODE $RELEASE_FLAG --bin heatmap -- --type $type $CHECK_MODE $REFRESH_MODE"
        for level in "${LEVELS[@]}"; do
            CMD="$CMD --level \"$level\""
        done

        eval "$CMD"
        echo ""
    done

else
    # Default: generate all types individually
    echo "Mode: All types individually"
    echo ""

    for type in "${ALL_TYPES[@]}"; do
        echo "----------------------------------------"
        echo "Generating: $type"
        echo "----------------------------------------"

        CMD="cargo run $BUILD_MODE $RELEASE_FLAG --bin heatmap -- --type $type $CHECK_MODE $REFRESH_MODE"
        for level in "${LEVELS[@]}"; do
            CMD="$CMD --level \"$level\""
        done

        eval "$CMD"
        echo ""
    done
fi

echo "========================================"
echo "Generation Complete"
echo "========================================"
echo ""

# Count outputs
PNG_COUNT=$(find "$OUTPUT_DIR" -name "heatmap_*.png" 2>/dev/null | wc -l | tr -d ' ')
TXT_COUNT=$(find "$OUTPUT_DIR" -name "heatmap_*.txt" 2>/dev/null | wc -l | tr -d ' ')

echo "Output directory: $OUTPUT_DIR"
echo "Generated files: $PNG_COUNT PNG, $TXT_COUNT TXT"
echo ""

# Show combined sheets
echo "Combined sheets:"
for sheet in showcase/heatmap_*_all.png; do
    if [ -f "$sheet" ]; then
        echo "  $sheet"
    fi
done
echo ""

# Show stats summary if requested
if [[ -n "$SHOW_STATS" ]] && [ -f "$STATS_FILE" ]; then
    echo "----------------------------------------"
    echo "Statistics Summary"
    echo "----------------------------------------"
    echo ""

    # Show low-contrast warnings
    if grep -q "Warning:" "$STATS_FILE" 2>/dev/null; then
        echo "Low-contrast warnings:"
        grep "Warning:" "$STATS_FILE" | head -10
        echo ""
    fi

    # Show stats by type
    for type in "${ALL_TYPES[@]}"; do
        COUNT=$(grep -c "\[$type" "$STATS_FILE" 2>/dev/null | tr -d '[:space:]' || echo "0")
        if [ "$COUNT" -gt 0 ] 2>/dev/null; then
            AVG_MEAN=$(grep "\[$type" "$STATS_FILE" | sed -n 's/.*mean \([0-9.]*\).*/\1/p' | awk '{sum+=$1; count++} END {if(count>0) printf "%.3f", sum/count; else print "N/A"}')
            echo "  $type: $COUNT levels, avg mean=$AVG_MEAN"
        fi
    done
    echo ""
fi

echo "Done!"
