#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "=== Ballgame Setup ==="

# 1. Create all directories
echo "Creating directories..."
mkdir -p logs training_logs sim_logs db ghost_trials
mkdir -p showcase/heatmaps showcase/heatmaps/overlays
mkdir -p showcase/snapshots
mkdir -p showcase/regression/baselines showcase/regression/current showcase/regression/diffs

# 2. Copy config templates
if [[ ! -f config/training_settings.json && -f config/training_settings.template.json ]]; then
  cp config/training_settings.template.json config/training_settings.json
  echo "Created config/training_settings.json from template"
fi

if [[ ! -f config/simulation_settings.json && -f config/simulation_settings.template.json ]]; then
  cp config/simulation_settings.template.json config/simulation_settings.json
  echo "Created config/simulation_settings.json from template"
fi

# 3. Verify required config files exist
echo "Checking required config files..."
MISSING=0
for f in levels.txt palettes.txt ai_profiles.txt game_presets.txt ball_options.txt gameplay_tuning.json; do
  if [[ ! -f "config/$f" ]]; then
    echo "  ERROR: Missing config/$f"
    MISSING=1
  fi
done
if [[ $MISSING -eq 1 ]]; then
  echo "Cannot continue - required config files missing"
  exit 1
fi
echo "  All required config files present"

# 4. Build all binaries
echo "Building all binaries..."
cargo build --release

# 5. Generate ball textures (required for game to run)
echo "Generating ball textures..."
cargo run --release --bin generate -- ball

# 6. Generate heatmaps (takes a few minutes)
echo "Generating heatmaps..."
cargo run --release --bin heatmap -- --full --refresh

# 7. Run tests to verify setup
echo "Running unit tests..."
cargo test

echo "Running scenario tests..."
cargo run --release --bin test-scenarios

# 8. Check for optional tools
echo ""
echo "=== Optional Tool Check ==="
if command -v magick &>/dev/null || command -v convert &>/dev/null; then
  echo "  ImageMagick: installed (visual regression diffs available)"
else
  echo "  ImageMagick: not found (install for visual regression diffs)"
fi

if cargo llvm-cov --version &>/dev/null 2>&1; then
  echo "  cargo-llvm-cov: installed (coverage reports available)"
else
  echo "  cargo-llvm-cov: not found (install with: cargo install cargo-llvm-cov)"
fi

# Verify reachability heatmaps are valid
echo "Verifying reachability heatmaps..."
cargo run --bin verify_reachability

echo ""
echo "=== Setup Complete ==="
echo "Run 'cargo run' to start the game"
