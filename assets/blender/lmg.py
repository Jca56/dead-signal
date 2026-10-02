"""The LMG, built into the arms' mesh: a long heavy receiver under a feed
cover, a thick barrel in a vented handguard with a folded bipod under it,
a pistol grip in the right hand, a skeleton stock, iron sights (a rear
notch, a hooded front post), and under it a green box of belted rounds,
the belt up into the feed. Its clips and bones are every magazine-fed
gun's (`magfed.py`): the box is its magazine, off and a fresh one on; a
round is three frames.
"""

import math

from mathutils import Vector

import magfed
from gun_kit import box, flash, frame_matrix, gun_frame

BLACK = (0.08, 0.08, 0.09)
POLYMER = (0.14, 0.15, 0.13)
STEEL = (0.30, 0.30, 0.31)
OLIVE = (0.26, 0.30, 0.20)
OLIVE_DARK = (0.19, 0.22, 0.15)
BRASS = (0.74, 0.57, 0.24)
DOT = (0.92, 0.91, 0.87)
POST = (0.95, 0.45, 0.08)

D = magfed.Design(
    sight_line=0.125,
    aim_distance=0.20,
    hip=Vector((0.12, 0.03, -0.165)),
    forend=0.36,
    well=Vector((0.0, 0.13, 0.0)),
    drop=Vector((-0.25, 0.1, -1.0)),
    handle=Vector((0.034, 0.02, 0.07)),
    handle_back=0.06,
    muzzle=Vector((0.0, 0.84, 0.05)),
    fire_frames=3,
    kick=2.6,
    reload_frames=150,
    left_shoulder=Vector((0.06, 0.15, 0.02)),
    right_shoulder=Vector((0.0, 0.05, 0.0)),
)


def build(b, wrist, fwd, back, across, hand, left=None):
    """Add the gun to builder `b` in the right hand; its bones and frame."""
    origin, right, barrel, up = gun_frame(wrist, fwd, back, across)
    to_rig = frame_matrix(origin, right, barrel, up)
    grip = math.radians(-18)
    # The grip, the trigger and its guard.
    box(b, to_rig, Vector((0.0, -0.012, -0.012)), (0.032, 0.048, 0.10), POLYMER, "gun", grip)
    box(b, to_rig, Vector((0.0, 0.035, -0.006)), (0.01, 0.065, 0.008), BLACK, "gun")
    box(b, to_rig, Vector((0.0, 0.028, 0.006)), (0.005, 0.008, 0.02), STEEL, "gun")
    # The receiver, the feed cover on top of it, the feed tray's lip.
    box(b, to_rig, Vector((0.0, 0.10, 0.05)), (0.052, 0.40, 0.07), BLACK, "gun")
    box(b, to_rig, Vector((0.0, 0.10, 0.09)), (0.046, 0.22, 0.012), STEEL, "gun")
    box(b, to_rig, Vector((-0.032, 0.13, 0.066)), (0.014, 0.08, 0.02), STEEL, "gun")
    # The handguard and its vents, the heavy barrel, the gas tube under
    # it, the flash hider.
    box(b, to_rig, Vector((0.0, 0.38, 0.045)), (0.058, 0.20, 0.06), POLYMER, "gun")
    for y in (0.31, 0.36, 0.41, 0.46):
        for side in (-1.0, 1.0):
            box(b, to_rig, Vector((side * 0.03, y, 0.05)), (0.004, 0.028, 0.014), BLACK, "gun")
    box(b, to_rig, Vector((0.0, 0.63, 0.05)), (0.03, 0.32, 0.03), BLACK, "gun")
    box(b, to_rig, Vector((0.0, 0.57, 0.022)), (0.016, 0.18, 0.016), STEEL, "gun")
    box(b, to_rig, Vector((0.0, 0.815, 0.05)), (0.036, 0.05, 0.036), STEEL, "gun")
    # The bipod, folded back under the barrel.
    for side in (-1.0, 1.0):
        box(b, to_rig, Vector((side * 0.02, 0.58, 0.004)), (0.008, 0.24, 0.008), STEEL, "gun")
    box(b, to_rig, Vector((0.0, 0.70, 0.012)), (0.05, 0.016, 0.024), STEEL, "gun")
    # The stock: a tube, a frame under it, the butt.
    box(b, to_rig, Vector((0.0, -0.20, 0.06)), (0.026, 0.22, 0.026), BLACK, "gun")
    box(b, to_rig, Vector((0.0, -0.21, 0.0)), (0.018, 0.20, 0.018), BLACK, "gun")
    box(b, to_rig, Vector((0.0, -0.315, 0.03)), (0.044, 0.02, 0.11), POLYMER, "gun")
    # The sights: a rear notch with white dots, a hooded orange post.
    y0, y1 = 0.02, 0.60
    for side in (-1.0, 1.0):
        box(b, to_rig, Vector((side * 0.011, y0, D.sight_line - 0.008)), (0.008, 0.01, 0.02), BLACK, "gun")
        box(b, to_rig, Vector((side * 0.011, y0 - 0.006, D.sight_line - 0.002)), (0.004, 0.002, 0.004), DOT, "gun")
        box(b, to_rig, Vector((side * 0.013, y1, D.sight_line - 0.006)), (0.003, 0.012, 0.03), BLACK, "gun")
    box(b, to_rig, Vector((0.0, y0, D.sight_line - 0.021)), (0.03, 0.01, 0.008), BLACK, "gun")
    box(b, to_rig, Vector((0.0, y1, 0.08)), (0.028, 0.012, 0.03), BLACK, "gun")
    box(b, to_rig, Vector((0.0, y1, D.sight_line - 0.012)), (0.004, 0.004, 0.024), POST, "gun")
    # The charging handle, out on its right side.
    box(b, to_rig, D.handle, (0.02, 0.024, 0.014), STEEL, "handle")
    # The box of belted rounds, hung under the feed on its left; the belt
    # out of it and up into the tray.
    box(b, to_rig, Vector((-0.012, 0.13, -0.06)), (0.085, 0.12, 0.10), OLIVE, "mag")
    box(b, to_rig, Vector((-0.012, 0.13, -0.008)), (0.089, 0.124, 0.008), OLIVE_DARK, "mag")
    for k in range(4):
        box(b, to_rig, Vector((-0.038, 0.105 + k * 0.016, 0.02 + k * 0.006)), (0.012, 0.008, 0.03), BRASS, "mag")
    flash(b, to_rig, "flash", D.muzzle, 1.5)
    return magfed.bones(D, to_rig, barrel, hand), (origin, right, barrel, up)


def animate(rig, gun_rest, left_rest):
    return magfed.animate(rig, gun_rest, left_rest, D)
