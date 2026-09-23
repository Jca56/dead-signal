"""The fire axe, in both hands: a long wooden haft, the right hand up near
the head, the left low on it; a red head, its blade forward, a pick at its
back. Its clips (the shoulders rolled forward to hold it):

    Idle   3 s     held across the body, the head up at the left
    Bash   0.93 s  an overhead chop: raised up and back (frame 9), brought
                   down into the middle of the view (landing on frame 14),
                   and back
"""

from mathutils import Vector

import melee_kit
import poses
from gun_kit import box, frame_matrix, gun_frame
from melee_kit import EDGE, RED, STEEL, WOOD

GRIP = Vector((0.12, 0.26, -0.16))
# Where the left hand holds the haft, below the right.
LOW = -0.3
LEFT_SHOULDER = Vector((0.06, 0.12, 0.02))
RIGHT_SHOULDER = Vector((0.0, 0.04, 0.0))


def build(b, wrist, fwd, back, across, hand, left=None):
    """Add the axe to builder `b` in the right hand; its bone, and its
    frame."""
    origin, right, barrel, up = gun_frame(wrist, fwd, back, across)
    to_rig = frame_matrix(origin, right, barrel, up)
    box(b, to_rig, Vector((0.0, 0.0, -0.02)), (0.035, 0.045, 0.70), WOOD, "gun")
    box(b, to_rig, Vector((0.0, 0.03, 0.36)), (0.022, 0.2, 0.1), RED, "gun")
    box(b, to_rig, Vector((0.0, 0.14, 0.36)), (0.012, 0.03, 0.14), STEEL, "gun")
    box(b, to_rig, Vector((0.0, 0.157, 0.36)), (0.008, 0.008, 0.14), EDGE, "gun")
    box(b, to_rig, Vector((0.0, -0.1, 0.37)), (0.016, 0.08, 0.03), RED, "gun")
    return {"gun": (origin, origin + barrel * 0.08, hand)}, (origin, right, barrel, up)


def animate(rig, gun_rest, left_rest):
    def at(offset=Vector(), pitch=0.0, roll=0.0, yaw=0.0):
        return poses.gun_pose(offset, pitch, roll, yaw, grip=GRIP)

    rest = at(pitch=-10.0, roll=-35.0)
    chop = [
        (0, rest),
        (9, at(Vector((-0.04, -0.12, 0.28)), pitch=65.0, roll=-10.0)),
        (12, at(Vector((-0.06, 0.08, 0.2)), pitch=10.0, roll=-5.0)),
        (14, at(Vector((-0.08, 0.26, -0.04)), pitch=-70.0, roll=-5.0)),
        (17, at(Vector((-0.08, 0.24, -0.1)), pitch=-80.0, roll=-5.0)),
        (28, rest),
    ]

    def left(g):
        return melee_kit.haft(g, LOW)

    return melee_kit.animate(rig, gun_rest, left_rest, rest, {"Bash": chop}, left, (LEFT_SHOULDER, RIGHT_SHOULDER))
