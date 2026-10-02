"""The .45 pistol, built into the arms' mesh in the right hand: a long
steel slide over a dark frame, walnut grip panels, a spur hammer, a
magazine in the grip, and plain sights (a rear notch with white dots, an
orange front post). Its clips and bones are every sidearm's
(`sidearm.py`).
"""

import math

from mathutils import Matrix, Vector

import sidearm
from gun_kit import box, flash, frame_matrix, gun_frame

STEEL = (0.40, 0.40, 0.42)
FRAME = (0.13, 0.13, 0.14)
WOOD = (0.38, 0.21, 0.10)
DOT = (0.92, 0.91, 0.87)
POST = (0.95, 0.45, 0.08)

MUZZLE = Vector((0.0, 0.168, 0.072))
GRIP_TILT = math.radians(-17)

D = sidearm.Design(
    sight_line=0.097,
    aim_distance=0.22,
    muzzle=MUZZLE,
    fire_frames=6,
    kick=9.0,
    slide_back=0.042,
    reload_frames=45,
    mag_out=0.40,
    swapped=True,
    rack=0.042,
)


def build(b, wrist, fwd, back, across, hand, left=None):
    """Add the pistol to builder `b` in the right hand; its bones and frame."""
    origin, right, barrel, up = gun_frame(wrist, fwd, back, across)
    to_rig = frame_matrix(origin, right, barrel, up)
    # The grip and its panels, the frame, the trigger and its guard, the
    # hammer's spur.
    box(b, to_rig, Vector((0.0, -0.012, -0.012)), (0.026, 0.048, 0.118), FRAME, "gun", GRIP_TILT)
    for side in (-1.0, 1.0):
        box(b, to_rig, Vector((side * 0.0145, -0.012, -0.016)), (0.004, 0.036, 0.085), WOOD, "gun", GRIP_TILT)
    box(b, to_rig, Vector((0.0, 0.052, 0.046)), (0.026, 0.14, 0.022), FRAME, "gun")
    box(b, to_rig, Vector((0.0, 0.03, 0.013)), (0.009, 0.05, 0.006), FRAME, "gun")
    box(b, to_rig, Vector((0.0, 0.054, 0.024)), (0.009, 0.006, 0.02), FRAME, "gun")
    box(b, to_rig, Vector((0.0, 0.022, 0.024)), (0.006, 0.006, 0.016), STEEL, "gun")
    box(b, to_rig, Vector((0.0, -0.07, 0.068)), (0.008, 0.014, 0.014), FRAME, "gun")
    box(b, to_rig, Vector((0.0, 0.163, 0.068)), (0.012, 0.008, 0.012), FRAME, "gun")
    # The slide and its sights: their tops make the sight line.
    box(b, to_rig, Vector((0.0, 0.048, 0.072)), (0.030, 0.222, 0.034), STEEL, "slide")
    top, height = D.sight_line, 0.010
    for side in (-1.0, 1.0):
        box(b, to_rig, Vector((side * 0.0075, -0.052, top - height / 2)), (0.007, 0.008, height), FRAME, "slide")
        box(b, to_rig, Vector((side * 0.0075, -0.0566, top - 0.0045)), (0.0035, 0.0012, 0.0035), DOT, "slide")
    box(b, to_rig, Vector((0.0, 0.148, top - height / 2)), (0.006, 0.006, height), POST, "slide")
    # The magazine: a body inside the grip and the base plate under it.
    box(b, to_rig, Vector((0.0, -0.013, -0.02)), (0.02, 0.034, 0.1), STEEL, "mag", GRIP_TILT)
    box(b, to_rig, Vector((0.0, -0.032, -0.078)), (0.029, 0.05, 0.01), FRAME, "mag", GRIP_TILT)
    flash(b, to_rig, "flash", MUZZLE, 1.1)

    grip_up = (to_rig.to_3x3() @ (Matrix.Rotation(GRIP_TILT, 3, "X") @ Vector((0.0, 0.0, 1.0)))).normalized()
    at = lambda v: to_rig @ v  # noqa: E731
    bones = {
        "gun": (origin, origin + barrel * 0.08, hand),
        "slide": (at(Vector((0.0, -0.06, 0.072))), at(Vector((0.0, 0.04, 0.072))), "gun"),
        "mag": (at(Vector((0.0, -0.03, -0.075))), at(Vector((0.0, -0.03, -0.075))) + grip_up * 0.08, "gun"),
        "flash": (at(MUZZLE), at(MUZZLE) + barrel * 0.04, "gun"),
    }
    return bones, (origin, right, barrel, up)


def animate(rig, gun_rest, left_rest):
    return sidearm.animate(rig, gun_rest, left_rest, D)
