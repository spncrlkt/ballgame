#!/bin/bash
# Quick screenshot script - runs game, takes startup screenshot, exits
# Usage: ./scripts/screenshot.sh [--quiet]
#
# The game automatically exits after capturing the startup screenshot.
# Returns the screenshot path on stdout (last line).

set -e

# Change to project root
cd "$(dirname "$0")/.."

QUIET=""
if [[ "$1" == "--quiet" ]]; then
    QUIET="1"
fi

# Clean old snapshots
rm -rf showcase/snapshots/

# Build if needed (suppress output in quiet mode)
if [[ -n "$QUIET" ]]; then
    cargo build --quiet 2>/dev/null
else
    cargo build --quiet
fi

# Run game with screenshot-and-quit flag
# The game will automatically exit after taking the startup screenshot
if [[ -n "$QUIET" ]]; then
    cargo run --quiet -- --screenshot-and-quit 2>/dev/null
else
    cargo run --quiet -- --screenshot-and-quit
fi

# Find and output the screenshot path
SCREENSHOT=$(ls -t showcase/snapshots/*startup*.png 2>/dev/null | head -1)
if [ -n "$SCREENSHOT" ]; then
    if [[ -z "$QUIET" ]]; then
        echo "Screenshot saved: $SCREENSHOT"
    fi
    # Always output just the path as the last line
    echo "$SCREENSHOT"
else
    echo "No screenshot captured" >&2
    exit 1
fi
