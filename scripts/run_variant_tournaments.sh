#!/usr/bin/env bash
set -euo pipefail

BASELINE_DB=${1:-db/tournament_20260127_125125.db}

python3 scripts/run_variant_tournaments.py \
  --baseline-db "$BASELINE_DB" \
  --matches-per-pair 2 \
  --parallel 16 \
  --levels-from-sim-settings \
  --skip-heatmaps
