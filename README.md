# Ballgame

A 2v2 ball sport game built with Bevy 0.17.3.

## Quick Start

```bash
cargo run              # Play the game
cargo run --release    # Play with optimizations
```

**Guides:**
- `HOW_TO_PLAY.md` - Controls and gameplay
- `TRAINING.md` - Training mode setup and analysis workflow

---

## All Binaries

| Binary | Purpose |
|--------|---------|
| `ballgame` | Main game |
| `training` | 1v1 vs AI with event logging |
| `simulate` | Headless AI vs AI simulation |
| `analyze` | Analyze event logs, generate reports |
| `run-ghost` | Run ghost trials (recorded inputs vs AI) |
| `extract-drives` | Extract drives from evlogs to ghost files |
| `test-scenarios` | Run scenario tests |
| `heatmap` | Generate scoring/shot heatmaps |

### Main Game

```bash
cargo run                              # Play
cargo run -- --replay <file.evlog>     # Replay a recorded game
cargo run -- --screenshot-and-quit     # Screenshot and exit (for testing)
```

### Training Mode

Play 1v1 against AI with full event logging for analysis.

```bash
cargo run --bin training                              # Default: 1 game vs Balanced
cargo run --bin training -- --games 5                 # 5 games
cargo run --bin training -- --games 3 --profile v3_Rush_Smart  # vs specific profile
```

Output: `training_logs/session_YYYYMMDD_HHMMSS/`

### Simulation (Headless)

Fast AI vs AI matches for testing and tournaments.

```bash
cargo run --bin simulate -- --help

# Key options:
#   --level <N>         Level 1-12 (default: random)
#   --left <PROFILE>    Left AI profile
#   --right <PROFILE>   Right AI profile
#   --duration <SECS>   Time limit (default: 60)
#   --matches <N>       Run N matches
#   --tournament [N]    All profile pairs, N rounds each
#   --shot-test [N]     Shot accuracy test (N iterations)
#   --log-events        Save .evlog files
#   --log-dir <DIR>     Where to save logs
#   --parallel <N>      Parallel workers (default: CPU count)
```

**Examples:**
```bash
cargo run --bin simulate -- --level 3 --left v2_Balanced --right v3_Rush_Smart
cargo run --bin simulate -- --tournament 5 --parallel 8
cargo run --bin simulate -- --shot-test 30 --level 3
```

### Ghost System

Test AI defense against recorded human play. Training sessions are complete drives (you start with the ball).

**Step 1: Record training games**
```bash
cargo run --bin training -- --games 5
```

**Step 2: Run ghost trials against AI** (no extraction needed)
```bash
cargo run --bin run-ghost training_logs/session_YYYYMMDD_HHMMSS/
cargo run --bin run-ghost training_logs/session_*/ --profile v3_Rush_Smart
cargo run --bin run-ghost training_logs/session_*/ --summary
```

### Analytics

```bash
cargo run --bin analyze -- training_logs/session_YYYYMMDD_HHMMSS/
cargo run --bin analyze -- logs/ --output report.txt
```

### Scenario Tests

```bash
cargo run --bin test-scenarios              # Run all 35 tests
cargo run --bin test-scenarios -- ball/     # Run category
cargo run --bin test-scenarios -- -v        # Verbose (show failures)
```

### Heatmaps

```bash
cargo run --bin heatmap -- score            # Scoring probability heatmap
cargo run --bin heatmap -- shots            # Shot distribution heatmap
```

---

## Controls Reference

See `HOW_TO_PLAY.md` for full controls, or quick reference:

| Action | Keyboard | Gamepad |
|--------|----------|---------|
| Move | A/D | Left Stick |
| Jump | Space/W | A (South) |
| Pickup/Steal | E | X (West) |
| Throw (hold) | F | RB (Right Bumper) |
| Cycle player | Q | LB (Left Bumper) |
| Reset level | R | Start |

---

## AI Profiles

53 profiles organized by lineage:
- `v1_*` - Original 5 profiles
- `v2_*` - Tournament champions (4 profiles)
- `v3_*` - Evolved variants (44 profiles)

**Key profiles:**
- `v2_Balanced` - Good all-around default
- `v3_Rush_Smart` - Fast, aggressive, improved decision-making
- `v3_Steady_Deep` - Patient, high-IQ defensive player
- `v3_Spec_Chaos` - Unpredictable, fast, low shot quality threshold

---

## Scripts

```bash
./scripts/screenshot.sh          # Capture screenshot
./scripts/regression.sh          # Compare to baseline
./scripts/regression.sh --update # Update baseline
```

## Build Commands

```bash
cargo build              # Debug build
cargo build --release    # Release build
cargo check              # Check without building
cargo fmt                # Format code
cargo clippy             # Lint
```

## Project Files

| File | Purpose |
|------|---------|
| `HOW_TO_PLAY.md` | Controls and gameplay guide |
| `TRAINING.md` | Training mode setup and analysis |
| `CLAUDE.md` | Development instructions, architecture docs |
| `todo.md` | Current sprint tasks |
| `milestones.md` | Full project plan (MVP -> V0 -> V1) |
| `open_questions.md` | Pending decisions |
| `todone.md` | Completed work archive |

## Reference

- [Bevy physics in fixed timestep](https://github.com/bevyengine/bevy/blob/main/examples/movement/physics_in_fixed_timestep.rs)
- [Bevy breakout example](https://github.com/bevyengine/bevy/blob/main/examples/games/breakout.rs)
