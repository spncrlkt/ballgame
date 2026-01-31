# Offline Manual Training Checklist (~60 min)

Goal: capture high-quality debug traces for reachability heatmaps, LOS/shot gating, AI navigation quirks, and profile tuning signals.

## Tournament of Champions - Top 10 Profiles

From the 64-profile Tournament of Champions bracket analysis:

| Rank | Profile | Match W-L | Notes |
|------|---------|-----------|-------|
| 1 | T17_v15_EvoAG_Speed | 10-2 | Speed archetype dominant |
| 2 | T14_v13_EvoD_Patient | 8-2 | Patient defender |
| 3 | T17_v17_EvoAGPatie_Speed | 7-0 | Undefeated, speed/patience hybrid |
| 4 | T15_v15_EvoJZone_Aggro | 6-2 | Zone aggressor |
| 5 | T16_v14_EvoQSpeed_Patient | 5-2 | Patient speed |
| 6 | T16_v16_EvoDPatien_Patient | 5-2 | Double patience |
| 7 | T14_v14_EvoYZone_Speed | 4-2 | Zone speed |
| 8 | T14_v14_EvoJ_Aggro | 4-2 | Raw aggro |
| 9 | T16_v15_EvoDZoneSp_Speed | 4-2 | Zone speed |
| 10 | T17_v17_EvoDPatien_Zone | 4-2 | Patient zoner |

## Non-debug levels to cover (12 levels)

- Islands
- Slopes
- Tower
- Arena
- Skyway
- Terraces
- Catwalk
- Bunker
- Pit
- Twin Towers
- Pursuit Arena
- Pursuit Arena 2

## Time budget

- Target per level: ~5 minutes
  - 2 min reachability sweep
  - 2 min LOS + shot quality checks
  - 1 min AI chase / nav stress
- Total: ~60 minutes for 12 levels

## Startup command

Use the champions profiles file for all sessions:

```bash
cargo run --bin training -- \
  --profiles-file config/ai_profiles_champions.txt \
  --mode goal \
  --iterations 3 \
  --level "<LEVEL_NAME>" \
  --profile <PROFILE>
```

Or update `config/training_settings.json` with:
```json
{
  "profiles_file": "config/ai_profiles_champions.txt"
}
```

## Level rotation plan (10 profiles across 12 levels)

Cycle champions in order, wrapping after profile 10:

| # | Level | Profile |
|---|-------|---------|
| 1 | Islands | T17_v15_EvoAG_Speed |
| 2 | Slopes | T14_v13_EvoD_Patient |
| 3 | Tower | T17_v17_EvoAGPatie_Speed |
| 4 | Arena | T15_v15_EvoJZone_Aggro |
| 5 | Skyway | T16_v14_EvoQSpeed_Patient |
| 6 | Terraces | T16_v16_EvoDPatien_Patient |
| 7 | Catwalk | T14_v14_EvoYZone_Speed |
| 8 | Bunker | T14_v14_EvoJ_Aggro |
| 9 | Pit | T16_v15_EvoDZoneSp_Speed |
| 10 | Twin Towers | T17_v17_EvoDPatien_Zone |
| 11 | Pursuit Arena | T17_v15_EvoAG_Speed |
| 12 | Pursuit Arena 2 | T14_v13_EvoD_Patient |

## Per-level tasks (do these every level)

### 1) Reachability sweep (~2 min)
- Floor sweep left→right at slow + sprint speeds.
- Touch every platform; traverse edges; drop off both sides.
- Do varied jumps: short tap, full hold, late jump, coyote jump.

### 2) LOS + shot gating (~2 min)
- Take shots with clear LOS, then force a few bad LOS shots.
- Hover at mid range and see if AI will/won't shoot.
- Record any spots where LOS seems wrong.

### 3) AI nav stress (~1 min)
- Kite the AI across platforms.
- Try to induce oscillation or stuck paths.
- Note any "no-go" regions for AI.

## After each level (fast)

Quit the session cleanly. Run debug analysis on the latest session:
```bash
cargo run --bin analyze -- --training-db $(ls -t db/training_*.db | head -1)
```

## End-of-hour wrap

- Skim the latest training debug reports under `training_logs/session_*/analysis/`.
- Note any obvious missing heatmaps or low-contrast warnings.
- Run the combined analysis script:
```bash
./offline_training/analyze_offline.sh
```

### Coverage targets for reachability analysis

Arena grid: 80 × 45 = 3,600 cells (20px cell size)

| Coverage | Cells | Assessment |
|----------|-------|------------|
| <10% | <360 | Insufficient - need more playtime |
| 10-20% | 360-720 | Low - basic patterns visible |
| 20-30% | 720-1080 | Good - meaningful analysis possible |
| >30% | >1080 | Excellent - comprehensive coverage |

Rule of thumb: ~5 minutes of active play per level for 25%+ coverage.

## Quick notes (fill per level)

- Islands:
- Slopes:
- Tower:
- Arena:
- Skyway:
- Terraces:
- Catwalk:
- Bunker:
- Pit:
- Twin Towers:
- Pursuit Arena:
- Pursuit Arena 2:

---

# Command Reference (examples + options)

## Training (manual with champions)

Example:
```bash
cargo run --bin training -- \
  --profiles-file config/ai_profiles_champions.txt \
  --mode goal \
  --iterations 3 \
  --level "Arena" \
  --profile T17_v17_EvoAGPatie_Speed
```

Options:
- `--profiles-file` AI profiles file (champions: config/ai_profiles_champions.txt)
- `--protocol` advanced-platform | pursuit | pursuit2
- `--mode` goal | game
- `--iterations` N
- `--level` number or name
- `--profile` AI profile name

## Debug analysis (training DB)

Example:
```bash
cargo run --bin analyze -- --training-db $(ls -t db/training_*.db | head -1)
```
Options:
- `--training-db <DB>`
- `--training-output <DIR>`

## Merge offline training DBs (combined report)

Example:
```bash
python3 offline_training/merge_training_dbs.py --list offline_training/db_list.txt --out db/combined_offline_training.db
cargo run --bin analyze -- --training-db db/combined_offline_training.db
```

## Heatmaps (per level)

Example:
```bash
cargo run --bin heatmap -- --type line_of_sight --level "Arena"
```
Options:
- `--type` speed | score | landing_safety | line_of_sight | elevation
- `--level` level name or id
- `--refresh` (clear old outputs; use once at start)

## Ghost trials (optional)

Example:
```bash
cargo run --bin simulate -- --ghost training_logs/session_<TIMESTAMP>/ghost_trials/ --right T17_v17_EvoAGPatie_Speed
```
Options:
- `--ghost <DIR>`
- `--right <PROFILE>`

---

# Time Aggregation Script

After your offline session, total time spent using DBs with:
```bash
python3 offline_training/calc_training_minutes.py db/training_*.db
```
Or provide a file listing DB paths:
```bash
python3 offline_training/calc_training_minutes.py --list offline_training/db_list.txt
```
