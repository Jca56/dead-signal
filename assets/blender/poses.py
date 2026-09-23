"""Posing a weapon's clips by where the hands and the weapon should be,
solved by Blender's IK, then baked into plain keyframes the game plays.

Each clip is laid out on its own stretch of the timeline: two empties say
where the right hand (carrying the weapon) and the left hand go, frame by
frame; IK on each hand bends its arm to get there; the weapon's moving
parts and the fingers are keyed directly. Baking turns the lot into one
keyframe per bone per frame, and the IK and empties go away. The clips
themselves are the weapon's (`pistol.py`'s `animate`).
"""

import math

import bpy
from mathutils import Matrix, Vector

FPS = 30

# Where the gun is held (its grip, in camera space: +Y ahead, +Z up), and
# where it points: at the middle of the screen, 8 m out.
GRIP_AT = Vector((0.085, 0.30, -0.165))
AIM = Vector((0.0, 8.0, 0.0))
CANT = math.radians(-5)  # the top tipped in, a touch


def rest(rig, bone):
    return rig.data.bones[bone].matrix_local.copy()


def frame_to(origin, x, y, z):
    m = Matrix.Identity(4)
    for r in range(3):
        m[r][0], m[r][1], m[r][2], m[r][3] = x[r], y[r], z[r], origin[r]
    return m


def gun_pose(offset=Vector(), pitch=0.0, roll=0.0, yaw=0.0, grip=GRIP_AT):
    """The gun's frame held at `grip` (+ `offset`), aimed at AIM, then
    turned by `pitch` (up), `roll` (clockwise) and `yaw` (left), degrees."""
    origin = grip + offset
    barrel = (AIM - (origin + Vector((0, 0, 0.07)))).normalized()
    up = Vector((0, 0, 1))
    up = (up - barrel * up.dot(barrel)).normalized()
    right = barrel.cross(up).normalized()
    turn = Matrix.Rotation(math.radians(yaw), 3, up) @ Matrix.Rotation(math.radians(pitch), 3, right) @ Matrix.Rotation(CANT + math.radians(roll), 3, barrel)
    barrel, up, right = turn @ barrel, turn @ up, turn @ right
    return frame_to(origin, right, barrel, up)


def aim_pose(sight, distance, offset=Vector(), pitch=0.0):
    """The gun's frame raised to the eye: dead level and square, its
    sight line (`sight` above its origin, in its own frame) on the line of
    sight `distance` ahead, then moved by `offset` and tipped up by
    `pitch` degrees about its sights."""
    right = Vector((1, 0, 0))
    turn = Matrix.Rotation(math.radians(pitch), 3, right)
    barrel, up = turn @ Vector((0, 1, 0)), turn @ Vector((0, 0, 1))
    origin = Vector((0, distance, 0)) - up * sight + offset
    return frame_to(origin, right, barrel, up)


class Rig:
    """What the poses are built from: the rig, its rest frames and targets."""

    def __init__(self, rig, gun_rest, left_rest):
        self.rig = rig
        # The gun's frame at rest (origin, right, barrel, up) and the hand
        # bone's rest matrix: the hand is carried with the gun.
        o, r, b, u = gun_rest
        self.gun_rest = frame_to(o, r, b, u)
        self.hand_r = rest(rig, "hand.R")
        # The left hand's frame at rest (wrist, fwd, back) and bone matrix.
        w, f, k = left_rest
        self.left_rest = frame_to(w, f.cross(k).normalized(), f, k)
        self.hand_l = rest(rig, "hand.L")
        # The gun bone's rest matrix: the gun itself is put exactly where
        # it's posed (IK brings the hand only close).
        self.gun_bone = rest(rig, "gun")
        self.targets = {}
        for side in ("R", "L", "gun"):
            e = bpy.data.objects.new(f"target.{side}", None)
            bpy.context.scene.collection.objects.link(e)
            e.rotation_mode = "QUATERNION"
            self.targets[side] = e

    def right_for(self, gun):
        """The right hand bone's matrix that holds the gun at `gun`."""
        return gun @ self.gun_rest.inverted() @ self.hand_r

    def left_for(self, wrist, fwd, back):
        """The left hand bone's matrix with its wrist at `wrist`, knuckles
        along `fwd`, back of the hand to `back`."""
        fwd = fwd.normalized()
        back = (back - fwd * back.dot(fwd)).normalized()
        want = frame_to(wrist, fwd.cross(back).normalized(), fwd, back)
        return want @ self.left_rest.inverted() @ self.hand_l

    def key(self, frame, gun, left, right=None):
        """Both hands for one frame, the gun at `gun`: the right carrying it
        (or, given `right`, gripping there as if it were a gun's grip: a
        bolt's handle, say), the left at (wrist, fwd, back)."""
        hand = self.right_for(gun if right is None else right)
        for side, m in (("R", hand), ("L", self.left_for(*left)), ("gun", gun @ self.gun_rest.inverted() @ self.gun_bone)):
            e = self.targets[side]
            e.matrix_world = m
            e.keyframe_insert("location", frame=frame)
            e.keyframe_insert("rotation_quaternion", frame=frame)

    def add_ik(self):
        for side in ("R", "L"):
            c = self.rig.pose.bones[f"hand.{side}"].constraints.new("IK")
            c.target = self.targets[side]
            c.use_tail = False
            c.use_rotation = True
            c.chain_count = 3
        c = self.rig.pose.bones["gun"].constraints.new("COPY_TRANSFORMS")
        c.target = self.targets["gun"]

    def clear(self):
        for bone in ("hand.R", "hand.L", "gun"):
            pb = self.rig.pose.bones[bone]
            for c in list(pb.constraints):
                pb.constraints.remove(c)
        for e in self.targets.values():
            bpy.data.objects.remove(e)


