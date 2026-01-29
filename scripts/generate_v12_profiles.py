#!/usr/bin/env python3
"""
Generate v12 AI profiles by blending/mutating top 16 tournament finishers.

Creates 48 new profiles + preserves top 16 = 64 total profiles for next tournament.
"""

import random
import hashlib
from dataclasses import dataclass
from typing import List, Tuple

@dataclass
class Profile:
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

# Top 16 finishers from bracket tournament (in order)
TOP_16 = [
    Profile("v4_Qual_32", "1285cc6cd1d47ad1", 26, 565, 0.32, 0.78, 120, 258, 0.32, 88, 0.82, 0.60, 0.13, 12, 0.54, 0.12),
    Profile("v4_RA_Prime", "7e1c3f18805d78f4", 24, 565, 0.27, 0.66, 128, 238, 0.27, 73, 0.90, 0.56, 0.11, 14, 0.42, 0.16),
    Profile("v4_Agg_82", "012982e60630926a", 25, 560, 0.30, 0.72, 120, 255, 0.27, 88, 0.82, 0.58, 0.13, 12, 0.52, 0.13),
    Profile("v4_Tri_B", "e55340852a4001f7", 24, 565, 0.28, 0.68, 128, 242, 0.26, 78, 0.86, 0.55, 0.12, 13, 0.48, 0.15),
    Profile("v4_RA_Core", "ae5d570db27b8420", 25, 555, 0.29, 0.70, 125, 242, 0.28, 77, 0.88, 0.54, 0.12, 13, 0.43, 0.15),
    Profile("v4_Tri_D", "cf786d7d29209233", 24, 570, 0.27, 0.66, 130, 238, 0.25, 75, 0.88, 0.54, 0.11, 14, 0.45, 0.16),
    Profile("v4_Agg_88", "d13906c405017767", 24, 550, 0.28, 0.68, 130, 242, 0.26, 78, 0.88, 0.54, 0.12, 13, 0.45, 0.15),
    Profile("v1_Rusher", "159401ee1127c29a", 25, 500, 0.20, 0.50, 150, 200, 0.02, 60, 1.00, 0.40, 0.10, 15, -0.35, 0.47),
    Profile("v4_RP_Zeta", "8265901a0e5fe28a", 26, 540, 0.30, 0.70, 130, 240, 0.26, 80, 0.88, 0.58, 0.12, 13, 0.45, 0.15),
    Profile("v4_RS_Hawk", "26ccc03cd057c994", 23, 640, 0.26, 0.62, 133, 215, 0.25, 72, 0.91, 0.50, 0.10, 14, 0.40, 0.16),
    Profile("v4_Elite_A", "f64c23003e4a4a5d", 24, 555, 0.28, 0.66, 132, 235, 0.25, 74, 0.90, 0.54, 0.11, 14, 0.42, 0.16),
    Profile("v4_RP_Epsilon", "098fc7f612ca4387", 25, 555, 0.27, 0.65, 138, 225, 0.23, 72, 0.93, 0.56, 0.10, 14, 0.40, 0.18),
    Profile("v4_Elite_C", "fe1b739e10f04bb3", 24, 560, 0.27, 0.65, 134, 232, 0.24, 72, 0.91, 0.52, 0.11, 14, 0.40, 0.17),
    Profile("v4_RS_Eagle", "306fbd9d4c7fb76c", 22, 655, 0.24, 0.58, 138, 205, 0.23, 68, 0.93, 0.48, 0.10, 15, 0.36, 0.18),
    Profile("v4_Pat_40", "23c349095572f453", 24, 558, 0.27, 0.65, 132, 232, 0.25, 74, 0.90, 0.52, 0.11, 14, 0.40, 0.17),
    Profile("v4_Elite_F", "6052f8b1309e537e", 25, 552, 0.29, 0.69, 131, 238, 0.26, 76, 0.89, 0.54, 0.11, 13, 0.44, 0.15),
]

def generate_id(name: str) -> str:
    """Generate a 16-char hex ID from profile name."""
    return hashlib.sha256(name.encode()).hexdigest()[:16]

def clamp(value: float, min_val: float, max_val: float) -> float:
    return max(min_val, min(max_val, value))

def blend_value(v1: float, v2: float, mutation_strength: float = 0.1) -> float:
    """Blend two parent values with optional mutation."""
    choice = random.random()

    if choice < 0.6:
        # Interpolate between parents
        t = random.random()
        value = v1 * t + v2 * (1 - t)
    elif choice < 0.9:
        # Use one parent's value directly
        value = random.choice([v1, v2])
    else:
        # Mutate from parent range
        parent_range = abs(v2 - v1)
        base = random.choice([v1, v2])
        mutation = random.gauss(0, max(parent_range * 0.3, mutation_strength))
        value = base + mutation

    return value

