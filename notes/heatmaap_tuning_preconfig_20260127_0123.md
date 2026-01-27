# Heatmap Tuning Configs (Prep Only)

Base per-level overrides already applied to Skyway, Pursuit Arena, Pursuit Arena 2 (hybrid):
- heatmap_score_weight: 0.300
- heatmap_los_threshold: 0.600
- heatmap_los_margin: 0.200

Baseline constants (for reference):
- HEATMAP_SCORE_WEIGHT_DEFAULT: 0.50
- HEATMAP_LOS_THRESHOLD_DEFAULT: 0.90
- HEATMAP_LOS_MARGIN_DEFAULT: 0.05

Baseline profile params touched here:
- min_shot_quality
- seek_threshold
- position_patience


## J Family (Heatmap Min + Loose LOS)

### J0 (base)
constants:
- HEATMAP_SCORE_WEIGHT_DEFAULT: 0.00
- HEATMAP_LOS_THRESHOLD_DEFAULT: 0.55
- HEATMAP_LOS_MARGIN_DEFAULT: 0.25
profile_deltas:
- min_shot_quality: -0.10
- seek_threshold: +0.15
- position_patience: -0.35

### J1 (close delta)
constants:
- HEATMAP_SCORE_WEIGHT_DEFAULT: 0.05
- HEATMAP_LOS_THRESHOLD_DEFAULT: 0.58
- HEATMAP_LOS_MARGIN_DEFAULT: 0.22
profile_deltas:
- min_shot_quality: -0.08
- seek_threshold: +0.12
- position_patience: -0.30

### J2 (close delta + big change in most significant vars)
most significant: los_threshold, los_margin, min_shot_quality
constants:
- HEATMAP_SCORE_WEIGHT_DEFAULT: 0.10
- HEATMAP_LOS_THRESHOLD_DEFAULT: 0.45
- HEATMAP_LOS_MARGIN_DEFAULT: 0.30
profile_deltas:
- min_shot_quality: -0.14
- seek_threshold: +0.12
- position_patience: -0.30

### J3 (huge change in all vars that differ from baseline)
constants:
- HEATMAP_SCORE_WEIGHT_DEFAULT: 0.00
- HEATMAP_LOS_THRESHOLD_DEFAULT: 0.35
- HEATMAP_LOS_MARGIN_DEFAULT: 0.40
profile_deltas:
- min_shot_quality: -0.18
- seek_threshold: +0.22
- position_patience: -0.55

### J4 (extreme changes in most variables)
constants:
- HEATMAP_SCORE_WEIGHT_DEFAULT: 0.00
- HEATMAP_LOS_THRESHOLD_DEFAULT: 0.25
- HEATMAP_LOS_MARGIN_DEFAULT: 0.55
profile_deltas:
- min_shot_quality: -0.25
- seek_threshold: +0.30
- position_patience: -0.80


## M Family (No Patience / Fast Decisions)

### M0 (base)
constants:
- HEATMAP_SCORE_WEIGHT_DEFAULT: 0.50
- HEATMAP_LOS_THRESHOLD_DEFAULT: 0.75
- HEATMAP_LOS_MARGIN_DEFAULT: 0.12
profile_deltas:
- position_patience: -0.60
- seek_threshold: +0.18
- min_shot_quality: -0.05

### M1 (close delta)
constants:
- HEATMAP_SCORE_WEIGHT_DEFAULT: 0.55
- HEATMAP_LOS_THRESHOLD_DEFAULT: 0.78
- HEATMAP_LOS_MARGIN_DEFAULT: 0.10
profile_deltas:
- position_patience: -0.50
- seek_threshold: +0.15
- min_shot_quality: -0.04

### M2 (close delta + big change in most significant vars)
most significant: position_patience, seek_threshold, los_threshold
constants:
- HEATMAP_SCORE_WEIGHT_DEFAULT: 0.60
- HEATMAP_LOS_THRESHOLD_DEFAULT: 0.68
- HEATMAP_LOS_MARGIN_DEFAULT: 0.18
profile_deltas:
- position_patience: -0.80
- seek_threshold: +0.22
- min_shot_quality: -0.06

### M3 (huge change in all vars that differ from baseline)
constants:
- HEATMAP_SCORE_WEIGHT_DEFAULT: 0.70
- HEATMAP_LOS_THRESHOLD_DEFAULT: 0.60
- HEATMAP_LOS_MARGIN_DEFAULT: 0.22
profile_deltas:
- position_patience: -1.00
- seek_threshold: +0.28
- min_shot_quality: -0.10

### M4 (extreme changes in most variables)
constants:
- HEATMAP_SCORE_WEIGHT_DEFAULT: 0.90
- HEATMAP_LOS_THRESHOLD_DEFAULT: 0.50
- HEATMAP_LOS_MARGIN_DEFAULT: 0.30
profile_deltas:
- position_patience: -1.20
- seek_threshold: +0.35
- min_shot_quality: -0.15


## P Family (Charge Micro + Quick Shots)

### P0 (base)
constants:
- HEATMAP_SCORE_WEIGHT_DEFAULT: 0.40
- HEATMAP_LOS_THRESHOLD_DEFAULT: 0.68
- HEATMAP_LOS_MARGIN_DEFAULT: 0.12
profile_deltas:
- charge_min: -0.25
- charge_max: -0.20
- min_shot_quality: -0.05
- seek_threshold: +0.08

### P1 (close delta)
constants:
- HEATMAP_SCORE_WEIGHT_DEFAULT: 0.45
- HEATMAP_LOS_THRESHOLD_DEFAULT: 0.70
- HEATMAP_LOS_MARGIN_DEFAULT: 0.10
profile_deltas:
- charge_min: -0.22
- charge_max: -0.18
- min_shot_quality: -0.04
- seek_threshold: +0.07

### P2 (close delta + big change in most significant vars)
most significant: charge_min, charge_max, min_shot_quality, los_threshold
constants:
- HEATMAP_SCORE_WEIGHT_DEFAULT: 0.50
- HEATMAP_LOS_THRESHOLD_DEFAULT: 0.60
- HEATMAP_LOS_MARGIN_DEFAULT: 0.18
profile_deltas:
- charge_min: -0.35
- charge_max: -0.30
- min_shot_quality: -0.08
- seek_threshold: +0.10

### P3 (huge change in all vars that differ from baseline)
constants:
- HEATMAP_SCORE_WEIGHT_DEFAULT: 0.60
- HEATMAP_LOS_THRESHOLD_DEFAULT: 0.55
- HEATMAP_LOS_MARGIN_DEFAULT: 0.22
profile_deltas:
- charge_min: -0.45
- charge_max: -0.40
- min_shot_quality: -0.12
- seek_threshold: +0.15

### P4 (extreme changes in most variables)
constants:
- HEATMAP_SCORE_WEIGHT_DEFAULT: 0.80
- HEATMAP_LOS_THRESHOLD_DEFAULT: 0.50
- HEATMAP_LOS_MARGIN_DEFAULT: 0.30
profile_deltas:
- charge_min: -0.60
- charge_max: -0.55
- min_shot_quality: -0.18
- seek_threshold: +0.22
