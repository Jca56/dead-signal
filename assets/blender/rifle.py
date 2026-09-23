"""The hunting rifle, built into the arms' mesh: a walnut stock in the
right hand, a blued receiver and a long barrel, the forestock in the left
hand, a 4× scope on two rings over the action, a bolt with its handle out
to the right; a round the left hand carries to load it; the flash. Bones
for its moving parts:

    gun    the stock, receiver, barrel, scope; follows the right hand
    bolt   the bolt and its handle: turned up about the barrel, drawn back
    round  a round in the left hand, only there while loading
    flash  a star at the muzzle, the frame a shot goes off

And its clips (the shoulders rolled forward, as for the shotgun); the
right hand leaves the stock to work the bolt:

    Idle         3 s    held low at the hip, breathing
    Fire         1.1 s  the kick; the bolt up, back (the case out),
                        forward, down; the hand home
    ReloadStart  0.5 s  the bolt opened, the left hand down for a round
    ReloadShell  0.6 s  a round up into the open action, pressed in
                        (frame 10), and down for the next (it loops)
    ReloadEnd    0.55 s the left hand back under it, the bolt closed
    Bash         0.6 s  a shove with the side of the gun, landing on
                        frame 8
    Aim          3 s    shouldered, the scope to the eye
    AimFire      1.1 s  the kick and the bolt from there
"""

import math

import bpy
from mathutils import Matrix, Vector

import poses
from gun_kit import box, cylinder, flash, frame_matrix, gun_frame

WALNUT = (0.42, 0.25, 0.13)
WALNUT_DARK = (0.30, 0.18, 0.09)
BLUED = (0.12, 0.13, 0.15)
BLUED_DARK = (0.07, 0.07, 0.08)
STEEL = (0.50, 0.50, 0.48)
PAD = (0.06, 0.06, 0.06)
SCOPE = (0.09, 0.09, 0.10)
LENS = (0.22, 0.42, 0.52)
BRASS = (0.78, 0.60, 0.26)
COPPER = (0.66, 0.36, 0.20)

# Where things are, in the gun's frame, metres: the muzzle, the middle of
# the forestock (where the left hand holds), the open action (where a
# round goes in), the bolt's handle (its knob), how far the bolt draws
# back, and how high the scope's middle stands (the sight line).
MUZZLE = Vector((0.0, 0.965, 0.045))
FOREND = 0.38
PORT = Vector((0.0, 0.09, 0.06))
KNOB = Vector((0.06, 0.035, 0.03))
BOLT_BACK = 0.075
# (Turned this way about the barrel, the handle comes up.)
BOLT_UP = -60.0
BOLT_AXIS = Vector((0.0, 0.0, 0.05))
SIGHT_LINE = 0.105
# Where it's held at the hip, and how far ahead of the eye shouldered.
HIP = Vector((0.11, 0.04, -0.15))
AIM_DISTANCE = 0.17
LEFT_SHOULDER = Vector((0.06, 0.15, 0.02))
RIGHT_SHOULDER = Vector((0.0, 0.05, 0.0))


