"""The machete, in the right hand: a wooden grip, a long, broad blade up
out of the fist, its edge forward. Its clips:

    Idle   3 s     held at the right, the blade up and back a little, the
                   left hand in guard
    Bash   0.53 s  a forehand slash: raised at the right, swept down across
                   to the left (landing on frame 7), and back
    Bash2  0.53 s  a backhand: raised at the left, swept down across to the
                   right, and back
"""

from mathutils import Vector

import melee_kit
import poses
from gun_kit import box, frame_matrix, gun_frame
from melee_kit import EDGE, STEEL, WOOD

GRIP = Vector((0.14, 0.24, -0.18))


def build(b, wrist, fwd, back, across, hand, left=None):
    """Add the machete to builder `b` in the right hand; its bone, and its
    frame."""
    origin, right, barrel, up = gun_frame(wrist, fwd, back, across)
    to_rig = frame_matrix(origin, right, barrel, up)
    box(b, to_rig, Vector((0.0, 0.0, 0.0)), (0.03, 0.035, 0.12), WOOD, "gun")
    box(b, to_rig, Vector((0.0, 0.0, 0.065)), (0.012, 0.045, 0.012), STEEL, "gun")
    box(b, to_rig, Vector((0.0, 0.0, 0.3)), (0.006, 0.045, 0.46), STEEL, "gun")
    box(b, to_rig, Vector((0.0, 0.026, 0.3)), (0.005, 0.008, 0.44), EDGE, "gun")
    box(b, to_rig, Vector((0.0, 0.01, 0.52)), (0.006, 0.065, 0.03), STEEL, "gun")
    return {"gun": (origin, origin + barrel * 0.08, hand)}, (origin, right, barrel, up)


def animate(rig, gun_rest, left_rest):
    def at(offset=Vector(), pitch=0.0, roll=0.0, yaw=0.0):
        return poses.gun_pose(offset, pitch, roll, yaw, grip=GRIP)

    rest = at(pitch=-15.0, roll=15.0)
    forehand = [
        (0, rest),
        (4, at(Vector((0.06, -0.04, 0.16)), pitch=25.0, roll=40.0)),
        (7, at(Vector((-0.12, 0.2, -0.02)), pitch=-55.0, roll=-50.0, yaw=18.0)),
        (9, at(Vector((-0.22, 0.15, -0.07)), pitch=-65.0, roll=-60.0, yaw=35.0)),
        (16, rest),
    ]
    backhand = [
        (0, rest),
        (4, at(Vector((-0.22, 0.0, 0.14)), pitch=20.0, roll=-45.0, yaw=30.0)),
        (7, at(Vector((0.02, 0.2, -0.03)), pitch=-55.0, roll=50.0, yaw=-18.0)),
        (9, at(Vector((0.08, 0.15, -0.08)), pitch=-65.0, roll=60.0, yaw=-35.0)),
        (16, rest),
    ]
    return melee_kit.animate(rig, gun_rest, left_rest, rest, {"Bash": forehand, "Bash2": backhand})
