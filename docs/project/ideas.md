# Ideas & Notes

*Non-prioritized ideas and notes. When an idea becomes actionable, move it to `todo.md` or `milestones.md`.*

---

## Gameplay Ideas

- **Animated gif for readme** - Create a good starting 3-2-1 -> 20 seconds of action ending in a point. Need a system to recreate a full replay (like ghost/replay system) and record it as a gif for the front page demo.

- **Archive level system** - Move to archived file, remove all references, ensure no path to read from archived levels file.

- **Steal balance testing** - The AI steals too easily and it is too hard for me to steal. Set up a training protocol for stealing only with a flat level and normal baskets (no stairs). Play for 60 seconds and record steal attempts and successful steals. Ensure logging is sufficient to analyze fairness of player stealing vs AI stealing.

- **Platforming training protocol** - The platforming and understanding of steps and platforms are still inefficient. Set up a training protocol for steps and platforming. Make the level look like Skyway.

---

## Technical Ideas

- **Symlinks for latest outputs** - Check where symlinks would help for newest outputs (e.g., latest training DB, latest tournament DB).

- **Parallelize heatmap generation** - Generate per-level + per-type in parallel to reduce offline workflow time.

---

## Feature Ideas

- More ball styles (yin yang, volleyball, pool balls, etc.)
- Debug level labels update color on palette change
- AI debug level: both players AI-controlled for testing
- Settings file: move init_settings out of VC, use template as default
