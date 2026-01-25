#!/usr/bin/env python3
"""
Analyze training session data and generate insights.

Usage:
    python scripts/analyze_training.py training_logs/session_20260125_163511/
    python scripts/analyze_training.py  # Analyzes most recent session
"""

import json
import sys
import os
from pathlib import Path
from dataclasses import dataclass, field
from typing import Optional, List, Tuple
from collections import defaultdict

# ============================================================================
# PARSING
# ============================================================================

@dataclass
class Goal:
    time_ms: int
    scorer: str
    left_score: int
    right_score: int

@dataclass
class Shot:
    time_ms: int
    shooter: str
    position: Tuple[float, float]
    charge: float

@dataclass
class Pickup:
    time_ms: int
    player: str

@dataclass
class Steal:
    time_ms: int
    attacker: str
    success: bool

@dataclass
class AIGoalChange:
    time_ms: int
    player: str
    goal: str

@dataclass
class Tick:
    time_ms: int
    tick_num: int
    left_pos: Tuple[float, float]
    left_vel: Tuple[float, float]
    right_pos: Tuple[float, float]
    right_vel: Tuple[float, float]
    ball_pos: Tuple[float, float]
    ball_vel: Tuple[float, float]
    ball_state: str

@dataclass
class Input:
    time_ms: int
    player: str
    move_x: float
    actions: str

@dataclass
class ParsedGame:
    filepath: str = ""
    session_id: str = ""
    level: int = 0
    level_name: str = ""
    left_profile: str = ""
    right_profile: str = ""
    goals: List[Goal] = field(default_factory=list)
    shots: List[Shot] = field(default_factory=list)
    pickups: List[Pickup] = field(default_factory=list)
    steals: List[Steal] = field(default_factory=list)
    ai_goals: List[AIGoalChange] = field(default_factory=list)
    ticks: List[Tick] = field(default_factory=list)
    inputs: List[Input] = field(default_factory=list)
    final_score_left: int = 0
    final_score_right: int = 0
    duration_ms: int = 0
    notes: str = ""

def parse_evlog(filepath: str) -> ParsedGame:
    """Parse an evlog file into structured data."""
    game = ParsedGame(filepath=filepath)

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

            time_ms = 0
            if time_part.startswith('T:'):
                try:
                    time_ms = int(time_part[2:])
                except ValueError:
                    pass

            if event_type == 'SE':
                game.session_id = parts[2] if len(parts) > 2 else ""

            elif event_type == 'MS':
                if len(parts) >= 6:
                    game.level = int(parts[2])
                    game.level_name = parts[3]
                    game.left_profile = parts[4]
                    game.right_profile = parts[5]

            elif event_type == 'G':
                if len(parts) >= 5:
                    game.goals.append(Goal(
                        time_ms=time_ms,
                        scorer=parts[2],
                        left_score=int(parts[3]),
                        right_score=int(parts[4])
                    ))

            elif event_type == 'SS':
                if len(parts) >= 5:
                    pos_parts = parts[3].split(',')
                    game.shots.append(Shot(
                        time_ms=time_ms,
                        shooter=parts[2],
                        position=(float(pos_parts[0]), float(pos_parts[1])) if len(pos_parts) == 2 else (0, 0),
                        charge=float(parts[4])
                    ))

            elif event_type == 'PU':
                if len(parts) >= 3:
                    game.pickups.append(Pickup(time_ms=time_ms, player=parts[2]))

            elif event_type == 'ST':
                if len(parts) >= 4:
                    game.steals.append(Steal(
                        time_ms=time_ms,
                        attacker=parts[2],
                        success=parts[3] == 'Y'
                    ))

            elif event_type == 'AG':
                if len(parts) >= 4:
                    game.ai_goals.append(AIGoalChange(
                        time_ms=time_ms,
                        player=parts[2],
                        goal=parts[3]
                    ))

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

            elif event_type == 'I':
                if len(parts) >= 4:
                    game.inputs.append(Input(
                        time_ms=time_ms,
                        player=parts[2],
                        move_x=float(parts[3]),
                        actions=parts[4] if len(parts) > 4 else '-'
                    ))

            elif event_type == 'ME':
                if len(parts) >= 4:
                    game.final_score_left = int(parts[2])
                    game.final_score_right = int(parts[3])
                    game.duration_ms = int(float(parts[4]) * 1000) if len(parts) > 4 else time_ms

    return game

