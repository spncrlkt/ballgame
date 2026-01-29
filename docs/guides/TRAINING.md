# Training Mode Guide

Play 1v1 against AI with full event logging and analysis.

## Quick Start

```bash
cargo run --bin training                        # 5 iterations vs Balanced AI
cargo run --bin training -- -n 10               # 10 iterations
cargo run --bin training -- -p v3_Rush_Smart    # vs specific AI
cargo run --bin training -- -l 3                # Force level 3
```

## Command Line Options

| Option | Description | Default |
|--------|-------------|---------|
| `--protocol NAME` | Training protocol | advanced-platform |
| `-m, --mode MODE` | `goal` or `game` | goal |
| `-n, --iterations N` | Number of iterations | 5 |
| `-w, --win-score N` | Points to win (game mode) | 5 |
| `-p, --profile NAME` | AI opponent profile | Balanced |
| `-l, --level N` | Force level (number or name) | random |
| `-s, --seed N` | RNG seed for reproducibility | random |
| `-t, --time-limit SECS` | Time limit per iteration | none |
| `--first-point-timeout SECS` | End if no score within SECS | none |
| `--ball-style NAME` | Ball visual style (`random` or name) | random |
| `--viewport N` | Viewport preset index | 2 |
| `--palette N` | Color palette index | 0 |
| `--drive-mode` | Start with ball, regain on loss | off |

**Protocols:**
- `advanced-platform` - Full 1v1 games on random levels (default)
- `pursuit` - Flat level chase test (verifies AI pursues player)
- `pursuit2` - Platform chase test (pursuit with center obstacle)
- `reachability` - Solo level exploration for coverage mapping (see below)

**Modes:**
- `goal` - Each iteration ends after one goal, then resets
- `game` - Full game to win_score points

## Settings File

Persist your preferences in `config/training_settings.json` (gitignored):

```json
{
  "iterations": 5,
  "ai_profile": "v3_Rush_Smart",
  "level": null,
  "ball_style": null,
  "exclude_levels": ["Pit"],
  "mode": "goal",
  "win_score": 5
}
```

| Field | Description |
|-------|-------------|
| `protocol` | Training protocol name |
| `mode` | `"goal"` or `"game"` |
| `iterations` | Rounds per session |
| `ai_profile` | Default opponent profile |
| `level` | `null` = random, or number/name (e.g., `7` or `"Skyway"`) |
| `ball_style` | `null` = random, or name like `"wedges"` |
| `exclude_levels` | Levels to skip in random selection |

CLI arguments override file settings.

## Output

All events are stored in SQLite: `db/training.db`

Session files saved to `training_logs/session_YYYYMMDD_HHMMSS/`:
```
training_logs/session_20260125_143022/
├── summary.json                      # Session summary
├── analysis.md                       # Human-readable report
└── analysis_request_20260125_143022.md  # AI review request template
```

## Controls

| Action | Keyboard | Gamepad |
|--------|----------|---------|
| Move | A/D | Left Stick |
| Jump | Space/W | A (South) |
| Pickup/Steal | E | X (West) |
| Throw (hold) | F | RB |
| Next Level (reachability) | Q | LB |
| Quit | Escape | - |

## Reachability Protocol

Solo exploration mode for generating coverage heatmaps. No AI opponent - just explore each level to map reachable areas.

### Quick Start

```bash
# Explore all levels sequentially (starts at Arena)
BALLGAME_SKIP_REACHABILITY_HEATMAPS=1 cargo run --bin training -- --protocol reachability

# Start at a specific level (skips earlier levels)
BALLGAME_SKIP_REACHABILITY_HEATMAPS=1 cargo run --bin training -- --protocol reachability -l "Open Floor"
BALLGAME_SKIP_REACHABILITY_HEATMAPS=1 cargo run --bin training -- --protocol reachability -l 4
```

The `BALLGAME_SKIP_REACHABILITY_HEATMAPS=1` env var skips expensive heatmap regeneration during training.

### Workflow

1. **Explore** - Move around the level, visiting all reachable areas
2. **Advance** - Press LB/Q when done with current level
3. **Repeat** - Continue through all levels (or quit with Escape)

### Data Pipeline

Position data flows from training to heatmaps:

```
Training Session → debug_events table → export script → heatmap .txt files
```

1. **Collect data**: Run reachability training sessions
2. **Check samples**: Query `debug_events` for coverage
   ```bash
   sqlite3 db/training.db "SELECT level_id, COUNT(*) FROM debug_events WHERE human_controlled=1 GROUP BY level_id;"
   ```
3. **Export heatmaps**: Generate `.txt` files for AI use
   ```bash
   python3 scripts/export_reachability.py db/training.db
   ```
4. **Output**: `showcase/heatmaps/heatmap_reachability_{level}_{id}.txt`

### Combining Sessions

Multiple training sessions can be combined for better coverage:

```bash
# List sessions to combine
ls db/training_*.db

# Combine into offline_training/db_list.txt and merge
# (see offline_training workflow)
```

### Minimum Samples

The export script requires 100+ samples per level by default:

```bash
# Export with lower threshold
python3 scripts/export_reachability.py db/training.db --min-samples 50
```

## SQL Analysis

