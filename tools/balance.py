"""The numbers behind Balance.md's charts: each gun's damage a shot at a
Shambler's chest over distance (its real spread, fired thousands of times
at its hit shape), and its damage a second at 6 m aimed, firing and over a
whole magazine and reload. Keep GUNS in step with src/weapon/spec.rs.

    python3 tools/balance.py
"""

import math
import random

random.seed(5)

# name: (damage, pellets, hip spread, aimed spread, falloff (near, far,
# least) or None, seconds between shots, magazine, seconds to reload from
# empty)
GUNS = {
    "PISTOL": (25, 1, 1.5, 0.0, None, 0.2, 12, 1.4),
    "SHOTGUN": (22, 10, 3.6, 2.4, (12, 32, 0.4), 0.6, 5, 0.4 + 5 * 13 / 30 + 0.5),
    "HUNTING RIFLE": (160, 1, 3.0, 0.0, None, 1.1, 5, 0.5 + 5 * 0.6 + 0.55),
    "SMG": (20, 1, 2.6, 0.8, (20, 60, 0.6), 0.075, 30, 2.0),
    "ASSAULT RIFLE": (42, 1, 2.4, 0.15, None, 0.1, 30, 2.3),
}
DISTANCES = (3, 6, 10, 15, 25)


def zone(x, y):
    """What of a Shambler, seen square on, is at (x, y) metres (its feet
    at 0): as src/zombie/figure.rs has it, roughly."""
    if x * x + (y - 1.68) ** 2 < 0.14 ** 2:
        return "head"
    if abs(x) < 0.21 and 0.95 < y < 1.5:
        return "body"
    if 0.02 < abs(x) < 0.2 and 0.08 < y < 0.93:
        return "limb"
    if 0.2 < abs(x) < 0.3 and 0.95 < y < 1.45:
        return "limb"
    return None


def per_shot(damage, pellets, spread, dist, falloff, n=4000):
    """The damage a shot does on average, aimed at the chest from `dist`."""
    total = 0.0
    for _ in range(n):
        for _ in range(pellets):
            r = math.radians(spread) * math.sqrt(random.random())
            a = random.random() * math.tau
            z = zone(math.tan(r * math.cos(a)) * dist, 1.22 + math.tan(r * math.sin(a)) * dist)
            if not z:
                continue
            d = damage
            if falloff:
                near, far, least = falloff
                d *= 1 + (least - 1) * min(max((dist - near) / (far - near), 0), 1)
            total += d * (3 if z == "head" else 1)
    return total / n


for name, (dmg, pellets, hip, aim, fall, gap, mag, reload) in GUNS.items():
    cells = " | ".join(f"{round(per_shot(dmg, pellets, hip, d, fall))} / {round(per_shot(dmg, pellets, aim, d, fall))}" for d in DISTANCES)
    six = per_shot(dmg, pellets, aim, 6, fall)
    print(f"| {name} | {cells} |   firing {six / gap:.0f}/s, sustained {six * mag / (mag * gap + reload):.0f}/s")
