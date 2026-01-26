# Training Analysis Workflow

## Overview

All training events are stored in SQLite (`training.db`). Analysis runs SQL queries directly, replacing file-based parsing.

## Running Training

```bash
cargo run --bin training                          # 5 games vs Balanced AI
cargo run --bin training -- --games N             # N games
cargo run --bin training -- --profile Aggressive  # Specific AI profile
```

## Generated Files

```
training.db                              # SQLite database with all events
training_logs/
└── session_YYYYMMDD_HHMMSS/
    ├── summary.json                     # Session summary
    ├── analysis.md                      # Human-readable report
    └── claude_prompt_YYYYMMDD_HHMM.txt  # AI review prompt
```

## Quick SQL Analysis

Open the database:
```bash
sqlite3 training.db
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

### Distance Analysis (Manual Query)
```sql
-- Find closest moment in latest match
SELECT time_ms, data
FROM events
WHERE match_id = (SELECT MAX(id) FROM matches)
  AND event_type = 'T'
ORDER BY time_ms
LIMIT 100;
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

## Analysis API

The Rust codebase provides analysis functions in `src/training/analysis.rs`:

```rust
// Evlog-based analysis (original)
pub fn analyze_session(session_dir: &Path, game_results: &[GameResult], protocol: TrainingProtocol) -> SessionAnalysis

// SQLite-based analysis (new)
pub fn analyze_session_from_db(db: &SimDatabase, session_id: &str, protocol: TrainingProtocol) -> Option<SessionAnalysis>
```

And database analysis methods in `src/simulation/db.rs`:

```rust
// Session queries
db.get_session_summary(session_id)?
db.get_session_matches(session_id)?
db.get_latest_session()?

// Match analysis
db.get_match_event_stats(match_id)?
db.analyze_distance(match_id)?
db.analyze_ai_inputs(match_id)?
db.get_goal_transitions(match_id)?
db.get_closest_moments(match_id, threshold)?
```

## Adding New Analyses

1. Add query function to `src/simulation/db.rs` in the Analysis Query Methods section
2. Add data structure if needed (in the Analysis Data Structures section)
3. Integrate into `analyze_game_from_db()` in `src/training/analysis.rs`
4. Update this document with new SQL examples

## Troubleshooting

### Database locked
```bash
# Check for processes using the database
lsof training.db

# Force close connections (if needed)
sqlite3 training.db "PRAGMA wal_checkpoint(TRUNCATE);"
```

### Missing events
Ensure `flush_events_to_sqlite` system is running in the training binary's Update schedule.

### Schema mismatch
If tables are missing columns, the database was created with an older schema. Either:
1. Delete `training.db` and re-run training
2. Manually add columns with `ALTER TABLE`
