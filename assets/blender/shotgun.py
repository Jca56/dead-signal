"""The pump shotgun, built into the arms' mesh: a walnut stock in the
right hand, a blued receiver and barrel, the tube magazine under it and
the pump's forend in the left hand, an orange bead at the muzzle; a shell
the left hand carries to load it; the flash. Bones for its moving parts:

    gun    the receiver, barrel, stock; follows the right hand
    pump   the forend, racked back along the tube and home
    shell  a shell in the left hand, only there while loading
    flash  a wide star at the muzzle, the frame a shot goes off

And its clips, the long gun shouldered (both shoulders rolled forward to
reach it: the arms were made for a pistol's guard):

    Idle         3 s    held low at the hip, breathing
    Fire         0.8 s  the kick, then the pump racked back and home
    ReloadStart  0.4 s  tipped over, the left hand down for a shell
    ReloadShell  0.43 s a shell up into the loading port and thumbed in
                        (frame 8), and down for the next (it loops)
    ReloadEnd    0.5 s  the left hand back on the forend, racked
    Bash         0.6 s  a shove with the side of the gun, landing on
                        frame 8
    Aim          3 s    shouldered, the bead on the middle of the view
    AimFire      0.8 s  the kick and the pump from there
"""

import math

import bpy
from mathutils import Vector

import poses
from gun_kit import box, cylinder, flash, frame_matrix, gun_frame

WALNUT = (0.40, 0.23, 0.12)
WALNUT_DARK = (0.28, 0.16, 0.08)
BLUED = (0.12, 0.13, 0.15)
BLUED_DARK = (0.07, 0.07, 0.08)
PAD = (0.06, 0.06, 0.06)
BEAD = (0.95, 0.45, 0.08)
HULL = (0.62, 0.10, 0.08)
BRASS = (0.78, 0.60, 0.26)

# Where things are, in the gun's frame, metres: the muzzle, the middle of
# the forend at rest (the pump racks it back `PUMP` along the tube), the
# loading port underneath, and how high the bead stands (the sight line).
MUZZLE = Vector((0.0, 0.83, 0.058))
FOREND = 0.30
PUMP = 0.075
PORT = Vector((0.0, 0.14, 0.0))
SIGHT_LINE = 0.080
# Where it's held at the hip (the right hand on the stock's wrist), and how
# far ahead of the eye when shouldered.
HIP = Vector((0.11, 0.04, -0.15))
AIM_DISTANCE = 0.14
# The shoulders, rolled forward to reach the forend (camera space).
LEFT_SHOULDER = Vector((0.06, 0.13, 0.02))
RIGHT_SHOULDER = Vector((0.0, 0.05, 0.0))


