"""The assault rifle, built into the arms' mesh: a flat-top receiver with a
long ribbed handguard for the left hand, a pistol grip in the right, a
curved magazine, a solid stock, a flash hider, and a red dot sight on top
(a slim open frame, the dot glowing in it). Its clips and bones are every
magazine-fed gun's (`magfed.py`); a round is three frames.
"""

import math

from mathutils import Vector

import magfed
from gun_kit import box, flash, frame_matrix, gun_frame

BLACK = (0.08, 0.08, 0.09)
TAN = (0.52, 0.46, 0.34)
STEEL = (0.30, 0.30, 0.31)
RED = (1.0, 0.10, 0.06)

D = magfed.Design(
    sight_line=0.12,
    aim_distance=0.19,
    hip=Vector((0.11, 0.04, -0.15)),
    forend=0.34,
    well=Vector((0.0, 0.10, 0.015)),
    drop=Vector((0.0, 0.2, -1.0)),
    handle=Vector((0.0, -0.03, 0.08)),
    handle_back=0.05,
    muzzle=Vector((0.0, 0.70, 0.045)),
    fire_frames=3,
    kick=3.5,
    reload_frames=69,
    left_shoulder=Vector((0.06, 0.15, 0.02)),
    right_shoulder=Vector((0.0, 0.05, 0.0)),
)


def build(b, wrist, fwd, back, across, hand, left=None):
    """Add the rifle to builder `b` in the right hand; its bones and frame."""
    origin, right, barrel, up = gun_frame(wrist, fwd, back, across)
    to_rig = frame_matrix(origin, right, barrel, up)
    grip = math.radians(-18)
    # The grip, the trigger and its guard.
    box(b, to_rig, Vector((0.0, -0.012, -0.012)), (0.03, 0.045, 0.10), BLACK, "gun", grip)
    box(b, to_rig, Vector((0.0, 0.035, -0.006)), (0.01, 0.065, 0.008), BLACK, "gun")
    box(b, to_rig, Vector((0.0, 0.028, 0.006)), (0.005, 0.008, 0.02), STEEL, "gun")
    # The receiver (flat-topped), the handguard with its ribs, the barrel,
    # the flash hider.
    box(b, to_rig, Vector((0.0, 0.07, 0.045)), (0.045, 0.30, 0.06), BLACK, "gun")
    box(b, to_rig, Vector((0.0, 0.07, 0.077)), (0.026, 0.28, 0.006), STEEL, "gun")
    box(b, to_rig, Vector((0.0, 0.34, 0.045)), (0.056, 0.26, 0.056), TAN, "gun")
    for y in (0.25, 0.31, 0.37, 0.43):
        box(b, to_rig, Vector((0.0, y, 0.074)), (0.04, 0.012, 0.004), BLACK, "gun")
    box(b, to_rig, Vector((0.0, 0.56, 0.045)), (0.02, 0.18, 0.02), BLACK, "gun")
    box(b, to_rig, Vector((0.0, 0.675, 0.045)), (0.026, 0.05, 0.026), STEEL, "gun")
    # The stock on its tube.
    box(b, to_rig, Vector((0.0, -0.13, 0.04)), (0.03, 0.12, 0.03), BLACK, "gun")
    box(b, to_rig, Vector((0.0, -0.25, 0.02)), (0.045, 0.16, 0.085), TAN, "gun")
    box(b, to_rig, Vector((0.0, -0.335, 0.02)), (0.047, 0.012, 0.09), BLACK, "gun")
    # The red dot sight, well forward on the rail: a low base, a slim hoop
    # of a frame round an open window (nothing drawn in it: the world's
    # seen through), and the dot floating in it on the sight line.
    y0 = 0.16
    box(b, to_rig, Vector((0.0, y0, 0.086)), (0.026, 0.05, 0.012), BLACK, "gun")
    for side in (-1.0, 1.0):
        box(b, to_rig, Vector((side * 0.0175, y0, 0.114)), (0.003, 0.03, 0.046), BLACK, "gun")
    box(b, to_rig, Vector((0.0, y0, 0.1385)), (0.038, 0.03, 0.003), BLACK, "gun")
    box(b, to_rig, Vector((0.0, y0 + 0.014, D.sight_line)), (0.003, 0.002, 0.003), RED, "gun", alpha=0.5)
    # The charging handle, and the curved magazine (two canted pieces).
    box(b, to_rig, D.handle, (0.03, 0.02, 0.01), STEEL, "handle")
    box(b, to_rig, Vector((0.0, 0.105, -0.05)), (0.024, 0.05, 0.12), BLACK, "mag", math.radians(10))
    box(b, to_rig, Vector((0.0, 0.132, -0.14)), (0.023, 0.048, 0.08), BLACK, "mag", math.radians(24))
    flash(b, to_rig, "flash", D.muzzle, 1.3)
    return magfed.bones(D, to_rig, barrel, hand), (origin, right, barrel, up)


def animate(rig, gun_rest, left_rest):
    return magfed.animate(rig, gun_rest, left_rest, D)
