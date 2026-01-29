#!/bin/bash
# Analyze offline training DBs - merge and generate combined report
#
# Usage:
#   ./offline_training/analyze_offline.sh              # Use default db_list.txt
#   ./offline_training/analyze_offline.sh my_list.txt  # Use custom list

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
cd "$ROOT_DIR"

LIST_FILE="${1:-offline_training/db_list.txt}"
COMBINED_DB="db/combined_offline_training.db"

if [ ! -f "$LIST_FILE" ]; then
    echo "Error: List file not found: $LIST_FILE"
    exit 1
fi

echo "=== Offline Training Analysis ==="
echo ""

# Step 1: Calculate training time
echo "--- Training Time ---"
python3 offline_training/calc_training_minutes.py --list "$LIST_FILE"
echo ""

# Step 2: Merge DBs
echo "--- Merging DBs ---"
python3 offline_training/merge_training_dbs.py --list "$LIST_FILE" --out "$COMBINED_DB"
echo ""

# Step 3: Analyze combined DB
echo "--- Running Analysis ---"
cargo run --bin analyze -- --training-db "$COMBINED_DB"
echo ""

echo "=== Done ==="
