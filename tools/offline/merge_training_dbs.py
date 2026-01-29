#!/usr/bin/env python3
"""Merge multiple training DBs into a combined DB with remapped IDs."""

from __future__ import annotations

import argparse
import sqlite3
from pathlib import Path
from typing import Dict, Iterable

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_LIST = ROOT / "offline_training" / "db_list.txt"
DEFAULT_OUT = ROOT / "db" / "combined_offline_training.db"

SCHEMA_SQL = r"""
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    created_at TEXT NOT NULL,
    session_type TEXT NOT NULL,
    config_json TEXT,
    display_name TEXT
);

CREATE TABLE IF NOT EXISTS matches (
    id INTEGER PRIMARY KEY,
    session_id TEXT REFERENCES sessions(id),
    display_name TEXT,
    seed INTEGER NOT NULL,
    level INTEGER NOT NULL,
    level_name TEXT NOT NULL,
    left_profile TEXT NOT NULL,
    right_profile TEXT NOT NULL,
    score_left INTEGER NOT NULL,
    score_right INTEGER NOT NULL,
    duration_secs REAL NOT NULL,
    winner TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS points (
    id INTEGER PRIMARY KEY,
    match_id INTEGER REFERENCES matches(id),
    point_index INTEGER NOT NULL,
    start_time_ms INTEGER NOT NULL,
    end_time_ms INTEGER,
    winner TEXT
);

CREATE TABLE IF NOT EXISTS player_stats (
    id INTEGER PRIMARY KEY,
    match_id INTEGER REFERENCES matches(id),
    side TEXT NOT NULL,
    goals INTEGER NOT NULL,
    shots_attempted INTEGER NOT NULL,
    shots_made INTEGER NOT NULL,
    steals_attempted INTEGER NOT NULL,
    steals_successful INTEGER NOT NULL,
    possession_time REAL NOT NULL,
    distance_traveled REAL NOT NULL,
    jumps INTEGER NOT NULL,
    nav_paths_completed INTEGER NOT NULL,
    nav_paths_failed INTEGER NOT NULL,
    avg_shot_x REAL NOT NULL DEFAULT 0.0,
    avg_shot_y REAL NOT NULL DEFAULT 0.0,
    avg_shot_quality REAL NOT NULL DEFAULT 0.0
);

CREATE TABLE IF NOT EXISTS events (
    id INTEGER PRIMARY KEY,
    match_id INTEGER REFERENCES matches(id),
    point_id INTEGER REFERENCES points(id),
    time_ms INTEGER NOT NULL,
    tick_frame INTEGER NOT NULL DEFAULT 0,
    event_type TEXT NOT NULL,
    data TEXT NOT NULL,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS debug_events (
    id INTEGER PRIMARY KEY,
    match_id INTEGER REFERENCES matches(id),
    time_ms INTEGER NOT NULL,
    tick_frame INTEGER NOT NULL,
    player TEXT NOT NULL,
    pos_x REAL NOT NULL,
    pos_y REAL NOT NULL,
    vel_x REAL NOT NULL,
    vel_y REAL NOT NULL,
    input_move_x REAL NOT NULL,
    input_jump INTEGER NOT NULL,
    grounded INTEGER NOT NULL,
    is_jumping INTEGER NOT NULL,
    coyote_timer REAL NOT NULL,
    jump_buffer_timer REAL NOT NULL,
    facing REAL NOT NULL,
    nav_active INTEGER NOT NULL,
    nav_path_index INTEGER NOT NULL,
    nav_action TEXT,
    level_id TEXT NOT NULL,
    human_controlled INTEGER NOT NULL,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_matches_session ON matches(session_id);
CREATE INDEX IF NOT EXISTS idx_matches_profiles ON matches(left_profile, right_profile);
CREATE INDEX IF NOT EXISTS idx_matches_level ON matches(level);
CREATE INDEX IF NOT EXISTS idx_player_stats_match ON player_stats(match_id);
CREATE INDEX IF NOT EXISTS idx_events_match ON events(match_id);
CREATE INDEX IF NOT EXISTS idx_events_point ON events(point_id);
CREATE INDEX IF NOT EXISTS idx_events_type ON events(event_type);
CREATE INDEX IF NOT EXISTS idx_events_time ON events(match_id, time_ms);
CREATE INDEX IF NOT EXISTS idx_events_tick ON events(match_id, tick_frame);
CREATE INDEX IF NOT EXISTS idx_points_match ON points(match_id);
CREATE INDEX IF NOT EXISTS idx_debug_match ON debug_events(match_id);
CREATE INDEX IF NOT EXISTS idx_debug_time ON debug_events(match_id, time_ms);
CREATE INDEX IF NOT EXISTS idx_debug_tick ON debug_events(match_id, tick_frame);
"""


def read_db_list(path: Path) -> list[Path]:
    if not path.exists():
        raise FileNotFoundError(f"List file not found: {path}")
    dbs = []
    for line in path.read_text().splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        dbs.append(Path(line.split("#", 1)[0].strip()))
    return dbs


def max_id(conn: sqlite3.Connection, table: str) -> int:
    cur = conn.execute(f"SELECT COALESCE(MAX(id), 0) FROM {table}")
    return int(cur.fetchone()[0])


