# Offline Manual Training Checklist (60 min)

Goal: capture high-quality debug traces for reachability heatmaps, LOS/shot gating, AI navigation quirks, and profile tuning signals.

Profiles (top 4 from rankings)
- v7_Disruptor_Patient
- v7_Anchor_Stealer
- v7_Opportunist_Patient
- v7_Fortress_Aggro

Non-debug levels to cover
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

## Time budget (educated guess)
- Target per level: ~6 minutes
  - 3 min reachability sweep
  - 2 min LOS + shot quality checks
  - 1 min AI chase / nav stress
- Total: ~60 minutes for 10 levels

## Per-level run template
Run one short session per level (goal mode, 3 iterations). Rotate profiles per level in order.

Command template:
```
cargo run --bin training -- --protocol advanced_platform --mode goal --iterations 3 --level "<LEVEL_NAME>" --profile <PROFILE>
```

### Level rotation plan (basic cycle repeats every 4 levels)
1) Islands — v7_Disruptor_Patient
2) Slopes — v7_Anchor_Stealer
3) Tower — v7_Opportunist_Patient
4) Arena — v7_Fortress_Aggro
5) Skyway — v7_Disruptor_Patient
6) Terraces — v7_Anchor_Stealer
7) Catwalk — v7_Opportunist_Patient
8) Bunker — v7_Fortress_Aggro
9) Pit — v7_Disruptor_Patient
10) Twin Towers — v7_Anchor_Stealer

## Per-level tasks (do these every level)
### 1) Reachability sweep (~3 min)
- Floor sweep left→right at slow + sprint speeds.
- Touch every platform; traverse edges; drop off both sides.
- Do varied jumps: short tap, full hold, late jump, coyote jump.

### 2) LOS + shot gating (~2 min)
- Take shots with clear LOS, then force a few bad LOS shots.
- Hover at mid range and see if AI will/ won’t shoot.
- Record any spots where LOS seems wrong.

### 3) AI nav stress (~1 min)
- Kite the AI across platforms.
- Try to induce oscillation or stuck paths.
- Note any “no‑go” regions for AI.

## After each level (fast)
- Quit the session cleanly.
- Run debug analysis on the latest session:
```
cargo run --bin analyze -- --training-db db/training.db
```

## End-of-hour wrap
- Skim the latest training debug reports under `training_logs/session_*/analysis/`.
- Note any obvious missing heatmaps or low-contrast warnings.

---

# Command Reference (examples + options)

## Training (manual)
Example:
```
cargo run --bin training -- --protocol advanced_platform --mode goal --iterations 3 --level "Arena" --profile v7_Disruptor_Patient
```
Options:
- `--protocol` advanced-platform | pursuit | pursuit2
- `--mode` goal | game
- `--iterations` N
- `--level` number or name
- `--profile` AI profile name

## Debug analysis (training DB)
Example:
```
cargo run --bin analyze -- --training-db db/training.db
```
Options:
- `--training-db <DB>`
- `--training-output <DIR>`

## Heatmaps (per level)
Example:
```
cargo run --bin heatmap -- --type line_of_sight --level "Arena"
```
Options:
- `--type` speed | score | landing_safety | line_of_sight | elevation
- `--level` level name or id
- `--refresh` (clear old outputs; use once at start)

## Ghost trials (optional)
Example:
```
cargo run --bin simulate -- --ghost training_logs/session_<TIMESTAMP>/ghost_trials/ --right v7_Fortress_Aggro
```
Options:
- `--ghost <DIR>`
- `--right <PROFILE>`

---

# Time Aggregation Script
After your offline session, total time spent using DBs with:
```
python3 offline_training/calc_training_minutes.py db/training_*.db
```
Or provide a file listing DB paths:
```
python3 offline_training/calc_training_minutes.py --list offline_training/db_list.txt
```
