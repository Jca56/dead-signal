"""The .44 Magnum, built into the arms' mesh in the right hand: a big
stainless revolver, a long barrel under a rib with a lug beneath it, a
fluted cylinder, a spur hammer, a walnut grip, a notch in the top strap
and an orange blade for sights. Its clips and bones are every sidearm's
(`sidearm.py`): its cylinder is its "magazine" (swung out to the left,
filled where it is, and shut), its hammer its "slide".
"""

import math

from mathutils import Vector

import sidearm
from gun_kit import box, flash, frame_matrix, gun_frame

STEEL = (0.56, 0.57, 0.58)
STEEL_DARK = (0.40, 0.41, 0.43)
SHADOW = (0.16, 0.16, 0.17)
WOOD = (0.40, 0.22, 0.10)
BRASS = (0.78, 0.60, 0.26)
POST = (0.95, 0.45, 0.08)

MUZZLE = Vector((0.0, 0.245, 0.076))
GRIP_TILT = math.radians(-22)
CYLINDER = Vector((0.0, 0.04, 0.066))

D = sidearm.Design(
    sight_line=0.104,
    aim_distance=0.22,
    muzzle=MUZZLE,
    fire_frames=9,
    kick=16.0,
    slide_back=0.01,
    reload_frames=78,
    mag_out=0.034,
    swapped=False,
    rack=0.01,
)


def build(b, wrist, fwd, back, across, hand, left=None):
    """Add the revolver to builder `b` in the right hand; its bones and
    frame."""
    origin, right, barrel, up = gun_frame(wrist, fwd, back, across)
    to_rig = frame_matrix(origin, right, barrel, up)
    # The grip, the frame round the cylinder (its back, its top strap, its
    # front), the trigger and its guard.
    box(b, to_rig, Vector((0.0, -0.016, -0.018)), (0.03, 0.046, 0.108), WOOD, "gun", GRIP_TILT)
    box(b, to_rig, Vector((0.0, -0.012, 0.055)), (0.026, 0.05, 0.06), STEEL, "gun")
    box(b, to_rig, Vector((0.0, 0.04, 0.094)), (0.022, 0.07, 0.008), STEEL, "gun")
    box(b, to_rig, Vector((0.0, 0.04, 0.038)), (0.02, 0.07, 0.008), STEEL, "gun")
    box(b, to_rig, Vector((0.0, 0.086, 0.066)), (0.026, 0.024, 0.064), STEEL, "gun")
    box(b, to_rig, Vector((0.0, 0.034, 0.012)), (0.009, 0.05, 0.006), STEEL, "gun")
    box(b, to_rig, Vector((0.0, 0.058, 0.024)), (0.009, 0.006, 0.022), STEEL, "gun")
    box(b, to_rig, Vector((0.0, 0.026, 0.024)), (0.006, 0.006, 0.016), STEEL_DARK, "gun")
    # The barrel, the rib over it, the lug under it.
    box(b, to_rig, Vector((0.0, 0.168, 0.076)), (0.022, 0.15, 0.022), STEEL, "gun")
    box(b, to_rig, Vector((0.0, 0.168, 0.09)), (0.008, 0.15, 0.006), STEEL_DARK, "gun")
    box(b, to_rig, Vector((0.0, 0.158, 0.058)), (0.016, 0.13, 0.014), STEEL, "gun")
    box(b, to_rig, Vector((0.0, 0.2445, 0.076)), (0.012, 0.004, 0.012), SHADOW, "gun")
    # The sights: a notch in the back of the top strap, the blade at the
    # muzzle; their tops make the sight line.
    top = D.sight_line
    for side in (-1.0, 1.0):
        box(b, to_rig, Vector((side * 0.0075, 0.008, top - 0.004)), (0.007, 0.008, 0.008), SHADOW, "gun")
    box(b, to_rig, Vector((0.0, 0.232, top - 0.006)), (0.005, 0.014, 0.012), POST, "gun")
    # The hammer, and the cylinder: its flutes, the rounds' heads at its
    # back.
    box(b, to_rig, Vector((0.0, -0.034, 0.09)), (0.008, 0.014, 0.024), STEEL_DARK, "slide")
    box(b, to_rig, CYLINDER, (0.044, 0.05, 0.044), STEEL_DARK, "mag")
    for dx, dz in ((0.0, 0.0225), (0.0, -0.0225), (0.0225, 0.0), (-0.0225, 0.0)):
        box(b, to_rig, CYLINDER + Vector((dx, 0.004, dz)), (0.012 if dx == 0 else 0.004, 0.034, 0.004 if dx == 0 else 0.012), SHADOW, "mag")
    for dx, dz in ((0.011, 0.011), (-0.011, 0.011), (0.011, -0.011), (-0.011, -0.011)):
        box(b, to_rig, CYLINDER + Vector((dx, -0.0255, dz)), (0.01, 0.002, 0.01), BRASS, "mag")
    flash(b, to_rig, "flash", MUZZLE, 1.5)

    at = lambda v: to_rig @ v  # noqa: E731
    bones = {
        "gun": (origin, origin + barrel * 0.08, hand),
        "slide": (at(Vector((0.0, -0.06, 0.09))), at(Vector((0.0, 0.02, 0.09))), "gun"),
        # (Along it is to the right: out is to the left, and a little down.)
        "mag": (at(CYLINDER), at(CYLINDER) + (right + up * 0.35).normalized() * 0.06, "gun"),
        "flash": (at(MUZZLE), at(MUZZLE) + barrel * 0.04, "gun"),
    }
    return bones, (origin, right, barrel, up)


def animate(rig, gun_rest, left_rest):
    return sidearm.animate(rig, gun_rest, left_rest, D)
