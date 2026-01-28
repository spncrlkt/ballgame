#!/usr/bin/env python3
"""Sum match durations across training DBs."""

import argparse
import sqlite3
from pathlib import Path


def iter_db_paths(args):
    if args.list:
        list_path = Path(args.list)
        if not list_path.exists():
            raise FileNotFoundError(f"List file not found: {list_path}")
        for line in list_path.read_text().splitlines():
            line = line.strip()
            if line:
                yield Path(line)
    for db in args.dbs:
        yield Path(db)


def sum_db(db_path: Path):
    if not db_path.exists():
        return None, None
    conn = sqlite3.connect(str(db_path))
    try:
        cur = conn.cursor()
        cur.execute("SELECT COUNT(*), COALESCE(SUM(duration_secs), 0) FROM matches")
        row = cur.fetchone()
        return row[0], row[1]
    finally:
        conn.close()


def main():
    parser = argparse.ArgumentParser(description="Sum training durations across DBs")
    parser.add_argument("dbs", nargs="*", help="DB paths to include")
    parser.add_argument("--list", help="File with DB paths (one per line)")
    args = parser.parse_args()

    total_matches = 0
    total_secs = 0.0
    any_missing = False

    for db_path in iter_db_paths(args):
        matches, secs = sum_db(db_path)
        if matches is None:
            print(f"Missing DB: {db_path}")
            any_missing = True
            continue
        total_matches += matches
        total_secs += secs
        print(f"{db_path}: {matches} matches, {secs:.1f}s")

    print("---")
    print(f"Total matches: {total_matches}")
    print(f"Total time: {total_secs:.1f}s ({total_secs/60.0:.1f} min)")
    if any_missing:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