# ============================================================================
# ANALYSIS
# ============================================================================

def analyze_possession(game: ParsedGame) -> dict:
    """Analyze ball possession over time."""
    left_possession_ms = 0
    right_possession_ms = 0
    contested_ms = 0

    prev_time = 0
    prev_state = 'F'

    for tick in game.ticks:
        duration = tick.time_ms - prev_time
        if prev_state == 'L':
            left_possession_ms += duration
        elif prev_state == 'R':
            right_possession_ms += duration
        else:
            contested_ms += duration
        prev_time = tick.time_ms
        prev_state = tick.ball_state

    total = left_possession_ms + right_possession_ms + contested_ms
    if total == 0:
        total = 1

    return {
        'left_possession_pct': round(100 * left_possession_ms / total, 1),
        'right_possession_pct': round(100 * right_possession_ms / total, 1),
        'contested_pct': round(100 * contested_ms / total, 1),
        'left_possession_sec': round(left_possession_ms / 1000, 1),
        'right_possession_sec': round(right_possession_ms / 1000, 1),
    }

def analyze_positioning(game: ParsedGame) -> dict:
    """Analyze player positioning patterns."""
    left_positions = [(t.left_pos[0], t.left_pos[1]) for t in game.ticks]
    right_positions = [(t.right_pos[0], t.right_pos[1]) for t in game.ticks]

    if not left_positions:
        return {}

    # Average positions
    left_avg_x = sum(p[0] for p in left_positions) / len(left_positions)
    left_avg_y = sum(p[1] for p in left_positions) / len(left_positions)
    right_avg_x = sum(p[0] for p in right_positions) / len(right_positions)
    right_avg_y = sum(p[1] for p in right_positions) / len(right_positions)

    # Position ranges
    left_x_range = (min(p[0] for p in left_positions), max(p[0] for p in left_positions))
    right_x_range = (min(p[0] for p in right_positions), max(p[0] for p in right_positions))

    # Time spent on each side of court (x < 0 = left side, x > 0 = right side)
    left_on_left_side = sum(1 for p in left_positions if p[0] < 0)
    left_on_right_side = len(left_positions) - left_on_left_side
    right_on_left_side = sum(1 for p in right_positions if p[0] < 0)
    right_on_right_side = len(right_positions) - right_on_left_side

    return {
        'left_player': {
            'avg_position': (round(left_avg_x, 1), round(left_avg_y, 1)),
            'x_range': (round(left_x_range[0], 1), round(left_x_range[1], 1)),
            'time_on_own_side_pct': round(100 * left_on_left_side / len(left_positions), 1),
            'time_on_attack_side_pct': round(100 * left_on_right_side / len(left_positions), 1),
        },
        'right_player': {
            'avg_position': (round(right_avg_x, 1), round(right_avg_y, 1)),
            'x_range': (round(right_x_range[0], 1), round(right_x_range[1], 1)),
            'time_on_own_side_pct': round(100 * right_on_right_side / len(right_positions), 1),
            'time_on_attack_side_pct': round(100 * right_on_left_side / len(right_positions), 1),
        }
    }

def analyze_ai_behavior(game: ParsedGame) -> dict:
    """Analyze AI goal transitions and decision patterns."""
    goal_counts = defaultdict(int)
    goal_durations = defaultdict(int)
    transitions = []

    prev_goal = None
    prev_time = 0

    for ag in game.ai_goals:
        goal_counts[ag.goal] += 1
        if prev_goal:
            goal_durations[prev_goal] += ag.time_ms - prev_time
            transitions.append((prev_goal, ag.goal))
        prev_goal = ag.goal
        prev_time = ag.time_ms

    # Add final goal duration
    if prev_goal and game.duration_ms:
        goal_durations[prev_goal] += game.duration_ms - prev_time

    # Common transitions
    transition_counts = defaultdict(int)
    for t in transitions:
        transition_counts[f"{t[0]} -> {t[1]}"] += 1

    return {
        'goal_counts': dict(goal_counts),
        'goal_durations_ms': dict(goal_durations),
        'transitions': dict(transition_counts),
        'total_goal_changes': len(game.ai_goals),
    }

