# Ballgame

A 2v2 ball sport game built with Bevy 0.17.3.

## Quick Start

```bash
cargo run              # Play the game
cargo run --release    # Play with optimizations
```

---

## Offline Work Guide

Tasks to run when Claude isn't available. Collect data now, analyze later.

### Data Collection

**Training runs (you vs AI):**
```bash
cargo run --bin training -- --games 10                        # vs Balanced
cargo run --bin training -- --games 10 --profile Aggressive   # vs Aggressive
cargo run --bin training -- --games 10 --profile Defensive    # vs Defensive
```
Output: `training_logs/session_YYYYMMDD_HHMMSS/`

**Simulation runs (AI vs AI, headless, fast):**
```bash
cargo run --bin simulate -- --tournament 10 --log-events --log-dir sim_logs/
```
Output: `sim_logs/*.evlog`

**Analyze collected data:**
```bash
cargo run --bin analyze -- training_logs/session_20260123_143022/
cargo run --bin analyze -- sim_logs/
```

### Manual Playtesting Notes

Play 15-30 min, jot notes on:
- **AI shooting**: When does it take dumb shots? (note level + situation)
- **AI positioning**: Where does it stand wrong? What would be better?
- **Movement feel**: What feels sluggish or floaty?
- **Level quality**: Which levels play well? Which are frustrating?

### Design Questions to Decide

**Priority 0 (decide before more coding):**
- What makes a shot "good"? Clear line? Distance? No defender nearby?
- What should AI do without ball? Guard basket? Chase ball? Cut to open space?

**Immediate verification:**
- Does AI climb corner steps on levels 7/8? (nav graph fix was made - verify it works)

**V0 scoping:**
- Win conditions: Score limit (first to 10?) or time limit (2 min games)?
- What would make you feel ready to share this with others?

### Priority Ranking

Rank 1-5 by impact on fun:
- [ ] AI shoots smarter
- [ ] AI positions better
- [ ] Movement feels better
- [ ] Viewport works at all sizes
- [ ] Win conditions / game structure

---

## All Binaries & Commands

### Main Game

```bash
cargo run                              # Play
cargo run -- --replay <file.evlog>     # Replay a recorded game
cargo run -- --screenshot-and-quit     # Screenshot and exit (for testing)
```

**Controls:**
| Action | Keyboard | Gamepad |
|--------|----------|---------|
| Move | A/D | Left Stick |
| Jump | Space/W | South (A/X) |
| Pickup/Steal | E | West (X/Square) |
| Throw (hold) | F | Right Bumper |
| Cycle player | Q | Left Bumper |
| Reset level | R | Start |
| Next/Prev level | ] / [ | - |
| Debug UI | Tab | - |
| Tweak panel | F1 | - |
| Snapshot | F4 | - |

**Replay controls:** Space=pause, arrows=speed, comma/period=step, Home/End=jump

### Training Mode

```bash
cargo run --bin training -- --help

# Options:
#   -g, --games N       Number of games (default: 1)
#   -p, --profile NAME  AI profile (default: Balanced)

# Profiles: Balanced, Aggressive, Defensive, Sniper, Rusher,
#           Turtle, Chaotic, Patient, Hunter, Goalie
```

### Simulation (Headless)

```bash
cargo run --bin simulate -- --help

# Key options:
#   --level <N>         Level 1-12 (default: 2)
#   --left <PROFILE>    Left AI (default: Balanced)
#   --right <PROFILE>   Right AI (default: Balanced)
#   --duration <SECS>   Time limit (default: 60)
#   --score-limit <N>   Score limit (default: none)
#   --matches <N>       Run N matches
#   --tournament [N]    All profile pairs, N each (default: 5)
#   --level-sweep [N]   One profile across all levels
#   --log-events        Save .evlog files
#   --log-dir <DIR>     Where to save logs (default: logs/)
```

**Examples:**
```bash
cargo run --bin simulate -- --level 3 --left Balanced --right Aggressive
cargo run --bin simulate -- --tournament 10
cargo run --bin simulate -- --level-sweep 5 --left Sniper
cargo run --bin simulate -- --tournament 5 --log-events --log-dir logs/
```

### Analytics

```bash
cargo run --bin analyze -- --help

# Usage: cargo run --bin analyze -- [LOG_DIR] [OPTIONS]
#   --targets <FILE>    Custom tuning targets (TOML)
#   --output <FILE>     Save report to file
#   --update-defaults   Update src/constants.rs with best profiles
```

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
| `CLAUDE.md` | Development instructions, architecture docs |
| `todo.md` | Current sprint tasks |
| `milestones.md` | Full project plan (MVP -> V0 -> V1) |
| `open_questions.md` | Pending decisions |
| `todone.md` | Completed work archive |

## Reference

- [Bevy physics in fixed timestep](https://github.com/bevyengine/bevy/blob/main/examples/movement/physics_in_fixed_timestep.rs)
- [Bevy breakout example](https://github.com/bevyengine/bevy/blob/main/examples/games/breakout.rs)
