# Tuning Workflows

This doc condenses the tuning methodology into three repeatable workflows. Each workflow has a
prereq stage that regenerates level/profile/game data (heatmaps, caches, configs) before running
simulations and analysis.

## Common Prereq: Regenerate Required Data
Run before any tuning workflow.

Checklist
- Heatmaps are generated for all levels involved in the tuning run.
- Any AI caches tied to level geometry are refreshed.
- Config files used by the run are current and on disk.

Suggested commands (adjust level/profile lists as needed)
- Heatmaps (reachability-dependent types skipped, specific levels): `cargo run --release --bin heatmap -- --type speed --check --refresh --level "Open Floor" --level "Skyway"` (repeat for score, landing_safety, line_of_sight, elevation)
- Heatmaps (levels from sim settings): `python3 scripts/run_variant_tournaments.py --baseline-db <DB> --levels-from-sim-settings`
- Level data validation: `cargo run --bin generate_level_showcase` (optional)
- Config reload sanity check: `cargo run --bin simulate -- --matches 1 --quiet`

Output
- Heatmap files for each tuned level.
- Logs in `logs/` or `training_logs/` if enabled.

---

## Workflow A: Level-Specific Tuning
Goal: Tune per-level behavior and overrides (heatmap weights, LOS thresholds, etc.).

Inputs
- Level list: `config/levels.txt`
- Level overrides: per-level fields in level data
- Variant set: level overrides in `config/heatmap_variants.json`

Steps
1. Prereq: regenerate heatmaps for the target levels (use `--levels-from-sim-settings` or `--heatmap-levels`), or use `--skip-heatmaps` if already fresh.
2. Apply level-specific variant (constants + per-level fields).
3. Run short tournament with top 4 profiles.
4. Run focused analysis + event audit vs baseline.
5. Aggregate results and rank variants.

Outputs
- Tournament DBs per variant.
- Focused analysis reports.
- Event audit reports.
- Summary report with top 3 variants.

---

## Workflow B: Global Game Engine Tuning
Goal: Tune global gameplay settings (physics + shot system) that affect all levels.

Inputs
- Global tuning file: `config/gameplay_tuning.json`
- Baseline: latest baseline DB.
- Variant set: candidate values in a global tuning variants file.

Steps
1. Prereq: regenerate heatmaps (gameplay changes can alter reachability) using level targeting, or use `--skip-heatmaps` if already fresh.
2. Apply global tuning variant (gameplay tuning config).
3. Run short tournament with top 4 profiles.
4. Run focused analysis + event audit vs baseline.
5. Aggregate results and rank variants.

Outputs
- Tournament DBs per variant.
- Focused analysis reports.
- Event audit reports.
- Summary report with top 3 variants.

---

## Workflow C: AI Profile Tuning
Goal: Tune AI profile parameters (min_shot_quality, seek_threshold, charge timings, etc.).

Inputs
- AI profiles file: `config/ai_profiles.txt`
- Variant set: profile deltas in a structured variants file.

Steps
1. Prereq: regenerate heatmaps for target levels using level targeting, or use `--skip-heatmaps` if already fresh.
2. Apply profile variant deltas to top profiles.
3. Run short tournament with top 4 profiles.
4. Run focused analysis + event audit vs baseline.
5. Aggregate results and rank variants.

Outputs
- Tournament DBs per variant.
- Focused analysis reports.
- Event audit reports.
- Summary report with top 3 variants.

---

## Notes
- Always keep a recent baseline DB for the same profile list and match count.
- Use `notes/analysis_runs/` to store summary and audit reports.
- If the top 3 variants are close, run pairwise audits between them.
- The automation script generates heatmaps once at the start of the full tournament sweep; pass `--skip-heatmaps` when you explicitly want to reuse existing heatmaps.
