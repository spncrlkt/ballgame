#!/usr/bin/env python3
"""Generate an SVG heatmap from training database position data."""

import sqlite3
import sys
from collections import defaultdict

# Arena dimensions
ARENA_WIDTH = 1400
ARENA_HEIGHT = 900
ARENA_FLOOR_Y = -400

# Grid resolution
CELL_SIZE = 15

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
        parts = data.split('|')
        if len(parts) >= 4:
            pos_str = parts[3]
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
        gx = int((x + ARENA_WIDTH/2) / CELL_SIZE)
        gy = int((y + ARENA_HEIGHT/2) / CELL_SIZE)
        grid[(gx, gy)] += 1

    return grid

def intensity_to_color(intensity):
    """Convert 0-1 intensity to RGB hex color."""
    if intensity < 0.25:
        # Blue to cyan
        t = intensity * 4
        r, g, b = 0, int(t * 200), int(200 + t * 55)
    elif intensity < 0.5:
        # Cyan to green
        t = (intensity - 0.25) * 4
        r, g, b = 0, int(200 + t * 55), int(255 - t * 255)
    elif intensity < 0.75:
        # Green to yellow
        t = (intensity - 0.5) * 4
        r, g, b = int(t * 255), 255, 0
    else:
        # Yellow to red
        t = (intensity - 0.75) * 4
        r, g, b = 255, int(255 - t * 255), 0

    return f"#{r:02x}{g:02x}{b:02x}"

def generate_svg(grid, positions, output_path):
    """Generate SVG heatmap."""
    if not positions:
        print("No position data!")
        return

    # SVG dimensions (scaled down)
    scale = 0.5
    svg_width = int(ARENA_WIDTH * scale)
    svg_height = int(ARENA_HEIGHT * scale)

    max_count = max(grid.values()) if grid else 1

    svg_parts = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{svg_width}" height="{svg_height}">',
        f'<rect width="100%" height="100%" fill="#1a1a2e"/>',
        '<!-- Arena outline -->',
        f'<rect x="0" y="0" width="{svg_width}" height="{svg_height}" fill="none" stroke="#444" stroke-width="2"/>',
    ]

    # Draw floor line
    floor_y = int((ARENA_HEIGHT/2 - ARENA_FLOOR_Y) * scale)
    svg_parts.append(f'<line x1="0" y1="{floor_y}" x2="{svg_width}" y2="{floor_y}" stroke="#666" stroke-width="2"/>')

    # Draw heatmap cells
    cell_w = int(CELL_SIZE * scale)
    cell_h = int(CELL_SIZE * scale)

    for (gx, gy), count in grid.items():
        intensity = count / max_count
        color = intensity_to_color(intensity)

        # Convert to SVG coords (flip Y)
        px = int(gx * CELL_SIZE * scale)
        py = svg_height - int(gy * CELL_SIZE * scale) - cell_h

        svg_parts.append(
            f'<rect x="{px}" y="{py}" width="{cell_w}" height="{cell_h}" '
            f'fill="{color}" opacity="0.8"/>'
        )

    # Add legend
    legend_y = 20
    svg_parts.append(f'<text x="10" y="{legend_y}" fill="white" font-size="12">Samples: {len(positions)}</text>')
    svg_parts.append(f'<text x="10" y="{legend_y + 15}" fill="white" font-size="12">Cells: {len(grid)}</text>')
    svg_parts.append(f'<text x="10" y="{legend_y + 30}" fill="white" font-size="12">Max: {max_count}</text>')

    # Color scale legend
    for i, label in enumerate(['Low', 'Med', 'High']):
        color = intensity_to_color(i / 2)
        svg_parts.append(f'<rect x="{svg_width - 80}" y="{20 + i*20}" width="15" height="15" fill="{color}"/>')
        svg_parts.append(f'<text x="{svg_width - 60}" y="{32 + i*20}" fill="white" font-size="11">{label}</text>')

    svg_parts.append('</svg>')

    svg_content = '\n'.join(svg_parts)

    with open(output_path, 'w') as f:
        f.write(svg_content)

    print(f"SVG heatmap saved to: {output_path}")

    # Also create HTML wrapper for easy viewing
    html_path = output_path.replace('.svg', '.html')
    html_content = f'''<!DOCTYPE html>
<html>
<head><title>Reachability Heatmap</title></head>
<body style="background: #111; margin: 20px;">
<h2 style="color: white;">Reachability Heatmap</h2>
<p style="color: #aaa;">Samples: {len(positions)} | Unique cells: {len(grid)} | Max visits: {max_count}</p>
{svg_content}
</body>
</html>'''

    with open(html_path, 'w') as f:
        f.write(html_content)

    print(f"HTML viewer saved to: {html_path}")

def main():
    db_path = sys.argv[1] if len(sys.argv) > 1 else "db/training.db"
    match_id = int(sys.argv[2]) if len(sys.argv) > 2 else None

    print(f"Loading positions from {db_path}...")
    positions = extract_positions(db_path, match_id)

    print(f"Found {len(positions)} position samples")

    if not positions:
        print("No position data found!")
        return

    grid = create_heatmap_grid(positions)
    generate_svg(grid, positions, "showcase/reachability_heatmap.svg")

if __name__ == "__main__":
    main()
