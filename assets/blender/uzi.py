"""The Mini Uzi, built into the arms' mesh in the right hand: a boxy
stamped receiver, its grip in the middle of it and a long stick magazine
up through the grip, a stub of barrel in its nut, the cocking knob on top,
a wire stock folded under, and a notch and a guarded post for sights. Its
clips and bones are every sidearm's (`sidearm.py`); a round is two frames,
over and over while the trigger's held.
"""

import math

from mathutils import Matrix, Vector

import sidearm
from gun_kit import box, flash, frame_matrix, gun_frame

BLACK = (0.07, 0.07, 0.08)
PARKED = (0.15, 0.16, 0.16)
STEEL = (0.34, 0.34, 0.35)
DOT = (0.92, 0.91, 0.87)
POST = (0.95, 0.45, 0.08)

MUZZLE = Vector((0.0, 0.205, 0.062))
GRIP_TILT = math.radians(-8)

D = sidearm.Design(
    sight_line=0.112,
    aim_distance=0.24,
    muzzle=MUZZLE,
    fire_frames=2,
    kick=3.0,
    slide_back=0.02,
    reload_frames=48,
    mag_out=0.42,
    swapped=True,
    rack=0.035,
)


def build(b, wrist, fwd, back, across, hand, left=None):
    """Add the gun to builder `b` in the right hand; its bones and frame."""
    origin, right, barrel, up = gun_frame(wrist, fwd, back, across)
    to_rig = frame_matrix(origin, right, barrel, up)
    # The grip (the magazine's well), the trigger and its guard.
    box(b, to_rig, Vector((0.0, -0.01, -0.012)), (0.032, 0.05, 0.115), BLACK, "gun", GRIP_TILT)
    box(b, to_rig, Vector((0.0, 0.036, 0.012)), (0.009, 0.05, 0.006), PARKED, "gun")
    box(b, to_rig, Vector((0.0, 0.058, 0.024)), (0.009, 0.006, 0.022), PARKED, "gun")
    box(b, to_rig, Vector((0.0, 0.026, 0.024)), (0.006, 0.006, 0.016), STEEL, "gun")
    # The receiver, its ribs, the barrel in its nut.
    box(b, to_rig, Vector((0.0, 0.03, 0.062)), (0.044, 0.25, 0.056), PARKED, "gun")
    for y in (-0.05, 0.0, 0.05, 0.10):
        box(b, to_rig, Vector((0.0, y, 0.062)), (0.047, 0.012, 0.03), BLACK, "gun")
    box(b, to_rig, Vector((0.0, 0.162, 0.062)), (0.032, 0.02, 0.032), STEEL, "gun")
    box(b, to_rig, Vector((0.0, 0.188, 0.062)), (0.018, 0.035, 0.018), BLACK, "gun")
    # The stock, folded under the back of it.
    box(b, to_rig, Vector((0.0, -0.105, 0.045)), (0.03, 0.016, 0.07), STEEL, "gun")
    for side in (-1.0, 1.0):
        box(b, to_rig, Vector((side * 0.013, -0.06, 0.026)), (0.005, 0.10, 0.005), STEEL, "gun")
    # The sights: a rear notch (two ears, a white dot on each), a post
    # between its guards; their tops make the sight line.
    top, height = D.sight_line, 0.022
    for side in (-1.0, 1.0):
        box(b, to_rig, Vector((side * 0.0085, -0.07, top - height / 2)), (0.007, 0.008, height), BLACK, "gun")
        box(b, to_rig, Vector((side * 0.0085, -0.0746, top - 0.005)), (0.0035, 0.0012, 0.0035), DOT, "gun")
        box(b, to_rig, Vector((side * 0.014, 0.13, top - 0.008)), (0.004, 0.014, 0.03), BLACK, "gun")
    box(b, to_rig, Vector((0.0, 0.13, top - height / 2)), (0.005, 0.005, height), BLACK, "gun")
    box(b, to_rig, Vector((0.0, 0.1295, top - 0.0015)), (0.005, 0.005, 0.005), POST, "gun")
    # The cocking knob on top, and the magazine up through the grip.
    box(b, to_rig, Vector((0.0, 0.03, 0.095)), (0.016, 0.03, 0.012), STEEL, "slide")
    box(b, to_rig, Vector((0.0, -0.018, -0.06)), (0.022, 0.034, 0.19), BLACK, "mag", GRIP_TILT)
    box(b, to_rig, Vector((0.0, -0.031, -0.156)), (0.027, 0.04, 0.008), STEEL, "mag", GRIP_TILT)
    flash(b, to_rig, "flash", MUZZLE, 1.0)

    grip_up = (to_rig.to_3x3() @ (Matrix.Rotation(GRIP_TILT, 3, "X") @ Vector((0.0, 0.0, 1.0)))).normalized()
    at = lambda v: to_rig @ v  # noqa: E731
    bones = {
        "gun": (origin, origin + barrel * 0.08, hand),
        "slide": (at(Vector((0.0, -0.02, 0.095))), at(Vector((0.0, 0.06, 0.095))), "gun"),
        "mag": (at(Vector((0.0, -0.03, -0.15))), at(Vector((0.0, -0.03, -0.15))) + grip_up * 0.08, "gun"),
        "flash": (at(MUZZLE), at(MUZZLE) + barrel * 0.04, "gun"),
    }
    return bones, (origin, right, barrel, up)


def animate(rig, gun_rest, left_rest):
    return sidearm.animate(rig, gun_rest, left_rest, D)
