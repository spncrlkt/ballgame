#!/usr/bin/env python3
"""Offline training prereq checker (PASS/FAIL output)."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TODO_PATH = ROOT / "offline_training" / "manual_todo.md"
LEVELS_FILE = ROOT / "config" / "levels.txt"
PROFILES_FILE = ROOT / "config" / "ai_profiles.txt"
HEATMAP_DIR = ROOT / "showcase" / "heatmaps"
DEBUG_CONFIG = ROOT / "config" / "debug_logging.json"
TRAINING_SETTINGS = ROOT / "config" / "training_settings.json"
DEFAULT_LIST = ROOT / "offline_training" / "db_list.txt"

RE_LEVELS_HEADER = re.compile(r"^Non-debug levels to cover$", re.IGNORECASE)
RE_PROFILES_HEADER = re.compile(r"^Profiles \(top 4 from rankings\)$", re.IGNORECASE)
BULLET = re.compile(r"^-\s+(.*)$")

REQUIRED_HEATMAP_TYPES = [
    "speed",
    "score",
    "landing_safety",
    "line_of_sight",
    "elevation",
]


def load_manual_lists(todo_path: Path):
    if not todo_path.exists():
        raise FileNotFoundError(f"missing manual todo: {todo_path}")

    levels = []
    profiles = []
    mode = None

    for line in todo_path.read_text().splitlines():
        line = line.strip()
        if not line:
            continue
        if RE_PROFILES_HEADER.match(line):
            mode = "profiles"
            continue
        if RE_LEVELS_HEADER.match(line):
            mode = "levels"
            continue
        match = BULLET.match(line)
        if not match:
            if line.startswith("##"):
                mode = None
            continue
        value = match.group(1).strip()
        if mode == "profiles":
            profiles.append(value)
        elif mode == "levels":
            levels.append(value)

    return levels, profiles


def load_levels_db(levels_path: Path):
    if not levels_path.exists():
        return []
    names = []
    for line in levels_path.read_text().splitlines():
        line = line.strip()
        if line.startswith("level:"):
            names.append(line.split(":", 1)[1].strip())
    return names


def load_profiles(path: Path):
    if not path.exists():
        return []
    names = []
    for line in path.read_text().splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("profile:"):
            names.append(line.split(":", 1)[1].strip())
        elif line.startswith("name:"):
            names.append(line.split(":", 1)[1].strip())
        else:
            # fallback: first token
            parts = line.split()
            if parts:
                names.append(parts[0])
    return names


def sanitize_level_name(name: str) -> str:
    out = []
    last_underscore = False
    for ch in name:
        if ch.isalnum():
            out.append(ch.lower())
            last_underscore = False
        elif not last_underscore:
            out.append("_")
            last_underscore = True
    return "".join(out).strip("_")


def load_level_ids(levels_path: Path):
    ids = {}
    current = None
    for line in levels_path.read_text().splitlines():
        line = line.strip()
        if line.startswith("level:"):
            current = line.split(":", 1)[1].strip()
        elif line.startswith("id:") and current:
            ids[current] = line.split(":", 1)[1].strip()
    return ids


def list_heatmaps_for_level(level_name: str, level_id: str | None):
    safe = sanitize_level_name(level_name)
    files = []
    for t in REQUIRED_HEATMAP_TYPES:
        if t in ("score", "line_of_sight"):
            for side in ("left", "right"):
                if level_id:
                    files.append(f"heatmap_{t}_{safe}_{level_id}_{side}.txt")
                    files.append(f"heatmap_{t}_{safe}_{level_id}_{side}.png")
                else:
                    files.append(f"heatmap_{t}_{safe}_*_{side}.txt")
                    files.append(f"heatmap_{t}_{safe}_*_{side}.png")
        else:
            if level_id:
                files.append(f"heatmap_{t}_{safe}_{level_id}.txt")
                files.append(f"heatmap_{t}_{safe}_{level_id}.png")
            else:
                files.append(f"heatmap_{t}_{safe}_*.txt")
                files.append(f"heatmap_{t}_{safe}_*.png")
    return files


def match_exists(pattern: str):
    if "*" in pattern:
        return any(HEATMAP_DIR.glob(pattern))
    return (HEATMAP_DIR / pattern).exists()


def check_debug_settings():
    if not DEBUG_CONFIG.exists():
        return False, "debug_logging.json missing"
    try:
        data = json.loads(DEBUG_CONFIG.read_text())
    except Exception as exc:
        return False, f"debug_logging.json invalid: {exc}"
    enabled = bool(data.get("enabled"))
    skip = bool(data.get("skip_reachability_heatmaps"))
    if not enabled:
        return False, "debug logging disabled"
    if not skip:
        return False, "skip_reachability_heatmaps is false"
    return True, "debug logging enabled; reachability skip true"

def check_training_settings(levels: list[str], profiles: list[str]):
    if not TRAINING_SETTINGS.exists():
        return False, f"training settings missing: {TRAINING_SETTINGS}"
    try:
        data = json.loads(TRAINING_SETTINGS.read_text())
    except Exception as exc:
        return False, f"training settings invalid: {exc}"

    level = data.get("level")
    ai_profile = data.get("ai_profile")
    protocol = str(data.get("protocol", "")).lower()
    offline_file = data.get("offline_levels_file")

    if level is not None and level not in levels:
        return False, f"training settings level not in offline list: {level}"
    if ai_profile is not None and ai_profile not in profiles:
        return False, f"training settings ai_profile not in offline list: {ai_profile}"
    if protocol and protocol not in ("advanced-platform", "advanced_platform", "advancedplatform"):
        return False, f"training settings protocol not advanced-platform: {protocol}"
    if not offline_file:
        return False, "training settings offline_levels_file missing"
    offline_path = (ROOT / offline_file).resolve()
    if not offline_path.exists():
        return False, f"offline levels file missing: {offline_path}"
    offline_levels = [
        line.strip().lower()
        for line in offline_path.read_text().splitlines()
        if line.strip() and not line.strip().startswith("#")
    ]
    missing_in_offline = [
        lvl for lvl in levels if lvl.lower() not in offline_levels
    ]
    if missing_in_offline:
        return False, f"offline levels file missing entries: {', '.join(missing_in_offline)}"

    return True, "training settings align with offline lists"

def main():
    print("Offline training prereq check (PASS/FAIL)\n")

    ok = True

    levels, profiles = load_manual_lists(TODO_PATH)
    if not levels:
        print("FAIL: No levels parsed from manual_todo.md")
        return 1
    if not profiles:
        print("FAIL: No profiles parsed from manual_todo.md")
        return 1

    print(f"Levels listed: {len(levels)}")
    print(f"Profiles listed: {len(profiles)}\n")

    level_db = load_levels_db(LEVELS_FILE)
    level_ids = load_level_ids(LEVELS_FILE)
    profile_db = load_profiles(PROFILES_FILE)

    missing_levels = [lvl for lvl in levels if lvl not in level_db]
    if missing_levels:
        ok = False
        print("FAIL: Missing levels in config/levels.txt:")
        for lvl in missing_levels:
            print(f"  - {lvl}")
    else:
        print("PASS: All levels exist in config/levels.txt")

    missing_profiles = [p for p in profiles if p not in profile_db]
    if missing_profiles:
        ok = False
        print("FAIL: Missing profiles in config/ai_profiles.txt:")
        for p in missing_profiles:
            print(f"  - {p}")
    else:
        print("PASS: All profiles exist in config/ai_profiles.txt")

    if not HEATMAP_DIR.exists():
        ok = False
        print(f"FAIL: Heatmap directory missing: {HEATMAP_DIR}")
    else:
        print(f"PASS: Heatmap directory exists: {HEATMAP_DIR}")

    missing_heatmaps = []
    for lvl in levels:
        level_id = level_ids.get(lvl)
        expected = list_heatmaps_for_level(lvl, level_id)
        for pattern in expected:
            if not match_exists(pattern):
                missing_heatmaps.append((lvl, pattern))

    if missing_heatmaps:
        ok = False
        print("FAIL: Missing heatmaps")
        for lvl, pattern in missing_heatmaps:
            print(f"  - {lvl}: {pattern}")
    else:
        print("PASS: All required heatmaps present")

    settings_ok, settings_msg = check_training_settings(levels, profiles)
    if settings_ok:
        print(f"PASS: {settings_msg}")
    else:
        ok = False
        print(f"FAIL: {settings_msg}")

    debug_ok, debug_msg = check_debug_settings()
    if debug_ok:
        print(f"PASS: {debug_msg}")
    else:
        ok = False
        print(f"FAIL: {debug_msg}")

    list_path = DEFAULT_LIST
    if list_path.exists():
        entries = [
            line
            for line in list_path.read_text().splitlines()
            if line.strip() and not line.strip().startswith("#")
        ]
        if entries:
            print(f"PASS: offline DB list has {len(entries)} entries")
        else:
            ok = False
            print("FAIL: offline DB list is empty")
    else:
        ok = False
        print(f"FAIL: offline DB list missing: {list_path}")

    print("\nResult:", "PASS" if ok else "FAIL")
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
