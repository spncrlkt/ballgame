# Variant Tournament Summary

Baseline DB: `db/tournament_20260127_125125.db`

Profiles: v4_Pat_50, v1_Rusher, v2_Rusher, v2_Balanced_Steady

Matches per pair: 2

Parallel: 16


## Variants

### J0 (base)

DB: `db/tournament_20260127_153542.db`

Focused report: `notes/analysis_runs/focused_20260127_153553.md`

Event audit: `notes/analysis_runs/event_audit_20260127_153555.md`

Constants:

- HEATMAP_SCORE_WEIGHT_DEFAULT: 0.0
- HEATMAP_LOS_THRESHOLD_DEFAULT: 0.55
- HEATMAP_LOS_MARGIN_DEFAULT: 0.25
Profile deltas:

- min_shot_quality: -0.1
- seek_threshold: 0.15
- position_patience: -0.35
Summary:

- Matches: 24.0
- Avg duration: 51.76
- Goals/match: 1.833
- Shots/match: 10.125
- Shot%: 0.156
- Avg shot quality: 0.412
- Scoreless rate: 0.167
- Steal success rate: 0.283


### J1 (close delta)

DB: `db/tournament_20260127_153614.db`

Focused report: `notes/analysis_runs/focused_20260127_153623.md`

Event audit: `notes/analysis_runs/event_audit_20260127_153624.md`

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
- Avg duration: 53.14
- Goals/match: 2.25
- Shots/match: 10.083
- Shot%: 0.207
- Avg shot quality: 0.4
- Scoreless rate: 0.042
- Steal success rate: 0.29


### J2 (close delta + big change in most significant vars)

DB: `db/tournament_20260127_153643.db`

Focused report: `notes/analysis_runs/focused_20260127_153651.md`

Event audit: `notes/analysis_runs/event_audit_20260127_153653.md`

Constants:

- HEATMAP_SCORE_WEIGHT_DEFAULT: 0.1
- HEATMAP_LOS_THRESHOLD_DEFAULT: 0.45
- HEATMAP_LOS_MARGIN_DEFAULT: 0.3
Profile deltas:

- min_shot_quality: -0.14
- seek_threshold: 0.12
- position_patience: -0.3
Summary:

- Matches: 24.0
- Avg duration: 49.31
- Goals/match: 1.667
- Shots/match: 9.083
- Shot%: 0.147
- Avg shot quality: 0.423
- Scoreless rate: 0.083
- Steal success rate: 0.279


### J3 (huge change in all vars that differ from baseline)

DB: `db/tournament_20260127_153711.db`

Focused report: `notes/analysis_runs/focused_20260127_153719.md`

Event audit: `notes/analysis_runs/event_audit_20260127_153721.md`

Constants:

- HEATMAP_SCORE_WEIGHT_DEFAULT: 0.0
- HEATMAP_LOS_THRESHOLD_DEFAULT: 0.35
- HEATMAP_LOS_MARGIN_DEFAULT: 0.4
Profile deltas:

- min_shot_quality: -0.18
- seek_threshold: 0.22
- position_patience: -0.55
Summary:

- Matches: 24.0
- Avg duration: 52.98
- Goals/match: 1.458
- Shots/match: 9.208
- Shot%: 0.136
- Avg shot quality: 0.407
- Scoreless rate: 0.208
- Steal success rate: 0.274


### J4 (extreme changes in most variables)

DB: `db/tournament_20260127_153740.db`

Focused report: `notes/analysis_runs/focused_20260127_153748.md`

Event audit: `notes/analysis_runs/event_audit_20260127_153750.md`

Constants:

- HEATMAP_SCORE_WEIGHT_DEFAULT: 0.0
- HEATMAP_LOS_THRESHOLD_DEFAULT: 0.25
- HEATMAP_LOS_MARGIN_DEFAULT: 0.55
Profile deltas:

- min_shot_quality: -0.25
- seek_threshold: 0.3
- position_patience: -0.8
Summary:

- Matches: 24.0
- Avg duration: 50.84
- Goals/match: 1.958
- Shots/match: 9.917
- Shot%: 0.176
- Avg shot quality: 0.396
- Scoreless rate: 0.083
- Steal success rate: 0.269


### M0 (base)

DB: `db/tournament_20260127_153809.db`

Focused report: `notes/analysis_runs/focused_20260127_153817.md`

