# Tools

Offline tooling for analysis, generation, and training workflows.

## Structure

```
tools/
├── offline/           # Offline training scripts
│   ├── check_offline_training.py   # Check/run offline training
│   ├── merge_training_dbs.py       # Merge training databases
│   ├── calc_training_minutes.py    # Calculate training time
│   ├── offline_levels.txt          # Levels for offline training
│   └── manual_todo.md              # Offline training notes
│
├── analysis/          # Analysis and tournament scripts
│   ├── run_variant_tournaments.py  # Run variant tournaments
│   └── run_variant_tournaments.sh  # Shell wrapper
│
└── config/            # Analysis configuration (separate from game config)
    ├── heatmap_variants.json       # Heatmap variant definitions
    └── heatmap_variants_*.json     # Alternative variant sets
```

## Offline Training

```bash
# Check offline training status
python tools/offline/check_offline_training.py

# Merge multiple training databases
python tools/offline/merge_training_dbs.py db1.db db2.db --output merged.db

# Calculate training time
python tools/offline/calc_training_minutes.py db/training.db
```

## Variant Tournaments

```bash
# Run variant tournaments with baseline comparison
python tools/analysis/run_variant_tournaments.py \
    --baseline-db baseline.db \
    --variants tools/config/heatmap_variants.json \
    --matches-per-pair 5
```

## Asset Generation

Asset generation is handled by the unified `generate` binary:

```bash
cargo run --bin generate ball        # Ball textures
cargo run --bin generate showcase    # Ball styles image
cargo run --bin generate levels      # Level showcase grid
cargo run --bin generate gif wedge   # Wedge rotation GIF
cargo run --bin generate gif baseball # Baseball rotation GIF
```