def build(b, wrist, fwd, back, across, hand, left=None):
    """Add the shotgun to builder `b` in the right hand, and a shell in the
    left (at `left`: its wrist, knuckles and back of the hand); the bones,
    as {name: (head, tail, parent)}, and the gun's frame."""
    origin, right, barrel, up = gun_frame(wrist, fwd, back, across)
    to_rig = frame_matrix(origin, right, barrel, up)
    grip = math.radians(-20)
    # The stock: its wrist in the hand, the butt back past the eye.
    box(b, to_rig, Vector((0.0, -0.02, -0.005)), (0.036, 0.10, 0.05), WALNUT, "gun", grip)
    box(b, to_rig, Vector((0.0, -0.20, -0.03)), (0.042, 0.28, 0.09), WALNUT, "gun")
    box(b, to_rig, Vector((0.0, -0.345, -0.03)), (0.044, 0.02, 0.10), PAD, "gun")
    # The receiver, its ejection port on the right, the loading port under.
    box(b, to_rig, Vector((0.0, 0.10, 0.04)), (0.046, 0.22, 0.07), BLUED, "gun")
    box(b, to_rig, Vector((0.023, 0.11, 0.05)), (0.004, 0.06, 0.02), BLUED_DARK, "gun")
    box(b, to_rig, PORT + Vector((0.0, 0.0, 0.004)), (0.026, 0.07, 0.004), BLUED_DARK, "gun")
    # The trigger guard and trigger.
    box(b, to_rig, Vector((0.0, 0.035, -0.012)), (0.012, 0.07, 0.01), BLUED, "gun")
    box(b, to_rig, Vector((0.0, 0.03, 0.0)), (0.005, 0.008, 0.02), BLUED_DARK, "gun")
    # The barrel, a rib along its top level with the receiver's, the bead.
    box(b, to_rig, Vector((0.0, 0.52, 0.058)), (0.026, 0.62, 0.026), BLUED, "gun")
    box(b, to_rig, Vector((0.0, 0.52, 0.073)), (0.008, 0.62, 0.004), BLUED_DARK, "gun")
    box(b, to_rig, Vector((0.0, 0.81, SIGHT_LINE - 0.001)), (0.009, 0.009, 0.009), BEAD, "gun")
    # The tube magazine and its cap.
    box(b, to_rig, Vector((0.0, 0.44, 0.028)), (0.024, 0.46, 0.024), BLUED, "gun")
    box(b, to_rig, Vector((0.0, 0.675, 0.028)), (0.028, 0.02, 0.028), BLUED_DARK, "gun")
    # The forend, grooved for the fingers.
    box(b, to_rig, Vector((0.0, FOREND, 0.024)), (0.052, 0.20, 0.046), WALNUT, "pump")
    for k in range(4):
        box(b, to_rig, Vector((0.0, FOREND - 0.06 + k * 0.04, 0.024)), (0.054, 0.008, 0.04), WALNUT_DARK, "pump")
    flash(b, to_rig, "flash", MUZZLE, 1.8)

    # A shell across the left palm, under the knuckles.
    l_wrist, l_fwd, l_back = left
    l_across = l_fwd.cross(l_back).normalized()
    at = l_wrist + l_fwd * 0.06 - l_back * 0.03
    cylinder(b, at, l_across, 0.0105, 0.055, HULL, "shell")
    cylinder(b, at - l_across * 0.031, l_across, 0.011, 0.01, BRASS, "shell")

    at_gun = lambda v: to_rig @ v  # noqa: E731
    bones = {
        "gun": (origin, origin + barrel * 0.08, hand),
        "pump": (at_gun(Vector((0.0, FOREND - 0.05, 0.024))), at_gun(Vector((0.0, FOREND + 0.05, 0.024))), "gun"),
        "flash": (at_gun(MUZZLE), at_gun(MUZZLE) + barrel * 0.04, "gun"),
        "shell": (at, at + l_across * 0.03, "hand.L"),
    }
    return bones, (origin, right, barrel, up)


def loading(gun, dx=0.0, dy=0.0, dz=0.0):
    """The left hand under the loading port, a shell's end to it, fingers
    pushing up and forward."""
    o = gun.translation
    barrel, up, right = gun.col[1].xyz, gun.col[2].xyz, gun.col[0].xyz
    port = o + barrel * PORT.y + up * PORT.z
    wrist = port - up * 0.075 - right * 0.035 - barrel * 0.07 + Vector((dx, dy, dz))
    fwd = barrel * 0.55 + up * 0.75 + right * 0.2
    back = -right * 0.8 - barrel * 0.2 + up * 0.1
    return wrist, fwd, back


