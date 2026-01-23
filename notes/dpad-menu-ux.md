# D-Pad Menu UX Improvements

## Current State

The D-pad cycle system was restructured to a 4-direction model:
- **Up**: Viewport
- **Down**: Composite → Movement → Ball → Shooting presets
- **Left**: AI (LT: player, RT: profile)
- **Right**: Level → Palette → BallStyle

Display is in top-left, always visible, 4 lines with `>` marking active direction.

## Problems to Evaluate

1. **No visual mapping** - Users can't tell which D-pad direction maps to which row
2. **Subtle active marker** - The `>` is easy to miss
3. **Dense AI line** - `[L* Aggressive] R Passive` is hard to parse quickly
4. **No interaction hints** - No indication of how to use (D-pad + LT/RT)

## Potential Solutions

### Option A: Add Direction Arrows
```
↑ Viewport: 1080p
↓ Composite: Default
← AI: L* Aggressive | R Passive
→ Level: 3/10
```

### Option B: Highlight Active Row More
Use brackets or different character for active:
```
  ↑ Viewport: 1080p
[ ↓ Composite: Default ]
  ← AI: L* Aggro | R Passive
  → Level: 3/10
```

### Option C: Simplify AI Display
Show selected player more clearly:
```
← AI: [L]* Aggressive  R Passive
```
Or split across two display states based on selection.

### Option D: Add Key Hints (subtle)
```
↑ Viewport: 1080p          [LT/RT]
↓ Composite: Default       [LT/RT]
← AI: L* Aggro | R Passive [LT:sel RT:prof]
→ Level: 3/10              [LT/RT]
```

## Next Steps

1. Run game and screenshot current display
2. Evaluate readability and usability
3. Pick improvement approach
4. Implement and test
