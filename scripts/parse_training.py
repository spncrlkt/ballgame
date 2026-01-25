#!/usr/bin/env python3
"""
Parse training evlog files into structured JSON for analysis.

Usage:
    python scripts/parse_training.py training_logs/session_20260125_163511/
    python scripts/parse_training.py training_logs/session_20260125_163511/game_1_level7.evlog
"""

import json
import sys
import os
from pathlib import Path
from dataclasses import dataclass, field, asdict
from typing import Optional

@dataclass
class MatchInfo:
    level: int = 0
    level_name: str = ""
    left_profile: str = ""
    right_profile: str = ""
    seed: int = 0

@dataclass
class Config:
    gravity_rise: float = 0
    gravity_fall: float = 0
    jump_velocity: float = 0
    move_speed: float = 0
    shot_max_power: float = 0
    steal_range: float = 0
    steal_success_chance: float = 0

@dataclass
class Goal:
    time_ms: int
    scorer: str  # L or R
    left_score: int
    right_score: int

@dataclass
class Shot:
    time_ms: int
    shooter: str  # L or R
    position: tuple
    charge: float

@dataclass
class Pickup:
    time_ms: int
    player: str  # L or R

@dataclass
class Steal:
    time_ms: int
    attacker: str  # L or R
    success: bool

@dataclass
class AIGoalChange:
    time_ms: int
    player: str  # L or R
    goal: str

@dataclass
class Tick:
    time_ms: int
    tick_num: int
    left_pos: tuple
    left_vel: tuple
    right_pos: tuple
    right_vel: tuple
    ball_pos: tuple
    ball_vel: tuple
    ball_state: str  # F=Free, L=Left holding, R=Right holding, I=InFlight

@dataclass
class Input:
    time_ms: int
    player: str  # L or R
    move_x: float
    actions: str  # J=jump, T=throw, P=pickup, etc.

@dataclass
class ParsedGame:
    session_id: str = ""
    match_info: MatchInfo = field(default_factory=MatchInfo)
    config: Optional[Config] = None
    goals: list = field(default_factory=list)
    shots: list = field(default_factory=list)
    pickups: list = field(default_factory=list)
    steals: list = field(default_factory=list)
    ai_goals: list = field(default_factory=list)
    ticks: list = field(default_factory=list)
    inputs: list = field(default_factory=list)
    final_score_left: int = 0
    final_score_right: int = 0
    duration_ms: int = 0

