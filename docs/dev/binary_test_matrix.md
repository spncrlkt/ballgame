# Binary Test Matrix

Maps all binary executables to their test coverage status.

**Legend:**
- ✅ Tested - Automated tests exist
- ⚠️ Partial - Some coverage
- ❌ Untested - No automated tests
- 🔧 Script - Tested via shell script

---

## Binary Overview

| Binary | Purpose | Primary Test |
|--------|---------|--------------|
| ballgame | Main game | regression.sh |
| training | 1v1 vs AI with logging | manual |
| simulate | Headless simulation | unit tests |
| heatmap | Generate heatmaps | manual |
| analyze | Database analytics | manual |
| run-ghost | Ghost replay testing | manual |
| extract-drives | Extract drives from DB | manual |
| test-scenarios | Scenario test runner | self |
| generate | Asset/config generation | manual |
| gamepad_debug | Controller debugging | manual |
| verify_reachability | Navigation verification | manual |

---

## Detailed Coverage

### ballgame (Main Game)

| Mode/Flag | Tested By | Coverage | Notes |
|-----------|-----------|----------|-------|
| default play | manual only | ❌ | Core gameplay |
| `--replay-db <id>` | manual only | ❌ | Replay mode |
| `--screenshot-and-quit` | regression.sh | 🔧 Script | Visual regression |
| `--level <n>` | manual only | ❌ | Level selection |
| `--preset <name>` | manual only | ❌ | Preset loading |

### training

| Mode/Flag | Tested By | Coverage | Notes |
|-----------|-----------|----------|-------|
| default (5 iterations) | manual only | ❌ | Basic training |
| `-n, --iterations N` | manual only | ❌ | Custom iterations |
| `-p, --profile X` | manual only | ❌ | AI profile selection |
| `-m, --mode goal` | manual only | ❌ | Goal mode |
| `-m, --mode game` | manual only | ❌ | Game mode |
| `--protocol pursuit` | manual only | ❌ | Pursuit training |
| `--protocol reachability` | manual only | ❌ | Reachability training |
| `--protocol advanced-platform` | manual only | ❌ | Platform training |
| SQLite output | sqlite_logger tests | ✅ | Event logging |

### simulate

| Mode/Flag | Tested By | Coverage | Notes |
|-----------|-----------|----------|-------|
| `--shot-test N` | manual only | ❌ | Shot accuracy test |
| `--shot-test --level L` | manual only | ❌ | Level-specific shots |
| `--tournament N` | manual only | ❌ | Round-robin tournament |
| `--tournament --parallel P` | manual only | ❌ | Parallel execution |
| `--bracket N` | unit tests | ⚠️ Partial | Fixed: bracket seeding |
| `--bracket --best-of B` | unit tests | ⚠️ Partial | Match format |
| `--reachability-test` | reachability_test.rs | ✅ | Position sampling |
| `--multihop-test` | multihop_test.rs | ✅ | Pathfinding |
| SQLite output | db tests | ✅ | Match storage |

### heatmap

| Mode/Flag | Tested By | Coverage | Notes |
|-----------|-----------|----------|-------|
| `score` | manual only | ❌ | Score heatmap |
| `reachability` | manual only | ❌ | Reachability heatmap |
| `--fast` | manual only | ❌ | Quick mode |
| `--level L` | manual only | ❌ | Level selection |
| PNG output | manual only | ❌ | Image generation |

### analyze

| Mode/Flag | Tested By | Coverage | Notes |
|-----------|-----------|----------|-------|
| `summary` | db_analytics tests | ✅ | Summary stats |
| `profile <id>` | db_analytics tests | ✅ | Profile analysis |
| `compare <a> <b>` | db_analytics tests | ✅ | Profile comparison |
| `events <match>` | manual only | ❌ | Event listing |

### run-ghost

