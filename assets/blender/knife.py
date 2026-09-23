"""The tactical knife, in the right hand: a black grip, a small guard, a
clip-point blade up out of the fist, its edge forward. Its clips:

    Idle   3 s     held low and forward, point up, the left hand in guard
    Bash   0.33 s  a quick stab at the middle of the view, landing on
                   frame 4
"""

from mathutils import Vector

import melee_kit
import poses
from gun_kit import box, frame_matrix, gun_frame
from melee_kit import EDGE, POLYMER, STEEL

# Held here, a little forward of the pistol's hand, point up.
GRIP = Vector((0.12, 0.26, -0.18))


def build(b, wrist, fwd, back, across, hand, left=None):
    """Add the knife to builder `b` in the right hand; its bone, and its
    frame."""
    origin, right, barrel, up = gun_frame(wrist, fwd, back, across)
    to_rig = frame_matrix(origin, right, barrel, up)
    box(b, to_rig, Vector((0.0, 0.0, 0.0)), (0.026, 0.03, 0.11), POLYMER, "gun")
    box(b, to_rig, Vector((0.0, 0.0, 0.06)), (0.022, 0.06, 0.012), POLYMER, "gun")
    box(b, to_rig, Vector((0.0, -0.004, 0.14)), (0.006, 0.026, 0.15), STEEL, "gun")
    box(b, to_rig, Vector((0.0, 0.011, 0.13)), (0.005, 0.006, 0.12), EDGE, "gun")
    box(b, to_rig, Vector((0.0, 0.0, 0.225)), (0.005, 0.016, 0.03), EDGE, "gun")
    return {"gun": (origin, origin + barrel * 0.08, hand)}, (origin, right, barrel, up)


def animate(rig, gun_rest, left_rest):
    def at(offset=Vector(), pitch=0.0, roll=0.0, yaw=0.0):
        return poses.gun_pose(offset, pitch, roll, yaw, grip=GRIP)

    rest = at(pitch=-25.0)
    stab = [
        (0, rest),
        (2, at(Vector((0.02, -0.06, 0.02)), pitch=-40.0)),
        (4, at(Vector((-0.09, 0.2, 0.08)), pitch=-85.0, yaw=8.0)),
        (6, at(Vector((-0.08, 0.18, 0.07)), pitch=-80.0, yaw=8.0)),
        (10, rest),
    ]
    return melee_kit.animate(rig, gun_rest, left_rest, rest, {"Bash": stab})