def parse_evlog(filepath: str) -> ParsedGame:
    """Parse an evlog file into structured data."""
    game = ParsedGame()

    with open(filepath, 'r') as f:
        for line in f:
            line = line.strip()
            if not line:
                continue

            parts = line.split('|')
            if len(parts) < 2:
                continue

            time_part = parts[0]
            event_type = parts[1]

            # Parse time (T:XXXXX format)
            time_ms = 0
            if time_part.startswith('T:'):
                try:
                    time_ms = int(time_part[2:])
                except ValueError:
                    pass

            # Session start
            if event_type == 'SE':
                game.session_id = parts[2] if len(parts) > 2 else ""

            # Match start
            elif event_type == 'MS':
                if len(parts) >= 6:
                    game.match_info = MatchInfo(
                        level=int(parts[2]),
                        level_name=parts[3],
                        left_profile=parts[4],
                        right_profile=parts[5],
                        seed=int(parts[6]) if len(parts) > 6 else 0
                    )

            # Config
            elif event_type == 'CF':
                if len(parts) >= 3:
                    try:
                        cfg = json.loads(parts[2])
                        game.config = Config(
                            gravity_rise=cfg.get('gravity_rise', 0),
                            gravity_fall=cfg.get('gravity_fall', 0),
                            jump_velocity=cfg.get('jump_velocity', 0),
                            move_speed=cfg.get('move_speed', 0),
                            shot_max_power=cfg.get('shot_max_power', 0),
                            steal_range=cfg.get('steal_range', 0),
                            steal_success_chance=cfg.get('steal_success_chance', 0),
                        )
                    except json.JSONDecodeError:
                        pass

            # Goal
            elif event_type == 'G':
                if len(parts) >= 5:
                    game.goals.append(Goal(
                        time_ms=time_ms,
                        scorer=parts[2],
                        left_score=int(parts[3]),
                        right_score=int(parts[4])
                    ))

            # Shot start
            elif event_type == 'SS':
                if len(parts) >= 5:
                    pos_parts = parts[3].split(',')
                    game.shots.append(Shot(
                        time_ms=time_ms,
                        shooter=parts[2],
                        position=(float(pos_parts[0]), float(pos_parts[1])) if len(pos_parts) == 2 else (0, 0),
                        charge=float(parts[4])
                    ))

            # Pickup
            elif event_type == 'PU':
                if len(parts) >= 3:
                    game.pickups.append(Pickup(
                        time_ms=time_ms,
                        player=parts[2]
                    ))

            # Steal attempt
            elif event_type == 'ST':
                if len(parts) >= 4:
                    game.steals.append(Steal(
                        time_ms=time_ms,
                        attacker=parts[2],
                        success=parts[3] == 'Y'
                    ))

            # AI goal change
            elif event_type == 'AG':
                if len(parts) >= 4:
                    game.ai_goals.append(AIGoalChange(
                        time_ms=time_ms,
                        player=parts[2],
                        goal=parts[3]
                    ))

            # Tick (game state snapshot)
            elif event_type == 'T':
                if len(parts) >= 9:
                    def parse_vec(s):
                        p = s.split(',')
                        return (float(p[0]), float(p[1])) if len(p) == 2 else (0, 0)

                    game.ticks.append(Tick(
                        time_ms=time_ms,
                        tick_num=int(parts[2]),
                        left_pos=parse_vec(parts[3]),
                        left_vel=parse_vec(parts[4]),
                        right_pos=parse_vec(parts[5]),
                        right_vel=parse_vec(parts[6]),
                        ball_pos=parse_vec(parts[7]),
                        ball_vel=parse_vec(parts[8]),
                        ball_state=parts[9] if len(parts) > 9 else 'F'
                    ))

            # Input
            elif event_type == 'I':
                if len(parts) >= 4:
                    game.inputs.append(Input(
                        time_ms=time_ms,
                        player=parts[2],
                        move_x=float(parts[3]),
                        actions=parts[4] if len(parts) > 4 else '-'
                    ))

            # Match end
            elif event_type == 'ME':
                if len(parts) >= 4:
                    game.final_score_left = int(parts[2])
                    game.final_score_right = int(parts[3])
                    game.duration_ms = int(float(parts[4]) * 1000) if len(parts) > 4 else time_ms

    return game

def game_to_dict(game: ParsedGame) -> dict:
    """Convert ParsedGame to a JSON-serializable dict."""
    return {
        'session_id': game.session_id,
        'match_info': asdict(game.match_info),
        'config': asdict(game.config) if game.config else None,
        'goals': [asdict(g) for g in game.goals],
        'shots': [asdict(s) for s in game.shots],
        'pickups': [asdict(p) for p in game.pickups],
        'steals': [asdict(s) for s in game.steals],
        'ai_goals': [asdict(a) for a in game.ai_goals],
        'ticks': [asdict(t) for t in game.ticks],
        'inputs': [asdict(i) for i in game.inputs],
        'final_score_left': game.final_score_left,
        'final_score_right': game.final_score_right,
        'duration_ms': game.duration_ms,
    }

def parse_session(session_path: str) -> dict:
    """Parse all evlogs in a session directory."""
    path = Path(session_path)

    if path.is_file() and path.suffix == '.evlog':
        # Single file
        game = parse_evlog(str(path))
        return {
            'session_dir': str(path.parent),
            'games': [game_to_dict(game)]
        }

    if path.is_dir():
        # Directory with multiple evlogs
        games = []
        for evlog in sorted(path.glob('*.evlog')):
            game = parse_evlog(str(evlog))
            games.append(game_to_dict(game))

        # Also load summary.json if present
        summary = {}
        summary_path = path / 'summary.json'
        if summary_path.exists():
            with open(summary_path) as f:
                summary = json.load(f)

        return {
            'session_dir': str(path),
            'summary': summary,
            'games': games
        }

    raise ValueError(f"Path not found or not valid: {session_path}")

def main():
    if len(sys.argv) < 2:
        print("Usage: python parse_training.py <session_dir_or_evlog>")
        print("Output: JSON to stdout")
        sys.exit(1)

    result = parse_session(sys.argv[1])
    print(json.dumps(result, indent=2))

if __name__ == '__main__':
    main()
