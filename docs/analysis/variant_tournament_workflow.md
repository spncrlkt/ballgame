# Variant Tournament Workflow

This workflow runs a short tournament for each heatmap variant, analyzes results, and produces
an aggregated summary report with pairwise deltas for the top 3 variants.

## Inputs
- Variants: `config/heatmap_variants.json`
- Baseline DB: `db/tournament_20260127_125125.db`
- Profiles: top 4 profiles in `config/ai_profiles.txt`
- Matches per pair: 2 (24 matches total)
- Parallel: 16

## Scripts
- `scripts/run_variant_tournaments.py`: end-to-end automation
- `scripts/run_variant_tournaments.sh`: convenience wrapper

## What it does
For each variant (J0–J4, M0–M4, P0–P4):
1. Apply variant constants in `src/constants.rs`.
2. Apply profile deltas to the top 4 profiles in `config/ai_profiles.txt`.
3. Run tournament with those 4 profiles.
4. Run focused analysis on the new DB.
5. Run event audit vs baseline DB.

After all variants:
- Rank top 3 variants by goals/match, shots/match, scoreless rate.
- Run pairwise event audits between top 3.
- Write a summary report under `notes/analysis_runs/`.
- Restore original constants + profile values.

## Run
```bash
# Default baseline DB
scripts/run_variant_tournaments.sh

# Explicit baseline DB
scripts/run_variant_tournaments.sh db/tournament_20260127_125125.db

# Skip heatmap regeneration (not recommended unless already fresh)
python3 scripts/run_variant_tournaments.py --baseline-db db/tournament_20260127_125125.db --skip-heatmaps

# Regenerate heatmaps for a subset of levels
python3 scripts/run_variant_tournaments.py --baseline-db db/tournament_20260127_125125.db --heatmap-levels "Open Floor,Skyway"

# Regenerate heatmaps for levels specified in config/simulation_settings.json (reachability skipped)
python3 scripts/run_variant_tournaments.py --baseline-db db/tournament_20260127_125125.db --levels-from-sim-settings
```

## Output
- Per-variant DBs: `db/tournament_YYYYMMDD_HHMMSS.db`
- Per-variant focused reports: `notes/analysis_runs/focused_*.md`
- Per-variant event audits: `notes/analysis_runs/event_audit_*.md`
- Final summary: `notes/analysis_runs/variant_tournament_summary_*.md`

## Notes
- Each variant modifies only the top 4 profiles (as ordered in `config/ai_profiles.txt`).
- If you want a different profile list, pass `--profiles` to the python script.
- If you want different run lengths, change `--matches-per-pair`.
