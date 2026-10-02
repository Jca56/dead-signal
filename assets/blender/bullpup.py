"""The bullpup, built into the arms' mesh: one long olive polymer body,
stock and receiver in a piece, its magazine (smoked, see-through-looking)
behind the grip; a big guard round the right hand, a folding grip ahead
for the left, a short barrel, and its sight built in over it (a raised
rail, a slim open frame, a red dot floating in it). Its clips and bones
are every magazine-fed gun's (`magfed.py`), its well behind the hand; a
round is two frames.
"""

import math

from mathutils import Vector

import magfed
from gun_kit import box, flash, frame_matrix, gun_frame

BLACK = (0.08, 0.08, 0.09)
OLIVE = (0.27, 0.30, 0.22)
OLIVE_DARK = (0.19, 0.21, 0.15)
STEEL = (0.30, 0.30, 0.31)
SMOKE = (0.34, 0.33, 0.28)
RED = (1.0, 0.10, 0.06)

D = magfed.Design(
    sight_line=0.125,
    aim_distance=0.17,
    hip=Vector((0.12, 0.10, -0.165)),
    forend=0.19,
    well=Vector((0.0, -0.15, 0.005)),
    drop=Vector((0.0, -0.12, -1.0)),
    handle=Vector((-0.032, 0.02, 0.062)),
    handle_back=0.05,
    muzzle=Vector((0.0, 0.47, 0.05)),
    fire_frames=2,
    kick=2.4,
    reload_frames=81,
    left_shoulder=Vector((0.06, 0.15, 0.02)),
    right_shoulder=Vector((0.0, 0.05, 0.0)),
)


def build(b, wrist, fwd, back, across, hand, left=None):
    """Add the rifle to builder `b` in the right hand; its bones and frame."""
    origin, right, barrel, up = gun_frame(wrist, fwd, back, across)
    to_rig = frame_matrix(origin, right, barrel, up)
    grip = math.radians(-14)
    # The grip, the trigger, and the guard round the whole hand.
    box(b, to_rig, Vector((0.0, -0.012, -0.012)), (0.03, 0.045, 0.10), OLIVE_DARK, "gun", grip)
    box(b, to_rig, Vector((0.0, 0.028, 0.006)), (0.005, 0.008, 0.02), STEEL, "gun")
    box(b, to_rig, Vector((0.0, 0.062, -0.028)), (0.012, 0.012, 0.085), OLIVE_DARK, "gun")
    box(b, to_rig, Vector((0.0, 0.03, -0.068)), (0.012, 0.075, 0.01), OLIVE_DARK, "gun")
    # The body: the receiver and stock in one, the butt's pad, the fore
    # end's swell.
    box(b, to_rig, Vector((0.0, -0.10, 0.042)), (0.05, 0.42, 0.074), OLIVE, "gun")
    box(b, to_rig, Vector((0.0, -0.315, 0.036)), (0.052, 0.016, 0.10), BLACK, "gun")
    box(b, to_rig, Vector((0.0, 0.15, 0.04)), (0.046, 0.10, 0.06), OLIVE, "gun")
    # The grip ahead, folded down for the left hand; the barrel and its
    # flash hider.
    box(b, to_rig, Vector((0.0, 0.20, -0.02)), (0.028, 0.03, 0.085), BLACK, "gun")
    box(b, to_rig, Vector((0.0, 0.31, 0.05)), (0.02, 0.26, 0.02), BLACK, "gun")
    box(b, to_rig, Vector((0.0, 0.445, 0.05)), (0.026, 0.05, 0.026), STEEL, "gun")
    # The sight, built in: a rail raised on two posts, a slim open frame
    # on it (nothing drawn in it: the world's seen through), and the dot
    # floating in it on the sight line.
    for y in (0.06, 0.19):
        box(b, to_rig, Vector((0.0, y, 0.087)), (0.02, 0.022, 0.018), OLIVE_DARK, "gun")
    box(b, to_rig, Vector((0.0, 0.125, 0.098)), (0.026, 0.17, 0.006), BLACK, "gun")
    y0 = 0.13
    for side in (-1.0, 1.0):
        box(b, to_rig, Vector((side * 0.018, y0, 0.124)), (0.003, 0.036, 0.046), BLACK, "gun")
    box(b, to_rig, Vector((0.0, y0, 0.1485)), (0.039, 0.036, 0.003), BLACK, "gun")
    box(b, to_rig, Vector((0.0, y0 + 0.016, D.sight_line)), (0.003, 0.002, 0.003), RED, "gun", alpha=0.5)
    # The charging handle, on its left; and the magazine, behind the hand.
    box(b, to_rig, D.handle, (0.02, 0.02, 0.012), STEEL, "handle")
    box(b, to_rig, Vector((0.0, -0.155, -0.045)), (0.026, 0.052, 0.12), SMOKE, "mag", math.radians(-6))
    box(b, to_rig, Vector((0.0, -0.148, -0.108)), (0.028, 0.056, 0.008), BLACK, "mag", math.radians(-6))
    flash(b, to_rig, "flash", D.muzzle, 1.2)
    return magfed.bones(D, to_rig, barrel, hand), (origin, right, barrel, up)


def animate(rig, gun_rest, left_rest):
    return magfed.animate(rig, gun_rest, left_rest, D)