def analyze_shots(game: ParsedGame) -> dict:
    """Analyze shooting patterns."""
    left_shots = [s for s in game.shots if s.shooter == 'L']
    right_shots = [s for s in game.shots if s.shooter == 'R']

    def shot_stats(shots):
        if not shots:
            return {'count': 0}
        charges = [s.charge for s in shots]
        positions = [s.position for s in shots]
        return {
            'count': len(shots),
            'avg_charge': round(sum(charges) / len(charges), 2),
            'min_charge': round(min(charges), 2),
            'max_charge': round(max(charges), 2),
            'avg_x': round(sum(p[0] for p in positions) / len(positions), 1),
            'avg_y': round(sum(p[1] for p in positions) / len(positions), 1),
        }

    return {
        'left_shots': shot_stats(left_shots),
        'right_shots': shot_stats(right_shots),
    }

def analyze_steals(game: ParsedGame) -> dict:
    """Analyze steal attempts."""
    left_steals = [s for s in game.steals if s.attacker == 'L']
    right_steals = [s for s in game.steals if s.attacker == 'R']

    def steal_stats(steals):
        if not steals:
            return {'attempts': 0, 'successes': 0, 'rate': 0}
        successes = sum(1 for s in steals if s.success)
        return {
            'attempts': len(steals),
            'successes': successes,
            'rate': round(100 * successes / len(steals), 1) if steals else 0,
        }

    return {
        'left_steals': steal_stats(left_steals),
        'right_steals': steal_stats(right_steals),
    }

def analyze_goals(game: ParsedGame) -> dict:
    """Analyze goal patterns."""
    if not game.goals:
        return {'total': 0}

    times_between = []
    prev_time = 0
    for g in game.goals:
        times_between.append(g.time_ms - prev_time)
        prev_time = g.time_ms

    return {
        'total': len(game.goals),
        'left_scored': sum(1 for g in game.goals if g.scorer == 'L'),
        'right_scored': sum(1 for g in game.goals if g.scorer == 'R'),
        'avg_time_between_sec': round(sum(times_between) / len(times_between) / 1000, 1),
        'goal_times_sec': [round(g.time_ms / 1000, 1) for g in game.goals],
    }

def analyze_inputs(game: ParsedGame) -> dict:
    """Analyze input patterns."""
    left_inputs = [i for i in game.inputs if i.player == 'L']
    right_inputs = [i for i in game.inputs if i.player == 'R']

    def input_stats(inputs):
        if not inputs:
            return {}

        # Movement analysis
        moving_left = sum(1 for i in inputs if i.move_x < -0.1)
        moving_right = sum(1 for i in inputs if i.move_x > 0.1)
        stationary = len(inputs) - moving_left - moving_right

        # Action counts
        jumps = sum(1 for i in inputs if 'J' in i.actions)
        throws = sum(1 for i in inputs if 'T' in i.actions)
        pickups = sum(1 for i in inputs if 'P' in i.actions)

        return {
            'movement': {
                'left_pct': round(100 * moving_left / len(inputs), 1),
                'right_pct': round(100 * moving_right / len(inputs), 1),
                'stationary_pct': round(100 * stationary / len(inputs), 1),
            },
            'actions': {
                'jumps': jumps,
                'throws': throws,
                'pickups': pickups,
            }
        }

    return {
        'left_inputs': input_stats(left_inputs),
        'right_inputs': input_stats(right_inputs),
    }