def animate(rig, gun_rest, left_rest):
    """Every clip, posed and baked; their names."""
    r = poses.Rig(rig, gun_rest, left_rest)
    r.add_ik()
    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.mode_set(mode="POSE")
    finger = poses.trigger_finger(rig)

    work = bpy.data.actions.new("work")
    rig.animation_data.action = work

    def steady(frame, flash=0.0, pump=0.0, shell=0.0):
        poses.key_bone(rig, "flash", frame, scale=flash)
        poses.key_bone(rig, "pump", frame, loc=Vector((0, -pump, 0)))
        poses.key_bone(rig, "shell", frame, scale=shell)
        poses.key_bone(rig, "index1.R", frame, rot=finger)
        poses.key_bone(rig, "gun", frame, scale=1.0)
        poses.shoulder(rig, "upper_arm.L", frame, LEFT_SHOULDER)
        poses.shoulder(rig, "upper_arm.R", frame, RIGHT_SHOULDER)

    def hip(offset=Vector(), pitch=0.0, roll=0.0, yaw=0.0):
        return poses.gun_pose(offset, pitch, roll, yaw, grip=HIP)

    def aimed(offset=Vector(), pitch=0.0):
        return poses.aim_pose(SIGHT_LINE, AIM_DISTANCE, offset, pitch)

    def held(g, pump=0.0):
        return poses.forend(g, FOREND - pump)

    spans = {}
    base = hip()

    # Idle: frames 1..91, a slow breath.
    start = 1
    for f in range(start, start + 91, 5):
        t = (f - start) / 90.0
        g = hip(Vector((0, 0, 0.005 * math.sin(t * math.tau))), pitch=0.8 * math.sin(t * math.tau))
        r.key(f, g, held(g))
        steady(f)
    spans["Idle"] = (start, start + 90)

    def fire(start, pose, rest, kick):
        """A shot from `rest` (`pose` makes the frame from an offset and a
        pitch), kicking `kick` degrees, then the pump: frames start..+24."""
        for f, g, pump, fl in (
            (0, rest, 0.0, 1.0),
            (1, pose(Vector((0.0, -0.05, 0.02)), kick), 0.0, 0.0),
            (4, pose(Vector((0.0, -0.02, 0.008)), kick * 0.35), 0.0, 0.0),
            (7, rest, 0.0, 0.0),
            (10, rest, PUMP, 0.0),
            (13, rest, PUMP, 0.0),
            (17, rest, 0.0, 0.0),
            (24, rest, 0.0, 0.0),
        ):
            r.key(start + f, g, held(g, pump))
            steady(start + f, flash=fl, pump=pump)

    # Fire: frames 101..125.
    fire(101, lambda o, p: hip(o, pitch=p), base, 12.0)
    spans["Fire"] = (101, 125)

    # ReloadStart: frames 201..213, tipped over, the left hand down.
    tipped = hip(Vector((-0.03, 0.02, 0.05)), pitch=8.0, roll=-35.0)
    low = poses.away(Vector((-0.18, 0.08, -0.42)))
    start = 201
    for f, g, left, shell in ((0, base, held(base), 0.0), (5, tipped, poses.away(Vector((-0.15, 0.12, -0.32))), 0.0), (11, tipped, low, 0.0), (12, tipped, low, 1.0)):
        r.key(start + f, g, left)
        steady(start + f, shell=shell)
    spans["ReloadStart"] = (start, start + 12)

    # ReloadShell: frames 301..314: up to the port, in, and down again.
    start = 301
    for f, left, shell in (
        (0, low, 1.0),
        (5, loading(tipped, dz=-0.03), 1.0),
        (7, loading(tipped), 1.0),
        (8, loading(tipped), 0.0),
        (9, loading(tipped, dz=0.01), 0.0),
        (12, low, 0.0),
        (13, low, 1.0),
    ):
        r.key(start + f, tipped, left)
        steady(start + f, shell=shell)
    spans["ReloadShell"] = (start, start + 13)

    # ReloadEnd: frames 401..416, back on the forend and racked.
    start = 401
    for f, g, pump in ((0, tipped, None), (6, base, 0.0), (9, base, PUMP), (12, base, 0.0), (15, base, 0.0)):
        r.key(start + f, g, low if pump is None else held(g, pump))
        steady(start + f, pump=pump or 0.0)
    spans["ReloadEnd"] = (start, start + 15)

    # Bash: frames 501..519, the side of the gun shoved into a face.
    start = 501
    wind = hip(Vector((0.04, -0.08, 0.02)), roll=-10.0, yaw=25.0)
    shove = hip(Vector((-0.10, 0.12, 0.05)), pitch=5.0, yaw=-55.0)
    for f, g in ((0, base), (5, wind), (8, shove), (10, shove), (18, base)):
        r.key(start + f, g, held(g))
        steady(start + f)
    spans["Bash"] = (start, start + 18)

    # Aim: frames 601..691, shouldered and still (the game sways it).
    start = 601
    up = aimed()
    for f in (start, start + 90):
        r.key(f, up, held(up))
        steady(f)
    spans["Aim"] = (start, start + 90)

    # AimFire: frames 701..725.
    fire(701, lambda o, p: aimed(o, p), up, 8.0)
    spans["AimFire"] = (701, 725)

    # The flash pops for one frame, and a shell is there or it isn't: no
    # easing into or out of either.
    for fc in work.fcurves if hasattr(work, "fcurves") else []:
        if "flash" in fc.data_path or '"shell"' in fc.data_path:
            for kp in fc.keyframe_points:
                kp.interpolation = "CONSTANT"

    for name, (a, b) in spans.items():
        rig.animation_data.action = work
        poses.bake(rig, name, a, b)
    r.clear()
    bpy.data.actions.remove(work)
    bpy.ops.object.mode_set(mode="OBJECT")
    return list(spans)
