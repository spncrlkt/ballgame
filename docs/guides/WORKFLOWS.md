# Workflows

Multi-step processes for development, testing, and analysis. Each workflow shows the full command chain with next steps built into the binaries.

## Quick Reference

| Workflow | Entry Point | Purpose |
|----------|-------------|---------|
| [Training & Analysis](#training--analysis) | `cargo run --bin training` | Play vs AI, analyze sessions |
| [Offline Training](#offline-training-manual-playtesting) | `tools/offline/manual_todo.md` | Structured playtesting vs champions |
| [Ghost Testing](#ghost-testing) | `cargo run --bin run-ghost` | Test AI against recorded play |
| [Tournament Simulation](#tournament-simulation) | `cargo run --bin simulate -- --tournament` | AI vs AI tournaments |
| [Bracket Tournament](#bracket-tournament) | `cargo run --bin simulate -- --bracket` | Elimination bracket tournaments |
| [Balance Testing](#balance-testing) | `cargo run --bin heatmap` | Shot analysis and tuning |
| [Reachability Mapping](#reachability-mapping) | `cargo run --bin training -- --protocol reachability` | Level coverage analysis |
| [Scenario Testing](#scenario-testing) | `cargo run --bin test-scenarios` | Automated mechanics tests |
| [Visual Regression](#visual-regression) | `./scripts/regression.sh` | Screenshot comparison |
| [Asset Generation](#asset-generation) | `cargo run --bin generate` | Ball textures, showcases |
| [Session Management](#session-management) | (manual) | Start/end dev session checklists |

---

## Training & Analysis

Play 1v1 against AI with full event logging, then analyze the session.

### Workflow Chain

```
training → analyze (training-db) → [iterate or done]
```

### Step 1: Run Training Session

```bash
cargo run --bin training                        # 5 iterations vs Balanced AI
cargo run --bin training -- -n 10               # 10 iterations
cargo run --bin training -- -p v3_Rush_Smart    # vs specific AI
cargo run --bin training -- -l 3                # Force level 3
```

**Output:**
- `db/training.db` - SQLite database with all events
- `training_logs/session_YYYYMMDD_HHMMSS/summary.json` - Session summary

**Next step shown by binary:**
```
NEXT STEPS
──────────
→ cargo run --bin analyze -- --training-db db/training.db
  Generate detailed analysis report
```

### Step 2: Analyze Session

```bash
cargo run --bin analyze -- --training-db db/training.db
```

**Output:**
- Analysis report with win/loss stats, event breakdowns, AI goal transitions

### Step 3: Iterate or Done

Based on analysis, either:
- Run more training with different profiles/levels
- Modify AI code in `src/ai/` based on findings
- Update `docs/project/todo.md` with improvement tasks

### SQL Analysis (Optional)

Query the database directly:

```bash
sqlite3 db/training.db

# Session summary
SELECT s.id, s.session_type, s.created_at, COUNT(m.id) as matches
FROM sessions s LEFT JOIN matches m ON m.session_id = s.id
GROUP BY s.id ORDER BY s.created_at DESC LIMIT 5;

# Win/loss by profile
SELECT right_profile, COUNT(*) as matches,
  SUM(CASE WHEN winner = 'left' THEN 1 ELSE 0 END) as human_wins
FROM matches GROUP BY right_profile;
```

See `docs/guides/TRAINING.md` for full SQL examples.

---

## Offline Training (Manual Playtesting)

Structured human playtesting against top tournament profiles for debug data collection.

### Entry Point

```
tools/offline/manual_todo.md
```

### Purpose

- Collect reachability traces for heatmap generation
- Test AI behavior against tournament-proven profiles
- Gather LOS/shot quality data for balance tuning
- Identify AI navigation issues

### Workflow Chain

```
manual_todo.md → training (--profiles-file) → analyze → [merge DBs] → combined report
```

### Step 1: Load Champions Profiles

```bash
cargo run --bin training -- \
  --profiles-file config/ai_profiles_champions.txt \
  --profile T17_v17_EvoAGPatie_Speed \
  --level "Arena"
```

### Step 2: Follow Per-Level Checklist

See `tools/offline/manual_todo.md` for:
- Level rotation plan (10 champions across 12 levels)
- Per-level tasks (reachability, LOS, AI stress)
- Coverage targets

### Step 3: Analyze Session

```bash
cargo run --bin analyze -- --training-db db/training.db
```

### Step 4: Merge Multiple Sessions (Optional)

```bash
python3 offline_training/merge_training_dbs.py \
  --list offline_training/db_list.txt \
  --out db/combined_offline_training.db

cargo run --bin analyze -- --training-db db/combined_offline_training.db
```

### Top 10 Champions (Reference)

| Rank | Profile | Record |
|------|---------|--------|
| 1 | T17_v15_EvoAG_Speed | 10-2 |
| 2 | T14_v13_EvoD_Patient | 8-2 |
| 3 | T17_v17_EvoAGPatie_Speed | 7-0 |
| 4 | T15_v15_EvoJZone_Aggro | 6-2 |
| 5 | T16_v14_EvoQSpeed_Patient | 5-2 |

Full list in `tools/offline/champions_profiles.txt`.

---

## Ghost Testing

Test AI defense against recorded human play. Training sessions are complete drives (you start with the ball).

### Workflow Chain

```
training → run-ghost → [compare profiles or extract more]
```

### Step 1: Record Training Data

```bash
cargo run --bin training -- -n 5
```

### Step 2: Run Ghost Trials

```bash
cargo run --bin run-ghost training_logs/session_YYYYMMDD_HHMMSS/
cargo run --bin run-ghost training_logs/session_*/ --profile v3_Rush_Smart
cargo run --bin run-ghost training_logs/session_*/ --summary
```

**Next step shown by binary:**
```
NEXT STEPS
──────────
→ cargo run --bin run-ghost -- <dir> --profile <other>
  Test a different AI profile

  cargo run --bin extract-drives -- --db <db>
  Extract more ghost trials from training sessions
```

### ⚠️ Extract Drives (Alternative Entry)

Extract drives from database instead of session directories:

```bash
cargo run --bin extract-drives -- --db db/training.db
cargo run --bin extract-drives -- --db db/training.db --session <session_id>
```

**Status:** Binary exists but limited testing. May require specific database schema.

---

## Tournament Simulation

Run AI vs AI tournaments for balance testing.

### Workflow Chain

```
simulate (tournament) → analyze (bracket) → [tune profiles or done]
```

### Step 1: Run Tournament

```bash
cargo run --bin simulate -- --tournament 5                    # 5 rounds each pair
cargo run --bin simulate -- --tournament 5 --parallel 8       # Parallel execution
cargo run --bin simulate -- --tournament 5 --db results.db    # Save to SQLite
```

**Output:**
- Console summary of wins/losses
- `db/tournament_YYYYMMDD_HHMMSS.db` (if `--db` specified)

**Next step shown by binary:**
```
NEXT STEPS
──────────
→ cargo run --bin analyze -- --bracket --bracket-db db/tournament_*.db
  Analyze tournament results

  cargo run --bin simulate -- --tournament 5 --parallel 8
  Run more tournament rounds
```

### Step 2: Analyze Results

```bash
cargo run --bin analyze -- --bracket --bracket-db db/tournament_*.db
```

### Step 3: Tune and Iterate

Based on analysis:
- Adjust AI profiles in `config/ai_profiles.txt`
- Modify AI decision code in `src/ai/`
- Re-run tournament to verify changes

---

## Bracket Tournament

Elimination-style bracket tournaments with profile evolution.

### Workflow Chain

```
simulate (bracket) → generate_bracket_profiles.py → [new tournament or done]
```

### Step 1: Run Bracket Tournament

```bash
cargo run --bin simulate -- --bracket 16              # 16-profile single elimination
cargo run --bin simulate -- --bracket 16 --parallel 8 # Parallel execution
```

**Next step shown by binary:**
```
NEXT STEPS
──────────
→ python3 scripts/generate_bracket_profiles.py --db db/bracket_*.db
  Generate improved AI profiles from bracket results

  cargo run --bin analyze -- --bracket --bracket-db db/bracket_*.db
  Analyze bracket tournament results
```

### Step 2: Generate New Profiles

```bash
python3 scripts/generate_bracket_profiles.py --db db/bracket_*.db
```

**Output:** New profile definitions based on tournament winners.

### Step 3: Run New Tournament

Add generated profiles to `config/ai_profiles.txt` and run another bracket.

---

## Balance Testing

Visualize and tune shot mechanics.

### Workflow Chain

```
heatmap (score) → verify shot_quality.rs → shot-test → [tune or done]
```

### Step 1: Generate Heatmaps

```bash
cargo run --bin heatmap -- score                      # Per-level scoring heatmaps
cargo run --bin heatmap -- --full --check             # Full bundles for changed levels
cargo run --bin heatmap -- --full --level "Arena"     # Full bundle for one level
```

**Output:**
- `showcase/heatmaps/heatmap_score_<level>_<uuid>_<side>.png`

**Next step shown by binary:**
```
NEXT STEPS
──────────
→ cargo run --bin heatmap -- --check
  Verify heatmaps are up-to-date with level changes

  cargo run --bin heatmap -- --full --level <name>
  Generate full bundle for a specific level
```

### Step 2: Verify AI Quality Estimates

Compare `src/ai/shot_quality.rs` values to heatmap visual zones:

1. Open heatmap PNG in image viewer
2. Sample 5+ key positions
3. Flag if >10% discrepancy between quality value and actual success rate
4. Update formulas if needed

### Step 3: Run Shot Tests

```bash
cargo run --bin simulate -- --shot-test 30 --level 3
cargo run --bin simulate -- --shot-test 50 --level 3  # More iterations for precision
```

**Target:** 40-60% overshoot/undershoot ratio

### Step 4: Tune and Iterate

Adjust physics in `src/constants.rs` or `src/shooting/`, then return to Step 1.

See `docs/dev/balance-testing.md` for detailed workflow.

---

## Reachability Mapping

Generate coverage heatmaps from player exploration data.

### Workflow Chain

```
training (reachability) → export_reachability.py → heatmap (reachability) → verify_reachability
```

### Step 1: Explore Levels

```bash
cargo run --bin training -- --protocol reachability
cargo run --bin training -- --protocol reachability -l "Open Floor"  # Start at specific level
```

**Controls:**
- Move around to record positions
- Press LB/Q to advance to next level
- Escape to quit

**Output:** Position data in `db/training.db` (`debug_events` table)

### Step 2: Export Reachability Data

```bash
python3 scripts/export_reachability.py db/training.db
python3 scripts/export_reachability.py db/training.db --min-samples 50  # Lower threshold
```

**Output:** `showcase/heatmaps/heatmap_reachability_{level}_{id}.txt`

### Step 3: Generate Reachability Heatmaps

```bash
cargo run --bin heatmap -- reachability
cargo run --bin heatmap -- --type reachability
```

### Step 4: Verify Coverage

```bash
cargo run --bin verify_reachability
```

**Next step shown by binary:**
```
NEXT STEPS
──────────
→ cargo run --bin heatmap -- reachability
  Generate reachability heatmaps for all levels

  cargo run --bin heatmap -- --full
  Regenerate all heatmap types
```

---

## Scenario Testing

Run deterministic mechanics tests.

### Workflow Chain

```
test-scenarios → [fix failures] → test-scenarios (-v)
```

### Step 1: Run All Tests

```bash
cargo run --bin test-scenarios              # Run all 35 tests
cargo run --bin test-scenarios -- ball/     # Run category
```

**Next step shown by binary (on failure):**
```
NEXT STEPS
──────────
→ cargo run --bin test-scenarios -- -v
  Run with verbose output to see failure details

  cargo run --bin test-scenarios -- <test_name>
  Run a specific failing test
```

### Step 2: Fix Failures

1. Run verbose to see details: `cargo run --bin test-scenarios -- -v`
2. Run specific test: `cargo run --bin test-scenarios -- ball/pickup`
3. Fix code
4. Re-run to verify

### Test Categories

- `movement/` - Walking, jumping, coyote time
- `ball/` - Pickup, throwing, bouncing
- `shooting/` - Charge mechanics, trajectory
- `steal/` - Range, cooldown, success/failure
- `scoring/` - Basket detection, score updates

---

## Visual Regression

Compare screenshots against baselines.

### Workflow Chain

```
regression.sh → [review diffs] → regression.sh --update (if intentional)
```

### Step 1: Capture and Compare

```bash
./scripts/regression.sh              # Capture and compare all scenarios
./scripts/regression.sh <scenario>   # Single scenario
./scripts/regression.sh --list       # List available scenarios
```

**Output:**
- `showcase/regression/current/*.png` - Latest captures
- `showcase/regression/diffs/*.png` - Visual differences (if ImageMagick installed)
- Exit code: 0 (pass), 1 (fail), 2 (error)

### Step 2: Review Differences

Read the current screenshots to verify changes:
- If intentional change: proceed to Step 3
- If unintentional: fix code and re-run

### Step 3: Update Baselines

```bash
./scripts/regression.sh --update
```

**Output:** Updates `showcase/regression/baselines/*.png`

### Combined Test Script

```bash
./scripts/test-all.sh      # Scenarios + regression
./scripts/test-all.sh -v   # Verbose
./scripts/test-all.sh -s   # Scenarios only
./scripts/test-all.sh -r   # Regression only
```

---

## Asset Generation

Generate ball textures and showcase images.

### Workflow Chain

```
generate ball → generate showcase → [generate levels]
```

### Step 1: Generate Ball Textures

```bash
cargo run --bin generate ball
```

**Output:** `assets/textures/balls/` (1650 PNGs)

**Next step shown by binary:**
```
NEXT STEPS
──────────
→ cargo run --bin generate showcase
  Generate ball styles showcase image
```

### Step 2: Generate Showcase

```bash
cargo run --bin generate showcase
```

**Output:** `showcase/ball_styles_showcase.png`

### Step 3: Generate Level Showcase (Optional)

```bash
cargo run --bin generate levels
```

**Output:** `showcase/level_showcase.png`

### Other Generate Commands

```bash
cargo run --bin generate gif wedge     # Wedge rotation GIF
cargo run --bin generate gif baseball  # Baseball rotation GIF
```

---

## Session Management

Checklists for development sessions.

### Get Started Checklist

Run at the beginning of each working session:

- [ ] Read `docs/project/todo.md` - Check current sprint tasks
- [ ] Read `docs/project/open_questions.md` - Review pending decisions
- [ ] Check git status - Note uncommitted work
- [ ] Run `cargo check` - Verify compilation
- [ ] Run `./scripts/regression.sh` - Verify visual baseline matches
- [ ] Identify scope - Decide which task(s) to work on

### Close Down Checklist

Run at the end of each session (or after ~10 changes):

- [ ] Run `cargo check` - Verify compilation
- [ ] Run `cargo clippy` - Check for warnings
- [ ] Run `./scripts/regression.sh` - Visual regression test
- [ ] Update baseline if needed - `./scripts/regression.sh --update`
- [ ] Read `showcase/regression/current/*.png` - Verify UI looks correct
- [ ] Update `docs/project/todo.md` - Mark completed, add new items
- [ ] Archive done items - Keep only last 5, move older to `docs/project/todone.md`
- [ ] Update `docs/project/open_questions.md` - Add new questions
- [ ] Update `docs/dev/audit_record.md` - Document changes
- [ ] Verify CLAUDE.md accuracy - Update if architecture changed

---

## ⚠️ Incomplete/Untested Workflows

These workflows exist in documentation or code but may need verification:

### Variant Tournament Testing

**Status:** Documented in `docs/archive/variant_tournament_workflow.md` but `scripts/run_variant_tournaments.py` does not exist.

**Intended workflow:**
```
run_variant_tournaments.py → per-variant tournaments → pairwise analysis → summary report
```

**What it should do:**
1. Apply variant constants to `src/constants.rs`
2. Modify top 4 AI profiles
3. Run tournament per variant
4. Compare results via event audit

**To fix:** Create `scripts/run_variant_tournaments.py` or remove from docs.

### Python Script Dependencies

These Python scripts exist but require verification:

```bash
scripts/export_reachability.py    # Requires: sqlite3
scripts/generate_bracket_profiles.py  # Requires: sqlite3
scripts/generate_v12_profiles.py  # Unclear purpose
scripts/heatmap_from_db.py        # Generate heatmaps from event logs
scripts/heatmap_svg.py            # SVG heatmap generation
```

**To verify:** Run each script with `--help` or test with sample data.

### Extract Drives Binary

**Status:** Binary exists (`src/bin/extract-drives.rs`) with next-step integration, but workflow not fully documented.

**Intended use:**
```bash
cargo run --bin extract-drives -- --db db/training.db
cargo run --bin extract-drives -- --db db/training.db --session <id>
```

**To verify:** Test with actual training database.

---

## Workflow Outputs Summary

| Workflow | Primary Output | Location |
|----------|----------------|----------|
| Training | SQLite events | `db/training.db` |
| Training | Session summary | `training_logs/session_*/summary.json` |
| Ghost | Trial results | Console output |
| Tournament | Match results | `db/tournament_*.db` |
| Heatmap | Score maps | `showcase/heatmaps/*.png` |
| Scenarios | Test results | Console output |
| Regression | Screenshots | `showcase/regression/current/*.png` |
| Generate | Textures | `assets/textures/balls/` |
| Generate | Showcases | `showcase/*.png` |

---

## Database Files

| Database | Created By | Purpose |
|----------|------------|---------|
| `db/training.db` | training binary | All training events |
| `db/tournament_*.db` | simulate --tournament | Tournament match results |
| `db/bracket_*.db` | simulate --bracket | Bracket tournament results |
