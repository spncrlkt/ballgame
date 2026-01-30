#!/bin/bash
# Generate metrics trend report from SQLite
#
# Usage:
#   ./scripts/metrics_report.sh           # Show last 10 commits
#   ./scripts/metrics_report.sh --all     # Show all history
#   ./scripts/metrics_report.sh --csv     # Export as CSV

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
DB="$PROJECT_DIR/db/audit_metrics.db"

# Parse arguments
SHOW_ALL=false
CSV_MODE=false
for arg in "$@"; do
    case $arg in
        --all)
            SHOW_ALL=true
            ;;
        --csv)
            CSV_MODE=true
            ;;
    esac
done

# Check if database exists
if [ ! -f "$DB" ]; then
    echo "Error: Metrics database not found at $DB"
    echo "Run ./scripts/collect_metrics.sh first"
    exit 1
fi

if [ "$CSV_MODE" = true ]; then
    # Export as CSV
    sqlite3 -header -csv "$DB" "SELECT * FROM audit_metrics ORDER BY timestamp DESC;"
    exit 0
fi

echo "╔══════════════════════════════════════════════════════════════════════════════╗"
echo "║                          AUDIT METRICS TREND REPORT                          ║"
echo "╚══════════════════════════════════════════════════════════════════════════════╝"
echo ""

# Limit clause
if [ "$SHOW_ALL" = true ]; then
    LIMIT_CLAUSE=""
else
    LIMIT_CLAUSE="LIMIT 10"
fi

echo "=== Test Results by Commit ==="
echo ""
sqlite3 -header -column "$DB" "
SELECT
    git_commit,
    MAX(CASE WHEN metric_name='unit_tests_passed' THEN CAST(metric_value AS INTEGER) END) as unit_pass,
    MAX(CASE WHEN metric_name='unit_tests_failed' THEN CAST(metric_value AS INTEGER) END) as unit_fail,
    MAX(CASE WHEN metric_name='scenario_tests_passed' THEN CAST(metric_value AS INTEGER) END) as scn_pass,
    MAX(CASE WHEN metric_name='scenario_tests_failed' THEN CAST(metric_value AS INTEGER) END) as scn_fail,
    MAX(CASE WHEN metric_name='clippy_warnings' THEN CAST(metric_value AS INTEGER) END) as clippy,
    substr(timestamp, 1, 10) as date
FROM audit_metrics
GROUP BY git_commit
ORDER BY timestamp DESC
$LIMIT_CLAUSE;
"

echo ""
echo "=== Code Quality Trends ==="
echo ""
sqlite3 -header -column "$DB" "
SELECT
    git_commit,
    MAX(CASE WHEN metric_name='rust_lines_of_code' THEN CAST(metric_value AS INTEGER) END) as rust_loc,
    MAX(CASE WHEN metric_name='todo_comments' THEN CAST(metric_value AS INTEGER) END) as todos,
    MAX(CASE WHEN metric_name='fixme_comments' THEN CAST(metric_value AS INTEGER) END) as fixmes,
    substr(timestamp, 1, 10) as date
FROM audit_metrics
GROUP BY git_commit
ORDER BY timestamp DESC
$LIMIT_CLAUSE;
"

echo ""
echo "=== Shot Accuracy (Level 3) ==="
echo ""
sqlite3 -header -column "$DB" "
SELECT
    git_commit,
    printf('%.1f%%', MAX(CASE WHEN metric_name='shot_accuracy_level3' THEN metric_value END)) as accuracy,
    printf('%.1f%%', MAX(CASE WHEN metric_name='shot_over_under_level3' THEN metric_value END)) as over_under,
    substr(timestamp, 1, 10) as date
FROM audit_metrics
WHERE metric_name IN ('shot_accuracy_level3', 'shot_over_under_level3')
GROUP BY git_commit
ORDER BY timestamp DESC
$LIMIT_CLAUSE;
"

echo ""
echo "=== Regressions Detected ==="
echo "(Metric decreased from previous commit)"
echo ""
sqlite3 "$DB" "
WITH ranked AS (
    SELECT
        *,
        LAG(metric_value) OVER (PARTITION BY metric_name ORDER BY timestamp) as prev_value
    FROM audit_metrics
    WHERE metric_name IN ('unit_tests_passed', 'scenario_tests_passed', 'shot_accuracy_level3')
)
SELECT
    substr(timestamp, 1, 10) as date,
    git_commit,
    metric_name as metric,
    CAST(prev_value AS INTEGER) || ' -> ' || CAST(metric_value AS INTEGER) as change
FROM ranked
WHERE metric_value < prev_value
ORDER BY timestamp DESC
LIMIT 10;
" || echo "(none detected)"

echo ""
echo "=== Summary Statistics ==="
echo ""

# Total metrics collected
TOTAL_METRICS=$(sqlite3 "$DB" "SELECT COUNT(*) FROM audit_metrics;")
TOTAL_COMMITS=$(sqlite3 "$DB" "SELECT COUNT(DISTINCT git_commit) FROM audit_metrics;")
FIRST_DATE=$(sqlite3 "$DB" "SELECT MIN(substr(timestamp, 1, 10)) FROM audit_metrics;")
LAST_DATE=$(sqlite3 "$DB" "SELECT MAX(substr(timestamp, 1, 10)) FROM audit_metrics;")

echo "Total metrics collected: $TOTAL_METRICS"
echo "Unique commits tracked:  $TOTAL_COMMITS"
echo "Date range:              $FIRST_DATE to $LAST_DATE"
echo ""
echo "Run with --all to see full history"
echo "Run with --csv to export data"
