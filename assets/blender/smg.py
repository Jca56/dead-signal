"""The SMG, built into the arms' mesh: a compact black receiver with a
short barrel in a shroud, a pistol grip in the right hand, a straight
stick magazine ahead of it, a handguard for the left hand, a folded wire
stock, and iron sights (a rear notch with white dots, an orange front
post). Its clips and bones are every magazine-fed gun's (`magfed.py`); a
round is two frames, over and over while the trigger's held.
"""

import math

from mathutils import Vector

import magfed
from gun_kit import box, flash, frame_matrix, gun_frame

BLACK = (0.08, 0.08, 0.09)
POLYMER = (0.13, 0.13, 0.12)
STEEL = (0.32, 0.32, 0.33)
DOT = (0.92, 0.91, 0.87)
POST = (0.95, 0.45, 0.08)

D = magfed.Design(
    sight_line=0.105,
    aim_distance=0.20,
    hip=Vector((0.11, 0.04, -0.14)),
    forend=0.22,
    well=Vector((0.0, 0.10, 0.015)),
    drop=Vector((0.0, 0.14, -1.0)),
    handle=Vector((0.0, 0.05, 0.078)),
    handle_back=0.04,
    muzzle=Vector((0.0, 0.39, 0.05)),
    fire_frames=2,
    kick=3.0,
    reload_frames=60,
    left_shoulder=Vector((0.06, 0.15, 0.02)),
    right_shoulder=Vector((0.0, 0.05, 0.0)),
)


def build(b, wrist, fwd, back, across, hand, left=None):
    """Add the SMG to builder `b` in the right hand; its bones and frame."""
    origin, right, barrel, up = gun_frame(wrist, fwd, back, across)
    to_rig = frame_matrix(origin, right, barrel, up)
    grip = math.radians(-15)
    # The grip, the trigger and its guard.
    box(b, to_rig, Vector((0.0, -0.01, -0.012)), (0.03, 0.045, 0.10), POLYMER, "gun", grip)
    box(b, to_rig, Vector((0.0, 0.035, -0.008)), (0.01, 0.06, 0.008), BLACK, "gun")
    box(b, to_rig, Vector((0.0, 0.028, 0.004)), (0.005, 0.008, 0.02), STEEL, "gun")
    # The receiver, the handguard, the barrel in its shroud.
    box(b, to_rig, Vector((0.0, 0.08, 0.045)), (0.042, 0.26, 0.055), BLACK, "gun")
    box(b, to_rig, Vector((0.0, 0.22, 0.032)), (0.048, 0.13, 0.05), POLYMER, "gun")
    box(b, to_rig, Vector((0.0, 0.32, 0.05)), (0.026, 0.10, 0.026), BLACK, "gun")
    box(b, to_rig, Vector((0.0, 0.375, 0.05)), (0.016, 0.03, 0.016), STEEL, "gun")
    # The wire stock, folded along the left side.
    for x in (-0.026, -0.036):
        box(b, to_rig, Vector((x, 0.02, 0.03 if x > -0.03 else 0.055)), (0.006, 0.22, 0.006), STEEL, "gun")
    box(b, to_rig, Vector((-0.031, 0.13, 0.042)), (0.012, 0.012, 0.04), STEEL, "gun")
    # Sights: the rear notch (two ears, a white dot on each), the front
    # post in its hood; their tops make the sight line.
    top, height = D.sight_line, 0.012
    box(b, to_rig, Vector((0.0, -0.02, 0.078)), (0.03, 0.02, 0.012), BLACK, "gun")
    for side in (-1.0, 1.0):
        box(b, to_rig, Vector((side * 0.0075, -0.02, top - height / 2)), (0.007, 0.008, height), BLACK, "gun")
        box(b, to_rig, Vector((side * 0.0075, -0.0245, top - 0.005)), (0.0035, 0.0012, 0.0035), DOT, "gun")
        box(b, to_rig, Vector((side * 0.012, 0.30, 0.088)), (0.004, 0.018, 0.03), BLACK, "gun")
    box(b, to_rig, Vector((0.0, 0.30, 0.078)), (0.02, 0.018, 0.012), BLACK, "gun")
    box(b, to_rig, Vector((0.0, 0.30, top - 0.013)), (0.005, 0.005, 0.02), BLACK, "gun")
    box(b, to_rig, Vector((0.0, 0.2995, top - 0.0015)), (0.005, 0.005, 0.005), POST, "gun")
    # The charging handle, and the magazine.
    box(b, to_rig, D.handle, (0.012, 0.03, 0.014), STEEL, "handle")
    box(b, to_rig, Vector((0.0, 0.11, -0.07)), (0.022, 0.034, 0.17), BLACK, "mag", math.radians(8))
    flash(b, to_rig, "flash", D.muzzle, 1.1)
    return magfed.bones(D, to_rig, barrel, hand), (origin, right, barrel, up)


def animate(rig, gun_rest, left_rest):
    return magfed.animate(rig, gun_rest, left_rest, D)
