#!/bin/bash
# Run N matches of AI v1 vs AI v2, log results
# Usage: ./scripts/ai-tournament.sh [num_matches]
#
# Examples:
#   ./scripts/ai-tournament.sh       # Run 5 matches (default)
#   ./scripts/ai-tournament.sh 10    # Run 10 matches

set -e

NUM_MATCHES=${1:-5}
PORT=9001
SERVER_URL="ws://localhost:$PORT"
RESULTS_FILE="tournament_results_$(date +%Y%m%d_%H%M%S).log"

echo "=== Ballgame AI Tournament ==="
echo "Matches: $NUM_MATCHES"
echo "Results: $RESULTS_FILE"
echo ""

# Build crates
echo "Building..."
cargo build --release -p ballgame -p ballgame-ai-v1 2>&1 | tail -3

# Initialize results
echo "# AI Tournament Results" > "$RESULTS_FILE"
echo "# Started: $(date)" >> "$RESULTS_FILE"
echo "# Matches: $NUM_MATCHES" >> "$RESULTS_FILE"
echo "" >> "$RESULTS_FILE"

V1_WINS=0
V2_WINS=0
TIES=0

for i in $(seq 1 $NUM_MATCHES); do
    echo "=== Match $i of $NUM_MATCHES ==="

    # Start server in tournament mode (auto-exits after match)
    cargo run --release -p ballgame -- --server --port $PORT --tournament &
    SERVER_PID=$!
    sleep 1

    # Check if server started
    if ! kill -0 $SERVER_PID 2>/dev/null; then
        echo "ERROR: Server failed to start for match $i"
        continue
    fi

    # Start both AIs
    cargo run --release -p ballgame-ai-v1 -- --server $SERVER_URL --name "AI-v1" &
    AI1_PID=$!

    cargo run --release -p ballgame-ai-v1 -- --server $SERVER_URL --name "AI-v2" &
    AI2_PID=$!

    # Wait for match to complete (server exits when match ends)
    wait $SERVER_PID 2>/dev/null || true

    # Kill AI clients if still running
    kill $AI1_PID 2>/dev/null || true
    kill $AI2_PID 2>/dev/null || true

    echo "Match $i complete"
    echo "" >> "$RESULTS_FILE"

    # Short pause between matches
    sleep 0.5
done

echo ""
echo "=== Tournament Complete ==="
echo "Results saved to: $RESULTS_FILE"
