# Binary Reference

Complete reference for all executables in the project. For workflow-oriented documentation showing how binaries chain together, see [WORKFLOWS.md](WORKFLOWS.md).

## Quick Reference

| Binary | Purpose | Primary Output |
|--------|---------|----------------|
| `ballgame` | Main game | Interactive play |
| `training` | 1v1 vs AI with logging | `db/training_*.db` |
| `simulate` | Headless AI vs AI | `db/tournament_*.db`, `db/bracket_*.db` |
| `analyze` | Analytics and reporting | Console/file reports |
| `test-scenarios` | Deterministic mechanics tests | Pass/fail results |
| `heatmap` | Shot/reachability heatmaps | `showcase/heatmaps/*.png` |
| `run-ghost` | Ghost trial playback | Trial results |
| `generate` | Asset generation | `assets/textures/`, `showcase/*.png` |
| `extract-drives` | Extract input sequences | `ghost_trials/*.ghost` |
| `verify_reachability` | Verify heatmap coverage | Console report |
| `gamepad_debug` | Controller debugging | Console output |

---

## Main Game (ballgame)

Interactive 2v2 ball sport game.

```bash
cargo run                              # Play the game
cargo run --release                    # Play with optimizations
cargo run -- --replay-db <MATCH_ID>    # Replay a recorded match
cargo run -- --screenshot-and-quit     # Screenshot and exit (testing)
```

### Flags

| Flag | Description |
|------|-------------|
| `--replay-db <ID>` | Replay match from SQLite database |
| `--screenshot-and-quit` | Capture screenshot and exit immediately |

### Controls

See [HOW_TO_PLAY.md](HOW_TO_PLAY.md) for full controls.

---

## training

Play 1v1 against AI with comprehensive event logging for analysis.

```bash
cargo run --bin training                              # Default settings
cargo run --bin training -- -n 10                     # 10 iterations
cargo run --bin training -- -p v3_Rush_Smart          # vs specific profile
cargo run --bin training -- --protocol pursuit        # Pursuit test protocol
cargo run --bin training -- --protocol reachability   # Level exploration
cargo run --bin training -- --help                    # Show all options
```

### Flags

| Flag | Short | Description | Default |
|------|-------|-------------|---------|
| `--protocol NAME` | | Training protocol | `advanced-platform` |
| `--mode MODE` | `-m` | `goal` or `game` | `goal` |
| `--iterations N` | `-n` | Number of iterations | `5` |
| `--win-score N` | `-w` | Points to win (game mode) | `5` |
| `--profile NAME` | `-p` | AI opponent profile | `Balanced` |
| `--level N/NAME` | `-l` | Force specific level | random |
| `--seed N` | `-s` | RNG seed for determinism | random |
| `--time-limit SECS` | `-t` | Time limit per iteration | none |
| `--first-point-timeout SECS` | | End if no score within SECS | none |
| `--viewport N` | | Viewport preset index | `2` |
| `--palette N` | | Color palette index | `0` |
| `--ball-style NAME` | | Ball visual style | random |
| `--drive-mode` | | Start with ball, first point wins | off |
| `--headless` | | Run without window | off |
| `--profiles-file PATH` | | Custom AI profiles file | `config/ai_profiles.txt` |
| `--profile-list PATH` | | Profile names file for multi-profile testing | |
| `--debug-log` | | Enable debug sample logging to SQLite | off |
| `--help` | `-h` | Show help | |

### Protocols

| Protocol | Description |
|----------|-------------|
| `advanced-platform` | Full 1v1 games on random levels (default) |
| `pursuit` | Flat level chase test (verifies AI pursues player) |
| `pursuit2` | Platform chase test (pursuit with obstacle) |
| `reachability` | Solo level exploration for coverage mapping (LB to advance) |
| `auto-reachability` | Automated random walk/hop exploration (headless compatible) |

### Output

- `db/training_YYYYMMDD_HHMMSS.db` - SQLite database with all events
- `training_logs/session_YYYYMMDD_HHMMSS/summary.json` - Session summary

### Settings Files

- `config/training_settings.json` - Local settings (gitignored)
- `config/training_settings.template.json` - Template with defaults

CLI arguments override file settings.

---

## simulate

Headless AI vs AI simulation for testing and tournaments.

```bash
cargo run --bin simulate -- --help                    # Show all options
cargo run --bin simulate -- --level 3 --left Balanced --right Aggressive
cargo run --bin simulate -- --tournament 5 --parallel 8
cargo run --bin simulate -- --bracket 64 --parallel 16
cargo run --bin simulate -- --shot-test 30 --level 3
```

### Flags

