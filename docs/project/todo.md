# Ballgame TODO - V0 Closing

*See `milestones.md` for V1 planning | `ideas.md` for notes*

---

## V0 Release Checklist

**Goal:** Ship V0 today. Tight, focused, fun game loop for dev friends.

### Verification (Automated)

- [x] `cargo check` passes
- [x] `cargo clippy` - minor style warnings only (no errors)
- [x] `cargo test` - 151 unit tests pass
- [x] `cargo run --bin test-scenarios` - 42 scenario tests pass
- [x] `./scripts/regression.sh` - 9/9 visual baselines match
- [x] DB paths use timestamped format (verified: `db/training_*.db` exists)

### Manual Playtest (User)

- [ ] Play 5 minutes against AI - game feels fun?
- [ ] Try all 10 levels - any broken geometry?
- [ ] Test pass/steal/block/turbo - mechanics work?
- [ ] Check countdown works on level change

### Critical Bug Check (User)

- [ ] No crashes during normal play
- [ ] No obvious visual glitches
- [ ] AI doesn't get permanently stuck

---

## Done (V0)

- [x] 2v2 gameplay with team mechanics
- [x] 10 AI profiles
- [x] Training mode + SQLite logging
- [x] Ghost system
- [x] 42 scenario tests + 151 unit tests
- [x] Preset system, palettes, ball styles
- [x] Visual regression testing
- [x] Project documentation cleanup

---

*V1 tasks in `milestones.md`*
