#!/bin/bash
# Multi-Scenario Visual Regression Test Script
# =============================================
#
# Usage:
#   ./scripts/regression.sh              # Run all scenarios, compare to baselines
#   ./scripts/regression.sh --update     # Update all baselines with current screenshots
#   ./scripts/regression.sh <scenario>   # Run single scenario
#   ./scripts/regression.sh --list       # List available scenarios
#
# Exit codes:
#   0 = Pass (all screenshots match or baselines updated)
#   1 = Fail (one or more screenshots differ)
#   2 = Error (missing baseline, build failure, etc.)

set -e

# Change to project root
cd "$(dirname "$0")/.."

REGRESSION_DIR="showcase/regression"
BASELINE_DIR="$REGRESSION_DIR/baselines"
CURRENT_DIR="$REGRESSION_DIR/current"
DIFF_DIR="$REGRESSION_DIR/diffs"
SCENARIO_FILE="config/regression_scenarios.txt"

# Tolerance: allow up to 1% of pixels to differ (handles timing variations)
# 2560x1440 = 3,686,400 pixels, 1% = 36,864 pixels
PIXEL_TOLERANCE=40000

# Ensure directories exist
mkdir -p "$BASELINE_DIR" "$CURRENT_DIR" "$DIFF_DIR"

# Parse command-line arguments
UPDATE_MODE=""
LIST_MODE=""
SINGLE_SCENARIO=""

case "$1" in
    --update)
        UPDATE_MODE="1"
        ;;
    --list)
        LIST_MODE="1"
        ;;
    "")
        # Run all scenarios
        ;;
    *)
        # Single scenario mode
        SINGLE_SCENARIO="$1"
        ;;
esac

