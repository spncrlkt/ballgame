#!/usr/bin/env python3
"""
Generate next bracket tournament profiles from previous bracket results.

Seeds 1-16: Top 16 performers from previous bracket (protected seeds)
Seeds 17-64: 48 extreme variant profiles generated from top 8 winners

Each of the top 8 winners gets 6 archetype variants applied, creating
extreme playstyle specializations to explore the parameter space.
"""

import argparse
import hashlib
import random
import re
import sqlite3
from dataclasses import dataclass, fields
from pathlib import Path
from typing import Dict, List, Optional


@dataclass
class Profile:
    """AI profile with all tunable parameters."""
    name: str
    id: str
    position_tolerance: float
    shoot_range: float
    charge_min: float
    charge_max: float
    steal_range: float
    defense_offset: float
    min_shot_quality: float
    pressure_distance: float
    aggression: float
    defensive_iq: float
    steal_reaction_time: float
    button_presses_per_sec: float
    position_patience: float
    seek_threshold: float

    def copy(self) -> "Profile":
        """Create a deep copy of this profile."""
        return Profile(**{f.name: getattr(self, f.name) for f in fields(self)})


# Archetype definitions - extreme parameter mutations
ARCHETYPES = {
    "Aggro": {
        "aggression": 1.0,
        "pressure_distance": 35,
        "steal_range": 200,
        "steal_reaction_time": 0.06,
        "position_patience": 0.25,
    },
    "Zone": {
        "aggression": 0.3,
        "pressure_distance": 250,
        "defensive_iq": 0.90,
        "steal_range": 80,
        "min_shot_quality": 0.30,
    },
    "Sniper": {
        "shoot_range": 700,
        "charge_max": 0.90,
        "min_shot_quality": 0.35,
        "position_patience": 1.2,
        "charge_min": 0.35,
    },
    "Brawl": {
        "shoot_range": 380,
        "charge_min": 0.10,
        "steal_range": 200,
        "charge_max": 0.35,
        "position_tolerance": 30,
    },
    "Speed": {
        "steal_reaction_time": 0.05,
        "button_presses_per_sec": 20,
        "position_patience": 0.30,
        "aggression": 0.90,
    },
    "Patient": {
        "position_patience": 1.5,
        "min_shot_quality": 0.40,
        "seek_threshold": 0.05,
        "defensive_iq": 0.85,
    },
}

# Parameter ranges for clamping and noise
PARAM_RANGES = {
    "position_tolerance": (20, 35),
    "shoot_range": (350, 700),
    "charge_min": (0.10, 0.60),
    "charge_max": (0.30, 1.10),
    "steal_range": (50, 200),
    "defense_offset": (100, 350),
    "min_shot_quality": (0.08, 0.40),
    "pressure_distance": (35, 300),
    "aggression": (0.0, 1.0),
    "defensive_iq": (0.1, 0.92),
    "steal_reaction_time": (0.05, 0.35),
    "button_presses_per_sec": (7, 20),
    "position_patience": (0.12, 1.50),
    "seek_threshold": (0.03, 0.35),
}


def generate_id(name: str) -> str:
    """Generate a 16-char hex ID from profile name."""
    return hashlib.sha256(name.encode()).hexdigest()[:16]


def clamp(value: float, min_val: float, max_val: float) -> float:
    """Clamp value to range."""
    return max(min_val, min(max_val, value))


def shorten_name(name: str) -> str:
    """Shorten profile name for variant naming.

    Examples:
        v4_RS_Eagle -> RSEagle
        v12_Evo_H -> EvoH
        v11_Blend_A -> BlendA
    """
    # Remove version prefix (v4_, v12_, etc.)
    short = re.sub(r'^v\d+_', '', name)
    # Remove underscores and join
    short = short.replace('_', '')
    # Limit length
    return short[:10]


def load_top_performers(db_path: str, tournament_id: Optional[int], count: int) -> List[str]:
    """Load top N performers from a bracket tournament.

    Args:
        db_path: Path to SQLite database
        tournament_id: Specific tournament ID, or None for most recent complete
        count: Number of top performers to return

    Returns:
        List of profile names ordered by performance (best first)
    """
    conn = sqlite3.connect(db_path)
    cursor = conn.cursor()

    if tournament_id is None:
        # Get most recent complete tournament
        cursor.execute("""
            SELECT id FROM bracket_tournaments
            WHERE is_complete = 1
            ORDER BY id DESC LIMIT 1
        """)
        row = cursor.fetchone()
        if not row:
            raise ValueError("No complete tournaments found in database")
        tournament_id = row[0]
        print(f"Using most recent tournament: ID {tournament_id}")

    # Get top performers ordered by match wins, then game wins
    cursor.execute("""
        SELECT profile_name, match_wins, game_wins
        FROM bracket_entries
        WHERE tournament_id = ?
        ORDER BY match_wins DESC, game_wins DESC
        LIMIT ?
    """, (tournament_id, count))

    results = cursor.fetchall()
    conn.close()

    if len(results) < count:
        print(f"Warning: Only found {len(results)} profiles in tournament {tournament_id}")

    return [row[0] for row in results]


