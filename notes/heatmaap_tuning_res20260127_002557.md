# Heatmap Tuning Tournament Results

Baseline DB: tournament_20260126_074832.db

Run date: 2026-01-27 00:25:57

## Variant A: Aggressive Shot Volume (Global)

- DB: /Users/spncr/dev/ballgame/db/tournament_20260127_002614.db
- Goals/match: 2.679
- Shots/match: 12.498
- Shot%: 0.189
- Avg shot quality: 0.423
- Scoreless rate: 0.037
- Winners: {'left': 383, 'right': 362, 'tie': 155}
- Best profile: v9_Rand_C (47.8%)

## Variant G: Charge Aggression

- DB: /Users/spncr/dev/ballgame/db/tournament_20260127_003244.db
- Goals/match: 1.799
- Shots/match: 12.304
- Shot%: 0.113
- Avg shot quality: 0.412
- Scoreless rate: 0.048
- Winners: {'left': 418, 'right': 363, 'tie': 119}
- Best profile: v9_Rand_D (49.4%)

## Variant I: Heatmap Max + Loose LOS

- DB: /Users/spncr/dev/ballgame/db/tournament_20260127_004041.db
- Goals/match: 2.211
- Shots/match: 13.584
- Shot%: 0.138
- Avg shot quality: 0.414
- Scoreless rate: 0.050
- Winners: {'left': 406, 'right': 339, 'tie': 155}
- Best profile: v10_Rand_F (46.1%)

## Variant J: Heatmap Min + Loose LOS

- DB: /Users/spncr/dev/ballgame/db/tournament_20260127_004556.db
- Goals/match: 3.214
- Shots/match: 14.209
- Shot%: 0.206
- Avg shot quality: 0.425
- Scoreless rate: 0.023
- Winners: {'left': 396, 'right': 370, 'tie': 134}
- Best profile: v10_Rand_C (50.6%)

## Variant K: Tight LOS + Aggro Profiles

- DB: /Users/spncr/dev/ballgame/db/tournament_20260127_005238.db
- Goals/match: 2.886
- Shots/match: 13.148
- Shot%: 0.196
- Avg shot quality: 0.424
- Scoreless rate: 0.029
- Winners: {'left': 422, 'right': 349, 'tie': 129}
- Best profile: v8_Rand_Alpha (50.6%)

## Variant L: Ultra Range Spray

- DB: /Users/spncr/dev/ballgame/db/tournament_20260127_005712.db
- Goals/match: 1.601
- Shots/match: 14.027
- Shot%: 0.080
- Avg shot quality: 0.402
- Scoreless rate: 0.088
- Winners: {'left': 400, 'right': 360, 'tie': 140}
- Best profile: v10_Rand_F (49.4%)

## Variant M: No Patience / Fast Decisions

- DB: /Users/spncr/dev/ballgame/db/tournament_20260127_010413.db
- Goals/match: 3.050
- Shots/match: 13.509
- Shot%: 0.205
- Avg shot quality: 0.420
- Scoreless rate: 0.034
- Winners: {'left': 401, 'right': 342, 'tie': 157}
- Best profile: v10_Rand_B (47.8%)

## Variant N: Wide LOS + Target Level Overdrive

- DB: /Users/spncr/dev/ballgame/db/tournament_20260127_010925.db
- Goals/match: 2.838
- Shots/match: 12.728
- Shot%: 0.197
- Avg shot quality: 0.428
- Scoreless rate: 0.021
- Winners: {'left': 409, 'right': 346, 'tie': 145}
- Best profile: v9_Rand_C (47.8%)

## Variant O: Heatmap Overdrive By Level

- DB: /Users/spncr/dev/ballgame/db/tournament_20260127_011340.db
- Goals/match: 2.839
- Shots/match: 12.742
- Shot%: 0.198
- Avg shot quality: 0.427
- Scoreless rate: 0.022
- Winners: {'left': 427, 'right': 349, 'tie': 124}
- Best profile: v10_Rand_A (50.6%)

## Variant P: Charge Micro + Quick Shots

- DB: /Users/spncr/dev/ballgame/db/tournament_20260127_011801.db
- Goals/match: 1.222
- Shots/match: 15.691
- Shot%: 0.043
- Avg shot quality: 0.405
- Scoreless rate: 0.072
- Winners: {'left': 408, 'right': 371, 'tie': 121}
- Best profile: v10_Rand_F (53.9%)

# Analysis

## Baseline
- Goals/match: 1.981
- Shots/match: 8.959
- Shot%: 0.196
- Avg shot quality: 0.427
- Scoreless rate: 0.121

## Winners by Metric
- Highest goals/match: Variant J (3.214)
- Highest shots/match: Variant P (15.691)
- Highest avg shot quality: Variant N (0.428)

## Recommendations
- For faster, higher-point games, favor variants that raise shots/match while keeping shot% near baseline (looser LOS + lower min_shot_quality, but avoid over-tight LOS).
- If shot volume jumps but goals drop, increase heatmap score weight or reduce shoot_range inflation to focus on higher-quality zones.
- If scoreless rate stays high, relax LOS thresholds on low-visibility levels (Skyway/Pursuit Arenas) and lower position_patience to reduce stalling.
- If avg shot quality plummets, tighten LOS margin slightly or raise min_shot_quality back toward baseline while keeping seek_threshold elevated.
- A combined approach usually works best: moderate LOS relax + small min_shot_quality drop + modest range bump.
