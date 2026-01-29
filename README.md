# Ballgame

A 2v2 ball sport game built with Bevy 0.17.3.


## Quick Start

```bash
cargo run              # Play the game
cargo run --release    # Play with optimizations
```

**Guides:**
- [`docs/guides/HOW_TO_PLAY.md`](docs/guides/HOW_TO_PLAY.md) - Controls and gameplay
- [`docs/guides/TRAINING.md`](docs/guides/TRAINING.md) - Training mode setup and analysis workflow

## Quick Reference

| What | Command |
|------|---------|
| Play | `cargo run` |
| Train | `cargo run --bin training` |
| Test | `cargo run --bin test-scenarios` |
| Simulate | `cargo run --bin simulate -- --tournament 5` |

---

## Generated Assets

Run offline workflows to generate analysis files:

```bash
cargo run --bin heatmap -- --full --check   # Heatmaps for new/changed levels
cargo run --bin generate ball               # Ball textures (all styles × palettes)
cargo run --bin generate showcase           # Ball styles showcase image
cargo run --bin generate levels             # Level showcase grid
```

**Output files:**
- `showcase/heatmaps/` - Shot probability maps per level
- `showcase/level_showcase.png` - All levels grid
- `showcase/ball_styles_showcase.png` - All ball styles
- `assets/textures/balls/` - Ball texture PNGs

---

## All Binaries

| Binary | Purpose |
|--------|---------|
| `ballgame` | Main game |
| `training` | 1v1 vs AI with event logging |
| `simulate` | Headless AI vs AI simulation |
| `analyze` | Analyze training sessions, generate reports |
| `run-ghost` | Run ghost trials (recorded inputs vs AI) |
| `test-scenarios` | Run scenario tests |
| `heatmap` | Generate per-level heatmaps (score, speed, reachability, etc.) |
| `generate` | Generate assets (ball textures, showcases, GIFs) |

### Main Game

```bash
cargo run                              # Play
cargo run -- --replay-db <match_id>    # Replay a recorded match from SQLite
cargo run -- --screenshot-and-quit     # Screenshot and exit (for testing)
```

### Training Mode

Play 1v1 against AI with full event logging for analysis.

```bash
cargo run --bin training                              # Default: 5 iterations vs Balanced
cargo run --bin training -- -n 10                     # 10 iterations
cargo run --bin training -- -n 3 -p v3_Rush_Smart     # vs specific profile
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

**Step 1: Record training iterations**
```bash
cargo run --bin training -- -n 5
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
cargo run --bin heatmap -- score                  # Per-level scoring heatmaps (left/right)
cargo run --bin heatmap -- --type reachability    # Reachability heatmaps
cargo run --bin heatmap -- --full --level "Arena" # Full bundle for one level
cargo run --bin heatmap -- --full --check         # Full bundles for changed/new levels
cargo run --bin heatmap -- --full --refresh       # Regenerate everything
```

---

## Controls Reference

See [`docs/guides/HOW_TO_PLAY.md`](docs/guides/HOW_TO_PLAY.md) for full controls, or quick reference:

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

## Project Structure

```
ballgame/
├── CLAUDE.md                 # AI assistant guidance, architecture docs
├── README.md                 # This file
├── Cargo.toml
│
├── config/                   # Game configuration files
│   ├── ai_profiles.txt       # 53 AI personality definitions
│   ├── levels.txt            # Level definitions
│   ├── palettes.txt          # Color palettes (30)
│   ├── ball_options.txt      # Ball style definitions
│   ├── game_presets.txt      # Physics/movement presets
│   └── init_settings.json    # Saved user preferences
│
├── db/                       # SQLite databases (training.db, simulation.db)
│
├── docs/
│   ├── project/              # Task tracking (todo.md, milestones.md)
│   ├── design/               # Design documents (functional_spec.md)
│   ├── planning/             # Active implementation plans
│   ├── dev/                  # Developer reference (guidelines, workflows)
│   ├── guides/               # User-facing guides
│   └── archive/              # Completed plans, historical docs
│
├── showcase/                 # Generated outputs
│   ├── snapshots/            # Game state captures (F4)
│   ├── regression/           # Visual regression baselines
│   ├── heatmaps/             # Shot analysis PNGs
│   └── rankings/             # Tournament results
│
├── assets/
│   └── textures/balls/       # Ball texture PNGs (1650)
│
├── tools/                    # Offline tooling (analysis, training scripts)
│   ├── offline/              # Offline training scripts
│   ├── analysis/             # Tournament and analysis scripts
│   └── config/               # Analysis config (heatmap variants)
│
├── src/                      # Source code
├── scripts/                  # Build/test scripts
├── tests/                    # Test files (scenarios/, fixtures/)
└── training_logs/            # Training session data
```

## Quick Links

**Most Used:**
| File | Purpose |
|------|---------|
| [`docs/project/todo.md`](docs/project/todo.md) | Current sprint tasks |
| [`docs/project/milestones.md`](docs/project/milestones.md) | Full project plan (MVP → V0 → V1) |
| [`config/ai_profiles.txt`](config/ai_profiles.txt) | AI personality definitions |
| [`config/levels.txt`](config/levels.txt) | Level definitions |
| [`showcase/`](showcase/) | Generated outputs (heatmaps, snapshots, regression) |

**Guides:**
| File | Purpose |
|------|---------|
| [`docs/guides/HOW_TO_PLAY.md`](docs/guides/HOW_TO_PLAY.md) | Controls and gameplay |
| [`docs/guides/TRAINING.md`](docs/guides/TRAINING.md) | Training mode setup |
| [`docs/guides/TESTING.md`](docs/guides/TESTING.md) | Running tests |

**Development:**
| File | Purpose |
|------|---------|
| [`CLAUDE.md`](CLAUDE.md) | Architecture, patterns, dev workflow |
| [`docs/dev/code_review_guidelines.md`](docs/dev/code_review_guidelines.md) | Code review best practices |
| [`docs/dev/balance-testing.md`](docs/dev/balance-testing.md) | Balance tuning process |
| [`docs/project/open_questions.md`](docs/project/open_questions.md) | Pending decisions |

**Design:**
| File | Purpose |
|------|---------|
| [`docs/design/functional_spec.md`](docs/design/functional_spec.md) | Full game specification |

## Reference

- [Bevy physics in fixed timestep](https://github.com/bevyengine/bevy/blob/main/examples/movement/physics_in_fixed_timestep.rs)
- [Bevy breakout example](https://github.com/bevyengine/bevy/blob/main/examples/games/breakout.rs)
