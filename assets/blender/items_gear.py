"""What's worn, for `items.py` to write with the rest: helmets (domes, on
the ground), vests and rigs (standing, their fronts to the icon's eye, +Y),
backpacks (standing, the straps behind), cargo pants (folded flat), a
bandolier (lying in an arc), an armor plate (flat).
"""

import math

import bmesh
from mathutils import Matrix

from item_kit import *  # noqa: F403

OLIVE = (0.30, 0.33, 0.22)
OLIVE_DARK = (0.22, 0.24, 0.16)
COYOTE = (0.52, 0.43, 0.30)
COYOTE_DARK = (0.40, 0.32, 0.21)
NAVY = (0.14, 0.17, 0.26)
BIKE_RED = (0.72, 0.16, 0.12)
STRAP = (0.14, 0.13, 0.12)
BUCKLE = (0.55, 0.55, 0.52)
DAY_BLUE = (0.18, 0.36, 0.62)
HIKE_ORANGE = (0.80, 0.42, 0.12)
HIKE_TEAL = (0.14, 0.40, 0.42)
KHAKI = (0.58, 0.52, 0.38)
KHAKI_DARK = (0.46, 0.41, 0.29)
LEATHER_DARK = (0.28, 0.17, 0.09)
PLATE_GREY = (0.26, 0.27, 0.27)


def dome(p, centre, rx, ry, rz, colour, alt=None):
    """The top half of a ball, its rim on `centre`."""
    m = Matrix.Translation(centre) @ Matrix.Diagonal((rx, ry, rz, 1.0))
    made = bmesh.ops.create_uvsphere(p.bm, u_segments=12, v_segments=8, radius=1.0, matrix=m)["verts"]
    low = [v for v in made if v.co.z < centre[2] - 1e-6]
    bmesh.ops.delete(p.bm, geom=low, context="VERTS")
    p.paint([v for v in made if v.is_valid], colour, alt)


def bike_helmet():
    p = Part("ITEM_BikeHelmet")
    dome(p, (0, 0, 0.0), 0.12, 0.15, 0.12, BIKE_RED)
    for x in (-0.05, 0.0, 0.05):
        p.box((x, 0.0, 0.118), (0.018, 0.2, 0.012), BLACK)
    p.box((0.0, 0.14, 0.03), (0.16, 0.03, 0.02), BLACK)
    p.finish()


def military_helmet():
    p = Part("ITEM_MilitaryHelmet")
    dome(p, (0, 0, 0.012), 0.14, 0.15, 0.13, OLIVE, OLIVE_DARK)
    p.hoop((0, 0, 0.012), 0.145, 0.012, OLIVE_DARK, segments=16)
    p.box((0.0, 0.16, 0.0), (0.12, 0.012, 0.012), STRAP)
    p.finish()


def vest_panel(p, colour, dark, thick=0.05):
    """A vest standing: the front panel, the shoulders over, the sides."""
    p.box((0.0, 0.0, 0.2), (0.34, thick, 0.4), colour)
    for x in (-0.11, 0.11):
        p.box((x, -0.03, 0.42), (0.08, 0.1, 0.04), dark)
    p.box((0.0, -0.02, 0.05), (0.36, thick + 0.01, 0.06), dark)


def light_vest():
    p = Part("ITEM_LightVest")
    vest_panel(p, NAVY, BLACK)
    p.box((0.0, 0.03, 0.3), (0.2, 0.012, 0.06), (0.8, 0.8, 0.76))
    p.finish()


def plate_carrier():
    p = Part("ITEM_PlateCarrier")
    vest_panel(p, OLIVE, OLIVE_DARK, 0.07)
    p.box((0.0, 0.045, 0.26), (0.28, 0.03, 0.26), OLIVE_DARK)
    for x in (-0.09, 0.0, 0.09):
        p.box((x, 0.07, 0.12), (0.075, 0.04, 0.1), OLIVE)
        p.box((x, 0.09, 0.16), (0.075, 0.004, 0.02), STRAP)
    p.finish()


def chest_rig():
    p = Part("ITEM_ChestRig")
    # The harness: straps up from the pouches, over the shoulders.
    for x in (-0.11, 0.11):
        p.box((x, -0.01, 0.24), (0.045, 0.025, 0.2), STRAP)
        p.box((x, -0.05, 0.345), (0.05, 0.1, 0.03), STRAP)
    p.box((0.0, 0.0, 0.1), (0.36, 0.03, 0.12), COYOTE_DARK)
    for x in (-0.12, -0.04, 0.04, 0.12):
        p.box((x, 0.03, 0.12), (0.07, 0.04, 0.12), COYOTE)
        p.box((x, 0.052, 0.16), (0.07, 0.004, 0.02), STRAP)
    p.finish()


