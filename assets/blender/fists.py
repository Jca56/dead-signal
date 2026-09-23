"""Bare fists: nothing added to the arms, and two clips.

    Idle   3 s    the guard, breathing
    Bash   0.5 s  a right jab: wind (frame 3), the fist lands (frame 6,
                  the shoulder behind it), held, and back to the guard

The jab is posed like the pistol's clips (`poses.py`): an empty says where
the right hand goes, IK on the hand bends the arm to it, and the lot is
baked. The shoulder is carried forward too: the arm is nearly straight at
rest, so the reach comes from behind it.
"""

import math

import bpy
from mathutils import Matrix, Vector

import poses


def build(b, wrist, fwd, back, across, hand):
    """Nothing in the hands: no parts, no bones."""
    return {}, None


def guard_idle(rig):
    """A slow breath: the arms rise and settle, the hands ease, over three
    seconds, back where they began."""
    action = bpy.data.actions.new("Idle")
    rig.animation_data_create()
    rig.animation_data.action = action
    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.mode_set(mode="POSE")
    pose = rig.pose.bones
    for pb in pose:
        pb.rotation_mode = "QUATERNION"
    keys = [(1, 0.0), (46, 1.0), (91, 0.0)]
    for frame, t in keys:
        for suffix, s in ((".R", 1.0), (".L", -1.0)):
            pb = pose[f"upper_arm{suffix}"]
            pb.rotation_quaternion = Matrix.Rotation(math.radians(-1.6 * t), 4, "X").to_quaternion()
            pb.keyframe_insert("rotation_quaternion", frame=frame)
            pb = pose[f"hand{suffix}"]
            pb.rotation_quaternion = Matrix.Rotation(math.radians(2.5 * t * s), 4, "Z").to_quaternion()
            pb.keyframe_insert("rotation_quaternion", frame=frame)
            for f in ("fingers1", "index1"):
                pb = pose[f"{f}{suffix}"]
                pb.rotation_quaternion = Matrix.Rotation(math.radians(4.0 * t), 4, "X").to_quaternion()
                pb.keyframe_insert("rotation_quaternion", frame=frame)
    bpy.ops.object.mode_set(mode="OBJECT")
    rig.animation_data.action = None
    return action


def jab(rig):
    """The right jab, baked into "Bash"."""
    hand_rest = poses.rest(rig, "hand.R")
    shoulder_rest = poses.rest(rig, "upper_arm.R").to_3x3()
    wrist = hand_rest.translation.copy()

    def hand_at(move, pitch=0.0, turn=0.0):
        """The hand bone moved by `move` (camera space) and turned about
        its wrist: `pitch` knuckles down, `turn` inwards, degrees."""
        spin = Matrix.Rotation(math.radians(turn), 4, "Z") @ Matrix.Rotation(math.radians(-pitch), 4, "X")
        return Matrix.Translation(wrist + move) @ spin @ Matrix.Translation(-wrist) @ hand_rest

    target = bpy.data.objects.new("target.R", None)
    bpy.context.scene.collection.objects.link(target)
    target.rotation_mode = "QUATERNION"
    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.mode_set(mode="POSE")
    ik = rig.pose.bones["hand.R"].constraints.new("IK")
    ik.target = target
    ik.use_tail = False
    ik.use_rotation = True
    ik.chain_count = 3

    work = bpy.data.actions.new("work")
    rig.animation_data.action = work
    start = 1
    # (frame, the shoulder's move, the hand's move, pitch, turn)
    keys = [
        (0, Vector(), Vector(), 0.0, 0.0),
        (3, Vector((0.01, -0.03, -0.01)), Vector((0.01, -0.05, -0.025)), 5.0, 0.0),
        (6, Vector((-0.05, 0.12, 0.03)), Vector((-0.09, 0.14, 0.05)), 25.0, 12.0),
        (9, Vector((-0.04, 0.10, 0.025)), Vector((-0.08, 0.11, 0.04)), 22.0, 10.0),
        (15, Vector(), Vector(), 0.0, 0.0),
    ]
    for f, shoulder, move, pitch, turn in keys:
        target.matrix_world = hand_at(move, pitch, turn)
        target.keyframe_insert("location", frame=start + f)
        target.keyframe_insert("rotation_quaternion", frame=start + f)
        # A pose bone's location is in its own rest frame.
        poses.key_bone(rig, "upper_arm.R", start + f, loc=shoulder_rest.inverted() @ shoulder)
    action = poses.bake(rig, "Bash", start, start + 15)
    pb = rig.pose.bones["hand.R"]
    for c in list(pb.constraints):
        pb.constraints.remove(c)
    bpy.data.objects.remove(target)
    bpy.data.actions.remove(work)
    bpy.ops.object.mode_set(mode="OBJECT")
    return action


def animate(rig, rest, left_rest):
    """Every clip; their names."""
    guard_idle(rig)
    jab(rig)
    return ["Idle", "Bash"]