def support(gun, dx=0.0, dy=0.0, dz=0.0):
    """The left hand cupped under the right on the grip of `gun`."""
    o = gun.translation
    barrel = gun.col[1].xyz
    up = gun.col[2].xyz
    right = gun.col[0].xyz
    wrist = o - right * 0.05 - barrel * 0.035 - up * 0.055 + Vector((dx, dy, dz))
    fwd = right * 0.75 + barrel * 0.5 + up * 0.35
    back = -right * 0.4 - up * 0.9
    return wrist, fwd, back


def forend(gun, along, dx=0.0, dy=0.0, dz=0.0):
    """The left hand under a long gun's forend, `along` its barrel from
    its origin, fingers wrapped up round its far side."""
    o = gun.translation
    barrel = gun.col[1].xyz
    up = gun.col[2].xyz
    right = gun.col[0].xyz
    wrist = o + barrel * (along - 0.05) - right * 0.03 - up * 0.05 + Vector((dx, dy, dz))
    fwd = right * 0.85 + barrel * 0.35 + up * 0.25
    back = -right * 0.3 - up * 0.95
    return wrist, fwd, back


def shoulder(rig, bone, frame, move):
    """Key `bone` (an upper arm) moved by `move`, camera space: a pose
    bone's location is in its own rest frame."""
    key_bone(rig, bone, frame, loc=rest(rig, bone).to_3x3().inverted() @ move)


def away(wrist):
    """The left hand dropped out of sight."""
    return wrist, Vector((0.4, 0.5, -0.6)), Vector((-0.8, 0.0, -0.3))


def key_bone(rig, bone, frame, loc=None, scale=None, rot=None):
    pb = rig.pose.bones[bone]
    if loc is not None:
        pb.location = loc
        pb.keyframe_insert("location", frame=frame)
    if scale is not None:
        pb.scale = (scale, scale, scale)
        pb.keyframe_insert("scale", frame=frame)
    if rot is not None:
        pb.rotation_mode = "QUATERNION"
        pb.rotation_quaternion = rot
        pb.keyframe_insert("rotation_quaternion", frame=frame)


def trigger_finger(rig):
    """The right index finger straightened along the frame, off the trigger:
    whichever way of turning its first joint takes the tip further from
    the palm."""
    pb = rig.pose.bones["index1.R"]
    pb.rotation_mode = "QUATERNION"
    palm = rig.data.bones["hand.R"].head_local
    best, far = None, -1.0
    for sign in (1, -1):
        q = Matrix.Rotation(math.radians(55 * sign), 4, "X").to_quaternion()
        pb.rotation_quaternion = q
        bpy.context.view_layer.update()
        tip = (rig.matrix_world @ rig.pose.bones["index2.R"].tail)
        d = (tip - palm).length
        if d > far:
            best, far = q, d
    pb.rotation_quaternion = (1, 0, 0, 0)
    return best


def bake(rig, name, start, end):
    """Bake frames start..end into an action called `name`."""
    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.mode_set(mode="POSE")
    for pb in rig.pose.bones:
        if hasattr(pb, "select"):
            pb.select = True
        else:
            pb.bone.select = True
    bpy.ops.nla.bake(frame_start=start, frame_end=end, step=1, only_selected=True, visual_keying=True, clear_constraints=False, clear_parents=False, use_current_action=False, clean_curves=False, bake_types={"POSE"})
    action = rig.animation_data.action
    action.name = name
    rig.animation_data.action = None
    return action