Open the database:
```bash
sqlite3 db/training.db
```

### Session Summary
```sql
SELECT s.id, s.session_type, s.created_at, COUNT(m.id) as matches
FROM sessions s
LEFT JOIN matches m ON m.session_id = s.id
GROUP BY s.id
ORDER BY s.created_at DESC
LIMIT 5;
```

### Latest Match Events
```sql
SELECT event_type, COUNT(*) as count
FROM events
WHERE match_id = (SELECT MAX(id) FROM matches)
GROUP BY event_type
ORDER BY count DESC;
```

### AI Goal Transitions
```sql
SELECT time_ms, data
FROM events
WHERE match_id = (SELECT MAX(id) FROM matches)
  AND event_type = 'AG'
ORDER BY time_ms;
```

### Win/Loss by Profile
```sql
SELECT
    right_profile as ai_profile,
    COUNT(*) as matches,
    SUM(CASE WHEN winner = 'left' THEN 1 ELSE 0 END) as human_wins,
    SUM(CASE WHEN winner = 'right' THEN 1 ELSE 0 END) as ai_wins
FROM matches
GROUP BY right_profile;
```

## Database Schema

### Tables

- **sessions** - Training session metadata
  - `id` (TEXT PRIMARY KEY) - UUID
  - `created_at` (TEXT) - ISO timestamp
  - `session_type` (TEXT) - "training", "simulation", "game"
  - `config_json` (TEXT) - Optional configuration

- **matches** - Per-match results
  - `id` (INTEGER PRIMARY KEY)
  - `session_id` (TEXT) - References sessions
  - `seed` (INTEGER) - Random seed
  - `level` / `level_name` (INTEGER/TEXT)
  - `left_profile` / `right_profile` (TEXT)
  - `score_left` / `score_right` (INTEGER)
  - `duration_secs` (REAL)
  - `winner` (TEXT) - "left", "right", "tie"

- **events** - All game events
  - `id` (INTEGER PRIMARY KEY)
  - `match_id` (INTEGER) - References matches
  - `time_ms` (INTEGER) - Game time in milliseconds
  - `event_type` (TEXT) - Event code (T, G, P, SR, etc.)
  - `data` (TEXT) - Serialized event data

- **player_stats** - Aggregate stats per player per match
  - Shots, goals, steals, possession time, etc.

- **debug_events** - Per-tick player state (for reachability analysis)
  - `match_id`, `time_ms`, `tick_frame`
  - `player`, `pos_x`, `pos_y`, `vel_x`, `vel_y`
  - `level_id`, `human_controlled`
  - `nav_active`, `nav_path_index`, `nav_action`

### Event Types

| Code | Description | Data Format |
|------|-------------|-------------|
| T | Tick (physics frame) | `T:ms\|T\|frame\|left_pos\|left_vel\|right_pos\|right_vel\|ball_pos\|ball_vel\|state` |
| G | Goal scored | `T:ms\|G\|player\|score_left\|score_right` |
| P | Ball pickup | `T:ms\|P\|player` |
| SR | Shot release | `T:ms\|SR\|player\|charge\|angle\|power` |
| SS | Shot start | `T:ms\|SS\|player\|pos_x,pos_y\|quality` |
| SA | Steal attempt | `T:ms\|SA\|player` |
| S+ | Steal success | `T:ms\|S+\|player` |
| S- | Steal fail | `T:ms\|S-\|player` |
| SO | Steal out of range | `T:ms\|SO\|player` |
| AG | AI goal change | `T:ms\|AG\|player\|goal_name` |
| CI | Controller input | `T:ms\|CI\|player\|source\|move_x\|jump\|jump_p\|throw\|throw_r\|pickup` |

## AI Profiles

53 profiles available. Key ones:

| Profile | Style |
|---------|-------|
| `v2_Balanced` | Good all-around default |
| `v3_Rush_Smart` | Fast, aggressive, smart decisions |
| `v3_Steady_Deep` | Patient, defensive, high IQ |
| `v3_Spec_Chaos` | Unpredictable, takes risky shots |
| `v3_Ultra_Rush` | Maximum aggression |

List all: check `config/ai_profiles.txt`

## Troubleshooting

### Database locked
```bash
# Check for processes using the database
lsof db/training.db

# Force close connections (if needed)
sqlite3 db/training.db "PRAGMA wal_checkpoint(TRUNCATE);"
```

### Missing events
Ensure `flush_events_to_sqlite` system is running in the training binary's Update schedule.

### Schema mismatch
If tables are missing columns, the database was created with an older schema. Either:
1. Delete `db/training.db` and re-run training
2. Manually add columns with `ALTER TABLE`

## Post-Session Analysis

Each training iteration is a complete drive (you start with the ball). No extraction needed.

```bash
# Training debug analysis
cargo run --bin analyze -- --training-db db/training.db
```

Ask Claude Code to analyze the training session:
```
"Analyze my training session in training_logs/session_20260123_143022/"
```

**Analysis goal:** When analyzing training sessions, the objective is to identify ways to improve AI behavior. Review the events, player notes, and AI goal transitions to find patterns where the AI makes poor decisions. Then examine the AI code in `src/ai/` and suggest specific changes.
