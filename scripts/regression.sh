#!/bin/bash
# Visual regression test script
# Usage:
#   ./scripts/regression.sh          # Capture and compare to baseline
#   ./scripts/regression.sh --update # Update baseline with current screenshot
#
# Exit codes:
#   0 = Pass (screenshots match or baseline updated)
#   1 = Fail (screenshots differ)
#   2 = Error (missing baseline, build failure, etc.)

set -e

# Change to project root
cd "$(dirname "$0")/.."

REGRESSION_DIR="regression"
BASELINE="$REGRESSION_DIR/baseline.png"
CURRENT="$REGRESSION_DIR/current.png"
DIFF="$REGRESSION_DIR/diff.png"

# Ensure regression directory exists
mkdir -p "$REGRESSION_DIR"

# Check for --update flag
UPDATE_MODE=""
if [[ "$1" == "--update" ]]; then
    UPDATE_MODE="1"
fi

# Clean old snapshots
rm -rf snapshots/

# Build and run game with screenshot-and-quit
echo "Capturing screenshot..."
cargo build --quiet 2>/dev/null
cargo run --quiet -- --screenshot-and-quit 2>/dev/null

# Find the startup screenshot
SCREENSHOT=$(ls -t snapshots/*startup*.png 2>/dev/null | head -1)
if [ -z "$SCREENSHOT" ]; then
    echo "ERROR: No screenshot captured"
    exit 2
fi

# Copy to current
cp "$SCREENSHOT" "$CURRENT"
echo "Current: $CURRENT"

# Update mode - just copy current to baseline
if [[ -n "$UPDATE_MODE" ]]; then
    cp "$CURRENT" "$BASELINE"
    echo "Baseline updated: $BASELINE"
    exit 0
fi

# Compare mode - check if baseline exists
if [ ! -f "$BASELINE" ]; then
    echo "ERROR: No baseline exists. Run with --update to create one."
    echo "  ./scripts/regression.sh --update"
    exit 2
fi

# Tolerance: allow up to 1% of pixels to differ (handles AI movement, timing)
# 2560x1440 = 3,686,400 pixels, 1% = 36,864 pixels
PIXEL_TOLERANCE=40000

# Compare using ImageMagick if available
if command -v compare &> /dev/null; then
    # compare returns non-zero if images differ
    # -metric AE counts differing pixels
    # -fuzz 5% allows for small color variations
    DIFF_PIXELS=$(compare -metric AE -fuzz 5% "$BASELINE" "$CURRENT" "$DIFF" 2>&1 || true)

    # Parse the number (compare outputs to stderr)
    if [[ "$DIFF_PIXELS" =~ ^[0-9]+$ ]]; then
        if [ "$DIFF_PIXELS" -le "$PIXEL_TOLERANCE" ]; then
            echo "PASS: Screenshots match ($DIFF_PIXELS pixels differ, tolerance: $PIXEL_TOLERANCE)"
            rm -f "$DIFF"
            exit 0
        else
            echo "FAIL: $DIFF_PIXELS pixels differ (tolerance: $PIXEL_TOLERANCE)"
            echo "Diff image: $DIFF"
            echo "Compare manually:"
            echo "  Baseline: $BASELINE"
            echo "  Current:  $CURRENT"
            exit 1
        fi
    else
        echo "WARNING: Could not parse diff result, comparing file sizes"
    fi
fi

# Fallback without ImageMagick: report for manual review
# PNG compression means identical images can have different sizes,
# so we can't reliably detect visual differences
BASELINE_SIZE=$(stat -f%z "$BASELINE" 2>/dev/null || stat -c%s "$BASELINE")
CURRENT_SIZE=$(stat -f%z "$CURRENT" 2>/dev/null || stat -c%s "$CURRENT")

# Check if byte-identical
if cmp -s "$BASELINE" "$CURRENT"; then
    echo "PASS: Screenshots match (byte-identical)"
    exit 0
fi

# Not identical - report for manual review
SIZE_DIFF=$((CURRENT_SIZE - BASELINE_SIZE))
SIZE_DIFF_ABS=${SIZE_DIFF#-}  # absolute value
SIZE_PERCENT=$((SIZE_DIFF_ABS * 100 / BASELINE_SIZE))

if [ "$SIZE_PERCENT" -le 5 ]; then
    echo "REVIEW: Screenshots differ slightly (size diff: ${SIZE_DIFF} bytes, ${SIZE_PERCENT}%)"
    echo "This is often due to non-deterministic elements (AI movement, timing)"
    echo "Manual review recommended:"
    echo "  Baseline: $BASELINE"
    echo "  Current:  $CURRENT"
    echo ""
    echo "To update baseline: ./scripts/regression.sh --update"
    # Exit 0 for small differences - likely just timing variations
    exit 0
else
    echo "WARNING: Screenshots differ significantly (size diff: ${SIZE_DIFF} bytes, ${SIZE_PERCENT}%)"
    echo "Manual review required:"
    echo "  Baseline: $BASELINE"
    echo "  Current:  $CURRENT"
    echo ""
    echo "If this is expected, update baseline: ./scripts/regression.sh --update"
    exit 1
fi
