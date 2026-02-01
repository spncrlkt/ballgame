# Quickstart

Get up and running with the ballgame workspace.

## Prerequisites

- Rust (edition 2024)
- macOS, Linux, or Windows

## Build & Run

```bash
# Build the entire workspace
cargo build

# Run the main game
cargo run -p ballgame

# Run with release optimizations (recommended for playing)
cargo run -p ballgame --release
```

## Workspace Structure

This is a Cargo workspace with 4 crates:

```
.
├── ballgame/           # Main game (Bevy 0.17.3)
├── ballgame-protocol/  # Shared client-server protocol types
├── ballgame-ai-v1/     # AI client - extracts current AI logic
└── ballgame-ai-v2/     # AI client template - minimal, no decision logic
```

### ballgame (main crate)

The game itself plus tooling binaries:

| Command | Purpose |
|---------|---------|
| `cargo run -p ballgame` | Play the game |
| `cargo run -p ballgame --bin training` | 1v1 vs AI with logging |
| `cargo run -p ballgame --bin simulate` | Headless AI tournaments |
| `cargo run -p ballgame --bin test-scenarios` | Run mechanics tests |
| `cargo run -p ballgame --bin analyze` | Analyze SQLite databases |

### ballgame-protocol

Shared types for client-server communication. No binaries.

### ballgame-ai-v1 / v2

Standalone AI clients that connect to the game server:

```bash
cargo run -p ballgame-ai-v1
cargo run -p ballgame-ai-v2
```

## Quick Commands

```bash
# Verify everything compiles
cargo check

# Run lints
cargo clippy

# Format code
cargo fmt

# Run scenario tests (42 expected)
cargo run -p ballgame --bin test-scenarios

# Run unit tests
cargo test
```

## Controls (Keyboard)

| Key | Action |
|-----|--------|
| A/D | Move left/right |
| Space/W | Jump |
| Shift | Turbo (speed boost) |
| Q | Pass / Steal / Pickup |
| E or F | Throw / Block / Pickup |
| Tab | Toggle debug UI |
| R | Reset level |
| ] / [ | Next / Previous level |

Gamepad supported (Xbox layout).

## Configuration

Config files live in `config/`:

- `levels.txt` - Level definitions
- `ai_profiles.txt` - AI personality profiles
- `ball_options.txt` - Ball visual styles
- `game_presets.txt` - Tuning presets

## More Documentation

- [HOW_TO_PLAY.md](HOW_TO_PLAY.md) - Full controls reference
- [BINARIES.md](BINARIES.md) - All binaries with flags
- [TRAINING.md](TRAINING.md) - Training mode and analysis
- [WORKFLOWS.md](WORKFLOWS.md) - Common workflows
