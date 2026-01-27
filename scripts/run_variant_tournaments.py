#!/usr/bin/env python3
import argparse
import json
import os
import pathlib
import re
import shutil
import subprocess
import sys
import time
from dataclasses import dataclass
from typing import Dict, List, Tuple, Optional

ROOT = pathlib.Path(__file__).resolve().parents[1]

@dataclass
class VariantResult:
    variant_id: str
    label: str
    constants: Dict[str, float]
    profile_deltas: Dict[str, float]
    db_path: str
    focused_report: str
    event_audit_report: str
    summary: Dict[str, float]


def read_text(path: pathlib.Path) -> str:
    return path.read_text()


def write_text(path: pathlib.Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content)


def run_cmd(cmd: List[str], cwd: pathlib.Path, env: Optional[Dict[str, str]] = None) -> Tuple[int, str]:
    run_env = None
    if env:
        run_env = os.environ.copy()
        run_env.update(env)
    proc = subprocess.run(
        cmd,
        cwd=cwd,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        env=run_env,
    )
    return proc.returncode, proc.stdout


def parse_top_profiles(ai_profiles_path: pathlib.Path, count: int) -> List[str]:
    names = []
    for line in ai_profiles_path.read_text().splitlines():
        if line.startswith("profile:"):
            names.append(line.split(":", 1)[1].strip())
            if len(names) >= count:
                break
    if len(names) < count:
        raise RuntimeError(f"Found only {len(names)} profiles in {ai_profiles_path}")
    return names


def parse_level_names(levels_path: pathlib.Path) -> List[str]:
    names = []
    for line in levels_path.read_text().splitlines():
        line = line.strip()
        if line.startswith("level:"):
            names.append(line.split(":", 1)[1].strip())
    return names


def resolve_levels_from_sim_settings(levels_path: pathlib.Path, sim_settings_path: pathlib.Path) -> List[str]:
    if not sim_settings_path.exists():
        return []
    data = json.loads(sim_settings_path.read_text())
    levels = data.get("levels", []) or []
    if not levels:
        return []
    level_names = parse_level_names(levels_path)
    resolved = []
    for lvl in levels:
        try:
            idx = int(lvl) - 1
        except (ValueError, TypeError):
            continue
        if 0 <= idx < len(level_names):
            resolved.append(level_names[idx])
    return resolved


def apply_constants(constants_path: pathlib.Path, constants: Dict[str, float]) -> None:
    text = constants_path.read_text()
    for key, value in constants.items():
        pattern = re.compile(rf"^pub const {re.escape(key)}: f32 = ([0-9.]+);$", re.M)
        if not pattern.search(text):
            raise RuntimeError(f"Could not find constant {key} in {constants_path}")
        text = pattern.sub(f"pub const {key}: f32 = {value};", text)
    constants_path.write_text(text)


def parse_profiles(ai_profiles_path: pathlib.Path) -> List[Dict[str, str]]:
    blocks = []
    current = []
    for line in ai_profiles_path.read_text().splitlines():
        if line.startswith("profile:") and current:
            blocks.append(current)
            current = [line]
        else:
            current.append(line)
    if current:
        blocks.append(current)
    return blocks


def update_profile_block(block: List[str], deltas: Dict[str, float]) -> List[str]:
    out = []
    profile_name = None
    for line in block:
        if line.startswith("profile:"):
            profile_name = line.split(":", 1)[1].strip()
        out.append(line)
    # Apply deltas in-place
    for i, line in enumerate(out):
        for key, delta in deltas.items():
            if line.startswith(f"{key}:"):
                base = float(line.split(":", 1)[1].strip())
                out[i] = f"{key}: {base + delta}"
    return out


def apply_profile_deltas(ai_profiles_path: pathlib.Path, top_profiles: List[str], deltas: Dict[str, float]) -> None:
    blocks = parse_profiles(ai_profiles_path)
    updated = []
    for block in blocks:
        name_line = next((line for line in block if line.startswith("profile:")), None)
        if not name_line:
            updated.append(block)
            continue
        name = name_line.split(":", 1)[1].strip()
        if name in top_profiles:
            updated.append(update_profile_block(block, deltas))
        else:
            updated.append(block)
    ai_profiles_path.write_text("\n".join("\n".join(b) for b in updated) + "\n")