def generate_insights(game: ParsedGame, analysis: dict) -> List[str]:
    """Generate human-readable insights from analysis."""
    insights = []

    # Winner
    if game.final_score_left > game.final_score_right:
        insights.append(f"Player (Left) won {game.final_score_left}-{game.final_score_right} in {game.duration_ms/1000:.1f}s")
    elif game.final_score_right > game.final_score_left:
        insights.append(f"AI (Right/{game.right_profile}) won {game.final_score_right}-{game.final_score_left} in {game.duration_ms/1000:.1f}s")

    # Possession
    poss = analysis.get('possession', {})
    if poss.get('left_possession_pct', 0) > 60:
        insights.append(f"Player dominated possession ({poss['left_possession_pct']}%)")
    elif poss.get('right_possession_pct', 0) > 60:
        insights.append(f"AI dominated possession ({poss['right_possession_pct']}%)")

    # Positioning
    pos = analysis.get('positioning', {})
    left_pos = pos.get('left_player', {})
    right_pos = pos.get('right_player', {})

    if left_pos.get('time_on_attack_side_pct', 0) > 60:
        insights.append(f"Player was aggressive, spending {left_pos['time_on_attack_side_pct']}% of time on attack side")

    if right_pos.get('time_on_attack_side_pct', 0) < 30:
        insights.append(f"AI stayed defensive, only {right_pos['time_on_attack_side_pct']}% time on attack side")

    # AI behavior
    ai = analysis.get('ai_behavior', {})
    goal_counts = ai.get('goal_counts', {})

    if goal_counts:
        most_common_goal = max(goal_counts.items(), key=lambda x: x[1])
        insights.append(f"AI most common goal: {most_common_goal[0]} ({most_common_goal[1]} times)")

    # Check for AI indecision (many goal changes)
    if ai.get('total_goal_changes', 0) > 10:
        insights.append(f"AI was indecisive with {ai['total_goal_changes']} goal changes")

    # Shots
    shots = analysis.get('shots', {})
    left_shots = shots.get('left_shots', {})
    right_shots = shots.get('right_shots', {})

    if left_shots.get('count', 0) > 0 and right_shots.get('count', 0) == 0:
        insights.append("AI never got a shot off")

    if left_shots.get('avg_charge', 0) < 0.3:
        insights.append(f"Player used quick shots (avg charge: {left_shots['avg_charge']})")
    elif left_shots.get('avg_charge', 0) > 0.7:
        insights.append(f"Player used power shots (avg charge: {left_shots['avg_charge']})")

    # Steals
    steals = analysis.get('steals', {})
    right_steals = steals.get('right_steals', {})
    if right_steals.get('attempts', 0) == 0:
        insights.append("AI never attempted a steal")
    elif right_steals.get('rate', 0) == 0 and right_steals.get('attempts', 0) > 0:
        insights.append(f"AI steal attempts all failed ({right_steals['attempts']} attempts)")

    return insights

def identify_ai_weaknesses(game: ParsedGame, analysis: dict) -> List[str]:
    """Identify specific AI weaknesses that could be improved."""
    weaknesses = []

    ai = analysis.get('ai_behavior', {})
    pos = analysis.get('positioning', {})
    shots = analysis.get('shots', {})
    steals = analysis.get('steals', {})

    # Check AI positioning
    right_pos = pos.get('right_player', {})
    if right_pos.get('time_on_attack_side_pct', 0) < 20:
        weaknesses.append("AI too passive - rarely moves to attack side")

    # Check if AI chases ball when player has it
    goal_counts = ai.get('goal_counts', {})
    if goal_counts.get('ChaseBall', 0) > goal_counts.get('InterceptDefense', 0) * 2:
        weaknesses.append("AI prioritizes ChaseBall over InterceptDefense even when player has ball")

    # Check AI shooting
    right_shots = shots.get('right_shots', {})
    if right_shots.get('count', 0) == 0 and game.duration_ms > 10000:
        weaknesses.append("AI never shoots - may be stuck in defensive/chase patterns")

    # Check AI steal behavior
    right_steals = steals.get('right_steals', {})
    if right_steals.get('attempts', 0) == 0:
        weaknesses.append("AI never attempts steals - AttemptSteal goal may not trigger or execute")

    # Check for goal oscillation
    transitions = ai.get('transitions', {})
    for trans, count in transitions.items():
        if count > 3:
            weaknesses.append(f"AI oscillates between goals: {trans} ({count} times)")

    return weaknesses

