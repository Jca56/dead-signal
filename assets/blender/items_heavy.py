"""The heavy weapons that can be carried, and what they're fed (ITEM_Lmg,
ITEM_Rounds762, ITEM_Flamethrower, ITEM_FlameFuel), for `items.py` to
write with the rest: each gun lying on its side, muzzle along +X, its
underside towards the front.
"""

from item_kit import *  # noqa: F403

OLIVE = (0.26, 0.30, 0.20)
OLIVE_DARK = (0.19, 0.22, 0.15)
STEEL = (0.30, 0.30, 0.31)
SCORCHED = (0.05, 0.045, 0.04)
TANK = (0.58, 0.12, 0.08)
TANK_DARK = (0.38, 0.07, 0.05)


def lmg():
    """A light machine gun lying on its side, muzzle along +X, its grip,
    its box of rounds and its folded bipod towards the front."""
    p = Part("ITEM_Lmg")
    z = 0.03
    # The stock (a tube, a frame under it, the butt), the receiver and
    # its feed cover.
    p.box((-0.27, -0.01, z), (0.22, 0.026, 0.026), BLACK)
    p.box((-0.28, 0.05, z), (0.20, 0.018, 0.018), BLACK)
    p.box((-0.385, 0.02, z), (0.02, 0.11, 0.044), PLASTIC)
    p.box((0.03, 0.0, z), (0.40, 0.07, 0.052), BLACK)
    p.box((0.03, -0.04, z), (0.22, 0.012, 0.046), STEEL)
    # The handguard, the heavy barrel, the gas tube, the flash hider.
    p.box((0.31, 0.005, z), (0.20, 0.06, 0.058), PLASTIC)
    p.box((0.56, 0.0, z), (0.32, 0.03, 0.03), BLACK)
    p.box((0.50, 0.028, z), (0.18, 0.016, 0.016), STEEL)
    p.box((0.745, 0.0, z), (0.05, 0.036, 0.036), STEEL)
    # The grip and its guard, the box of rounds, the bipod folded back.
    p.box((-0.07, 0.07, z), (0.048, 0.10, 0.032), PLASTIC, turn=-0.3)
    p.box((-0.01, 0.05, z), (0.07, 0.008, 0.016), BLACK)
    p.box((0.06, 0.10, z + 0.012), (0.12, 0.10, 0.085), OLIVE)
    p.box((0.06, 0.048, z + 0.012), (0.124, 0.008, 0.089), OLIVE_DARK)
    p.box((0.51, 0.046, z - 0.02), (0.24, 0.008, 0.008), STEEL)
    p.box((0.51, 0.046, z + 0.02), (0.24, 0.008, 0.008), STEEL)
    # The sights.
    p.box((-0.05, -0.065, z), (0.01, 0.02, 0.03), BLACK)
    p.box((0.53, -0.03, z), (0.012, 0.03, 0.028), BLACK)
    p.finish()


def rounds_762():
    """An ammunition can, its lid back, a belt folded in it."""
    p = Part("ITEM_Rounds762")
    w, d, h = 0.24, 0.10, 0.15
    p.box((0, 0, h / 2), (w, d, h), OLIVE)
    p.box((0, 0, h * 0.82), (w + 0.006, d + 0.006, 0.012), OLIVE_DARK)
    p.box((0.0, d / 2 + 0.008, h * 0.55), (0.10, 0.012, 0.012), BLACK)
    # The belt: rounds side by side across the top, links between them.
    for i in range(11):
        x = -0.10 + i * 0.02
        p.box((x, 0.0, h + 0.008), (0.011, 0.07, 0.012), BRASS)
        p.box((x, 0.03, h + 0.008), (0.008, 0.016, 0.01), LEAD)
    p.box((0, -0.005, h + 0.004), (0.22, 0.02, 0.006), BLACK)
    p.finish()


def flamethrower():
    """A flamethrower lying on its side, its nozzle along +X, its grips
    and its tank towards the front."""
    p = Part("ITEM_Flamethrower")
    z = 0.045
    # The body and its bands, the lance, the flared nozzle (scorched), the
    # igniter under it.
    p.box((0.14, 0.0, z), (0.46, 0.05, 0.05), OLIVE)
    for x in (-0.07, 0.36):
        p.box((x, 0.0, z), (0.02, 0.056, 0.056), STEEL)
    p.box((0.52, 0.0, z), (0.32, 0.026, 0.026), STEEL)
    p.box((0.705, 0.0, z), (0.07, 0.046, 0.046), SCORCHED)
    p.box((0.745, 0.0, z), (0.014, 0.054, 0.054), SCORCHED)
    p.box((0.68, 0.032, z), (0.05, 0.014, 0.014), BRASS)
    p.box((0.53, 0.025, z), (0.28, 0.008, 0.008), BRASS)
    # The grip and its guard, the foregrip, the valve's lever on top.
    p.box((-0.07, 0.07, z), (0.048, 0.10, 0.032), BLACK, turn=-0.3)
    p.box((-0.01, 0.05, z), (0.07, 0.008, 0.016), BLACK)
    p.box((0.26, 0.065, z), (0.036, 0.085, 0.03), BLACK)
    p.box((0.01, -0.032, z), (0.05, 0.012, 0.016), BRASS)
    # The tank under it, banded; its cap, and the hose up into the body.
    p.box((0.09, 0.105, z), (0.22, 0.085, 0.085), TANK)
    for x in (0.01, 0.17):
        p.box((x, 0.105, z), (0.016, 0.089, 0.089), TANK_DARK)
    p.box((0.212, 0.105, z), (0.024, 0.03, 0.03), BRASS)
    p.box((0.225, 0.06, z), (0.014, 0.08, 0.014), HANDLE)
    p.finish()


def flame_fuel():
    """A tank of fuel for it, stood on end: red, banded, a brass cap."""
    p = Part("ITEM_FlameFuel")
    p.can((0, 0, 0.0), 0.05, 0.22, TANK, segments=8)
    for zc in (0.03, 0.17):
        p.can((0, 0, zc), 0.053, 0.016, TANK_DARK, segments=8)
    p.can((0, 0, 0.22), 0.018, 0.03, BRASS, segments=8)
    p.box((0.0, 0.051, 0.11), (0.05, 0.004, 0.06), LABEL)
    p.finish()