| Flag | Description | Default |
|------|-------------|---------|
| `--settings FILE` | Load settings from JSON file | |
| `--level N` | Level number (1-12) | random |
| `--levels LIST` | Comma-separated level numbers | all |
| `--profiles LIST` | Comma-separated profile names | all |
| `--profiles-file FILE` | Custom profiles file | `config/ai_profiles.txt` |
| `--left PROFILE` | Left player AI profile | `Balanced` |
| `--right PROFILE` | Right player AI profile | `Balanced` |
| `--duration SECS` | Match duration limit | `60` |
| `--score-limit N` | End when player reaches N points | no limit |
| `--matches N` | Run N matches with same config | |
| `--tournament [N]` | All profile combinations, N matches each | `5` |
| `--level-sweep [N]` | Test profile across all levels | `3` |
| `--regression` | Compare to baseline metrics | |
| `--shot-test [N]` | Shot accuracy test, N shots per position | `30` |
| `--ghost PATH` | Run ghost trials from file/directory | |
| `--multihop-test` | Test NavGraph multi-hop reachability | |
| `--reachability-test` | Validate NavGraph against exploration data | |
| `--samples N` | Samples for reachability test | `50` |
| `--bracket [N]` | Double elimination bracket, N entrants | `64` |
| `--best-of N` | Games per bracket match | `3` |
| `--warmup-seeding [PROFILE] [GAMES]` | Seed bracket by win rate vs baseline | |
| `--seed N` | RNG seed for reproducibility | random |
| `--output FILE` | Output JSON to file | stdout |
| `--quiet`, `-q` | Suppress progress output | |
| `--parallel N` | Run simulations with N threads | sequential |
| `--db FILE` | Store results in SQLite database | |
| `--est-run-time` | Estimate runtime and exit | |
| `--run-timeout SECS` | Wall-clock timeout for run | `600` |
| `--debug-log` | Enable debug sample logging | off |
| `--help`, `-h` | Show help | |

### Output

- `db/tournament_YYYYMMDD_HHMMSS.db` - Tournament results
- `db/bracket_YYYYMMDD_HHMMSS.db` - Bracket tournament results

---

## analyze

Analyze simulation results and generate reports.

```bash
cargo run --bin analyze -- --help
cargo run --bin analyze -- db/training.db
cargo run --bin analyze -- --training-db db/training_*.db
cargo run --bin analyze -- --bracket --bracket-db db/bracket_*.db
cargo run --bin analyze -- --event-audit db/baseline.db db/current.db
```

### Flags

| Flag | Description |
|------|-------------|
| `DB_PATH` | SQLite database path (positional) |
| `--targets FILE` | Load tuning targets from TOML file |
| `--output FILE`, `-o` | Write full report to file |
| `--event-audit BASE CURRENT` | Compare two DBs via event audit |
| `--audit-output FILE` | Write audit report to file |
| `--focused DB` | Run focused analysis on single DB |
| `--focused-output FILE` | Write focused report to file |
| `--training-db DB` | Run training debug analysis |
| `--training-output DIR` | Output directory for training analysis |
| `--bracket` | Analyze most recent bracket tournament |
| `--bracket-db DB` | Override DB path for bracket analysis |
| `--bracket-output DIR` | Output directory for bracket reports |
| `--bracket-rankings` | Export standings to auto-generated rankings file |
| `--request NAME` | Run a stored SQL analysis request |
| `--request-output FILE` | Write request report to file |
| `--request-db DB` | Override DB path for request |
| `--request-list` | List available analysis requests |
| `--requests-file FILE` | Use alternate requests file |
| `--request-add NAME` | Add new request (requires `--request-sql`) |
| `--request-sql SQL` | SQL for `--request-add` |
| `--request-desc TEXT` | Description for `--request-add` |
| `--update-defaults` | Update default profiles in `src/constants.rs` |
| `--help`, `-h` | Show help |

---

## test-scenarios

Run deterministic mechanics tests.

```bash
cargo run --bin test-scenarios                # Run all tests
cargo run --bin test-scenarios -- ball/       # Run category
cargo run --bin test-scenarios -- -v          # Verbose (show failures)
cargo run --bin test-scenarios -- ball/pickup # Run specific test
```

### Flags

| Flag | Short | Description |
|------|-------|-------------|
| `--verbose` | `-v` | Show failure details |
| `PATTERN` | | Filter tests by name/category |

### Test Categories

- `movement/` - Walking, jumping, coyote time
- `ball/` - Pickup, throwing, bouncing
- `shooting/` - Charge mechanics, trajectory
- `steal/` - Range, cooldown, success/failure
- `scoring/` - Basket detection, score updates

### Output

Console pass/fail results. Exit code 0 on success, 1 on failure.

---

## heatmap

Generate heatmaps for shot analysis and level coverage.

```bash
cargo run --bin heatmap                           # Default: speed heatmap
cargo run --bin heatmap -- score                  # Scoring percentage heatmaps
cargo run --bin heatmap -- score --fast           # Quick iteration (25 trials)
cargo run --bin heatmap -- score --accurate       # Publication quality (100 trials)
cargo run --bin heatmap -- --type reachability    # Reachability heatmaps
cargo run --bin heatmap -- --full --level "Arena" # Full bundle for one level
cargo run --bin heatmap -- --full --check         # Generate for changed levels
cargo run --bin heatmap -- --full --refresh       # Regenerate everything
```

