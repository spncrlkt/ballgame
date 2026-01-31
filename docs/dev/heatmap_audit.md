# Heatmap Workflow Audit

Run this checklist after changes to:
- `src/bin/heatmap.rs`
- `src/simulation/` (reachability)
- Level geometry (`config/levels.txt`)

---

## Score Heatmap

### Basic Score Heatmap

```bash
cargo run --bin heatmap -- score --fast --level "Arena"
```

**Expected:**
- [ ] PNG generated in `showcase/heatmaps/`
- [ ] Filename includes level name
- [ ] File size reasonable (100KB - 2MB)

### Verify Output

```bash
ls -la showcase/heatmaps/*.png | tail -5
```

- [ ] File exists
- [ ] Timestamp is current
- [ ] Size is non-zero

### Visual Check

Read the generated heatmap PNG to verify:
- [ ] Colors show gradient (not all one color)
- [ ] Basket areas are hot (high score probability)
- [ ] Edges are cold (low score probability)
- [ ] Level geometry visible in pattern

---

## Reachability Heatmap

### Quick Verification

```bash
cargo run --bin verify_reachability
```

This binary validates all reachability heatmaps exist and have varied data. It's also run automatically at the end of `./init.sh`.

### Basic Reachability

```bash
cargo run --bin heatmap -- --type reachability --level "Arena"
```

**Expected:**
- [ ] TXT file generated
- [ ] Contains position data
- [ ] Coverage percentage reported

### Verify Coverage

```bash
cargo run --bin heatmap -- --type reachability --level "Arena" 2>&1 | grep -E "coverage|reachable"
```

- [ ] Coverage > 50% of level area
- [ ] No unreachable critical positions

### Multi-Level Check

```bash
for level in "Arena" "Platforms" "Towers"; do
    echo "=== $level ==="
    cargo run --bin heatmap -- --type reachability --level "$level" 2>&1 | grep -E "coverage|reachable"
done
```

- [ ] All levels generate output
- [ ] Coverage varies appropriately

---

## Performance

### Fast Mode

```bash
time cargo run --bin heatmap -- score --fast --level "Arena"
```

- [ ] Completes in < 30 seconds
- [ ] Output quality acceptable for quick checks

### Full Mode

```bash
time cargo run --bin heatmap -- score --level "Arena"
```

- [ ] Completes in < 5 minutes
- [ ] Higher resolution than fast mode

---

## Output Files

### Directory Structure

```bash
ls -la showcase/heatmaps/
```

**Expected files:**
- `score_<level>_<timestamp>.png` - Score heatmaps
- `reachability_<level>.txt` - Reachability data

### PNG Validation

```bash
# Check PNG is valid (requires ImageMagick)
identify showcase/heatmaps/*.png 2>/dev/null | tail -3
```

- [ ] Format: PNG
- [ ] Dimensions appropriate (800x600 or similar)
- [ ] Color depth: RGB

---

## Common Issues

### "Level not found"

```bash
# List available levels
grep "^name:" config/levels.txt | head -10
```
- Level names are case-sensitive

### Output is All One Color

This indicates:
- Score heatmap: No variance in shot success
- Reachability: All positions same reachability

**Debug:**
```bash
# Check raw data
cargo run --bin heatmap -- score --fast --level "Arena" --debug 2>&1 | head -50
```

### File Too Large

```bash
# Check file size
du -h showcase/heatmaps/*.png | sort -h | tail -5
```

If > 5MB, resolution may be too high:
```bash
# Use fast mode for smaller output
cargo run --bin heatmap -- score --fast --level "Arena"
```

---

## Integration with Simulation

### Verify Heatmap Matches Simulation

```bash
# Run shot test
cargo run --bin simulate -- --shot-test 50 --level 3

# Generate heatmap for same level
cargo run --bin heatmap -- score --level 3
```

Compare:
- [ ] High-scoring positions in heatmap match areas where shots succeed in simulation
- [ ] Edge positions show lower scores

---

*Last updated: 2026-01-30*
