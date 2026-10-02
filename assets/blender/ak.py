"""The AK-47, built into the arms' mesh: a stamped black receiver under
its ribbed dust cover, walnut furniture (a handguard for the left hand
under the gas tube, a solid stock), a pistol grip in the right, a long
curved bakelite magazine, a slanted brake on the muzzle, the bolt's handle
out on its right side, and iron sights (a leaf's notch over the handguard,
a hooded post by the muzzle). Its clips and bones are every magazine-fed
gun's (`magfed.py`); a round is three frames.
"""

import math

from mathutils import Vector

import magfed
from gun_kit import box, flash, frame_matrix, gun_frame

BLACK = (0.08, 0.08, 0.09)
COVER = (0.15, 0.15, 0.16)
STEEL = (0.30, 0.30, 0.31)
WOOD = (0.42, 0.22, 0.09)
WOOD_DARK = (0.30, 0.15, 0.06)
BAKELITE = (0.44, 0.17, 0.07)
DOT = (0.92, 0.91, 0.87)
POST = (0.95, 0.45, 0.08)

D = magfed.Design(
    sight_line=0.108,
    aim_distance=0.20,
    hip=Vector((0.11, 0.04, -0.15)),
    forend=0.30,
    well=Vector((0.0, 0.11, 0.012)),
    drop=Vector((0.0, 0.35, -1.0)),
    handle=Vector((0.03, 0.07, 0.066)),
    handle_back=0.05,
    muzzle=Vector((0.0, 0.69, 0.05)),
    fire_frames=3,
    kick=4.6,
    reload_frames=75,
    left_shoulder=Vector((0.06, 0.15, 0.02)),
    right_shoulder=Vector((0.0, 0.05, 0.0)),
)


def furniture(b, to_rig, barrel_to):
    """What an AK and an RPK share: the grip, the receiver and its cover,
    the handguard and the gas tube over it, the barrel out to `barrel_to`,
    the sights, the bolt's handle."""
    grip = math.radians(-18)
    box(b, to_rig, Vector((0.0, -0.012, -0.012)), (0.03, 0.045, 0.10), WOOD_DARK, "gun", grip)
    box(b, to_rig, Vector((0.0, 0.035, -0.006)), (0.01, 0.065, 0.008), BLACK, "gun")
    box(b, to_rig, Vector((0.0, 0.028, 0.006)), (0.005, 0.008, 0.02), STEEL, "gun")
    # The receiver, its dust cover and the ribs across it.
    box(b, to_rig, Vector((0.0, 0.06, 0.04)), (0.042, 0.27, 0.05), BLACK, "gun")
    box(b, to_rig, Vector((0.0, 0.04, 0.072)), (0.036, 0.21, 0.016), COVER, "gun")
    for y in (-0.03, 0.02, 0.07):
        box(b, to_rig, Vector((0.0, y, 0.0815)), (0.03, 0.012, 0.004), BLACK, "gun")
    # The handguard, the gas tube in its wood over it, the gas block, the
    # barrel.
    box(b, to_rig, Vector((0.0, 0.29, 0.034)), (0.046, 0.19, 0.042), WOOD, "gun")
    box(b, to_rig, Vector((0.0, 0.29, 0.068)), (0.03, 0.17, 0.022), WOOD_DARK, "gun")
    box(b, to_rig, Vector((0.0, 0.40, 0.064)), (0.022, 0.03, 0.034), STEEL, "gun")
    length = barrel_to - 0.385
    box(b, to_rig, Vector((0.0, 0.385 + length / 2, 0.05)), (0.018, length, 0.018), BLACK, "gun")
    # The sights: the leaf's notch (two ears, a white dot on each), and the
    # post in its hood, well down the barrel; their tops make the line.
    top, y0, y1 = D.sight_line, 0.17, barrel_to - 0.07
    box(b, to_rig, Vector((0.0, y0, 0.086)), (0.024, 0.05, 0.012), BLACK, "gun")
    for side in (-1.0, 1.0):
        box(b, to_rig, Vector((side * 0.0075, y0 - 0.02, top - 0.006)), (0.007, 0.008, 0.012), BLACK, "gun")
        box(b, to_rig, Vector((side * 0.0075, y0 - 0.0246, top - 0.005)), (0.0035, 0.0012, 0.0035), DOT, "gun")
        box(b, to_rig, Vector((side * 0.012, y1, top - 0.012)), (0.004, 0.016, 0.034), BLACK, "gun")
    box(b, to_rig, Vector((0.0, y1, 0.07)), (0.02, 0.02, 0.024), BLACK, "gun")
    box(b, to_rig, Vector((0.0, y1, top - 0.012)), (0.005, 0.005, 0.024), BLACK, "gun")
    box(b, to_rig, Vector((0.0, y1 - 0.0005, top - 0.0015)), (0.005, 0.005, 0.005), POST, "gun")
    box(b, to_rig, D.handle, (0.022, 0.02, 0.012), STEEL, "handle")


def build(b, wrist, fwd, back, across, hand, left=None):
    """Add the rifle to builder `b` in the right hand; its bones and frame."""
    origin, right, barrel, up = gun_frame(wrist, fwd, back, across)
    to_rig = frame_matrix(origin, right, barrel, up)
    furniture(b, to_rig, 0.66)
    # The slanted brake.
    box(b, to_rig, Vector((0.0, 0.672, 0.05)), (0.024, 0.036, 0.024), STEEL, "gun", math.radians(12))
    # The stock: its wrist, the butt, the plate.
    box(b, to_rig, Vector((0.0, -0.13, 0.034)), (0.034, 0.12, 0.048), WOOD, "gun")
    box(b, to_rig, Vector((0.0, -0.26, 0.018)), (0.036, 0.15, 0.085), WOOD, "gun")
    box(b, to_rig, Vector((0.0, -0.34, 0.018)), (0.038, 0.012, 0.09), STEEL, "gun")
    # The curved magazine: three canted pieces.
    box(b, to_rig, Vector((0.0, 0.112, -0.04)), (0.026, 0.06, 0.10), BAKELITE, "mag", math.radians(12))
    box(b, to_rig, Vector((0.0, 0.142, -0.118)), (0.025, 0.058, 0.085), BAKELITE, "mag", math.radians(28))
    box(b, to_rig, Vector((0.0, 0.19, -0.18)), (0.024, 0.056, 0.07), BAKELITE, "mag", math.radians(44))
    flash(b, to_rig, "flash", D.muzzle, 1.35)
    return magfed.bones(D, to_rig, barrel, hand), (origin, right, barrel, up)


def animate(rig, gun_rest, left_rest):
    return magfed.animate(rig, gun_rest, left_rest, D)