def parse_focused_summary(report_path: pathlib.Path) -> Dict[str, float]:
    summary = {}
    text = report_path.read_text()
    summary_block = re.search(r"## Summary\n(.*?)\n\n", text, re.S)
    if not summary_block:
        return summary
    for line in summary_block.group(1).splitlines():
        m = re.match(r"- ([^:]+): ([0-9.]+)", line.strip())
        if m:
            summary[m.group(1).strip()] = float(m.group(2))
    return summary


def pick_top_variants(results: List[VariantResult], count: int = 3) -> List[VariantResult]:
    def score(r: VariantResult) -> Tuple[float, float, float]:
        goals = r.summary.get("Goals/match", 0.0)
        shots = r.summary.get("Shots/match", 0.0)
        scoreless = r.summary.get("Scoreless rate", 1.0)
        return (goals, shots, -scoreless)
    return sorted(results, key=score, reverse=True)[:count]


def main() -> int:
    parser = argparse.ArgumentParser(description="Run variant tournaments + analysis")
    parser.add_argument("--variants", default="config/heatmap_variants.json")
    parser.add_argument("--baseline-db", required=True)
    parser.add_argument("--matches-per-pair", type=int, default=2)
    parser.add_argument("--profiles", default=None)
    parser.add_argument("--parallel", type=int, default=16)
    parser.add_argument("--skip-heatmaps", action="store_true")
    parser.add_argument("--heatmap-levels", default=None)
    parser.add_argument("--levels-from-sim-settings", action="store_true")
    parser.add_argument("--result-file", default=None)
    args = parser.parse_args()

    variants_path = ROOT / args.variants
    data = json.loads(variants_path.read_text())
    variants = data.get("variants", [])
    if not variants:
        print("No variants found.")
        return 1

    constants_path = ROOT / "src/constants.rs"
    ai_profiles_path = ROOT / "config/ai_profiles.txt"
    levels_path = ROOT / "config/levels.txt"
    sim_settings_path = ROOT / "config/simulation_settings.json"

    if args.profiles:
        top_profiles = [p.strip() for p in args.profiles.split(",") if p.strip()]
    else:
        top_profiles = parse_top_profiles(ai_profiles_path, 4)

    baseline_db = args.baseline_db

    # Backup files
    constants_backup = constants_path.read_text()
    profiles_backup = ai_profiles_path.read_text()

    results: List[VariantResult] = []
    try:
        skip_reachability = False
        if not args.skip_heatmaps:
            level_list = []
            if args.heatmap_levels:
                level_list = [lvl.strip() for lvl in args.heatmap_levels.split(",") if lvl.strip()]
            elif args.levels_from_sim_settings:
                level_list = resolve_levels_from_sim_settings(levels_path, sim_settings_path)
            if not level_list:
                raise RuntimeError(
                    "No heatmap levels specified. Provide --heatmap-levels or ensure config/simulation_settings.json has levels."
                )
            skip_reachability = True
            print("NOTE: reachability-dependent heatmaps disabled (TODO in src/bin/heatmap.rs).")
            heatmap_kinds = [
                "speed",
                "score",
                "landing_safety",
                "line_of_sight",
                "elevation",
            ]
            for idx, kind in enumerate(heatmap_kinds):
                heatmap_cmd = [
                    "cargo",
                    "run",
                    "--release",
                    "--bin",
                    "heatmap",
                    "--",
                    "--type",
                    kind,
                    "--check",
                ]
                if idx == 0:
                    heatmap_cmd.append("--refresh")
                for level in level_list:
                    heatmap_cmd.extend(["--level", level])
                code, out = run_cmd(heatmap_cmd, ROOT)
                if code != 0:
                    print(out)
                    raise RuntimeError(f"Heatmap generation failed for type {kind}")

        for variant in variants:
            vid = variant["id"]
            label = variant.get("label", "")
            constants = variant.get("constants", {})
            deltas = variant.get("profile_deltas", {})

            apply_constants(constants_path, constants)
            apply_profile_deltas(ai_profiles_path, top_profiles, deltas)

            cmd = [
                "cargo", "run", "--release", "--bin", "simulate", "--",
                "--tournament", str(args.matches_per_pair),
                "--parallel", str(args.parallel),
                "--profiles", ",".join(top_profiles),
            ]
            env = {"BALLGAME_SKIP_REACHABILITY_HEATMAPS": "1"} if skip_reachability else None
            code, out = run_cmd(cmd, ROOT, env=env)
            if code != 0:
                print(out)
                raise RuntimeError(f"Tournament failed for {vid}")
            m = re.search(r"Using database: (db/[^\s]+)", out)
            if not m:
                raise RuntimeError("Could not parse tournament DB path")
            db_path = m.group(1)

            code, out = run_cmd(["cargo", "run", "--bin", "analyze", "--", "--focused", db_path], ROOT)
            if code != 0:
                print(out)
                raise RuntimeError(f"Focused analysis failed for {vid}")
            m = re.search(r"Focused analysis written to (notes/[^\s]+)", out)
            if not m:
                raise RuntimeError("Could not parse focused report path")
            focused_report = m.group(1)

            code, out = run_cmd([
                "cargo", "run", "--bin", "analyze", "--",
                "--event-audit", baseline_db, db_path
            ], ROOT)
            if code != 0:
                print(out)
                raise RuntimeError(f"Event audit failed for {vid}")
            m = re.search(r"Event audit written to (notes/[^\s]+)", out)
            if not m:
                raise RuntimeError("Could not parse event audit path")
            event_audit_report = m.group(1)

            summary = parse_focused_summary(ROOT / focused_report)
            results.append(VariantResult(
                variant_id=vid,
                label=label,
                constants=constants,
                profile_deltas=deltas,
                db_path=db_path,
                focused_report=focused_report,
                event_audit_report=event_audit_report,
                summary=summary,
            ))

        top3 = pick_top_variants(results, 3)
        pairwise = []
        for i in range(len(top3)):
            for j in range(i + 1, len(top3)):
                left = top3[i]
                right = top3[j]
                code, out = run_cmd([
                    "cargo", "run", "--bin", "analyze", "--",
                    "--event-audit", left.db_path, right.db_path
                ], ROOT)
                if code != 0:
                    print(out)
                    raise RuntimeError("Pairwise event audit failed")
                m = re.search(r"Event audit written to (notes/[^\s]+)", out)
                if not m:
                    raise RuntimeError("Could not parse pairwise audit path")
                pairwise.append((left.variant_id, right.variant_id, m.group(1)))

        timestamp = time.strftime("%Y%m%d_%H%M%S")
        result_path = pathlib.Path(args.result_file) if args.result_file else ROOT / f"notes/analysis_runs/variant_tournament_summary_{timestamp}.md"

        lines = []
        lines.append("# Variant Tournament Summary\n")
        lines.append(f"Baseline DB: `{baseline_db}`\n")
        lines.append(f"Profiles: {', '.join(top_profiles)}\n")
        lines.append(f"Matches per pair: {args.matches_per_pair}\n")
        lines.append(f"Parallel: {args.parallel}\n")
        lines.append("\n## Variants\n")
        for r in results:
            lines.append(f"### {r.variant_id} ({r.label})\n")
            lines.append(f"DB: `{r.db_path}`\n")
            lines.append(f"Focused report: `{r.focused_report}`\n")
            lines.append(f"Event audit: `{r.event_audit_report}`\n")
            lines.append("Constants:\n")
            for k, v in r.constants.items():
                lines.append(f"- {k}: {v}")
            lines.append("Profile deltas:\n")
            for k, v in r.profile_deltas.items():
                lines.append(f"- {k}: {v}")
            lines.append("Summary:\n")
            for k, v in r.summary.items():
                lines.append(f"- {k}: {v}")
            lines.append("\n")

        lines.append("## Top 3 Variants (by goals/match, shots/match, scoreless rate)\n")
        for r in top3:
            lines.append(f"- {r.variant_id}: goals {r.summary.get('Goals/match', 0.0):.3f}, shots {r.summary.get('Shots/match', 0.0):.3f}, scoreless {r.summary.get('Scoreless rate', 0.0):.3f}")
        lines.append("\n## Pairwise Deltas (Top 3)\n")
        for a, b, report in pairwise:
            lines.append(f"- {a} vs {b}: `{report}`")

        lines.append("\n## Suggestions\n")
        lines.append("- Favor variants that increase shots/match without dropping shot% below baseline.")
        lines.append("- If scoreless rate rises above 0.20, raise min_shot_quality or tighten LOS threshold.")
        lines.append("- For higher tempo, reduce position_patience and seek_threshold, but watch steal attempts.")

        write_text(result_path, "\n".join(lines) + "\n")
        print(f"Summary written to {result_path}")

    finally:
        constants_path.write_text(constants_backup)
        ai_profiles_path.write_text(profiles_backup)

    return 0

if __name__ == "__main__":
    raise SystemExit(main())
