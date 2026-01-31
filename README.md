# Ballgame

A 2v2 ball sport game built with Bevy 0.17.3.


## Quick Start

```bash
cargo run              # Play the game
cargo run --release    # Play with optimizations
```

**Guides:**
- [`docs/guides/HOW_TO_PLAY.md`](docs/guides/HOW_TO_PLAY.md) - Controls and gameplay
- [`docs/guides/BINARIES.md`](docs/guides/BINARIES.md) - All binaries with flags and usage
- [`docs/guides/TRAINING.md`](docs/guides/TRAINING.md) - Training mode setup and analysis workflow
- [`docs/guides/WORKFLOWS.md`](docs/guides/WORKFLOWS.md) - Multi-step development workflows

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

## Binaries

| Binary | Purpose |
|--------|---------|
| `ballgame` | Main game |
| `training` | 1v1 vs AI with event logging |
| `simulate` | Headless AI vs AI simulation |
| `analyze` | Analyze sessions, generate reports |
| `test-scenarios` | Deterministic mechanics tests |
| `heatmap` | Per-level heatmaps |
| `run-ghost` | Ghost trial playback |
| `generate` | Asset generation |
| `extract-drives` | Extract input sequences |
| `verify_reachability` | Verify heatmap coverage |
| `gamepad_debug` | Controller debugging |

**Full documentation:** [`docs/guides/BINARIES.md`](docs/guides/BINARIES.md) - All flags, options, and examples.

---

## Controls Reference

See [`docs/guides/HOW_TO_PLAY.md`](docs/guides/HOW_TO_PLAY.md) for full controls, or quick reference:

**Modal input** - Controls change based on ball possession:

| Context | LB / Q | RB / F | X / E |
|---------|--------|--------|-------|
| Holding ball | Pass | Shoot | Turbo |
| Opponent has ball | Steal | Block | Turbo |
| Free ball | Pickup | Pickup | Turbo |

| Action | Keyboard | Gamepad |
|--------|----------|---------|
| Move | A/D | Left Stick |
| Jump | Space/W | A (South) |
| Turbo | E | X (West) |
| Pass/Steal | Q | LB (Left Bumper) |
| Throw/Block | F | RB (Right Bumper) |
| Cycle character | ] | D-pad Right |
| Reset level | R | Start |

---

## AI Profiles

68 profiles organized by lineage:
- `v1_*` to `v4_*` - Evolution from original to tournament optimized
- `v5_*` to `v6_*` - Experimental playstyles (Sniper, Brawler, etc.)
- `v7_*` to `v11_*` - Randomized and blended from top performers
- `CatchPartner` - Debug AI for cooperative pass practice

**Key profiles:**
- `v11_Blend_A` - Top performer from V11 tournament (default)
- `v3_Rush_Smart` - Fast, aggressive, improved decision-making
- `v5_Sniper` - Long range, patient, selective shooter
- `v5_Brawler` - Close range, aggressive steals, pressure
- `CatchPartner` - Catches and returns passes (for team-interaction protocol)

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
| [`docs/guides/BINARIES.md`](docs/guides/BINARIES.md) | All binaries reference |
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