def load_top_performers_from_rankings(rankings_path: str, count: int) -> List[str]:
    """Load top N performers from a rankings file.

    Supports two formats:
    1. CSV: Rank,Profile,MatchW,MatchL,GameW,GameL (from analyze --bracket-rankings)
    2. Pipe-delimited: Rank | Profile | Match W-L | Game W-L (from simulate --bracket)

    Args:
        rankings_path: Path to rankings file
        count: Number of top performers to return

    Returns:
        List of profile names ordered by rank (best first)
    """
    profiles = []
    with open(rankings_path, 'r') as f:
        for line in f:
            line = line.strip()
            # Skip comments and empty lines
            if not line or line.startswith('#'):
                continue

            # Try pipe-delimited format first (simulate output)
            if '|' in line:
                parts = [p.strip() for p in line.split('|')]
                if len(parts) >= 2:
                    profile = parts[1].strip()
                    if profile:
                        profiles.append(profile)
            # Fall back to CSV format (analyze output)
            elif ',' in line:
                parts = line.split(',')
                if len(parts) >= 2:
                    profiles.append(parts[1].strip())

            if len(profiles) >= count:
                break

    if len(profiles) < count:
        print(f"Warning: Only found {len(profiles)} profiles in rankings file")

    return profiles


def parse_profiles(profiles_path: str) -> Dict[str, Profile]:
    """Parse profiles from config file.

    Returns:
        Dict mapping profile name to Profile object
    """
    profiles = {}
    current_profile = {}

    with open(profiles_path, 'r') as f:
        for line in f:
            line = line.strip()

            # Skip comments and empty lines
            if not line or line.startswith('#'):
                continue

            # Parse key: value
            if ':' in line:
                key, value = line.split(':', 1)
                key = key.strip()
                value = value.strip()

                if key == 'profile':
                    # Save previous profile if exists
                    if current_profile and 'name' in current_profile:
                        profiles[current_profile['name']] = dict_to_profile(current_profile)
                    current_profile = {'name': value}
                elif key == 'id':
                    current_profile['id'] = value
                else:
                    # Numeric parameter
                    try:
                        current_profile[key] = float(value)
                    except ValueError:
                        current_profile[key] = value

    # Save last profile
    if current_profile and 'name' in current_profile:
        profiles[current_profile['name']] = dict_to_profile(current_profile)

    return profiles


def dict_to_profile(d: Dict) -> Profile:
    """Convert dict to Profile, filling missing fields with defaults."""
    defaults = {
        'name': 'Unknown',
        'id': '0000000000000000',
        'position_tolerance': 25,
        'shoot_range': 500,
        'charge_min': 0.25,
        'charge_max': 0.60,
        'steal_range': 120,
        'defense_offset': 200,
        'min_shot_quality': 0.20,
        'pressure_distance': 80,
        'aggression': 0.80,
        'defensive_iq': 0.50,
        'steal_reaction_time': 0.10,
        'button_presses_per_sec': 12,
        'position_patience': 0.40,
        'seek_threshold': 0.15,
    }
    merged = {**defaults, **d}
    return Profile(**{f.name: merged[f.name] for f in fields(Profile)})


def generate_variant(base: Profile, archetype_name: str, archetype_params: Dict,
                     version: str, noise_pct: float = 0.05) -> Profile:
    """Generate an extreme variant of a base profile.

    Args:
        base: Base profile to mutate
        archetype_name: Name of the archetype (Aggro, Zone, etc.)
        archetype_params: Parameter overrides for this archetype
        version: Version prefix (e.g., "v13")
        noise_pct: Random noise percentage for non-archetype params (default 5%)

    Returns:
        New Profile with archetype mutations applied
    """
    variant = base.copy()

    # Apply archetype parameters (exact values, no noise)
    for param, value in archetype_params.items():
        setattr(variant, param, value)

    # Add small noise to non-archetype parameters
    for field in fields(Profile):
        if field.name in ('name', 'id'):
            continue
        if field.name in archetype_params:
            continue  # Don't add noise to archetype-defined params

        current = getattr(variant, field.name)
        if field.name in PARAM_RANGES:
            min_val, max_val = PARAM_RANGES[field.name]
            # Add ±noise_pct random noise
            noise = current * random.uniform(-noise_pct, noise_pct)
            new_val = clamp(current + noise, min_val, max_val)
            setattr(variant, field.name, new_val)

    # Set new name and ID
    short_name = shorten_name(base.name)
    variant.name = f"{version}_{short_name}_{archetype_name}"
    variant.id = generate_id(variant.name)

    return variant


