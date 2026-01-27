# Variant Tournament Summary

Baseline DB: `db/tournament_20260127_125125.db`

Profiles: v4_Pat_50, v1_Rusher, v2_Rusher, v2_Balanced_Steady

Matches per pair: 2

Parallel: 16


## Variants

### P0 (base)

DB: `db/tournament_20260127_134249.db`

Focused report: `notes/analysis_runs/focused_20260127_134301.md`

Event audit: `notes/analysis_runs/event_audit_20260127_134303.md`

Constants:

- HEATMAP_SCORE_WEIGHT_DEFAULT: 0.4
- HEATMAP_LOS_THRESHOLD_DEFAULT: 0.68
- HEATMAP_LOS_MARGIN_DEFAULT: 0.12
Profile deltas:

- charge_min: -0.25
- charge_max: -0.2
- min_shot_quality: -0.05
- seek_threshold: 0.08
Summary:

- Matches: 24.0
- Avg duration: 52.43
- Goals/match: 1.25
- Shots/match: 10.25
- Shot%: 0.11
- Avg shot quality: 0.402
- Scoreless rate: 0.25
- Steal success rate: 0.266


## Top 3 Variants (by goals/match, shots/match, scoreless rate)

- P0: goals 1.250, shots 10.250, scoreless 0.250

## Pairwise Deltas (Top 3)


## Suggestions

- Favor variants that increase shots/match without dropping shot% below baseline.
- If scoreless rate rises above 0.20, raise min_shot_quality or tighten LOS threshold.
- For higher tempo, reduce position_patience and seek_threshold, but watch steal attempts.
