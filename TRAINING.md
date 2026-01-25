# Training Mode Guide

Play 1v1 against AI with full event logging for analysis.

## Quick Start

```bash
cargo run --bin training                    # 5 iterations vs Balanced AI
cargo run --bin training -- -n 10           # 10 iterations
cargo run --bin training -- -p v3_Rush_Smart   # vs specific AI
cargo run --bin training -- -l 3            # Force level 3
```

## Command Line Options

| Option | Description | Default |
|--------|-------------|---------|
| `-n, --iterations N` | Number of iterations | 5 |
| `-p, --profile NAME` | AI opponent profile | Balanced |
| `-l, --level N` | Force level (number or name) | random |
| `-m, --mode MODE` | `goal` or `game` | goal |
| `-w, --win-score N` | Points to win (game mode) | 5 |
| `-t, --time-limit SECS` | Time limit per iteration | none |
| `-s, --seed N` | RNG seed for reproducibility | random |
| `--ball-style NAME` | Ball visual style (`random` or name) | random |

**Modes:**
- `goal` - Each iteration ends after one goal, then resets
- `game` - Full game to win_score points

## Settings File

Persist your preferences in `assets/training_settings.json` (gitignored):

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
| `ai_profile` | Default opponent profile |
| `level` | `null` = random, or number/name (e.g., `7` or `"Skyway"`) |
| `ball_style` | `null` = random, or name like `"wedges"` |
| `exclude_levels` | Levels to skip in random selection |
| `iterations` | Rounds per session |
| `mode` | `"goal"` or `"game"` |

CLI arguments override file settings.

## Output

Sessions are saved to `training_logs/session_YYYYMMDD_HHMMSS/`:
```
training_logs/session_20260125_143022/
├── game_1_level3.evlog
├── game_2_level7.evlog
├── game_3_level2.evlog
└── summary.json
```

## Post-Session Analysis

Each training iteration is a complete drive (you start with the ball). No extraction needed.

```bash
# Run ghost trials directly on training logs
cargo run --bin run-ghost training_logs/session_YYYYMMDD_HHMMSS/

# Test specific AI profile's defense
cargo run --bin run-ghost training_logs/session_*/ --profile v3_Rush_Smart

# Analyze events
cargo run --bin analyze -- training_logs/session_YYYYMMDD_HHMMSS/
```

## Controls

| Action | Keyboard | Gamepad |
|--------|----------|---------|
| Move | A/D | Left Stick |
| Jump | Space/W | A (South) |
| Pickup/Steal | E | X (West) |
| Throw (hold) | F | RB |
| Quit | Escape | - |

## AI Profiles

53 profiles available. Key ones:

| Profile | Style |
|---------|-------|
| `v2_Balanced` | Good all-around default |
| `v3_Rush_Smart` | Fast, aggressive, smart decisions |
| `v3_Steady_Deep` | Patient, defensive, high IQ |
| `v3_Spec_Chaos` | Unpredictable, takes risky shots |
| `v3_Ultra_Rush` | Maximum aggression |

List all: check `assets/ai_profiles.txt`