def format_profile(p: Profile) -> str:
    """Format a profile for the config file."""
    return f"""profile: {p.name}
id: {p.id}
position_tolerance: {int(round(p.position_tolerance))}
shoot_range: {int(round(p.shoot_range))}
charge_min: {p.charge_min:.2f}
charge_max: {p.charge_max:.2f}
steal_range: {int(round(p.steal_range))}
defense_offset: {int(round(p.defense_offset))}
min_shot_quality: {p.min_shot_quality:.2f}
pressure_distance: {int(round(p.pressure_distance))}
aggression: {p.aggression:.2f}
defensive_iq: {p.defensive_iq:.2f}
steal_reaction_time: {p.steal_reaction_time:.2f}
button_presses_per_sec: {int(round(p.button_presses_per_sec))}
position_patience: {p.position_patience:.2f}
seek_threshold: {p.seek_threshold:.2f}
"""


def main():
    parser = argparse.ArgumentParser(
        description="Generate next bracket tournament profiles from previous results",
        epilog="""
Examples:
  # From SQLite database (standard method):
  python3 scripts/generate_bracket_profiles.py \\
    --db db/tournament_20260129_040643.db \\
    --profiles config/ai_profiles_v12.txt \\
    --output config/ai_profiles_v13.txt

  # From rankings file (generated by analyze --bracket-rankings):
  python3 scripts/generate_bracket_profiles.py \\
    --rankings config/bracket_rankings.txt \\
    --profiles config/ai_profiles_v12.txt \\
    --output config/ai_profiles_v13.txt
        """
    )
    parser.add_argument(
        "--db", default=None,
        help="Path to SQLite database with bracket results (e.g., db/tournament_20260129_040643.db)"
    )
    parser.add_argument(
        "--rankings", default=None,
        help="Path to rankings file from 'analyze --bracket-rankings' (alternative to --db)"
    )
    parser.add_argument(
        "--tournament-id", type=int, default=None,
        help="Tournament ID to use (default: most recent complete in db)"
    )
    parser.add_argument(
        "--profiles", required=True,
        help="Path to profiles file used in tournament (REQUIRED, e.g., config/ai_profiles_v12.txt)"
    )
    parser.add_argument(
        "--output", required=True,
        help="Output file path (REQUIRED, e.g., config/ai_profiles_v13.txt)"
    )
    parser.add_argument(
        "--top-seeds", type=int, default=16,
        help="Number of top performers to preserve as seeds (default: 16, adjusts down if fewer available)"
    )
    parser.add_argument(
        "--variants", type=int, default=48,
        help="Number of variant profiles to generate (default: 48)"
    )
    parser.add_argument(
        "--version", default="v13",
        help="Version prefix for new profiles (default: v13)"
    )
    parser.add_argument(
        "--seed", type=int, default=None,
        help="Random seed for reproducibility (default: random)"
    )

    args = parser.parse_args()

    # Validate input source
    if not args.db and not args.rankings:
        parser.error("Either --db or --rankings is required")
    if args.db and args.rankings:
        parser.error("Specify either --db or --rankings, not both")

    if args.seed is not None:
        random.seed(args.seed)

    # Load top performers from bracket - request more than we need
    request_count = max(args.top_seeds + 8, 24)

    if args.rankings:
        print(f"Loading top performers from rankings file: {args.rankings}...")
        top_names = load_top_performers_from_rankings(args.rankings, request_count)
        source_file = args.rankings
    else:
        print(f"Loading top performers from database: {args.db}...")
        top_names = load_top_performers(args.db, args.tournament_id, request_count)
        source_file = args.db

    # Load profile definitions
    print(f"Loading profiles from {args.profiles}...")
    all_profiles = parse_profiles(args.profiles)

    # Get Profile objects for top performers
    top_profiles = []
    for name in top_names:
        if name in all_profiles:
            top_profiles.append(all_profiles[name])
        else:
            print(f"Warning: Profile '{name}' not found in profiles file, skipping")

    if len(top_profiles) == 0:
        print(f"Error: No matching profiles found between tournament results and profiles file")
        return 1

    # Adjust top_seeds if we don't have enough profiles
    actual_top_seeds = min(args.top_seeds, len(top_profiles))
    if actual_top_seeds < args.top_seeds:
        print(f"Note: Adjusting top-seeds from {args.top_seeds} to {actual_top_seeds} (only {len(top_profiles)} profiles available)")

    # Split into protected seeds and variant bases
    protected_seeds = top_profiles[:actual_top_seeds]
    # Use top N for variant generation (at least 1, up to 8)
    variant_base_count = min(8, len(top_profiles))
    variant_bases = top_profiles[:variant_base_count]

    # Capture 8th place profile for warmup seeding (if available)
    warmup_seed_name = top_profiles[7].name if len(top_profiles) >= 8 else top_profiles[-1].name

    print(f"\nProtected seeds ({len(protected_seeds)}):")
    for i, p in enumerate(protected_seeds, 1):
        print(f"  {i:2}. {p.name}")

    print(f"\nVariant bases ({len(variant_bases)}):")
    for p in variant_bases:
        print(f"  - {p.name}")

    # Generate variants: 6 archetypes × N base profiles
    print(f"\nGenerating {args.variants} extreme variants from {len(variant_bases)} bases...")
    variants = []

    archetype_list = list(ARCHETYPES.items())
    variant_count = 0
    base_index = 0

    while variant_count < args.variants:
        base = variant_bases[base_index % len(variant_bases)]
        arch_name, arch_params = archetype_list[variant_count % len(archetype_list)]

        variant = generate_variant(base, arch_name, arch_params, args.version)
        variants.append(variant)

        variant_count += 1
        if variant_count % len(archetype_list) == 0:
            base_index += 1

    # Count variants per archetype
    arch_counts = {}
    for v in variants:
        arch = v.name.split('_')[-1]
        arch_counts[arch] = arch_counts.get(arch, 0) + 1

    print(f"  Variants by archetype: {arch_counts}")

    # Generate output file
    total = len(protected_seeds) + len(variants)
    output = f"""# AI Profiles - Evolution {args.version} ({total} profiles)
#
# DEPRECATION POLICY: Never remove profiles from this file.
# Mark deprecated profiles with "# DEPRECATED" comment instead.
# This preserves historical data for tournament analysis.
#
# WARMUP_SEED: {warmup_seed_name}
#
# Lineage:
#   v1 = Original 5 profiles
#   v2 = Top 4 from 25-profile tournament
#   v3 = 50 variants, winner: v3_Rush_Patient (26.9%)
#   v4 = 50 new variants based on v3 learnings
#   v5-v6 = Experimental playstyles (Sniper, Brawler, Fortress, etc.)
#   v7 = Single parameter variants from top performers
#   v8-v9 = Randomized from successful parameter ranges
#   v10 = Randomized from top v8/v9 performers
#   v11 = Blends of top v10 performers
#   v12 = Evolved from top 16 bracket finishers
#   {args.version} = Top {actual_top_seeds} seeds + {len(variants)} extreme archetype variants
#
# Source: {source_file}
# Source profiles: {args.profiles}
#
# Archetype variants applied to top {len(variant_bases)} winners:
#   - Aggro: max aggression, tight pressure, fast steals
#   - Zone: defensive, wide pressure, high IQ
#   - Sniper: long range, high charge, patient positioning
#   - Brawl: close range, quick shots, physical play
#   - Speed: fast reactions, high button rate, aggressive
#   - Patient: wait for quality shots, defensive positioning
#
# Generation method: scripts/generate_bracket_profiles.py

# =============================================================================
# TOP {actual_top_seeds} PROTECTED SEEDS (from previous bracket)
# =============================================================================

"""

    for i, p in enumerate(protected_seeds, 1):
        output += f"# Seed {i}\n"
        output += format_profile(p) + "\n"

    output += f"""# =============================================================================
# {args.version.upper()} EXTREME VARIANTS ({len(variants)} profiles)
# Generated from top {len(variant_bases)} winners × 6 archetypes
# =============================================================================

"""

    for v in variants:
        output += format_profile(v) + "\n"

    # Write output
    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)

    with open(output_path, 'w') as f:
        f.write(output)

    print(f"\nGenerated {total} profiles ({len(protected_seeds)} seeds + {len(variants)} variants)")
    print(f"Output: {args.output}")

    return 0


if __name__ == "__main__":
    exit(main())
