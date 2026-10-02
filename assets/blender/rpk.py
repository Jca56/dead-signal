"""The RPK, built into the arms' mesh: the AK grown into a machine gun
(`ak.py`'s receiver, handguard and sights), with a longer, heavier barrel,
a bipod folded back under it, a club-footed walnut stock, and a
seventy-five round drum under it. Its clips and bones are every
magazine-fed gun's (`magfed.py`): the drum's its magazine; a round is
three frames.
"""

import math

from mathutils import Vector

import ak
import magfed
from gun_kit import box, flash, frame_matrix, gun_frame

D = magfed.Design(
    sight_line=ak.D.sight_line,
    aim_distance=0.20,
    hip=Vector((0.115, 0.035, -0.16)),
    forend=0.30,
    well=Vector((0.0, 0.12, 0.005)),
    drop=Vector((0.0, 0.2, -1.0)),
    handle=ak.D.handle,
    handle_back=0.05,
    muzzle=Vector((0.0, 0.83, 0.05)),
    fire_frames=3,
    kick=3.2,
    reload_frames=102,
    left_shoulder=Vector((0.06, 0.15, 0.02)),
    right_shoulder=Vector((0.0, 0.05, 0.0)),
)


def build(b, wrist, fwd, back, across, hand, left=None):
    """Add the gun to builder `b` in the right hand; its bones and frame."""
    origin, right, barrel, up = gun_frame(wrist, fwd, back, across)
    to_rig = frame_matrix(origin, right, barrel, up)
    ak.furniture(b, to_rig, 0.82)
    # The heavier barrel over the last of it, and its plain muzzle nut.
    box(b, to_rig, Vector((0.0, 0.52, 0.05)), (0.024, 0.20, 0.024), ak.BLACK, "gun")
    box(b, to_rig, Vector((0.0, 0.815, 0.05)), (0.024, 0.03, 0.024), ak.STEEL, "gun")
    # The bipod, folded back under the barrel.
    for side in (-1.0, 1.0):
        box(b, to_rig, Vector((side * 0.016, 0.60, 0.026)), (0.007, 0.26, 0.007), ak.STEEL, "gun")
    box(b, to_rig, Vector((0.0, 0.725, 0.034)), (0.044, 0.016, 0.022), ak.STEEL, "gun")
    # The stock: its wrist, the club-footed butt, the plate.
    box(b, to_rig, Vector((0.0, -0.13, 0.034)), (0.034, 0.12, 0.048), ak.WOOD, "gun")
    box(b, to_rig, Vector((0.0, -0.25, 0.022)), (0.036, 0.13, 0.07), ak.WOOD, "gun")
    box(b, to_rig, Vector((0.0, -0.30, -0.012)), (0.036, 0.07, 0.07), ak.WOOD, "gun")
    box(b, to_rig, Vector((0.0, -0.34, 0.006)), (0.038, 0.012, 0.115), ak.STEEL, "gun")
    # The drum: its neck up into the well, and the drum itself (two
    # blocks crossed, to round it).
    box(b, to_rig, Vector((0.0, 0.12, -0.03)), (0.026, 0.05, 0.05), ak.BLACK, "mag", math.radians(10))
    box(b, to_rig, Vector((0.0, 0.135, -0.105)), (0.062, 0.13, 0.13), ak.COVER, "mag")
    box(b, to_rig, Vector((0.0, 0.135, -0.105)), (0.064, 0.094, 0.094), ak.BLACK, "mag", math.radians(45))
    box(b, to_rig, Vector((0.0, 0.135, -0.105)), (0.068, 0.03, 0.03), ak.STEEL, "mag")
    flash(b, to_rig, "flash", D.muzzle, 1.45)
    return magfed.bones(D, to_rig, barrel, hand), (origin, right, barrel, up)


def animate(rig, gun_rest, left_rest):
    return magfed.animate(rig, gun_rest, left_rest, D)