# ============================================================================
# MAIN
# ============================================================================

def find_most_recent_session() -> Optional[Path]:
    """Find the most recent training session directory."""
    logs_dir = Path('training_logs')
    if not logs_dir.exists():
        return None

    sessions = sorted(logs_dir.glob('session_*'), reverse=True)
    return sessions[0] if sessions else None

def analyze_session(session_path: Path) -> dict:
    """Analyze all games in a session."""
    games = []

    # Load summary if exists
    summary = {}
    summary_path = session_path / 'summary.json'
    if summary_path.exists():
        with open(summary_path) as f:
            summary = json.load(f)

    # Parse and analyze each evlog
    for evlog in sorted(session_path.glob('*.evlog')):
        game = parse_evlog(str(evlog))

        # Add notes from summary
        if summary.get('games'):
            for sg in summary['games']:
                if sg.get('evlog') == evlog.name:
                    game.notes = sg.get('notes', '')

        analysis = {
            'possession': analyze_possession(game),
            'positioning': analyze_positioning(game),
            'ai_behavior': analyze_ai_behavior(game),
            'shots': analyze_shots(game),
            'steals': analyze_steals(game),
            'goals': analyze_goals(game),
            'inputs': analyze_inputs(game),
        }

        insights = generate_insights(game, analysis)
        weaknesses = identify_ai_weaknesses(game, analysis)

        games.append({
            'file': evlog.name,
            'level': game.level_name,
            'score': f"{game.final_score_left}-{game.final_score_right}",
            'duration_sec': round(game.duration_ms / 1000, 1),
            'ai_profile': game.right_profile,
            'notes': game.notes,
            'analysis': analysis,
            'insights': insights,
            'ai_weaknesses': weaknesses,
        })

    return {
        'session': str(session_path),
        'summary': summary,
        'games': games,
    }

def print_report(result: dict):
    """Print a human-readable report."""
    print("=" * 70)
    print(f"TRAINING SESSION ANALYSIS: {result['session']}")
    print("=" * 70)

    summary = result.get('summary', {})
    if summary:
        print(f"\nSession: {summary.get('games_played', 0)} games")
        print(f"Player Wins: {summary.get('player_wins', 0)}")
        print(f"AI Wins: {summary.get('ai_wins', 0)}")
        print(f"AI Profile: {summary.get('ai_profile', 'Unknown')}")

    for game in result['games']:
        print("\n" + "-" * 70)
        print(f"GAME: {game['file']}")
        print(f"Level: {game['level']} | Score: {game['score']} | Duration: {game['duration_sec']}s")
        print(f"AI Profile: {game['ai_profile']}")

        if game['notes']:
            print(f"Notes: {game['notes']}")

        print("\n## Insights")
        for insight in game['insights']:
            print(f"  - {insight}")

        print("\n## AI Weaknesses")
        if game['ai_weaknesses']:
            for weakness in game['ai_weaknesses']:
                print(f"  - {weakness}")
        else:
            print("  - No obvious weaknesses detected")

        print("\n## Detailed Analysis")
        analysis = game['analysis']

        # Possession
        poss = analysis['possession']
        print(f"  Possession: Player {poss['left_possession_pct']}% | AI {poss['right_possession_pct']}% | Contested {poss['contested_pct']}%")

        # AI Goals
        ai = analysis['ai_behavior']
        if ai['goal_counts']:
            goals_str = ', '.join(f"{k}:{v}" for k, v in ai['goal_counts'].items())
            print(f"  AI Goals: {goals_str}")

        # Shots
        shots = analysis['shots']
        left_s = shots['left_shots']
        right_s = shots['right_shots']
        print(f"  Shots: Player {left_s.get('count', 0)} (avg charge {left_s.get('avg_charge', 0)}) | AI {right_s.get('count', 0)}")

        # Steals
        steals = analysis['steals']
        left_st = steals['left_steals']
        right_st = steals['right_steals']
        print(f"  Steals: Player {left_st['attempts']} attempts ({left_st['rate']}% success) | AI {right_st['attempts']} attempts ({right_st['rate']}% success)")

    print("\n" + "=" * 70)
    print("END OF REPORT")
    print("=" * 70)

