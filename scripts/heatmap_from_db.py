#!/usr/bin/env python3
"""Generate a heatmap from training database position data."""

import sqlite3
import sys
from collections import defaultdict

# Arena dimensions (from constants.rs)
ARENA_WIDTH = 1400
ARENA_HEIGHT = 900
ARENA_FLOOR_Y = -400

# Grid resolution
CELL_SIZE = 20

def extract_positions(db_path, match_id=None):
    """Extract player positions from tick events."""
    conn = sqlite3.connect(db_path)
    cursor = conn.cursor()

    if match_id:
        cursor.execute("SELECT data FROM events WHERE event_type = 'T' AND match_id = ?", (match_id,))
    else:
        cursor.execute("SELECT data FROM events WHERE event_type = 'T'")

    positions = []
    for (data,) in cursor.fetchall():
        # Format: T:time|T|tick|x,y|vx,vy|...
        parts = data.split('|')
        if len(parts) >= 4:
            pos_str = parts[3]  # Player position is 4th field
            try:
                x, y = map(float, pos_str.split(','))
                positions.append((x, y))
            except ValueError:
                pass

    conn.close()
    return positions

def create_heatmap_grid(positions):
    """Create a 2D grid counting visits per cell."""
    grid = defaultdict(int)

    for x, y in positions:
        # Convert to grid coordinates
        gx = int((x + ARENA_WIDTH/2) / CELL_SIZE)
        gy = int((y - ARENA_FLOOR_Y) / CELL_SIZE)
        grid[(gx, gy)] += 1

    return grid

def render_ascii_heatmap(grid, positions):
    """Render heatmap as ASCII art."""
    if not positions:
        print("No position data found!")
        return

    # Find bounds
    min_x = min(p[0] for p in positions)
    max_x = max(p[0] for p in positions)
    min_y = min(p[1] for p in positions)
    max_y = max(p[1] for p in positions)

    print(f"\nPosition range: X[{min_x:.0f}, {max_x:.0f}] Y[{min_y:.0f}, {max_y:.0f}]")
    print(f"Total samples: {len(positions)}")
    print(f"Unique cells: {len(grid)}")

    # Grid bounds
    gx_min = int((min_x + ARENA_WIDTH/2) / CELL_SIZE)
    gx_max = int((max_x + ARENA_WIDTH/2) / CELL_SIZE)
    gy_min = int((min_y - ARENA_FLOOR_Y) / CELL_SIZE)
    gy_max = int((max_y - ARENA_FLOOR_Y) / CELL_SIZE)

    # Find max count for normalization
    max_count = max(grid.values()) if grid else 1

    # Intensity characters
    chars = ' .:-=+*#%@'

    print(f"\nHeatmap (cell size: {CELL_SIZE}px, max visits: {max_count}):")
    print("-" * (gx_max - gx_min + 3))

    # Render top to bottom (high Y first)
    for gy in range(gy_max, gy_min - 1, -1):
        row = "|"
        for gx in range(gx_min, gx_max + 1):
            count = grid.get((gx, gy), 0)
            intensity = int((count / max_count) * (len(chars) - 1))
            row += chars[intensity]
        row += "|"
        print(row)

    print("-" * (gx_max - gx_min + 3))

    # Legend
    print("\nLegend: ' '=0  '.'=low  '#'=med  '@'=high")

def render_png_heatmap(grid, positions, output_path):
    """Render heatmap as PNG image."""
    try:
        from PIL import Image
    except ImportError:
        print("PIL not available, skipping PNG output")
        return False

    if not positions:
        return False

    # Image dimensions based on arena
    img_width = ARENA_WIDTH // 2
    img_height = ARENA_HEIGHT // 2
    scale = 0.5

    # Create image
    img = Image.new('RGB', (img_width, img_height), (20, 20, 30))
    pixels = img.load()

    # Find max count for normalization
    max_count = max(grid.values()) if grid else 1

    # Draw heatmap
    for (gx, gy), count in grid.items():
        # Convert grid coords back to pixel coords
        px = int(gx * CELL_SIZE * scale)
        py = img_height - int(gy * CELL_SIZE * scale)  # Flip Y

        # Color based on intensity (blue -> green -> yellow -> red)
        intensity = count / max_count
        if intensity < 0.33:
            r, g, b = 0, int(intensity * 3 * 255), int((1 - intensity * 3) * 255)
        elif intensity < 0.66:
            t = (intensity - 0.33) * 3
            r, g, b = int(t * 255), 255, 0
        else:
            t = (intensity - 0.66) * 3
            r, g, b = 255, int((1 - t) * 255), 0

        # Draw cell
        cell_w = int(CELL_SIZE * scale)
        cell_h = int(CELL_SIZE * scale)
        for dx in range(cell_w):
            for dy in range(cell_h):
                nx, ny = px + dx, py + dy
                if 0 <= nx < img_width and 0 <= ny < img_height:
                    pixels[nx, ny] = (r, g, b)

    img.save(output_path)
    print(f"\nPNG heatmap saved to: {output_path}")
    return True

def main():
    db_path = sys.argv[1] if len(sys.argv) > 1 else "db/training.db"
    match_id = int(sys.argv[2]) if len(sys.argv) > 2 else None

    print(f"Loading positions from {db_path}...")
    positions = extract_positions(db_path, match_id)

    if not positions:
        print("No position data found!")
        return

    grid = create_heatmap_grid(positions)
    render_ascii_heatmap(grid, positions)

    # Try PNG output
    render_png_heatmap(grid, positions, "showcase/reachability_heatmap.png")

if __name__ == "__main__":
    main()