def armored_rig():
    p = Part("ITEM_ArmoredRig")
    vest_panel(p, COYOTE, COYOTE_DARK, 0.06)
    for x in (-0.09, 0.0, 0.09):
        p.box((x, 0.05, 0.13), (0.075, 0.04, 0.11), COYOTE_DARK)
        p.box((x, 0.072, 0.17), (0.075, 0.004, 0.02), STRAP)
    p.box((0.0, 0.035, 0.32), (0.22, 0.02, 0.1), COYOTE_DARK)
    p.finish()


def backpack(name, w, d, h, colour, dark, pockets=1, roll=None):
    """A pack standing: its body, a flap over the top, pockets on the front
    (+Y), the straps behind."""
    p = Part(name)
    p.box((0.0, 0.0, h / 2), (w, d, h), colour)
    p.box((0.0, 0.01, h - 0.02), (w + 0.01, d + 0.02, 0.06), dark)
    for k in range(pockets):
        pw = w * 0.8 / pockets
        x = -w * 0.4 + pw * (k + 0.5)
        p.box((x, d / 2 + 0.025, h * 0.3), (pw * 0.9, 0.05, h * 0.32), dark)
        p.box((x, d / 2 + 0.052, h * 0.44), (pw * 0.3, 0.006, 0.02), BUCKLE)
    for x in (-w * 0.3, w * 0.3):
        p.box((x, -d / 2 - 0.02, h * 0.55), (0.05, 0.03, h * 0.8), STRAP)
    if roll:
        p.roll((0.0, 0.0, h + 0.05), 0.05, w * 1.05, roll, roll)
    p.finish()


def cargo_pants():
    """Laid out flat: the waist at the left, the legs out to the right,
    a pocket on each thigh, the belt through the loops."""
    p = Part("ITEM_CargoPants")
    p.box((-0.14, 0.0, 0.02), (0.1, 0.26, 0.04), KHAKI)
    p.box((-0.18, 0.0, 0.043), (0.025, 0.27, 0.01), LEATHER_DARK)
    for side in (-1.0, 1.0):
        p.box((0.06, side * 0.07, 0.018), (0.34, 0.11, 0.036), KHAKI, turn=side * 0.1)
        p.box((0.0, side * 0.075, 0.04), (0.09, 0.08, 0.012), KHAKI_DARK, turn=side * 0.1)
    p.finish()


def bandolier():
    p = Part("ITEM_Bandolier")
    n = 9
    for k in range(n):
        a = math.radians(-50 + 100 * k / (n - 1))
        x, y = math.sin(a) * 0.2, math.cos(a) * 0.08 - 0.05
        p.box((x, y, 0.012), (0.05, 0.03, 0.024), LEATHER, turn=a)
        p.box((x, y + 0.012, 0.03), (0.018, 0.018, 0.03), BRASS, turn=a)
    p.box((0.0, 0.07, 0.01), (0.05, 0.02, 0.02), BUCKLE)
    p.finish()


def armor_plate():
    p = Part("ITEM_ArmorPlate")
    p.box((0.0, 0.0, 0.015), (0.26, 0.3, 0.03), PLATE_GREY)
    p.box((0.0, 0.0, 0.034), (0.2, 0.24, 0.008), BLACK)
    p.box((0.0, 0.04, 0.04), (0.12, 0.05, 0.004), (0.85, 0.82, 0.70))
    p.finish()


def all_gear():
    bike_helmet()
    military_helmet()
    light_vest()
    plate_carrier()
    chest_rig()
    armored_rig()
    backpack("ITEM_Daypack", 0.26, 0.14, 0.34, DAY_BLUE, NAVY)
    backpack("ITEM_Rucksack", 0.32, 0.18, 0.44, OLIVE, OLIVE_DARK, pockets=2)
    backpack("ITEM_HikingPack", 0.34, 0.2, 0.56, HIKE_ORANGE, HIKE_TEAL, pockets=2, roll=HIKE_TEAL)
    backpack("ITEM_MilitaryRuck", 0.42, 0.24, 0.6, OLIVE, OLIVE_DARK, pockets=3, roll=OLIVE_DARK)
    cargo_pants()
    bandolier()
    armor_plate()
