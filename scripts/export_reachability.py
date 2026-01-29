#!/usr/bin/env python3
"""Export reachability heatmaps from player exploration data.

Reads position data from debug_events table in SQLite and generates
heatmap CSV files for each level.

Usage:
    python3 scripts/export_reachability.py db/training.db
    python3 scripts/export_reachability.py db/training.db --levels-file config/levels.txt
"""

from __future__ import annotations
import argparse
import sqlite3
import sys
from collections import defaultdict
from pathlib import Path
from typing import Optional

# Arena dimensions (from constants.rs)
ARENA_WIDTH = 1600
ARENA_HEIGHT = 900

# Grid resolution (must match HEATMAP_CELL_SIZE in constants.rs)
CELL_SIZE = 20
GRID_WIDTH = ARENA_WIDTH // CELL_SIZE   # 80
GRID_HEIGHT = ARENA_HEIGHT // CELL_SIZE  # 45

HEATMAP_DIR = Path("showcase/heatmaps")


def load_levels_config(levels_file: str) -> dict[str, str]:
    """Load level ID to name mapping from levels.txt config file."""
    levels = {}
    current_name = None
    current_id = None

    with open(levels_file, 'r') as f:
        for line in f:
            line = line.strip()
            if line.startswith('#') or not line:
                continue
            if line.startswith('level:'):
                current_name = line.split(':', 1)[1].strip()
            elif line.startswith('id:'):
                current_id = line.split(':', 1)[1].strip()
                if current_name and current_id:
                    levels[current_id] = current_name
                    current_name = None
                    current_id = None

    return levels


def sanitize_level_name(name: str) -> str:
    """Sanitize level name for use in filename (matches Rust sanitize_level_name)."""
    result = []
    last_was_underscore = False

    for ch in name:
        if ch.isalnum():
            result.append(ch.lower())
            last_was_underscore = False
        elif not last_was_underscore:
            result.append('_')
            last_was_underscore = True

    return ''.join(result).strip('_')


def world_to_cell(x: float, y: float) -> Optional[tuple]:
    """Convert world coordinates to grid cell coordinates."""
    # Match Rust world_to_cell function in heatmaps.rs
    cx = int((x + ARENA_WIDTH / 2) / CELL_SIZE)
    cy = int((ARENA_HEIGHT / 2 - y) / CELL_SIZE)

    if cx < 0 or cy < 0:
        return None
    if cx >= GRID_WIDTH or cy >= GRID_HEIGHT:
        return None

    return (cx, cy)


def cell_to_world(cx: int, cy: int) -> tuple[float, float]:
    """Convert grid cell coordinates to world coordinates (cell center)."""
    # Reverse of world_to_cell
    x = (cx + 0.5) * CELL_SIZE - ARENA_WIDTH / 2
    y = ARENA_HEIGHT / 2 - (cy + 0.5) * CELL_SIZE
    return (x, y)


def extract_positions_by_level(db_path: str, human_only: bool = True) -> dict[str, list[tuple[float, float]]]:
    """Extract player positions from debug_events, grouped by level_id."""
    conn = sqlite3.connect(db_path)
    cursor = conn.cursor()

    # Query positions with level_id, optionally filtering for human-controlled
    if human_only:
        cursor.execute("""
            SELECT level_id, pos_x, pos_y
            FROM debug_events
            WHERE human_controlled = 1
        """)
    else:
        cursor.execute("""
            SELECT level_id, pos_x, pos_y
            FROM debug_events
        """)

    positions_by_level: dict[str, list[tuple[float, float]]] = defaultdict(list)
    for level_id, x, y in cursor.fetchall():
        if level_id:
            positions_by_level[level_id].append((x, y))

    conn.close()
    return dict(positions_by_level)


def create_reachability_grid(positions: list[tuple[float, float]]) -> dict[tuple[int, int], int]:
    """Create a grid counting visits per cell."""
    grid: dict[tuple[int, int], int] = defaultdict(int)

    for x, y in positions:
        cell = world_to_cell(x, y)
        if cell:
            grid[cell] += 1

    return dict(grid)