def build(b, wrist, fwd, back, across, hand, left=None):
    """Add the rifle to builder `b` in the right hand, and a round in the
    left (at `left`); the bones, and the gun's frame."""
    origin, right, barrel, up = gun_frame(wrist, fwd, back, across)
    to_rig = frame_matrix(origin, right, barrel, up)
    grip = math.radians(-20)
    # The stock: its wrist in the hand, the butt back past the eye.
    box(b, to_rig, Vector((0.0, -0.02, -0.01)), (0.034, 0.10, 0.05), WALNUT, "gun", grip)
    box(b, to_rig, Vector((0.0, -0.22, -0.035)), (0.04, 0.30, 0.10), WALNUT, "gun")
    box(b, to_rig, Vector((0.0, -0.375, -0.035)), (0.042, 0.02, 0.105), PAD, "gun")
    # The receiver, the trigger and its guard, the floorplate.
    box(b, to_rig, Vector((0.0, 0.10, 0.035)), (0.036, 0.20, 0.045), BLUED, "gun")
    box(b, to_rig, Vector((0.0, 0.035, -0.012)), (0.012, 0.07, 0.01), BLUED, "gun")
    box(b, to_rig, Vector((0.0, 0.03, 0.0)), (0.005, 0.008, 0.02), BLUED_DARK, "gun")
    box(b, to_rig, Vector((0.0, 0.10, 0.006)), (0.03, 0.08, 0.012), BLUED_DARK, "gun")
    # The forestock under the barrel, and the barrel to the muzzle.
    box(b, to_rig, Vector((0.0, FOREND - 0.02, 0.02)), (0.045, 0.34, 0.04), WALNUT, "gun")
    box(b, to_rig, Vector((0.0, FOREND + 0.13, 0.004)), (0.04, 0.02, 0.02), WALNUT_DARK, "gun")
    box(b, to_rig, Vector((0.0, 0.58, 0.045)), (0.02, 0.76, 0.02), BLUED, "gun")
    # The scope on its rings: the tube, the bell at its front, the
    # eyepiece at its back, a turret on top, the lens.
    for y in (0.035, 0.165):
        box(b, to_rig, Vector((0.0, y, 0.083)), (0.03, 0.02, 0.04), BLUED_DARK, "gun")
    box(b, to_rig, Vector((0.0, 0.10, SIGHT_LINE)), (0.03, 0.30, 0.03), SCOPE, "gun")
    box(b, to_rig, Vector((0.0, 0.27, SIGHT_LINE)), (0.044, 0.06, 0.044), SCOPE, "gun")
    box(b, to_rig, Vector((0.0, -0.075, SIGHT_LINE)), (0.038, 0.05, 0.038), SCOPE, "gun")
    box(b, to_rig, Vector((0.0, 0.10, SIGHT_LINE + 0.022)), (0.018, 0.018, 0.016), BLUED_DARK, "gun")
    box(b, to_rig, Vector((0.0, 0.301, SIGHT_LINE)), (0.036, 0.004, 0.036), LENS, "gun")
    # The bolt: its body along the action, the handle out to the right.
    box(b, to_rig, Vector((0.0, 0.075, 0.05)), (0.02, 0.11, 0.02), STEEL, "bolt")
    box(b, to_rig, Vector((0.035, 0.035, 0.042)), (0.05, 0.01, 0.01), STEEL, "bolt")
    box(b, to_rig, KNOB, (0.02, 0.02, 0.02), BLUED_DARK, "bolt")
    flash(b, to_rig, "flash", MUZZLE, 1.4)

    # A round across the left palm, under the knuckles.
    l_wrist, l_fwd, l_back = left
    l_across = l_fwd.cross(l_back).normalized()
    at = l_wrist + l_fwd * 0.06 - l_back * 0.03
    cylinder(b, at, l_across, 0.006, 0.05, BRASS, "round")
    cylinder(b, at + l_across * 0.033, l_across, 0.004, 0.016, COPPER, "round")

    at_gun = lambda v: to_rig @ v  # noqa: E731
    bones = {
        "gun": (origin, origin + barrel * 0.08, hand),
        "bolt": (at_gun(BOLT_AXIS + Vector((0.0, 0.02, 0.0))), at_gun(BOLT_AXIS + Vector((0.0, 0.12, 0.0))), "gun"),
        "flash": (at_gun(MUZZLE), at_gun(MUZZLE) + barrel * 0.04, "gun"),
        "round": (at, at + l_across * 0.03, "hand.L"),
    }
    return bones, (origin, right, barrel, up)