# Parse scenario file and extract scenario data
# Returns: name|level|palette|width|height|wait_frames|description
parse_scenarios() {
    local scenarios=()
    local current_name=""
    local current_level=""
    local current_palette=""
    local current_width=""
    local current_height=""
    local current_wait=""
    local current_desc=""

    while IFS= read -r line || [[ -n "$line" ]]; do
        # Skip empty lines and comments
        [[ -z "$line" || "$line" =~ ^[[:space:]]*# ]] && continue

        # Trim whitespace
        line=$(echo "$line" | xargs)

        if [[ "$line" =~ ^scenario:[[:space:]]*(.+) ]]; then
            # Save previous scenario if exists
            if [[ -n "$current_name" ]]; then
                scenarios+=("$current_name|$current_level|$current_palette|$current_width|$current_height|$current_wait|$current_desc")
            fi
            current_name="${BASH_REMATCH[1]}"
            current_level=""
            current_palette=""
            current_width=""
            current_height=""
            current_wait=""
            current_desc=""
        elif [[ "$line" =~ ^level:[[:space:]]*(.+) ]]; then
            current_level="${BASH_REMATCH[1]}"
        elif [[ "$line" =~ ^palette:[[:space:]]*([0-9]+) ]]; then
            current_palette="${BASH_REMATCH[1]}"
        elif [[ "$line" =~ ^viewport:[[:space:]]*([0-9]+)[[:space:]]+([0-9]+) ]]; then
            current_width="${BASH_REMATCH[1]}"
            current_height="${BASH_REMATCH[2]}"
        elif [[ "$line" =~ ^wait_frames:[[:space:]]*([0-9]+) ]]; then
            current_wait="${BASH_REMATCH[1]}"
        elif [[ "$line" =~ ^description:[[:space:]]*(.+) ]]; then
            current_desc="${BASH_REMATCH[1]}"
        fi
    done < "$SCENARIO_FILE"

    # Don't forget the last scenario
    if [[ -n "$current_name" ]]; then
        scenarios+=("$current_name|$current_level|$current_palette|$current_width|$current_height|$current_wait|$current_desc")
    fi

    # Return scenarios array
    for s in "${scenarios[@]}"; do
        echo "$s"
    done
}

# List available scenarios
list_scenarios() {
    echo "Available regression scenarios:"
    echo "==============================="
    while IFS='|' read -r name level palette width height wait desc; do
        echo ""
        echo "  $name"
        echo "    Level: $level, Palette: $palette, Viewport: ${width}x${height}"
        echo "    $desc"
    done < <(parse_scenarios)
    echo ""
}

# Run a single scenario
# Args: name level palette width height wait_frames description
run_scenario() {
    local name="$1"
    local level="$2"
    local palette="$3"
    local width="$4"
    local height="$5"
    local wait_frames="$6"
    local desc="$7"

    echo "Running scenario: $name"
    echo "  Level: $level, Palette: $palette, Viewport: ${width}x${height}"

    # Clean old snapshots
    rm -rf showcase/snapshots/

    # Run game with scenario-specific flags
    cargo run --quiet -- \
        --screenshot-and-quit \
        --level "$level" \
        --palette "$palette" \
        --viewport "$width" "$height" \
        2>/dev/null

    # Find the startup screenshot
    local screenshot
    screenshot=$(ls -t showcase/snapshots/*startup*.png 2>/dev/null | head -1)
    if [ -z "$screenshot" ]; then
        echo "  ERROR: No screenshot captured"
        return 2
    fi

    # Copy to current directory with scenario name
    cp "$screenshot" "$CURRENT_DIR/${name}.png"
    echo "  Captured: $CURRENT_DIR/${name}.png"

    return 0
}

# Compare scenario screenshot to baseline
# Args: name
# Returns: 0 = pass, 1 = fail, 2 = no baseline
compare_scenario() {
    local name="$1"
    local baseline="$BASELINE_DIR/${name}.png"
    local current="$CURRENT_DIR/${name}.png"
    local diff="$DIFF_DIR/${name}.png"

    if [ ! -f "$baseline" ]; then
        echo "  NO BASELINE: $baseline"
        return 2
    fi

    # Compare using ImageMagick if available
    if command -v compare &> /dev/null; then
        local diff_pixels
        diff_pixels=$(compare -metric AE -fuzz 5% "$baseline" "$current" "$diff" 2>&1 || true)

        # Parse the number
        if [[ "$diff_pixels" =~ ^[0-9]+$ ]]; then
            if [ "$diff_pixels" -le "$PIXEL_TOLERANCE" ]; then
                echo "  PASS: $diff_pixels pixels differ (tolerance: $PIXEL_TOLERANCE)"
                rm -f "$diff"
                return 0
            else
                echo "  FAIL: $diff_pixels pixels differ (tolerance: $PIXEL_TOLERANCE)"
                echo "  Diff: $diff"
                return 1
            fi
        else
            echo "  WARNING: Could not parse diff result"
        fi
    fi

    # Fallback: byte comparison
    if cmp -s "$baseline" "$current"; then
        echo "  PASS: byte-identical"
        return 0
    fi

    echo "  REVIEW: Files differ (manual review needed)"
    return 1
}

# Update baseline for a scenario
# Args: name
update_baseline() {
    local name="$1"
    local current="$CURRENT_DIR/${name}.png"
    local baseline="$BASELINE_DIR/${name}.png"

    if [ -f "$current" ]; then
        cp "$current" "$baseline"
        echo "  Updated: $baseline"
    else
        echo "  ERROR: No current screenshot for $name"
        return 1
    fi
}

# Main execution
main() {
    # Check scenario file exists
    if [ ! -f "$SCENARIO_FILE" ]; then
        echo "ERROR: Scenario file not found: $SCENARIO_FILE"
        exit 2
    fi

    # List mode
    if [[ -n "$LIST_MODE" ]]; then
        list_scenarios
        exit 0
    fi

    # Build once
    echo "Building..."
    cargo build --quiet 2>/dev/null

    # Parse scenarios (macOS compatible - no mapfile)
    local scenarios=()
    while IFS= read -r line; do
        scenarios+=("$line")
    done < <(parse_scenarios)

    if [ ${#scenarios[@]} -eq 0 ]; then
        echo "ERROR: No scenarios found in $SCENARIO_FILE"
        exit 2
    fi

    # Filter to single scenario if specified
    if [[ -n "$SINGLE_SCENARIO" ]]; then
        local found=0
        local filtered_scenarios=()
        for s in "${scenarios[@]}"; do
            IFS='|' read -r name rest <<< "$s"
            if [[ "$name" == "$SINGLE_SCENARIO" ]]; then
                filtered_scenarios+=("$s")
                found=1
                break
            fi
        done
        if [ $found -eq 0 ]; then
            echo "ERROR: Scenario not found: $SINGLE_SCENARIO"
            echo "Use --list to see available scenarios"
            exit 2
        fi
        scenarios=("${filtered_scenarios[@]}")
    fi

    local total=${#scenarios[@]}
    local passed=0
    local failed=0
    local no_baseline=0
    local errors=0

    echo ""
    echo "Running $total scenario(s)..."
    echo ""

    for scenario in "${scenarios[@]}"; do
        IFS='|' read -r name level palette width height wait_frames desc <<< "$scenario"

        # Run scenario
        if ! run_scenario "$name" "$level" "$palette" "$width" "$height" "$wait_frames" "$desc"; then
            ((errors++))
            echo ""
            continue
        fi

        if [[ -n "$UPDATE_MODE" ]]; then
            # Update mode: copy current to baseline
            update_baseline "$name"
        else
            # Compare mode: check against baseline
            local result
            compare_scenario "$name"
            result=$?
            if [ $result -eq 0 ]; then
                ((passed++))
            elif [ $result -eq 1 ]; then
                ((failed++))
            else
                ((no_baseline++))
            fi
        fi

        echo ""
    done

    # Summary
    echo "==============================="
    if [[ -n "$UPDATE_MODE" ]]; then
        echo "Baselines updated: $total"
        exit 0
    else
        echo "Results: $passed passed, $failed failed, $no_baseline missing baselines, $errors errors"
        echo ""
        echo "Baselines: $BASELINE_DIR/"
        echo "Current:   $CURRENT_DIR/"
        echo "Diffs:     $DIFF_DIR/"

        if [ $failed -gt 0 ] || [ $errors -gt 0 ]; then
            echo ""
            echo "To update baselines: ./scripts/regression.sh --update"
            exit 1
        elif [ $no_baseline -gt 0 ]; then
            echo ""
            echo "Missing baselines. Run with --update to create them."
            exit 2
        else
            exit 0
        fi
    fi
}

main
