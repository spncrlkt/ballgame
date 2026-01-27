#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

mkdir -p logs training_logs sim_logs showcase/heatmaps showcase/heatmaps/overlays

if [[ ! -f config/training_settings.json && -f config/training_settings.template.json ]]; then
  cp config/training_settings.template.json config/training_settings.json
  echo "Created config/training_settings.json from template"
fi

if [[ ! -f config/simulation_settings.json && -f config/simulation_settings.template.json ]]; then
  cp config/simulation_settings.template.json config/simulation_settings.json
  echo "Created config/simulation_settings.json from template"
fi

# Generate full heatmap bundle and overlays (per level)
# This can take several minutes on a clean machine.
cargo run --bin heatmap -- --full --refresh

