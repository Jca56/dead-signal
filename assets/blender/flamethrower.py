"""The flamethrower, built into the arms' mesh: a pistol grip in the right
hand behind a long body, a foregrip for the left, a thin lance out to a
scorched, flared nozzle with its igniter under it, a valve lever on its side,
and under the body its tank of fuel, red, a hose from it up into the
body. Its clips and bones are every magazine-fed gun's (`magfed.py`): the
tank is its magazine, off and a fresh one on, the lever its handle; a puff
of the stream is two frames, over and over while the trigger's held, the
flash at its nozzle flickering with them.
"""

import math

from mathutils import Vector

import magfed
from gun_kit import box, flash, frame_matrix, gun_frame

BLACK = (0.08, 0.08, 0.09)
STEEL = (0.30, 0.30, 0.31)
OLIVE = (0.26, 0.30, 0.20)
SCORCHED = (0.05, 0.045, 0.04)
TANK = (0.58, 0.12, 0.08)
TANK_DARK = (0.38, 0.07, 0.05)
BRASS = (0.74, 0.57, 0.24)
HOSE = (0.11, 0.10, 0.09)
POST = (0.95, 0.45, 0.08)

D = magfed.Design(
    sight_line=0.085,
    aim_distance=0.22,
    hip=Vector((0.12, 0.04, -0.16)),
    forend=0.33,
    well=Vector((0.0, 0.11, -0.01)),
    drop=Vector((0.0, 0.05, -1.0)),
    handle=Vector((-0.032, 0.03, 0.05)),
    handle_back=0.035,
    muzzle=Vector((0.0, 0.78, 0.045)),
    fire_frames=2,
    kick=0.5,
    reload_frames=90,
    left_shoulder=Vector((0.06, 0.15, 0.02)),
    right_shoulder=Vector((0.0, 0.05, 0.0)),
)


def build(b, wrist, fwd, back, across, hand, left=None):
    """Add it to builder `b` in the right hand; its bones and frame."""
    origin, right, barrel, up = gun_frame(wrist, fwd, back, across)
    to_rig = frame_matrix(origin, right, barrel, up)
    grip = math.radians(-18)
    # The grip, the trigger and its guard.
    box(b, to_rig, Vector((0.0, -0.012, -0.012)), (0.032, 0.048, 0.10), BLACK, "gun", grip)
    box(b, to_rig, Vector((0.0, 0.035, -0.006)), (0.01, 0.065, 0.008), BLACK, "gun")
    box(b, to_rig, Vector((0.0, 0.028, 0.006)), (0.005, 0.008, 0.02), STEEL, "gun")
    # The body, a band round it at either end; the foregrip under it.
    box(b, to_rig, Vector((0.0, 0.16, 0.045)), (0.05, 0.46, 0.05), OLIVE, "gun")
    for y in (-0.05, 0.38):
        box(b, to_rig, Vector((0.0, y, 0.045)), (0.056, 0.02, 0.056), STEEL, "gun")
    box(b, to_rig, Vector((0.0, 0.33, -0.012)), (0.03, 0.036, 0.085), BLACK, "gun")
    # The lance, the flared nozzle (scorched), the igniter under it and
    # its little pipe back to the body.
    box(b, to_rig, Vector((0.0, 0.54, 0.045)), (0.026, 0.32, 0.026), STEEL, "gun")
    box(b, to_rig, Vector((0.0, 0.725, 0.045)), (0.046, 0.07, 0.046), SCORCHED, "gun")
    box(b, to_rig, Vector((0.0, 0.765, 0.045)), (0.054, 0.014, 0.054), SCORCHED, "gun")
    box(b, to_rig, Vector((0.0, 0.70, 0.012)), (0.014, 0.05, 0.014), BRASS, "gun")
    box(b, to_rig, Vector((0.0, 0.55, 0.02)), (0.008, 0.28, 0.008), BRASS, "gun")
    # Rough sights: a notch at the back of the body, a post back of the
    # nozzle.
    for side in (-1.0, 1.0):
        box(b, to_rig, Vector((side * 0.011, -0.03, D.sight_line - 0.007)), (0.008, 0.01, 0.016), BLACK, "gun")
    box(b, to_rig, Vector((0.0, 0.66, D.sight_line - 0.012)), (0.004, 0.004, 0.024), POST, "gun")
    # The valve's lever, on its left side (clear of the sight line).
    box(b, to_rig, D.handle, (0.012, 0.05, 0.016), BRASS, "handle")
    # The tank under it: red, banded, its cap; the hose up into the body.
    box(b, to_rig, Vector((0.0, 0.11, -0.065)), (0.085, 0.22, 0.085), TANK, "mag")
    for y in (0.03, 0.19):
        box(b, to_rig, Vector((0.0, y, -0.065)), (0.089, 0.016, 0.089), TANK_DARK, "mag")
    box(b, to_rig, Vector((0.0, 0.232, -0.065)), (0.03, 0.024, 0.03), BRASS, "mag")
    box(b, to_rig, Vector((0.0, 0.245, -0.02)), (0.014, 0.014, 0.08), HOSE, "mag")
    flash(b, to_rig, "flash", D.muzzle, 1.1)
    return magfed.bones(D, to_rig, barrel, hand), (origin, right, barrel, up)


def animate(rig, gun_rest, left_rest):
    return magfed.animate(rig, gun_rest, left_rest, D)