def on_bolt(gun, back=0.0, up=0.0):
    """Where the right hand holds as it works the bolt: gripping the knob
    as it would a grip, the bolt drawn `back` and turned `up` degrees."""
    turn = Matrix.Rotation(math.radians(up), 4, "Y")
    knob = BOLT_AXIS + turn.to_3x3() @ (KNOB - BOLT_AXIS) - Vector((0.0, back, 0.0))
    return gun @ Matrix.Translation(knob - Vector((0.0, 0.0, 0.02))) @ turn @ Matrix.Rotation(math.radians(-35), 4, "Y")


def loading(gun, dx=0.0, dy=0.0, dz=0.0):
    """The left hand over the open action, a round's tip to it, pressing
    down."""
    o = gun.translation
    barrel, up, right = gun.col[1].xyz, gun.col[2].xyz, gun.col[0].xyz
    port = o + barrel * PORT.y + up * PORT.z
    wrist = port + up * 0.06 - right * 0.06 - barrel * 0.06 + Vector((dx, dy, dz))
    fwd = barrel * 0.6 - up * 0.5 + right * 0.5
    back = up * 0.8 - right * 0.4
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

    def steady(frame, flash=0.0, back=0.0, up=0.0, round_=0.0):
        poses.key_bone(rig, "flash", frame, scale=flash)
        turn = Matrix.Rotation(math.radians(up), 4, "Y").to_quaternion()
        poses.key_bone(rig, "bolt", frame, loc=Vector((0, -back, 0)), rot=turn)
        poses.key_bone(rig, "round", frame, scale=round_)
        poses.key_bone(rig, "index1.R", frame, rot=finger)
        poses.key_bone(rig, "gun", frame, scale=1.0)
        poses.shoulder(rig, "upper_arm.L", frame, LEFT_SHOULDER)
        poses.shoulder(rig, "upper_arm.R", frame, RIGHT_SHOULDER)

    def hip(offset=Vector(), pitch=0.0, roll=0.0, yaw=0.0):
        return poses.gun_pose(offset, pitch, roll, yaw, grip=HIP)

    def aimed(offset=Vector(), pitch=0.0):
        return poses.aim_pose(SIGHT_LINE, AIM_DISTANCE, offset, pitch)

    def held(g):
        return poses.forend(g, FOREND)

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
        """A shot from `rest`, kicking `kick` degrees, then the bolt worked:
        frames start..+33."""
        keys = [
            (0, rest, None, 0.0, 0.0, 1.0),
            (1, pose(Vector((0.0, -0.06, 0.02)), kick), None, 0.0, 0.0, 0.0),
            (4, pose(Vector((0.0, -0.02, 0.008)), kick * 0.35), None, 0.0, 0.0, 0.0),
            (8, rest, None, 0.0, 0.0, 0.0),
            (11, rest, on_bolt(rest), 0.0, 0.0, 0.0),
            (14, rest, on_bolt(rest, up=BOLT_UP), 0.0, BOLT_UP, 0.0),
            (17, rest, on_bolt(rest, BOLT_BACK, BOLT_UP), BOLT_BACK, BOLT_UP, 0.0),
            (21, rest, on_bolt(rest, 0.0, BOLT_UP), 0.0, BOLT_UP, 0.0),
            (24, rest, on_bolt(rest), 0.0, 0.0, 0.0),
            (28, rest, None, 0.0, 0.0, 0.0),
            (33, rest, None, 0.0, 0.0, 0.0),
        ]
        for f, g, hand, back, up, fl in keys:
            r.key(start + f, g, held(g), hand)
            steady(start + f, flash=fl, back=back, up=up)

    # Fire: frames 101..134.
    fire(101, lambda o, p: hip(o, pitch=p), base, 14.0)
    spans["Fire"] = (101, 134)

    # ReloadStart: frames 201..216: the bolt opened, the left hand down.
    tipped = hip(Vector((-0.02, 0.02, 0.04)), pitch=6.0, roll=-20.0)
    low = poses.away(Vector((-0.18, 0.08, -0.42)))
    start = 201
    for f, g, hand, left, back, up, round_ in (
        (0, base, None, held(base), 0.0, 0.0, 0.0),
        (4, base, on_bolt(base), held(base), 0.0, 0.0, 0.0),
        (7, tipped, on_bolt(tipped, up=BOLT_UP), poses.away(Vector((-0.15, 0.12, -0.32))), 0.0, BOLT_UP, 0.0),
        (10, tipped, on_bolt(tipped, BOLT_BACK, BOLT_UP), low, BOLT_BACK, BOLT_UP, 0.0),
        (13, tipped, None, low, BOLT_BACK, BOLT_UP, 0.0),
        (14, tipped, None, low, BOLT_BACK, BOLT_UP, 1.0),
        (15, tipped, None, low, BOLT_BACK, BOLT_UP, 1.0),
    ):
        r.key(start + f, g, left, hand)
        steady(start + f, back=back, up=up, round_=round_)
    spans["ReloadStart"] = (start, start + 15)

    # ReloadShell: frames 301..319: up to the action, pressed in, down.
    start = 301
    for f, left, round_ in (
        (0, low, 1.0),
        (7, loading(tipped, dz=0.03), 1.0),
        (9, loading(tipped), 1.0),
        (10, loading(tipped), 0.0),
        (11, loading(tipped, dz=0.01), 0.0),
        (16, low, 0.0),
        (17, low, 1.0),
        (18, low, 1.0),
    ):
        r.key(start + f, tipped, left)
        steady(start + f, back=BOLT_BACK, up=BOLT_UP, round_=round_)
    spans["ReloadShell"] = (start, start + 18)

    # ReloadEnd: frames 401..417: the left hand back, the bolt closed.
    start = 401
    for f, g, hand, left, back, up in (
        (0, tipped, None, low, BOLT_BACK, BOLT_UP),
        (5, base, None, held(base), BOLT_BACK, BOLT_UP),
        (7, base, on_bolt(base, BOLT_BACK, BOLT_UP), held(base), BOLT_BACK, BOLT_UP),
        (10, base, on_bolt(base, 0.0, BOLT_UP), held(base), 0.0, BOLT_UP),
        (12, base, on_bolt(base), held(base), 0.0, 0.0),
        (15, base, None, held(base), 0.0, 0.0),
        (16, base, None, held(base), 0.0, 0.0),
    ):
        r.key(start + f, g, left, hand)
        steady(start + f, back=back, up=up)
    spans["ReloadEnd"] = (start, start + 16)

    # Bash: frames 501..519, the side of the gun shoved into a face.
    start = 501
    wind = hip(Vector((0.04, -0.08, 0.02)), roll=-10.0, yaw=25.0)
    shove = hip(Vector((-0.10, 0.12, 0.05)), pitch=5.0, yaw=-55.0)
    for f, g in ((0, base), (5, wind), (8, shove), (10, shove), (18, base)):
        r.key(start + f, g, held(g))
        steady(start + f)
    spans["Bash"] = (start, start + 18)

    # Aim: frames 601..691, shouldered and still.
    start = 601
    up = aimed()
    for f in (start, start + 90):
        r.key(f, up, held(up))
        steady(f)
    spans["Aim"] = (start, start + 90)

    # AimFire: frames 701..734.
    fire(701, lambda o, p: aimed(o, p), up, 8.0)
    spans["AimFire"] = (701, 734)

    # The flash pops for one frame, and a round is there or it isn't.
    for fc in work.fcurves if hasattr(work, "fcurves") else []:
        if "flash" in fc.data_path or '"round"' in fc.data_path:
            for kp in fc.keyframe_points:
                kp.interpolation = "CONSTANT"

    for name, (a, b) in spans.items():
        rig.animation_data.action = work
        poses.bake(rig, name, a, b)
    r.clear()
    bpy.data.actions.remove(work)
    bpy.ops.object.mode_set(mode="OBJECT")
    return list(spans)