Event audit: `notes/analysis_runs/event_audit_20260127_153819.md`

Constants:

- HEATMAP_SCORE_WEIGHT_DEFAULT: 0.5
- HEATMAP_LOS_THRESHOLD_DEFAULT: 0.75
- HEATMAP_LOS_MARGIN_DEFAULT: 0.12
Profile deltas:

- position_patience: -0.6
- seek_threshold: 0.18
- min_shot_quality: -0.05
Summary:

- Matches: 24.0
- Avg duration: 51.8
- Goals/match: 1.833
- Shots/match: 9.917
- Shot%: 0.164
- Avg shot quality: 0.412
- Scoreless rate: 0.125
- Steal success rate: 0.267


### M1 (close delta)

DB: `db/tournament_20260127_153837.db`

Focused report: `notes/analysis_runs/focused_20260127_153845.md`

Event audit: `notes/analysis_runs/event_audit_20260127_153846.md`

Constants:

- HEATMAP_SCORE_WEIGHT_DEFAULT: 0.55
- HEATMAP_LOS_THRESHOLD_DEFAULT: 0.78
- HEATMAP_LOS_MARGIN_DEFAULT: 0.1
Profile deltas:

- position_patience: -0.5
- seek_threshold: 0.15
- min_shot_quality: -0.04
Summary:

- Matches: 24.0
- Avg duration: 48.04
- Goals/match: 2.083
- Shots/match: 8.833
- Shot%: 0.193
- Avg shot quality: 0.416
- Scoreless rate: 0.0
- Steal success rate: 0.233


### M2 (close delta + big change in most significant vars)

DB: `db/tournament_20260127_153906.db`

Focused report: `notes/analysis_runs/focused_20260127_153915.md`

Event audit: `notes/analysis_runs/event_audit_20260127_153916.md`

Constants:

- HEATMAP_SCORE_WEIGHT_DEFAULT: 0.6
- HEATMAP_LOS_THRESHOLD_DEFAULT: 0.68
- HEATMAP_LOS_MARGIN_DEFAULT: 0.18
Profile deltas:

- position_patience: -0.8
- seek_threshold: 0.22
- min_shot_quality: -0.06
Summary:

- Matches: 24.0
- Avg duration: 51.92
- Goals/match: 1.917
- Shots/match: 10.0
- Shot%: 0.179
- Avg shot quality: 0.41
- Scoreless rate: 0.167
- Steal success rate: 0.267


### M3 (huge change in all vars that differ from baseline)

DB: `db/tournament_20260127_153935.db`

Focused report: `notes/analysis_runs/focused_20260127_153943.md`

Event audit: `notes/analysis_runs/event_audit_20260127_153945.md`

Constants:

- HEATMAP_SCORE_WEIGHT_DEFAULT: 0.7
- HEATMAP_LOS_THRESHOLD_DEFAULT: 0.6
- HEATMAP_LOS_MARGIN_DEFAULT: 0.22
Profile deltas:

- position_patience: -1.0
- seek_threshold: 0.28
- min_shot_quality: -0.1
Summary:

- Matches: 24.0
- Avg duration: 51.13
- Goals/match: 2.167
- Shots/match: 9.833
- Shot%: 0.203
- Avg shot quality: 0.418
- Scoreless rate: 0.125
- Steal success rate: 0.261


### M4 (extreme changes in most variables)

DB: `db/tournament_20260127_154003.db`

Focused report: `notes/analysis_runs/focused_20260127_154012.md`

Event audit: `notes/analysis_runs/event_audit_20260127_154013.md`

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
- Avg duration: 52.24
- Goals/match: 2.375
- Shots/match: 9.542
- Shot%: 0.236
- Avg shot quality: 0.414
- Scoreless rate: 0.042
- Steal success rate: 0.254


### P0 (base)

DB: `db/tournament_20260127_154032.db`

Focused report: `notes/analysis_runs/focused_20260127_154041.md`

Event audit: `notes/analysis_runs/event_audit_20260127_154043.md`

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
- Avg duration: 50.85
- Goals/match: 1.542
- Shots/match: 11.5
- Shot%: 0.105
- Avg shot quality: 0.401
- Scoreless rate: 0.125
- Steal success rate: 0.239


### P1 (close delta)

DB: `db/tournament_20260127_154101.db`

