# Repository Guidelines

## Project Structure & Module Organization
- `src/` holds the Bevy game code; key subsystems live in module folders like `player/`, `ball/`, `shooting/`, `ui/`, and `training/`.
- `src/bin/` contains extra binaries (`training`, `simulate`, `analyze`, `heatmap`, `test-scenarios`).
- `tests/scenarios/` stores TOML scenario tests organized by topic (e.g., `movement/`, `stealing/`).
- `assets/` contains game assets; ball textures live under `assets/textures/balls/`.
- `config/` provides data-driven settings (profiles, levels, palettes, presets). Runtime logs and outputs go to `training_logs/`, `logs/`, and `showcase/`.

## Build, Test, and Development Commands
- `cargo run` / `cargo run --release`: play the game (debug vs optimized).
- `cargo run --bin training -- --games 5`: 1v1 training sessions with logging in `training_logs/`.
- `cargo run --bin simulate -- --help`: headless AI vs AI simulation.
- `cargo run --bin analyze -- training_logs/<session>/`: analyze event logs.
- `cargo run --bin test-scenarios`: run scenario test suite; add `-- <category>/` to filter.
- `cargo build`, `cargo check`, `cargo fmt`, `cargo clippy`: build, validate, format, lint.
- `./scripts/regression.sh`: visual regression comparison (use `--update` when intentional).

## Coding Style & Naming Conventions
- Rust 2024 edition; run `cargo fmt` before submitting.
- Indentation follows rustfmt defaults (4 spaces, no tabs).
- Naming: `snake_case` modules/files, `CamelCase` types, `SCREAMING_SNAKE_CASE` constants.
- Put tunable gameplay values in `src/constants.rs` and avoid magic numbers.

## Architecture & Performance Conventions
- Run physics in FixedUpdate and visuals/UI/input capture in Update.
- Scale continuous physics by `time.delta_secs()` or `.powf(time.delta_secs())`; avoid frame-based multipliers (e.g., `*= 0.98`).
- Buffer `just_pressed()` input in Update and consume it in FixedUpdate.
- Avoid allocations and string formatting in per-frame systems; preallocate and reuse buffers.
- Prefer `With<T>`/`Without<T>` filters over `Option<&T>` and avoid O(n^2) entity loops.
- Use flag fields instead of frequent component add/remove; keep components small and focused.
- If `src/main.rs` changes, keep `src/bin/training.rs` and `src/bin/test_scenarios.rs` in sync.

## Testing Guidelines
- Scenario tests live in `tests/scenarios/**/*.toml` and use descriptive `snake_case` names.
- Prefer adding or updating a scenario test when fixing gameplay logic.
- Run the manual checklist in `docs/guides/TESTING.md` after major input/physics changes.

## Commit & Pull Request Guidelines
- Commit history favors short, lowercase subjects (no strict prefix); keep messages concise and descriptive.
- PRs should include: purpose summary, key commands run (e.g., `cargo clippy`), and screenshots for visual changes.
- Link relevant docs or logs when you add new training or analysis outputs.

## Configuration & Data Notes
- Update `config/*.txt` files for levels, palettes, or AI profile tweaks; keep formats consistent with existing entries.
- Large generated outputs belong in `showcase/` or `training_logs/`, not `src/`.
