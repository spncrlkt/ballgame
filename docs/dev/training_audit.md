# Training Workflow Audit

Run this checklist after changes to:
- `src/bin/training.rs`
- `src/training/`
- `src/ai/`
- `src/events/`

---

## Functional Tests

### 1. Basic Training Starts

```bash
cargo run --bin training -- -n 1
```

**Expected:**
- [ ] Window opens with 1v1 match
- [ ] Human player on left, AI on right
- [ ] Ball spawns at center
- [ ] Countdown starts (3-2-1-GO!)
- [ ] AI begins moving toward ball after GO!

### 2. Training Protocols

```bash
# Pursuit protocol - AI chases player
cargo run --bin training -- -n 1 --protocol pursuit
```
- [ ] AI actively follows human player

```bash
# Reachability protocol - exploration mode
cargo run --bin training -- -n 1 --protocol reachability
```
- [ ] AI explores platform reachability

```bash
# Advanced platform protocol
cargo run --bin training -- -n 1 --protocol advanced-platform
```
- [ ] AI navigates complex platform layouts

### 3. Session Completion

```bash
cargo run --bin training -- -n 3 -p Aggressive
```

- [ ] Runs exactly 3 iterations
- [ ] Uses Aggressive AI profile
- [ ] Session completes without crash
- [ ] Summary written to `training_logs/session_*/`

### 4. Early Quit

```bash
cargo run --bin training -- -n 5
# Press Escape after 2 iterations
```

- [ ] Session ends cleanly
- [ ] Partial summary still written
- [ ] SQLite contains all events up to quit

---

## SQLite Verification

### Database Created

```bash
ls -la db/training*.db
```
- [ ] Database file exists
- [ ] Size is non-zero

### Schema Correct

```bash
sqlite3 db/training.db ".schema"
```
- [ ] `sessions` table exists
- [ ] `matches` table exists
- [ ] `events` table exists

### Event Types Present

```bash
sqlite3 db/training.db "
SELECT event_type, COUNT(*) as count
FROM events
GROUP BY event_type
ORDER BY count DESC;
"
```

**Expected event types:**
| Code | Description | Expected Count |
|------|-------------|----------------|
| T | Tick | Many (physics frames) |
| AG | AI goal change | Several per iteration |
| P | Pickup | At least 1 per iteration |
| SR | Shot release | Several per iteration |
| G | Goal | 1 per iteration (goal mode) |
| SA | Steal attempt | Varies |
| CI | Controller input | Many |

### Session Metadata

```bash
sqlite3 db/training.db "
SELECT
    id,
    start_time,
    profile_name,
    iterations_completed,
    mode
FROM sessions
ORDER BY id DESC
LIMIT 5;
"
```
- [ ] Session records created
- [ ] Profile name matches command line
- [ ] Iterations match actual runs

---

## Metric Checks

### Goals Per Match

```bash
sqlite3 db/training.db "
SELECT
    s.id as session,
    COUNT(DISTINCT CASE WHEN e.event_type='G' THEN e.id END) as goals,
    s.iterations_completed as iterations,
    CAST(COUNT(DISTINCT CASE WHEN e.event_type='G' THEN e.id END) AS FLOAT) /
        MAX(s.iterations_completed, 1) as goals_per_iteration
FROM sessions s
LEFT JOIN matches m ON m.session_id = s.id
LEFT JOIN events e ON e.match_id = m.id
GROUP BY s.id
ORDER BY s.id DESC
LIMIT 5;
"
```
- [ ] Goals per iteration > 1.0 (AI should score sometimes)

### Shot Attempts Per Match

```bash
sqlite3 db/training.db "
SELECT
    s.id as session,
    COUNT(DISTINCT CASE WHEN e.event_type='SR' THEN e.id END) as shots,
    s.iterations_completed as iterations
FROM sessions s
LEFT JOIN matches m ON m.session_id = s.id
LEFT JOIN events e ON e.match_id = m.id
GROUP BY s.id
ORDER BY s.id DESC
LIMIT 5;
"
```
- [ ] Shots per iteration > 3 (AI should try to shoot)

### AI Goal Oscillations

```bash
sqlite3 db/training.db "
WITH goal_changes AS (
    SELECT
        match_id,
        json_extract(data, '$.new_goal') as new_goal,
        json_extract(data, '$.old_goal') as old_goal
    FROM events
    WHERE event_type = 'AG'
)
SELECT
    match_id,
    COUNT(*) as transitions
FROM goal_changes
GROUP BY match_id
HAVING COUNT(*) > 20
ORDER BY transitions DESC
LIMIT 5;
"
```
- [ ] No match has > 20 goal transitions (AI shouldn't flicker)

---

## Output Files

### Session Directory

```bash
ls -la training_logs/session_*/
```
- [ ] `summary.json` exists
- [ ] `analysis.md` exists (human-readable report)
- [ ] `analysis_request_*.md` exists (AI review prompt)

### Summary JSON Structure

```bash
cat training_logs/session_*/summary.json | jq 'keys'
```
- [ ] Contains `session_id`
- [ ] Contains `iterations`
- [ ] Contains `profile`
- [ ] Contains `results` array

---

## Performance Checks

### Training Doesn't Hang

```bash
timeout 60 cargo run --bin training -- -n 2
```
- [ ] Completes within 60 seconds
- [ ] No infinite loops

### Memory Usage

```bash
# Run training and monitor
cargo run --bin training -- -n 5 &
PID=$!
while kill -0 $PID 2>/dev/null; do
    ps -o rss= -p $PID | awk '{print $1/1024 " MB"}'
    sleep 5
done
```
- [ ] Memory stays under 500MB
- [ ] No memory leak (growth < 10MB/iteration)

---

## Common Issues

### "No AI profile found"

```bash
# Check profiles exist
cat config/ai_profiles.txt | head -20
```
- Ensure profile name matches exactly (case-sensitive)

### "Database locked"

```bash
# Check for stale processes
ps aux | grep training
# Kill if needed
pkill -f "target.*training"
```

### AI Doesn't Move

Check AI goal state in events:
```bash
sqlite3 db/training.db "
SELECT tick, json_extract(data, '$.new_goal') as goal
FROM events
WHERE event_type = 'AG'
ORDER BY tick
LIMIT 20;
"
```
- Should see transitions: ChaseBall → AttackWithBall → ChargeShot → ChaseBall

---

*Last updated: 2026-01-30*
