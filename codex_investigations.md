# Codex Investigations

## Gameplay Timing + Input

- Cooldown timing: `steal_cooldown_update` runs in Update + FixedUpdate in `src/main.rs`, `src/bin/training.rs`, `src/bin/run-ghost.rs`, `src/simulation/runner.rs`, doubling timers.
- UI dependency: `src/ui/steal_indicators.rs` reads `StealCooldown`/`StealContest` in Update; needs a single authoritative timer cadence.
- Input capture: `capture_input` early-return keeps stale `PlayerInput`; `copy_human_input` continues applying it.
- System wiring: duplicated system chains across main/training/simulation/run-ghost/testing; divergence already observed.

## AI Defense Notes

- AI-vs-AI: `ai_navigation_update` targets `HumanControlled` only; `ai_decision_update` uses "any other player".
- AI logging: `events/emitter.rs` emits `AiGoal`; `simulation/metrics.rs` tracks goal time; `training/analysis.rs` can compute transitions/oscillation.
- Defense direction: use zone mapping from level geometry (basket height/ramps/platform bounds).

## EventBus

- EventBus growth: `export_events`/`drain` append to `processed` without clearing.

## Evlog Elimination Critical Notes

- Replay loading from SQLite: confirm events table schema supports ordered tick + game events; define ordering/sequence key (tick index + event id).
- Match lookup: `find_match_by_session(session_id, game_num)` needs reliable session/game mapping in DB (ensure `matches` stores both).
- Training outputs: ensure `GameResult.match_id` is persisted everywhere (session summary, analysis, CLI output).
- Parser removal: verify no tests/tools depend on `parse_event()` or evlog parsing (including scenario tooling or analysis scripts).
- Binaries/docs drift: `src/bin/analyze.rs`, `src/bin/run-ghost.rs`, and docs likely still reference `--replay` or `.evlog`; track and deprecate explicitly.
- Verification gap: add a DB replay smoke test (load match_id, run replay) before deleting evlog code.
- Shell warning: `/opt/homebrew/Library/Homebrew/cmd/shellenv.sh` fails `ps` (Operation not permitted) in login shells; prefer non-login shell for tooling.