def copy_sessions(dest: sqlite3.Connection, src: sqlite3.Connection):
    rows = src.execute(
        "SELECT id, created_at, session_type, config_json, display_name FROM sessions"
    ).fetchall()
    for row in rows:
        dest.execute(
            "INSERT OR IGNORE INTO sessions (id, created_at, session_type, config_json, display_name) VALUES (?, ?, ?, ?, ?)",
            row,
        )


def merge_db(dest: sqlite3.Connection, src_path: Path):
    if not src_path.exists():
        print(f"Skipping missing DB: {src_path}")
        return
    src = sqlite3.connect(str(src_path))
    try:
        copy_sessions(dest, src)

        match_offset = max_id(dest, "matches")
        point_offset = max_id(dest, "points")
        player_stats_offset = max_id(dest, "player_stats")
        events_offset = max_id(dest, "events")
        debug_offset = max_id(dest, "debug_events")

        match_map: Dict[int, int] = {}
        point_map: Dict[int, int] = {}

        # Matches
        rows = src.execute(
            "SELECT id, session_id, display_name, seed, level, level_name, left_profile, right_profile, score_left, score_right, duration_secs, winner FROM matches"
        ).fetchall()
        for idx, row in enumerate(rows, start=1):
            new_id = match_offset + idx
            match_map[row[0]] = new_id
            dest.execute(
                "INSERT INTO matches (id, session_id, display_name, seed, level, level_name, left_profile, right_profile, score_left, score_right, duration_secs, winner) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                (new_id, *row[1:]),
            )

        # Points
        rows = src.execute(
            "SELECT id, match_id, point_index, start_time_ms, end_time_ms, winner FROM points"
        ).fetchall()
        for idx, row in enumerate(rows, start=1):
            new_id = point_offset + idx
            point_map[row[0]] = new_id
            dest.execute(
                "INSERT INTO points (id, match_id, point_index, start_time_ms, end_time_ms, winner) VALUES (?, ?, ?, ?, ?, ?)",
                (new_id, match_map.get(row[1]), row[2], row[3], row[4], row[5]),
            )

        # Player stats
        rows = src.execute(
            "SELECT id, match_id, side, goals, shots_attempted, shots_made, steals_attempted, steals_successful, possession_time, distance_traveled, jumps, nav_paths_completed, nav_paths_failed, avg_shot_x, avg_shot_y, avg_shot_quality FROM player_stats"
        ).fetchall()
        for idx, row in enumerate(rows, start=1):
            new_id = player_stats_offset + idx
            dest.execute(
                "INSERT INTO player_stats (id, match_id, side, goals, shots_attempted, shots_made, steals_attempted, steals_successful, possession_time, distance_traveled, jumps, nav_paths_completed, nav_paths_failed, avg_shot_x, avg_shot_y, avg_shot_quality) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                (new_id, match_map.get(row[1]), *row[2:]),
            )

        # Events
        rows = src.execute(
            "SELECT id, match_id, point_id, time_ms, tick_frame, event_type, data, created_at FROM events"
        ).fetchall()
        for idx, row in enumerate(rows, start=1):
            new_id = events_offset + idx
            dest.execute(
                "INSERT INTO events (id, match_id, point_id, time_ms, tick_frame, event_type, data, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                (
                    new_id,
                    match_map.get(row[1]),
                    point_map.get(row[2]) if row[2] is not None else None,
                    row[3],
                    row[4],
                    row[5],
                    row[6],
                    row[7],
                ),
            )

        # Debug events
        rows = src.execute(
            "SELECT id, match_id, time_ms, tick_frame, player, pos_x, pos_y, vel_x, vel_y, input_move_x, input_jump, grounded, is_jumping, coyote_timer, jump_buffer_timer, facing, nav_active, nav_path_index, nav_action, level_id, human_controlled, created_at FROM debug_events"
        ).fetchall()
        for idx, row in enumerate(rows, start=1):
            new_id = debug_offset + idx
            dest.execute(
                "INSERT INTO debug_events (id, match_id, time_ms, tick_frame, player, pos_x, pos_y, vel_x, vel_y, input_move_x, input_jump, grounded, is_jumping, coyote_timer, jump_buffer_timer, facing, nav_active, nav_path_index, nav_action, level_id, human_controlled, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                (new_id, match_map.get(row[1]), *row[2:]),
            )

        dest.commit()
        print(f"Merged {src_path}")
    finally:
        src.close()


def main():
    parser = argparse.ArgumentParser(description="Merge training DBs into one combined DB")
    parser.add_argument("--list", default=str(DEFAULT_LIST), help="DB list file")
    parser.add_argument("--out", default=str(DEFAULT_OUT), help="Output DB path")
    args = parser.parse_args()

    dbs = read_db_list(Path(args.list))
    if not dbs:
        print("No DBs found in list.")
        return 1

    out_path = Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    if out_path.exists():
        out_path.unlink()

    dest = sqlite3.connect(str(out_path))
    try:
        dest.executescript(SCHEMA_SQL)
        for db in dbs:
            merge_db(dest, db)
    finally:
        dest.close()

    print(f"Combined DB written to {out_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