def generate_markdown_summary(result: dict) -> str:
    """Generate a compact markdown summary for sharing with Claude."""
    lines = []
    lines.append(f"# Training Session Analysis")
    lines.append(f"**Session:** `{result['session']}`\n")

    summary = result.get('summary', {})
    if summary:
        lines.append(f"**Games:** {summary.get('games_played', 0)} | "
                    f"**Player Wins:** {summary.get('player_wins', 0)} | "
                    f"**AI Wins:** {summary.get('ai_wins', 0)} | "
                    f"**AI Profile:** {summary.get('ai_profile', 'Unknown')}")
        lines.append("")

    for i, game in enumerate(result['games'], 1):
        lines.append(f"## Game {i}: {game['level']}")
        lines.append(f"**Score:** {game['score']} | **Duration:** {game['duration_sec']}s | **AI:** {game['ai_profile']}")

        if game['notes']:
            lines.append(f"\n**Player Notes:** {game['notes']}")

        # Key metrics in a compact table
        analysis = game['analysis']
        poss = analysis['possession']
        shots = analysis['shots']
        steals = analysis['steals']
        ai = analysis['ai_behavior']

        lines.append("\n### Metrics")
        lines.append("| Metric | Player | AI |")
        lines.append("|--------|--------|-----|")
        lines.append(f"| Possession | {poss['left_possession_pct']}% | {poss['right_possession_pct']}% |")
        lines.append(f"| Shots | {shots['left_shots'].get('count', 0)} (charge: {shots['left_shots'].get('avg_charge', 0)}) | {shots['right_shots'].get('count', 0)} |")
        lines.append(f"| Steal Attempts | {steals['left_steals']['attempts']} ({steals['left_steals']['rate']}% success) | {steals['right_steals']['attempts']} ({steals['right_steals']['rate']}% success) |")

        # AI behavior
        if ai['goal_counts']:
            goals_str = ', '.join(f"{k}: {v}" for k, v in sorted(ai['goal_counts'].items(), key=lambda x: -x[1]))
            lines.append(f"\n**AI Goals:** {goals_str}")

        if ai['transitions']:
            trans_str = ', '.join(f"{k} ({v}x)" for k, v in sorted(ai['transitions'].items(), key=lambda x: -x[1])[:5])
            lines.append(f"**AI Transitions:** {trans_str}")

        # Insights
        if game['insights']:
            lines.append("\n### Insights")
            for insight in game['insights']:
                lines.append(f"- {insight}")

        # Weaknesses
        if game['ai_weaknesses']:
            lines.append("\n### AI Weaknesses")
            for weakness in game['ai_weaknesses']:
                lines.append(f"- {weakness}")

        lines.append("")

    return '\n'.join(lines)

def main():
    # Determine session path
    if len(sys.argv) >= 2 and not sys.argv[1].startswith('--'):
        session_path = Path(sys.argv[1])
    else:
        session_path = find_most_recent_session()
        if not session_path:
            print("No training sessions found in training_logs/")
            sys.exit(1)
        print(f"Analyzing most recent session: {session_path}\n")

    if not session_path.exists():
        print(f"Path not found: {session_path}")
        sys.exit(1)

    # Analyze
    result = analyze_session(session_path)

    # Output options
    if '--json' in sys.argv:
        print(json.dumps(result, indent=2))
    elif '--markdown' in sys.argv or '--md' in sys.argv:
        md = generate_markdown_summary(result)
        print(md)
    else:
        # Default: print report AND save markdown summary
        print_report(result)

        # Save markdown to session directory
        md = generate_markdown_summary(result)
        md_path = session_path / 'analysis.md'
        with open(md_path, 'w') as f:
            f.write(md)
        print(f"\n📝 Markdown summary saved to: {md_path}")
        print("   (Copy this file's contents to share with Claude)")

if __name__ == '__main__':
    main()
