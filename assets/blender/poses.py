"""The pistol's animations, posed by where the hands and gun should be and
solved by Blender's IK, then baked into plain keyframes the game plays.

Each action is laid out on its own stretch of the timeline: two empties
say where the right hand (carrying the gun) and the left hand go, frame by
frame; IK on each hand bends its arm to get there; the slide, magazine and
flash bones and the trigger finger are keyed directly. Baking turns the
lot into one keyframe per bone per frame, and the IK and empties go away.

    PistolIdle    3 s   a two-handed grip, breathing
    PistolFire    0.2 s the kick, the slide back and home, the flash
    PistolReload  1.4 s tilt, mag out, left hand away and back with a new
                        one, seated, slide racked
    PistolMelee   0.5 s a pistol-whip: wind up, strike (frame 9), recover
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


def gun_pose(offset=Vector(), pitch=0.0, roll=0.0, yaw=0.0):
    """The gun's frame held at GRIP_AT (+ `offset`), aimed at AIM, then
    turned by `pitch` (up), `roll` (clockwise) and `yaw` (left), degrees."""
    origin = GRIP_AT + offset
    barrel = (AIM - (origin + Vector((0, 0, 0.07)))).normalized()
    up = Vector((0, 0, 1))
    up = (up - barrel * up.dot(barrel)).normalized()
    right = barrel.cross(up).normalized()
    turn = Matrix.Rotation(math.radians(yaw), 3, up) @ Matrix.Rotation(math.radians(pitch), 3, right) @ Matrix.Rotation(CANT + math.radians(roll), 3, barrel)
    barrel, up, right = turn @ barrel, turn @ up, turn @ right
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
        self.targets = {}
        for side in ("R", "L"):
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

    def key(self, frame, gun, left):
        """Both hands for one frame: the right carrying the gun at `gun`,
        the left at (wrist, fwd, back)."""
        for side, m in (("R", self.right_for(gun)), ("L", self.left_for(*left))):
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

    def clear(self):
        for side in ("R", "L"):
            pb = self.rig.pose.bones[f"hand.{side}"]
            for c in list(pb.constraints):
                pb.constraints.remove(c)
            bpy.data.objects.remove(self.targets[side])


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


def build(rig, gun_rest, left_rest):
    r = Rig(rig, gun_rest, left_rest)
    r.add_ik()
    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.mode_set(mode="POSE")
    finger = trigger_finger(rig)

    # One working action holds the directly keyed bones for every stretch.
    work = bpy.data.actions.new("work")
    rig.animation_data.action = work

    def steady(frame, flash=0.0, slide=0.0, mag=0.0, mag_scale=1.0):
        key_bone(rig, "flash", frame, scale=flash)
        key_bone(rig, "slide", frame, loc=Vector((0, slide, 0)))
        key_bone(rig, "mag", frame, loc=Vector((0, mag, 0)), scale=mag_scale)
        key_bone(rig, "index1.R", frame, rot=finger)
        key_bone(rig, "gun", frame, scale=1.0)

    spans = {}

    # PistolIdle: frames 1..91, a slow breath.
    start = 1
    for f in range(start, start + 91, 5):
        t = (f - start) / 90.0
        lift = Vector((0, 0, 0.004 * math.sin(t * math.tau)))
        g = gun_pose(lift, pitch=0.6 * math.sin(t * math.tau))
        r.key(f, g, support(g))
        steady(f)
    spans["PistolIdle"] = (start, start + 90)

    # PistolFire: frames 101..107.
    start = 101
    base = gun_pose()
    kick = gun_pose(Vector((0.0, -0.025, 0.012)), pitch=7.0)
    for f, g, slide, fl in ((0, base, 0.0, 1.0), (1, kick, -0.038, 0.0), (3, gun_pose(Vector((0, -0.01, 0.004)), pitch=2.0), 0.0, 0.0), (6, base, 0.0, 0.0)):
        r.key(start + f, g, support(g))
        steady(start + f, flash=fl, slide=slide)
    spans["PistolFire"] = (start, start + 6)

    # PistolReload: frames 201..243.
    start = 201
    tilt = gun_pose(Vector((0.01, -0.02, 0.025)), pitch=10.0, roll=25.0)
    low = Vector((-0.20, 0.10, -0.46))
    frames = [
        (0, base, support(base), 0.0, 1.0, 0.0),
        (5, tilt, away(Vector((-0.12, 0.18, -0.30))), 0.0, 1.0, 0.0),
        (9, tilt, away(low), -0.18, 1.0, 0.0),
        (12, tilt, away(low), -0.40, 0.0, 0.0),
        (18, tilt, away(low), -0.40, 0.0, 0.0),
        (20, tilt, away(low), -0.16, 1.0, 0.0),
        (25, tilt, support(tilt, 0.0, 0.0, -0.06), -0.12, 1.0, 0.0),
        (29, tilt, support(tilt, 0.0, 0.0, -0.02), 0.0, 1.0, 0.0),
        (32, tilt, support(tilt, 0.02, -0.06, 0.07), 0.0, 1.0, 0.0),
        (34, tilt, support(tilt, 0.02, -0.12, 0.07), 0.0, 1.0, -0.04),
        (35, tilt, support(tilt, 0.02, -0.12, 0.05), 0.0, 1.0, 0.0),
        (42, base, support(base), 0.0, 1.0, 0.0),
    ]
    for f, g, left, mag, mag_scale, slide in frames:
        r.key(start + f, g, left)
        steady(start + f, slide=slide, mag=mag, mag_scale=mag_scale)
    spans["PistolReload"] = (start, start + 42)

    # PistolMelee: frames 301..316, the strike landing on 309.
    start = 301
    wind = gun_pose(Vector((0.05, -0.06, 0.08)), pitch=35.0, roll=20.0, yaw=-10.0)
    strike = gun_pose(Vector((-0.06, 0.12, -0.03)), pitch=-55.0, roll=-10.0, yaw=15.0)
    guard = away(Vector((-0.20, 0.16, -0.34)))
    for f, g, left in ((0, base, support(base)), (4, wind, guard), (8, strike, guard), (10, strike, guard), (15, base, support(base))):
        r.key(start + f, g, left)
        steady(start + f)
    spans["PistolMelee"] = (start, start + 15)

    # The flash pops for one frame: no easing into or out of it.
    for fc in work.fcurves if hasattr(work, "fcurves") else []:
        if "flash" in fc.data_path:
            for kp in fc.keyframe_points:
                kp.interpolation = "CONSTANT"

    for name, (a, b) in spans.items():
        rig.animation_data.action = work
        bake(rig, name, a, b)
    r.clear()
    bpy.data.actions.remove(work)
    bpy.ops.object.mode_set(mode="OBJECT")
    return list(spans)
