# Variant Tournament Summary

Baseline DB: `db/tournament_20260127_125125.db`

Profiles: v4_Pat_50, v1_Rusher, v2_Rusher, v2_Balanced_Steady

Matches per pair: 2

Parallel: 16


## Variants

### M4 (endpoint)

DB: `db/tournament_20260127_160441.db`

Focused report: `notes/analysis_runs/focused_20260127_160451.md`

Event audit: `notes/analysis_runs/event_audit_20260127_160453.md`

Constants:

- HEATMAP_SCORE_WEIGHT_DEFAULT: 0.9
- HEATMAP_LOS_THRESHOLD_DEFAULT: 0.5
- HEATMAP_LOS_MARGIN_DEFAULT: 0.3
Profile deltas:

- position_patience: -1.2
- seek_threshold: 0.35
- min_shot_quality: -0.15
Summary:

- Matches: 24.0
- Avg duration: 53.69
- Goals/match: 1.833
- Shots/match: 10.25
- Shot%: 0.159
- Avg shot quality: 0.414
- Scoreless rate: 0.083
- Steal success rate: 0.263


### J1 (endpoint)

DB: `db/tournament_20260127_160512.db`

Focused report: `notes/analysis_runs/focused_20260127_160520.md`

Event audit: `notes/analysis_runs/event_audit_20260127_160522.md`

Constants:

- HEATMAP_SCORE_WEIGHT_DEFAULT: 0.05
- HEATMAP_LOS_THRESHOLD_DEFAULT: 0.58
- HEATMAP_LOS_MARGIN_DEFAULT: 0.22
Profile deltas:

- min_shot_quality: -0.08
- seek_threshold: 0.12
- position_patience: -0.3
Summary:

- Matches: 24.0
- Avg duration: 54.68
- Goals/match: 2.583
- Shots/match: 10.167
- Shot%: 0.242
- Avg shot quality: 0.415
- Scoreless rate: 0.083
- Steal success rate: 0.273


### H1 (hybrid_high_weight_loose_los)

DB: `db/tournament_20260127_160540.db`

Focused report: `notes/analysis_runs/focused_20260127_160548.md`

Event audit: `notes/analysis_runs/event_audit_20260127_160550.md`

Constants:

- HEATMAP_SCORE_WEIGHT_DEFAULT: 0.8
- HEATMAP_LOS_THRESHOLD_DEFAULT: 0.58
- HEATMAP_LOS_MARGIN_DEFAULT: 0.24
Profile deltas:

- position_patience: -0.9
- seek_threshold: 0.25
- min_shot_quality: -0.1
Summary:

- Matches: 24.0
- Avg duration: 49.09
- Goals/match: 2.0
- Shots/match: 9.25
- Shot%: 0.203
- Avg shot quality: 0.414
- Scoreless rate: 0.042
- Steal success rate: 0.254


### H2 (hybrid_mid_weight_tighter_los)

DB: `db/tournament_20260127_160608.db`

Focused report: `notes/analysis_runs/focused_20260127_160616.md`

Event audit: `notes/analysis_runs/event_audit_20260127_160617.md`

Constants:

- HEATMAP_SCORE_WEIGHT_DEFAULT: 0.7
- HEATMAP_LOS_THRESHOLD_DEFAULT: 0.65
- HEATMAP_LOS_MARGIN_DEFAULT: 0.2
Profile deltas:

- position_patience: -0.8
- seek_threshold: 0.22
- min_shot_quality: -0.08
Summary:

- Matches: 24.0
- Avg duration: 54.08
- Goals/match: 2.125
- Shots/match: 9.625
- Shot%: 0.199
- Avg shot quality: 0.426
- Scoreless rate: 0.125
- Steal success rate: 0.25


### H3 (hybrid_mid_weight_looser_margin)

DB: `db/tournament_20260127_160635.db`

Focused report: `notes/analysis_runs/focused_20260127_160643.md`

Event audit: `notes/analysis_runs/event_audit_20260127_160645.md`

Constants:

- HEATMAP_SCORE_WEIGHT_DEFAULT: 0.75
- HEATMAP_LOS_THRESHOLD_DEFAULT: 0.6
- HEATMAP_LOS_MARGIN_DEFAULT: 0.28
Profile deltas:

- position_patience: -1.0
- seek_threshold: 0.28
- min_shot_quality: -0.12
Summary:

- Matches: 24.0
- Avg duration: 53.19
- Goals/match: 2.25
- Shots/match: 10.667
- Shot%: 0.191
- Avg shot quality: 0.404
- Scoreless rate: 0.042
- Steal success rate: 0.289


### H4 (hybrid_lower_weight_strict_los)

DB: `db/tournament_20260127_160703.db`

Focused report: `notes/analysis_runs/focused_20260127_160711.md`

Event audit: `notes/analysis_runs/event_audit_20260127_160713.md`

Constants:

- HEATMAP_SCORE_WEIGHT_DEFAULT: 0.6
- HEATMAP_LOS_THRESHOLD_DEFAULT: 0.7
- HEATMAP_LOS_MARGIN_DEFAULT: 0.18
Profile deltas:

- position_patience: -0.6
- seek_threshold: 0.18
- min_shot_quality: -0.06
Summary:

- Matches: 24.0
- Avg duration: 48.79
- Goals/match: 2.667
- Shots/match: 9.625
- Shot%: 0.255
- Avg shot quality: 0.402
- Scoreless rate: 0.042
- Steal success rate: 0.268


## Top 3 Variants (by goals/match, shots/match, scoreless rate)

- H4: goals 2.667, shots 9.625, scoreless 0.042
- J1: goals 2.583, shots 10.167, scoreless 0.083
- H3: goals 2.250, shots 10.667, scoreless 0.042

## Pairwise Deltas (Top 3)

- H4 vs J1: `notes/analysis_runs/event_audit_20260127_160715.md`
- H4 vs H3: `notes/analysis_runs/event_audit_20260127_160717.md`
- J1 vs H3: `notes/analysis_runs/event_audit_20260127_160718.md`

## Suggestions

- Favor variants that increase shots/match without dropping shot% below baseline.
- If scoreless rate rises above 0.20, raise min_shot_quality or tighten LOS threshold.
- For higher tempo, reduce position_patience and seek_threshold, but watch steal attempts.