def create_evolved_profile(name: str, parent1: Profile, parent2: Profile) -> Profile:
    """Create a new profile by blending/mutating two parents."""
    return Profile(
        name=name,
        id=generate_id(name),
        position_tolerance=clamp(blend_value(parent1.position_tolerance, parent2.position_tolerance, 2), 20, 35),
        shoot_range=clamp(blend_value(parent1.shoot_range, parent2.shoot_range, 30), 450, 700),
        charge_min=clamp(blend_value(parent1.charge_min, parent2.charge_min, 0.03), 0.20, 0.40),
        charge_max=clamp(blend_value(parent1.charge_max, parent2.charge_max, 0.05), 0.50, 0.85),
        steal_range=clamp(blend_value(parent1.steal_range, parent2.steal_range, 10), 100, 180),
        defense_offset=clamp(blend_value(parent1.defense_offset, parent2.defense_offset, 20), 150, 280),
        min_shot_quality=clamp(blend_value(parent1.min_shot_quality, parent2.min_shot_quality, 0.03), 0.10, 0.40),
        pressure_distance=clamp(blend_value(parent1.pressure_distance, parent2.pressure_distance, 8), 45, 100),
        aggression=clamp(blend_value(parent1.aggression, parent2.aggression, 0.05), 0.70, 1.0),
        defensive_iq=clamp(blend_value(parent1.defensive_iq, parent2.defensive_iq, 0.05), 0.40, 0.75),
        steal_reaction_time=clamp(blend_value(parent1.steal_reaction_time, parent2.steal_reaction_time, 0.02), 0.05, 0.18),
        button_presses_per_sec=clamp(blend_value(parent1.button_presses_per_sec, parent2.button_presses_per_sec, 1), 10, 18),
        position_patience=clamp(blend_value(parent1.position_patience, parent2.position_patience, 0.05), 0.30, 0.65),
        seek_threshold=clamp(blend_value(parent1.seek_threshold, parent2.seek_threshold, 0.02), 0.08, 0.25),
    )

def format_profile(p: Profile) -> str:
    """Format a profile for the config file."""
    return f"""profile: {p.name}
id: {p.id}
position_tolerance: {int(p.position_tolerance)}
shoot_range: {int(p.shoot_range)}
charge_min: {p.charge_min:.2f}
charge_max: {p.charge_max:.2f}
steal_range: {int(p.steal_range)}
defense_offset: {int(p.defense_offset)}
min_shot_quality: {p.min_shot_quality:.2f}
pressure_distance: {int(p.pressure_distance)}
aggression: {p.aggression:.2f}
defensive_iq: {p.defensive_iq:.2f}
steal_reaction_time: {p.steal_reaction_time:.2f}
button_presses_per_sec: {int(p.button_presses_per_sec)}
position_patience: {p.position_patience:.2f}
seek_threshold: {p.seek_threshold:.2f}
"""

def main():
    random.seed(42)  # Reproducible generation

    # Generate 48 new profiles
    new_profiles = []

    # Name suffixes for variety: A-Z, then AA-AV
    suffixes = [chr(ord('A') + i) for i in range(26)] + \
               ['A' + chr(ord('A') + i) for i in range(22)]

    for i in range(48):
        # Weight towards top finishers (tournament selection)
        # Higher finishers have higher chance of being selected
        weights = [1 / (j + 1) for j in range(len(TOP_16))]
        total = sum(weights)
        weights = [w / total for w in weights]

        parent1, parent2 = random.choices(TOP_16, weights=weights, k=2)

        # Avoid same parent breeding
        while parent2 == parent1:
            parent2 = random.choices(TOP_16, weights=weights, k=1)[0]

        name = f"v12_Evo_{suffixes[i]}"
        profile = create_evolved_profile(name, parent1, parent2)
        new_profiles.append(profile)

    # Generate output
    output = """# AI Profiles - Evolution v12 (64 profiles)
#
# DEPRECATION POLICY: Never remove profiles from this file.
# Mark deprecated profiles with "# DEPRECATED" comment instead.
# This preserves historical data for tournament analysis.
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
#   v11 = Blends of top v10 performers (v10_Rand_E + v10_Rand_B)
#   v12 = Evolved from top 16 bracket finishers (blend/mutation)
#
# Bracket Tournament Winners (64-player double elimination):
#   1st: v4_Qual_32 (7-1 matches)
#   2nd: v4_RA_Prime (8-2 matches)
#   3rd-4th: v4_Agg_82, v4_Tri_B
#
# Generation: 16 top finishers + 48 evolved variants

# =============================================================================
# TOP 16 BRACKET FINISHERS (preserved from v4-era)
# =============================================================================

"""
    # Add top 16
    for p in TOP_16:
        output += format_profile(p) + "\n"

    output += """# =============================================================================
# V12 EVOLVED PROFILES (48 new variants)
# Generated by blending/mutating top 16 finisher parameters
# =============================================================================

"""
    # Add new evolved profiles
    for p in new_profiles:
        output += format_profile(p) + "\n"

    # Write to new file
    with open("config/ai_profiles_v12.txt", "w") as f:
        f.write(output)

    print(f"Generated {len(TOP_16)} preserved + {len(new_profiles)} new = {len(TOP_16) + len(new_profiles)} total profiles")
    print("Output: config/ai_profiles_v12.txt")

if __name__ == "__main__":
    main()
