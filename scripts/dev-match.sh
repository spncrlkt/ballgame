#!/bin/bash
# Start server + AI clients in one command
# Usage: ./scripts/dev-match.sh [--human] [--v1] [--v2]
#
# Examples:
#   ./scripts/dev-match.sh                  # Server + both AIs (no human)
#   ./scripts/dev-match.sh --human          # Server with local player + both AIs
#   ./scripts/dev-match.sh --human --v1     # Server with local player + AI v1 only
#   ./scripts/dev-match.sh --v1 --v2        # Server + both AIs (explicit)

set -e

PORT=${PORT:-9000}
SERVER_URL="ws://localhost:$PORT"

echo "=== Ballgame Dev Match ==="
echo "Port: $PORT"

# Build all crates first
echo "Building..."
cargo build --release -p ballgame -p ballgame-ai-v1 2>&1 | tail -5

# Trap to cleanup background processes on exit
cleanup() {
    echo ""
    echo "Shutting down..."
    kill $(jobs -p) 2>/dev/null || true
    wait 2>/dev/null || true
}
trap cleanup EXIT INT TERM

# Start server (with or without local player)
if [[ "$*" == *"--human"* ]]; then
    echo "Starting server with local player on slot 0..."
    cargo run --release -p ballgame -- --server --port $PORT --local-slot 0 &
else
    echo "Starting headless server..."
    cargo run --release -p ballgame -- --server --port $PORT &
fi
SERVER_PID=$!

# Wait for server to start
sleep 2

# Check if server is running
if ! kill -0 $SERVER_PID 2>/dev/null; then
    echo "ERROR: Server failed to start"
    exit 1
fi

# Start AI clients based on flags
# Default: start enough AIs for a 2v2 match
if [[ "$*" != *"--v1"* ]] && [[ "$*" != *"--v2"* ]] && [[ "$*" != *"--human"* ]]; then
    # No flags: 4 AIs for full 2v2
    echo "Starting 4 AI players..."
    cargo run --release -p ballgame-ai-v1 -- --server $SERVER_URL --name "AI-1" &
    cargo run --release -p ballgame-ai-v1 -- --server $SERVER_URL --name "AI-2" &
    cargo run --release -p ballgame-ai-v1 -- --server $SERVER_URL --name "AI-3" &
    cargo run --release -p ballgame-ai-v1 -- --server $SERVER_URL --name "AI-4" &
elif [[ "$*" == *"--human"* ]] && [[ "$*" != *"--v1"* ]] && [[ "$*" != *"--v2"* ]]; then
    # Human flag only: 3 AIs to complete 2v2
    echo "Starting 3 AI players..."
    cargo run --release -p ballgame-ai-v1 -- --server $SERVER_URL --name "AI-1" &
    cargo run --release -p ballgame-ai-v1 -- --server $SERVER_URL --name "AI-2" &
    cargo run --release -p ballgame-ai-v1 -- --server $SERVER_URL --name "AI-3" &
else
    # Specific flags provided
    if [[ "$*" == *"--v1"* ]]; then
        echo "Starting AI v1..."
        cargo run --release -p ballgame-ai-v1 -- --server $SERVER_URL &
    fi

    if [[ "$*" == *"--v2"* ]]; then
        echo "Starting AI v2..."
        cargo run --release -p ballgame-ai-v1 -- --server $SERVER_URL --name "AI-v2" &
    fi
fi

echo ""
echo "Match running. Press Ctrl+C to stop."
echo ""

# Wait for server to exit
wait $SERVER_PID
