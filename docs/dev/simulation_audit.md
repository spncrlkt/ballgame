# Simulation Workflow Audit

Run this checklist after changes to:
- `src/bin/simulate.rs`
- `src/simulation/`
- `src/shooting/` (for shot-test)
- `src/ai/` (for tournament/bracket)

---

## Shot Test

### Basic Shot Test

```bash
cargo run --bin simulate -- --shot-test 30 --level 3
```

**Expected output:**
- Shot count: 30
- Level: 3
- Over/under ratio displayed
- No crashes or infinite loops

**Validation:**
- [ ] Over/under ratio between 40-60%
- [ ] All 30 shots complete
- [ ] Execution time < 30 seconds

### Multi-Level Shot Test

```bash
for level in 1 3 5 7; do
    echo "Level $level:"
    cargo run --bin simulate -- --shot-test 20 --level $level 2>&1 | grep -E "ratio|accuracy"
done
```

- [ ] Each level completes
- [ ] Ratios vary by level (harder levels = lower accuracy)

---

## Tournament

### Basic Tournament

```bash
cargo run --bin simulate -- --tournament 3 --parallel 4
```

**Expected:**
- [ ] 3 profiles compete in round-robin
- [ ] Uses 4 parallel threads
- [ ] All matches complete
- [ ] Results written to SQLite

### Tournament Results

```bash
sqlite3 db/simulation.db "
SELECT
    profile_name,
    COUNT(*) as matches,
    SUM(CASE WHEN winner = profile_id THEN 1 ELSE 0 END) as wins,
    printf('%.1f%%', 100.0 * SUM(CASE WHEN winner = profile_id THEN 1 ELSE 0 END) / COUNT(*)) as win_rate
FROM (
    SELECT
        m.id,
        p1.id as p1_id, p1.name as p1_name,
        p2.id as p2_id, p2.name as p2_name,
        m.winner
    FROM matches m
    JOIN profiles p1 ON m.profile1_id = p1.id
    JOIN profiles p2 ON m.profile2_id = p2.id
) combined
CROSS JOIN (
    SELECT profile_id, profile_name FROM (
        SELECT p1_id as profile_id, p1_name as profile_name FROM combined
        UNION
        SELECT p2_id, p2_name FROM combined
    )
) profiles
WHERE combined.p1_id = profiles.profile_id OR combined.p2_id = profiles.profile_id
GROUP BY profile_id
ORDER BY wins DESC;
"
```

**Validation:**
- [ ] Each profile has matches recorded
- [ ] No profile wins 100% (indicates bug)
- [ ] No profile wins 0% (indicates bug)
- [ ] Win rates spread across range

---

## Bracket

### Basic Bracket

```bash
cargo run --bin simulate -- --bracket 8 --best-of 3
```

**Expected:**
- [ ] 8-player double elimination bracket
- [ ] Best-of-3 matches
- [ ] Winner determined
- [ ] Results stored in SQLite

### Bracket Seeding

```bash
cargo run --bin simulate -- --bracket 4 --best-of 1 2>&1 | grep -E "Match|seed"
```

**Validation:**
- [ ] Match 0: seed 1 vs seed 4
- [ ] Match 1: seed 2 vs seed 3
- [ ] Proper bracket structure

### Bracket Results

```bash
sqlite3 db/simulation.db "
SELECT
    bracket_id,
    COUNT(*) as total_matches,
    COUNT(DISTINCT winner) as unique_winners,
    MAX(round) as final_round
FROM bracket_matches
GROUP BY bracket_id
ORDER BY bracket_id DESC
LIMIT 5;
"
```

- [ ] Total matches appropriate for bracket size
- [ ] Single champion (final unique winner = 1)

---

## Reachability Test

```bash
cargo run --bin simulate -- --reachability-test
```

**Expected:**
- [ ] Samples positions across level
- [ ] Reports reachable percentage
- [ ] No crashes

### Reachability by Level

```bash
for level in 1 3 5 7; do
    echo "Level $level:"
    cargo run --bin simulate -- --reachability-test --level $level 2>&1 | grep -E "reachable|coverage"
done
```

- [ ] Coverage varies by level complexity
- [ ] All levels have > 50% coverage

---

## Multihop Test

```bash
cargo run --bin simulate -- --multihop-test
```

**Expected:**
- [ ] Tests pathfinding across platforms
- [ ] Reports success/failure rates
- [ ] Identifies unreachable positions

---

## Database Verification

### Schema Check

```bash
sqlite3 db/simulation.db ".tables"
```

**Expected tables:**
- [ ] `profiles`
- [ ] `matches`
- [ ] `bracket_matches` (if bracket run)
- [ ] `events`

### Match Data

```bash
sqlite3 db/simulation.db "
SELECT
    m.id,
    p1.name as player1,
    p2.name as player2,
    m.score1,
    m.score2,
    CASE WHEN m.winner = p1.id THEN p1.name ELSE p2.name END as winner
FROM matches m
JOIN profiles p1 ON m.profile1_id = p1.id
JOIN profiles p2 ON m.profile2_id = p2.id
ORDER BY m.id DESC
LIMIT 10;
"
```

- [ ] Scores recorded correctly
- [ ] Winner matches higher score

---

## Performance Checks

### Tournament Performance

```bash
time cargo run --bin simulate -- --tournament 5 --parallel 8
```

- [ ] 5-profile tournament completes in < 5 minutes
- [ ] Parallelization working (check CPU usage)

### Shot Test Performance

```bash
time cargo run --bin simulate -- --shot-test 100 --level 3
```

- [ ] 100 shots complete in < 60 seconds

---

## Balance Verification

### Shot Accuracy Target

```bash
cargo run --bin simulate -- --shot-test 50 --level 3 2>&1 | grep -E "over|under"
```

**Target:** Over/under ratio between 40-60%

If outside range:
- < 40%: Shots too weak, increase boost or reduce variance
- > 60%: Shots too strong, reduce boost or increase variance

### AI Balance

```bash
# Run tournament with all profiles
cargo run --bin simulate -- --tournament 10 --parallel 8

# Check for dominant strategy
sqlite3 db/simulation.db "
SELECT
    name,
    wins,
    losses,
    printf('%.1f%%', 100.0 * wins / (wins + losses)) as win_rate
FROM (
    SELECT
        p.name,
        SUM(CASE WHEN m.winner = p.id THEN 1 ELSE 0 END) as wins,
        SUM(CASE WHEN m.winner != p.id AND (m.profile1_id = p.id OR m.profile2_id = p.id) THEN 1 ELSE 0 END) as losses
    FROM profiles p
    LEFT JOIN matches m ON m.profile1_id = p.id OR m.profile2_id = p.id
    GROUP BY p.id
)
WHERE wins + losses > 0
ORDER BY win_rate DESC;
"
```

**Target:** No profile > 70% or < 30% win rate

---

## Common Issues

### "Thread panic"

Check for race conditions:
```bash
RUST_BACKTRACE=1 cargo run --bin simulate -- --tournament 3 --parallel 1
```
- Run with single thread to isolate

### "Database locked"

```bash
# Check for stale processes
ps aux | grep simulate
# Remove stale lock
rm -f db/simulation.db-journal
```

### Bracket Never Completes

Check for infinite loops in match simulation:
```bash
timeout 120 cargo run --bin simulate -- --bracket 4 --best-of 1
```
- Should complete in < 2 minutes for 4 players

---

*Last updated: 2026-01-30*
