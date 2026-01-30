#!/bin/bash
# Collect audit metrics and store in SQLite
#
# Usage:
#   ./scripts/collect_metrics.sh           # Collect all metrics
#   ./scripts/collect_metrics.sh --quick   # Skip slow tests (tournament)
#
# Output:
#   db/audit_metrics.db - SQLite database with historical metrics

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
DB="$PROJECT_DIR/db/audit_metrics.db"
COMMIT=$(git -C "$PROJECT_DIR" rev-parse --short HEAD 2>/dev/null || echo "unknown")
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

# Parse arguments
QUICK_MODE=false
for arg in "$@"; do
    case $arg in
        --quick)
            QUICK_MODE=true
            ;;
    esac
done

cd "$PROJECT_DIR"

# Ensure db directory exists
mkdir -p "$(dirname "$DB")"

# Initialize database if needed
sqlite3 "$DB" <<'EOF'
CREATE TABLE IF NOT EXISTS audit_metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    git_commit TEXT,
    metric_name TEXT NOT NULL,
    metric_value REAL NOT NULL,
    notes TEXT
);

CREATE INDEX IF NOT EXISTS idx_metrics_timestamp ON audit_metrics(timestamp);
CREATE INDEX IF NOT EXISTS idx_metrics_name ON audit_metrics(metric_name);
CREATE INDEX IF NOT EXISTS idx_metrics_commit ON audit_metrics(git_commit);
EOF

echo "=== Collecting Audit Metrics ==="
echo "Commit: $COMMIT"
echo "Timestamp: $TIMESTAMP"
echo ""

# Function to insert metric
insert_metric() {
    local name="$1"
    local value="$2"
    local notes="${3:-}"
    sqlite3 "$DB" "INSERT INTO audit_metrics (timestamp, git_commit, metric_name, metric_value, notes) VALUES ('$TIMESTAMP', '$COMMIT', '$name', $value, '$notes');"
    printf "  %-30s %s\n" "$name:" "$value"
}

# 1. Run unit tests
echo "Running unit tests..."
UNIT_OUTPUT=$(cargo test --lib 2>&1 || true)
UNIT_PASSED=$(echo "$UNIT_OUTPUT" | grep -oE '[0-9]+ passed' | grep -oE '[0-9]+' | head -1 || echo "0")
UNIT_FAILED=$(echo "$UNIT_OUTPUT" | grep -oE '[0-9]+ failed' | grep -oE '[0-9]+' | head -1 || echo "0")
insert_metric "unit_tests_passed" "${UNIT_PASSED:-0}"
insert_metric "unit_tests_failed" "${UNIT_FAILED:-0}"

# 2. Run scenario tests
echo "Running scenario tests..."
SCENARIO_OUTPUT=$(cargo run --bin test-scenarios 2>&1 || true)
SCENARIO_PASSED=$(echo "$SCENARIO_OUTPUT" | grep -oE 'Passed: [0-9]+' | grep -oE '[0-9]+' || echo "0")
SCENARIO_FAILED=$(echo "$SCENARIO_OUTPUT" | grep -oE 'Failed: [0-9]+' | grep -oE '[0-9]+' || echo "0")
SCENARIO_TOTAL=$(echo "$SCENARIO_OUTPUT" | grep -oE 'Total: [0-9]+' | grep -oE '[0-9]+' || echo "0")
insert_metric "scenario_tests_passed" "${SCENARIO_PASSED:-0}"
insert_metric "scenario_tests_failed" "${SCENARIO_FAILED:-0}"
insert_metric "scenario_tests_total" "${SCENARIO_TOTAL:-0}"

# 3. Run clippy
echo "Running clippy..."
CLIPPY_OUTPUT=$(cargo clippy 2>&1 || true)
# Count warnings (grep -c returns 0 with exit code 1 if no matches, handle gracefully)
CLIPPY_WARNINGS=$(echo "$CLIPPY_OUTPUT" | grep -c "warning:" || true)
CLIPPY_ERRORS=$(echo "$CLIPPY_OUTPUT" | grep -c 'error\[' || true)
# Default to 0 if empty
: "${CLIPPY_WARNINGS:=0}"
: "${CLIPPY_ERRORS:=0}"
insert_metric "clippy_warnings" "$CLIPPY_WARNINGS"
insert_metric "clippy_errors" "$CLIPPY_ERRORS"

# 4. Run shot accuracy test (if not quick mode)
if [ "$QUICK_MODE" = false ]; then
    echo "Running shot accuracy test..."
    SHOT_OUTPUT=$(cargo run --bin simulate -- --shot-test 30 --level 3 2>&1 || true)
    # Parse accuracy from output (format varies, try to find percentage)
    ACCURACY=$(echo "$SHOT_OUTPUT" | grep -oE 'accuracy[: ]+[0-9.]+' | grep -oE '[0-9.]+' | head -1 || echo "0")
    OVER_UNDER=$(echo "$SHOT_OUTPUT" | grep -oE 'over/under[: ]+[0-9.]+' | grep -oE '[0-9.]+' | head -1 || echo "0")
    insert_metric "shot_accuracy_level3" "${ACCURACY:-0}"
    insert_metric "shot_over_under_level3" "${OVER_UNDER:-0}"
else
    echo "Skipping shot test (--quick mode)"
fi

# 5. Count source lines of code
echo "Counting source code..."
RUST_LOC=$(find "$PROJECT_DIR/src" -name "*.rs" -exec cat {} \; | wc -l | tr -d ' ')
TEST_LOC=$(find "$PROJECT_DIR/tests" -name "*.toml" -exec cat {} \; | wc -l | tr -d ' ')
insert_metric "rust_lines_of_code" "${RUST_LOC:-0}"
insert_metric "test_scenario_lines" "${TEST_LOC:-0}"

# 6. Count TODO/FIXME comments
echo "Counting TODO/FIXME..."
TODO_COUNT=$(grep -r "TODO" "$PROJECT_DIR/src" 2>/dev/null | wc -l | tr -d ' ' || echo "0")
FIXME_COUNT=$(grep -r "FIXME" "$PROJECT_DIR/src" 2>/dev/null | wc -l | tr -d ' ' || echo "0")
insert_metric "todo_comments" "${TODO_COUNT:-0}"
insert_metric "fixme_comments" "${FIXME_COUNT:-0}"

echo ""
echo "=== Metrics Collected ==="
echo "Stored in: $DB"
echo ""
echo "View trends with: ./scripts/metrics_report.sh"
