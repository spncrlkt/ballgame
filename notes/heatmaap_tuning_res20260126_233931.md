# Heatmap Tuning Tournament Results

Baseline DB: tournament_20260126_074832.db

Run date: 2026-01-26 23:45:47

## Variant A: Aggressive Shot Volume (Global)

- DB: /Users/spncr/dev/ballgame/db/tournament_20260126_234605.db
- Goals/match: 1.983
- Shots/match: 8.994
- Shot%: 0.194
- Avg shot quality: 0.429
- Scoreless rate: 0.114
- Winners: {'left': 353, 'right': 326, 'tie': 221}
- Best profile: v10_Rand_A (47.8%)

## Variant B: Heatmap-Driven, More Shots

- DB: /Users/spncr/dev/ballgame/db/tournament_20260126_234950.db
- Goals/match: 1.986
- Shots/match: 8.958
- Shot%: 0.194
- Avg shot quality: 0.429
- Scoreless rate: 0.107
- Winners: {'left': 347, 'right': 337, 'tie': 216}
- Best profile: v10_Rand_F (42.2%)

## Variant C: LOS Relax Only (Minimal)

- DB: /Users/spncr/dev/ballgame/db/tournament_20260126_235227.db
- Goals/match: 1.990
- Shots/match: 8.857
- Shot%: 0.201
- Avg shot quality: 0.428
- Scoreless rate: 0.120
- Winners: {'left': 373, 'right': 298, 'tie': 229}
- Best profile: v9_Rand_C (44.4%)

## Variant D: Per-Level LOS Relax (Targeted)

- DB: /Users/spncr/dev/ballgame/db/tournament_20260126_235504.db
- Goals/match: 1.849
- Shots/match: 8.736
- Shot%: 0.188
- Avg shot quality: 0.430
- Scoreless rate: 0.117
- Winners: {'left': 369, 'right': 295, 'tie': 236}
- Best profile: v8_Rand_Alpha (42.2%)

## Variant E: Lower Seek, Faster Shooting

- DB: /Users/spncr/dev/ballgame/db/tournament_20260126_235740.db
- Goals/match: 1.914
- Shots/match: 8.967
- Shot%: 0.189
- Avg shot quality: 0.428
- Scoreless rate: 0.117
- Winners: {'left': 378, 'right': 323, 'tie': 199}
- Best profile: v10_Rand_E (43.9%)

## Variant F: High Scoring Bias (Quality-first)

- DB: /Users/spncr/dev/ballgame/db/tournament_20260127_000013.db
- Goals/match: 1.944
- Shots/match: 8.863
- Shot%: 0.196
- Avg shot quality: 0.423
- Scoreless rate: 0.116
- Winners: {'left': 355, 'right': 318, 'tie': 227}
- Best profile: v10_Rand_A (41.1%)

## Variant G: Charge Aggression

- DB: /Users/spncr/dev/ballgame/db/tournament_20260127_000249.db
- Goals/match: 1.357
- Shots/match: 9.508
- Shot%: 0.120
- Avg shot quality: 0.415
- Scoreless rate: 0.244
- Winners: {'left': 319, 'right': 278, 'tie': 303}
- Best profile: v9_Rand_D (37.2%)

## Variant H: Composite More Everything

- DB: /Users/spncr/dev/ballgame/db/tournament_20260127_000525.db
- Goals/match: 1.936
- Shots/match: 9.102
- Shot%: 0.187
- Avg shot quality: 0.429
- Scoreless rate: 0.111
- Winners: {'left': 364, 'right': 323, 'tie': 213}
- Best profile: v10_Rand_A (42.8%)

# Analysis

## Baseline
- Goals/match: 1.981
- Shots/match: 8.959
- Shot%: 0.196
- Avg shot quality: 0.427
- Scoreless rate: 0.121

## Winners by Metric
- Highest goals/match: Variant C (1.990)
- Highest shots/match: Variant G (9.508)
- Highest avg shot quality: Variant D (0.430)

## Recommendations
- If the goal is faster, higher-scoring games, prioritize variants that raise shots/match without tanking shot% (typically lower LOS thresholds + lower min_shot_quality).
- If shot volume rises but goals/match falls, increase score heatmap weight slightly or reduce LOS threshold only on low-shot levels.
- If avg shot quality rises but shots drop, reduce position_patience or seek_threshold to stop over-seeking.
- If scoreless rate stays high on levels 8/14/15, lower heatmap_los_threshold and raise margin for Skyway/Pursuit Arena levels.
- Consider a combined approach: modest LOS relax + small min_shot_quality drop + mild shoot_range bump.