Focused report: `notes/analysis_runs/focused_20260127_154109.md`

Event audit: `notes/analysis_runs/event_audit_20260127_154111.md`

Constants:

- HEATMAP_SCORE_WEIGHT_DEFAULT: 0.45
- HEATMAP_LOS_THRESHOLD_DEFAULT: 0.7
- HEATMAP_LOS_MARGIN_DEFAULT: 0.1
Profile deltas:

- charge_min: -0.22
- charge_max: -0.18
- min_shot_quality: -0.04
- seek_threshold: 0.07
Summary:

- Matches: 24.0
- Avg duration: 56.08
- Goals/match: 0.667
- Shots/match: 13.75
- Shot%: 0.03
- Avg shot quality: 0.391
- Scoreless rate: 0.542
- Steal success rate: 0.261


### P2 (close delta + big change in most significant vars)

DB: `db/tournament_20260127_154129.db`

Focused report: `notes/analysis_runs/focused_20260127_154138.md`

Event audit: `notes/analysis_runs/event_audit_20260127_154140.md`

Constants:

- HEATMAP_SCORE_WEIGHT_DEFAULT: 0.5
- HEATMAP_LOS_THRESHOLD_DEFAULT: 0.6
- HEATMAP_LOS_MARGIN_DEFAULT: 0.18
Profile deltas:

- charge_min: -0.35
- charge_max: -0.3
- min_shot_quality: -0.08
- seek_threshold: 0.1
Summary:

- Matches: 24.0
- Avg duration: 58.94
- Goals/match: 0.292
- Shots/match: 16.583
- Shot%: 0.018
- Avg shot quality: 0.375
- Scoreless rate: 0.792
- Steal success rate: 0.238


### P3 (huge change in all vars that differ from baseline)

DB: `db/tournament_20260127_154158.db`

Focused report: `notes/analysis_runs/focused_20260127_154207.md`

Event audit: `notes/analysis_runs/event_audit_20260127_154208.md`

Constants:

- HEATMAP_SCORE_WEIGHT_DEFAULT: 0.6
- HEATMAP_LOS_THRESHOLD_DEFAULT: 0.55
- HEATMAP_LOS_MARGIN_DEFAULT: 0.22
Profile deltas:

- charge_min: -0.45
- charge_max: -0.4
- min_shot_quality: -0.12
- seek_threshold: 0.15
Summary:

- Matches: 24.0
- Avg duration: 60.02
- Goals/match: 0.0
- Shots/match: 17.875
- Shot%: 0.0
- Avg shot quality: 0.38
- Scoreless rate: 1.0
- Steal success rate: 0.254


### P4 (extreme changes in most variables)

DB: `db/tournament_20260127_154227.db`

Focused report: `notes/analysis_runs/focused_20260127_154236.md`

Event audit: `notes/analysis_runs/event_audit_20260127_154238.md`

Constants:

- HEATMAP_SCORE_WEIGHT_DEFAULT: 0.8
- HEATMAP_LOS_THRESHOLD_DEFAULT: 0.5
- HEATMAP_LOS_MARGIN_DEFAULT: 0.3
Profile deltas:

- charge_min: -0.6
- charge_max: -0.55
- min_shot_quality: -0.18
- seek_threshold: 0.22
Summary:

- Matches: 24.0
- Avg duration: 58.55
- Goals/match: 0.125
- Shots/match: 14.292
- Shot%: 0.006
- Avg shot quality: 0.355
- Scoreless rate: 0.875
- Steal success rate: 0.223


## Top 3 Variants (by goals/match, shots/match, scoreless rate)

- M4: goals 2.375, shots 9.542, scoreless 0.042
- J1: goals 2.250, shots 10.083, scoreless 0.042
- M3: goals 2.167, shots 9.833, scoreless 0.125

## Pairwise Deltas (Top 3)

- M4 vs J1: `notes/analysis_runs/event_audit_20260127_154239.md`
- M4 vs M3: `notes/analysis_runs/event_audit_20260127_154241.md`
- J1 vs M3: `notes/analysis_runs/event_audit_20260127_154243.md`

## Suggestions

- Favor variants that increase shots/match without dropping shot% below baseline.
- If scoreless rate rises above 0.20, raise min_shot_quality or tighten LOS threshold.
- For higher tempo, reduce position_patience and seek_threshold, but watch steal attempts.