### Flags

| Flag | Description |
|------|-------------|
| `speed` | Shot angle and required speed (default) |
| `score` | Scoring percentage via Monte Carlo |
| `--type TYPE` | Heatmap type (speed, score, reachability) |
| `--level NAME/UUID` | Generate for specific level only |
| `--fast` | Quick iteration (25 trials, ~4x faster) |
| `--accurate` | Publication quality (100 trials) |
| `--full` | Generate full bundles |
| `--check` | Only generate for changed/new levels |
| `--refresh` | Clear and regenerate all |
| `--clear-manual-trainings` | Also clear manually trained heatmaps |

### Output

- `showcase/heatmaps/heatmap_speed_<level>_<uuid>.png`
- `showcase/heatmaps/heatmap_score_<level>_<uuid>_<side>.png`
- `showcase/heatmaps/heatmap_reachability_<level>_<uuid>.txt`
- `showcase/heatmap_<type>_all.png` - Combined sheet

---

## run-ghost

Play back recorded human inputs against AI to test defensive capability.

```bash
cargo run --bin run-ghost training_logs/session_*/
cargo run --bin run-ghost training_logs/session_*/ --profile v3_Rush_Smart
cargo run --bin run-ghost training_logs/session_*/ --summary
cargo run --bin run-ghost ghost_trials/trial_001.ghost
```

### Flags

| Flag | Description |
|------|-------------|
| `PATH` | Ghost trial file or directory (required) |
| `--profile NAME` | AI profile to test against |
| `--summary` | Show summary only |
| `--verbose` | Verbose output |
| `--debug-log` | Enable debug logging |

### Input

- `training_logs/session_*/` - Session directories with ghost data
- `ghost_trials/*.ghost` - Individual ghost trial files

### Output

Console results showing AI defensive performance against recorded human play.

---

## generate

Generate game assets (textures, showcases, GIFs).

```bash
cargo run --bin generate ball               # Generate ball textures
cargo run --bin generate showcase           # Generate ball styles showcase
cargo run --bin generate levels             # Generate level showcase grid
cargo run --bin generate gif wedge          # Generate wedge rotation GIF
cargo run --bin generate gif baseball       # Generate baseball rotation GIF
cargo run --bin generate --help             # Show help
```

### Subcommands

| Subcommand | Description | Output |
|------------|-------------|--------|
| `ball` | Generate ball textures (all styles x palettes) | `assets/textures/balls/` |
| `showcase` | Generate ball styles showcase image | `showcase/ball_styles_showcase.png` |
| `levels` | Generate level showcase grid | `showcase/level_showcase.png` |
| `gif wedge` | Generate wedge rotation GIF | `showcase/wedge_rotation.gif` |
| `gif baseball` | Generate baseball rotation GIF | `showcase/baseball_rotation.gif` |

---

## extract-drives

Extract player input sequences as "drives" from SQLite event logs.

```bash
cargo run --bin extract-drives -- --db db/training.db
cargo run --bin extract-drives -- --db db/training.db --session <SESSION_ID>
cargo run --bin extract-drives -- --db db/training.db --match <MATCH_ID> --output ghost_trials/
```

### Flags

| Flag | Description |
|------|-------------|
| `--db PATH` | SQLite database path (required) |
| `--session ID` | Extract drives from specific session |
| `--match ID` | Extract drives from specific match |
| `--output DIR` | Output directory for ghost files |

### Output

- `ghost_trials/drive_*.ghost` - Ghost trial files for playback with `run-ghost`

---

## verify_reachability

Verify that reachability heatmaps are properly loaded and have varied data.

```bash
cargo run --bin verify_reachability
```

### Flags

None.

### Output

Console report showing which levels have reachability data and whether the data has sufficient variation.

---

## gamepad_debug

Minimal controller debugging tool. Prints all controller events to console.

```bash
cargo run --bin gamepad_debug
```

### Flags

None.

### Controls

- Press any button or move any stick to see events
- Press Escape to quit

### Output

Console output showing controller connections, button presses, and axis values.

---

## Configuration Files

Many binaries load settings from configuration files:

| Binary | Settings File | Template |
|--------|---------------|----------|
| `training` | `config/training_settings.json` | `config/training_settings.template.json` |
| `simulate` | `config/simulation_settings.json` | `config/simulation_settings.template.json` |

CLI arguments always override file settings.

---

## Database Files

| Database | Created By | Purpose |
|----------|------------|---------|
| `db/training_*.db` | `training` | Training session events |
| `db/tournament_*.db` | `simulate --tournament` | Tournament match results |
| `db/bracket_*.db` | `simulate --bracket` | Bracket tournament results |

Query databases directly with SQLite:

```bash
sqlite3 db/training.db "SELECT event_type, COUNT(*) FROM events GROUP BY event_type;"
```

See [TRAINING.md](TRAINING.md) for SQL query examples.