def export_heatmap_csv(
    output_path: Path,
    grid: dict[tuple[int, int], int],
    default_value: float = 0.0
) -> None:
    """Export grid as CSV heatmap with ALL 3600 cells filled."""
    max_count = max(grid.values()) if grid else 1

    with open(output_path, 'w') as f:
        f.write("x,y,value\n")

        # CRITICAL: Iterate through ALL cells in the grid
        # Order: top-to-bottom (cy=0 is top), left-to-right
        for cy in range(GRID_HEIGHT):
            for cx in range(GRID_WIDTH):
                world_x, world_y = cell_to_world(cx, cy)

                # Normalize visit count to 0.0-1.0
                visit_count = grid.get((cx, cy), 0)
                if visit_count > 0:
                    # Normalize by max count
                    value = visit_count / max_count
                else:
                    # Unvisited cell - use default value
                    value = default_value

                f.write(f"{world_x:.2f},{world_y:.2f},{value:.3f}\n")


def main():
    parser = argparse.ArgumentParser(
        description='Export reachability heatmaps from training database'
    )
    parser.add_argument('db_path', help='Path to SQLite database')
    parser.add_argument(
        '--levels-file',
        default='config/levels.txt',
        help='Path to levels.txt config file'
    )
    parser.add_argument(
        '--include-ai',
        action='store_true',
        help='Include AI positions (default: human only)'
    )
    parser.add_argument(
        '--min-samples',
        type=int,
        default=100,
        help='Minimum samples required to export a level (default: 100)'
    )
    parser.add_argument(
        '--output-dir',
        default='showcase/heatmaps',
        help='Output directory for heatmap files'
    )

    args = parser.parse_args()

    # Validate inputs
    db_path = Path(args.db_path)
    if not db_path.exists():
        print(f"Error: Database not found: {db_path}")
        sys.exit(1)

    levels_file = Path(args.levels_file)
    if not levels_file.exists():
        print(f"Error: Levels file not found: {levels_file}")
        sys.exit(1)

    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    # Load level names
    print(f"Loading levels from {levels_file}...")
    levels = load_levels_config(str(levels_file))
    print(f"  Found {len(levels)} levels")

    # Extract positions
    human_only = not args.include_ai
    print(f"Extracting positions from {db_path}...")
    print(f"  Filter: {'human only' if human_only else 'human + AI'}")
    positions_by_level = extract_positions_by_level(str(db_path), human_only=human_only)

    total_samples = sum(len(p) for p in positions_by_level.values())
    print(f"  Found {total_samples} total samples across {len(positions_by_level)} levels")

    # Export heatmaps
    exported = 0
    skipped = 0

    for level_id, positions in positions_by_level.items():
        level_name = levels.get(level_id, f"unknown_{level_id[:8]}")
        safe_name = sanitize_level_name(level_name)

        sample_count = len(positions)

        if sample_count < args.min_samples:
            print(f"  Skipping {level_name}: {sample_count} samples (need {args.min_samples})")
            skipped += 1
            continue

        # Create reachability grid
        grid = create_reachability_grid(positions)
        cells_visited = len(grid)
        coverage = cells_visited / (GRID_WIDTH * GRID_HEIGHT) * 100

        # Export to CSV
        # Format: heatmap_reachability_{safe_name}_{level_id}.txt
        output_path = output_dir / f"heatmap_reachability_{safe_name}_{level_id}.txt"
        export_heatmap_csv(output_path, grid, default_value=0.0)

        print(f"  Exported {level_name}: {sample_count} samples, {cells_visited} cells ({coverage:.1f}% coverage)")
        print(f"    -> {output_path}")
        exported += 1

    # Summary
    print(f"\nSummary:")
    print(f"  Exported: {exported} levels")
    print(f"  Skipped: {skipped} levels (insufficient samples)")
    print(f"  Total cells per heatmap: {GRID_WIDTH * GRID_HEIGHT}")

    if exported == 0:
        print("\nWarning: No heatmaps exported. Run training sessions to collect exploration data:")
        print("  cargo run --bin training -- -n 5")


if __name__ == "__main__":
    main()
