#!/bin/bash
# Generate code coverage report using cargo-llvm-cov
#
# Prerequisites:
#   cargo install cargo-llvm-cov
#   rustup component add llvm-tools-preview
#
# Usage:
#   ./scripts/coverage.sh           # Full coverage report
#   ./scripts/coverage.sh --quick   # Unit tests only (faster)
#   ./scripts/coverage.sh --html    # Generate HTML report

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
COVERAGE_DIR="$PROJECT_DIR/coverage"

# Parse arguments
QUICK_MODE=false
HTML_MODE=false
for arg in "$@"; do
    case $arg in
        --quick)
            QUICK_MODE=true
            ;;
        --html)
            HTML_MODE=true
            ;;
        *)
            echo "Unknown argument: $arg"
            echo "Usage: $0 [--quick] [--html]"
            exit 1
            ;;
    esac
done

# Check if cargo-llvm-cov is installed
if ! command -v cargo-llvm-cov &> /dev/null; then
    echo "Error: cargo-llvm-cov not installed"
    echo "Install with: cargo install cargo-llvm-cov"
    echo "Also run: rustup component add llvm-tools-preview"
    exit 1
fi

cd "$PROJECT_DIR"

echo "=== Code Coverage Report ==="
echo ""

# Clean previous coverage data
echo "Cleaning previous coverage data..."
cargo llvm-cov clean --workspace 2>/dev/null || true

# Create coverage directory
mkdir -p "$COVERAGE_DIR"

if [ "$QUICK_MODE" = true ]; then
    echo "Running unit tests with coverage (quick mode)..."
    if [ "$HTML_MODE" = true ]; then
        cargo llvm-cov --lib --html --output-dir "$COVERAGE_DIR"
    else
        cargo llvm-cov --lib
    fi
else
    echo "Running full test suite with coverage..."

    # Export coverage environment
    eval "$(cargo llvm-cov show-env --export-prefix)"

    # Build with coverage instrumentation
    cargo build --lib 2>/dev/null
    cargo build --bin test-scenarios 2>/dev/null

    # Run unit tests
    echo ""
    echo "Running unit tests..."
    cargo test --lib 2>&1 | tail -5

    # Run scenario tests
    echo ""
    echo "Running scenario tests..."
    cargo run --bin test-scenarios 2>&1 | tail -10

    # Generate report
    echo ""
    echo "Generating coverage report..."
    if [ "$HTML_MODE" = true ]; then
        cargo llvm-cov report --html --output-dir "$COVERAGE_DIR"
        echo ""
        echo "HTML report: $COVERAGE_DIR/html/index.html"
    else
        cargo llvm-cov report
    fi
fi

echo ""
echo "=== Coverage Summary ==="

# Show per-module summary if lcov format available
if [ "$HTML_MODE" = true ]; then
    echo "View detailed report at: $COVERAGE_DIR/html/index.html"
else
    echo ""
    echo "Key modules to monitor:"
    echo "  src/ball/      - Ball physics (target: 70%)"
    echo "  src/shooting/  - Shot mechanics (target: 70%)"
    echo "  src/ai/        - AI decision system (target: 60%)"
    echo "  src/simulation/ - Headless sim (target: 50%)"
    echo ""
    echo "Run with --html for detailed per-file breakdown"
fi