| Mode/Flag | Tested By | Coverage | Notes |
|-----------|-----------|----------|-------|
| `<session_dir>` | manual only | ❌ | Load session |
| `--profile X` | manual only | ❌ | AI override |
| `--summary` | manual only | ❌ | Statistics |
| Playback | manual only | ❌ | Ghost playback |
| Defense metrics | manual only | ❌ | Result tracking |

### extract-drives

| Mode/Flag | Tested By | Coverage | Notes |
|-----------|-----------|----------|-------|
| `<db_path>` | manual only | ❌ | Database input |
| `--output <dir>` | manual only | ❌ | Output directory |
| Drive segmentation | manual only | ❌ | Event parsing |

### test-scenarios

| Mode/Flag | Tested By | Coverage | Notes |
|-----------|-----------|----------|-------|
| default (all) | self | ✅ | All scenarios |
| `<pattern>` | self | ✅ | Category filter |
| `-v, --verbose` | self | ✅ | Verbose output |
| TOML parsing | parser tests | ✅ | Scenario parsing |

### generate

| Mode/Flag | Tested By | Coverage | Notes |
|-----------|-----------|----------|-------|
| `levels` | manual only | ❌ | Level generation |
| `palettes` | manual only | ❌ | Palette generation |
| `presets` | manual only | ❌ | Preset generation |

### gamepad_debug

| Mode/Flag | Tested By | Coverage | Notes |
|-----------|-----------|----------|-------|
| default | manual only | 📋 Manual | Interactive tool |

### verify_reachability

| Mode/Flag | Tested By | Coverage | Notes |
|-----------|-----------|----------|-------|
| default | manual only | ❌ | Visual verification |
| `--level L` | manual only | ❌ | Level selection |

---

## Coverage Summary

| Binary | Modes/Flags | Tested | Partial | Untested |
|--------|-------------|--------|---------|----------|
| ballgame | 5 | 0 | 1 | 4 |
| training | 9 | 1 | 0 | 8 |
| simulate | 8 | 3 | 2 | 3 |
| heatmap | 5 | 0 | 0 | 5 |
| analyze | 4 | 3 | 0 | 1 |
| run-ghost | 5 | 0 | 0 | 5 |
| extract-drives | 3 | 0 | 0 | 3 |
| test-scenarios | 4 | 4 | 0 | 0 |
| generate | 3 | 0 | 0 | 3 |
| gamepad_debug | 1 | 0 | 0 | 1 |
| verify_reachability | 2 | 0 | 0 | 2 |
| **TOTAL** | **49** | **11** | **3** | **35** |

**Coverage Rate:** 22% tested, 6% partial, 72% untested

---

## Priority Gaps

### High Priority (Core Workflows)
1. **training** - 8 untested modes, primary development workflow
2. **simulate --shot-test** - Balance testing workflow
3. **simulate --tournament** - AI comparison workflow
4. **ballgame --replay-db** - Debug/analysis workflow

### Medium Priority (New Features)
5. **run-ghost** - 5 untested modes, ghost testing feature
6. **heatmap** - 5 untested modes, visualization feature
7. **extract-drives** - 3 untested modes, data pipeline

### Low Priority (Development Tools)
8. **generate** - Development-time tool
9. **verify_reachability** - Debug tool
10. **gamepad_debug** - Hardware debug tool

---

## Recommended Test Additions

### Smoke Tests (Quick Validation)
```bash
# Training binary starts
cargo run --bin training -- -n 1 --protocol pursuit

# Simulate shot test runs
cargo run --bin simulate -- --shot-test 10 --level 3

# Heatmap generates output
cargo run --bin heatmap -- score --fast --level "Arena"

# Ghost replay loads
cargo run --bin run-ghost -- <session_dir> --summary
```

### Integration Tests
- training → SQLite → analyze pipeline
- simulate --tournament → db/simulation.db → analyze
- extract-drives → run-ghost → metrics

### Regression Tests
- Shot accuracy should stay within 40-60% over/under
- Training goals per match should be > 1.0
- Tournament should complete without crashes

---

*Last updated: 2026-01-30*
