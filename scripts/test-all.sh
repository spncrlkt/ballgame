#!/bin/bash
# Unified test runner - runs both scenario tests and visual regression
# Usage: ./scripts/test-all.sh [options]
#   -v, --verbose    Show detailed output
#   -s, --scenarios  Run only scenario tests
#   -r, --regression Run only visual regression

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

VERBOSE=false
RUN_SCENARIOS=true
RUN_REGRESSION=true

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        -s|--scenarios)
            RUN_REGRESSION=false
            shift
            ;;
        -r|--regression)
            RUN_SCENARIOS=false
            shift
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: ./scripts/test-all.sh [-v|--verbose] [-s|--scenarios] [-r|--regression]"
            exit 1
            ;;
    esac
done

SCENARIO_STATUS=0
REGRESSION_STATUS=0

echo "========================================"
echo "         Running All Tests"
echo "========================================"
echo ""

# Run scenario tests
if [ "$RUN_SCENARIOS" = true ]; then
    echo -e "${YELLOW}=== Scenario Tests ===${NC}"
    echo ""

    if [ "$VERBOSE" = true ]; then
        cargo run --bin test-scenarios -- -v || SCENARIO_STATUS=$?
    else
        cargo run --bin test-scenarios || SCENARIO_STATUS=$?
    fi

    echo ""
fi

# Run visual regression
if [ "$RUN_REGRESSION" = true ]; then
    echo -e "${YELLOW}=== Visual Regression ===${NC}"
    echo ""

    ./scripts/regression.sh || REGRESSION_STATUS=$?

    echo ""
fi

# Summary
echo "========================================"
echo "              Summary"
echo "========================================"

if [ "$RUN_SCENARIOS" = true ]; then
    if [ $SCENARIO_STATUS -eq 0 ]; then
        echo -e "Scenario Tests:     ${GREEN}PASS${NC}"
    else
        echo -e "Scenario Tests:     ${RED}FAIL${NC}"
    fi
fi

if [ "$RUN_REGRESSION" = true ]; then
    if [ $REGRESSION_STATUS -eq 0 ]; then
        echo -e "Visual Regression:  ${GREEN}PASS${NC}"
    else
        echo -e "Visual Regression:  ${RED}FAIL${NC} (or REVIEW needed)"
    fi
fi

echo "========================================"

# Exit with failure if any test failed
if [ $SCENARIO_STATUS -ne 0 ] || [ $REGRESSION_STATUS -ne 0 ]; then
    exit 1
fi
