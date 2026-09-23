"""The pistol, built into the arms' mesh in the right hand: a boxy low poly
service pistol (slide, frame, grip, trigger guard, magazine) and a muzzle
flash, each part on a bone of its own under the hand; and its clips.

    Idle    3 s   a two-handed grip, breathing
    Fire    0.2 s the kick, the slide back and home, the flash
    Reload  1.4 s tilt, mag out, left hand away and back with a new one,
                  seated, slide racked
    Bash    0.5 s a pistol-whip: wind up, strike (frame 9), recover
    Aim     3 s   raised to the eye, the sights on the middle of the view,
                  held dead still
    AimFire 0.2 s the kick and the flash from there

The parts' bones:

    gun    the frame and grip; follows the hand
    slide  racks back along the barrel when it fires
    mag    drops out of the grip along it on a reload
    flash  a star at the muzzle, scaled to nothing except the frame a shot
           goes off; its colour's alpha of 0.5 tells the game to draw it
           unlit

The gun's own frame (x right, y down the barrel, z up) sits in the palm:
the grip runs up towards the index finger, the barrel along the knuckles.
"""

import math

import bmesh
import bpy
from mathutils import Matrix, Vector

import poses

SLIDE = (0.17, 0.17, 0.18)
FRAME = (0.10, 0.10, 0.10)
GRIP = (0.13, 0.12, 0.11)
# The sights, easy to see: white dots either side of the rear notch, a
# bright front post.
DOT = (0.92, 0.91, 0.87)
POST = (0.95, 0.45, 0.08)
FLASH = (1.0, 0.78, 0.35)

# Where things are, in the gun's frame, metres: the muzzle, and how high
# the tops of the sights stand over the origin (the line they make runs
# along the barrel).
MUZZLE = Vector((0.0, 0.14, 0.07))
SIGHT_LINE = 0.094
# How far ahead of the eye the gun is held up to it (its origin).
AIM_DISTANCE = 0.22
GRIP_TILT = math.radians(-15)  # the grip's top leans forward


def gun_frame(wrist, fwd, back, across):
    """The gun in a right hand at rest: its grip in the palm, up towards
    the index finger, the barrel along the knuckles. (origin, right, barrel,
    up), all in the rig's space."""
    up = across.normalized()
    barrel = (fwd - up * fwd.dot(up)).normalized()
    right = barrel.cross(up).normalized()
    origin = wrist + fwd * 0.045 - back * 0.03
    return origin, right, barrel, up


def frame_matrix(origin, right, barrel, up):
    m = Matrix.Identity(4)
    for row in range(3):
        m[row][0], m[row][1], m[row][2], m[row][3] = right[row], barrel[row], up[row], origin[row]
    return m


def box(b, to_rig, centre, size, colour, bone, tilt=0.0):
    """A box in the gun's frame (turned `tilt` about its x axis), weighted
    whole to `bone`."""
    m = to_rig @ Matrix.Translation(centre) @ Matrix.Rotation(tilt, 4, "X") @ Matrix.Diagonal((size[0] / 2, size[1] / 2, size[2] / 2, 1.0))
    made = bmesh.ops.create_cube(b.bm, size=2.0, matrix=m)
    group = b.group(bone)
    for v in made["verts"]:
        v[b.deform][group] = 1.0
    faces = sorted({f for v in made["verts"] for f in v.link_faces}, key=lambda f: f.index if f.index >= 0 else 0)
    for f in faces:
        for loop in f.loops:
            loop[b.col] = (*colour, 1.0)


def flash(b, to_rig, bone):
    """A six-pointed star of crossed diamonds, seen from either side."""
    group = b.group(bone)
    centre = to_rig @ (MUZZLE + Vector((0.0, 0.035, 0.0)))
    axis = (to_rig.to_3x3() @ Vector((0.0, 1.0, 0.0))).normalized()
    side = (to_rig.to_3x3() @ Vector((1.0, 0.0, 0.0))).normalized()
    for k in range(3):
        across = Matrix.Rotation(k * math.pi / 3, 3, axis) @ side
        tip_a, tip_b = centre + across * 0.045, centre - across * 0.045
        back, front = centre - axis * 0.02, centre + axis * 0.07
        # Front and back faces each on their own corners (one set of
        # corners can't carry two faces).
        for order in ((back, tip_a, front, tip_b), (tip_b, front, tip_a, back)):
            verts = [b.bm.verts.new(p) for p in order]
            for v in verts:
                v[b.deform][group] = 1.0
            f = b.bm.faces.new(verts)
            for loop in f.loops:
                loop[b.col] = (*FLASH, 0.5)


def build(b, wrist, fwd, back, across, hand):
    """Add the pistol to builder `b` in the right hand; its bones, as
    {name: (head, tail, parent)}, and its frame."""
    origin, right, barrel, up = gun_frame(wrist, fwd, back, across)
    to_rig = frame_matrix(origin, right, barrel, up)
    # Frame and grip.
    box(b, to_rig, Vector((0.0, -0.012, -0.012)), (0.028, 0.046, 0.115), GRIP, "gun", GRIP_TILT)
    box(b, to_rig, Vector((0.0, 0.045, 0.045)), (0.026, 0.12, 0.022), FRAME, "gun")
    box(b, to_rig, Vector((0.0, 0.03, 0.013)), (0.009, 0.05, 0.006), FRAME, "gun")
    box(b, to_rig, Vector((0.0, 0.054, 0.024)), (0.009, 0.006, 0.02), FRAME, "gun")
    box(b, to_rig, Vector((0.0, 0.022, 0.024)), (0.006, 0.006, 0.016), FRAME, "gun")
    box(b, to_rig, Vector((0.0, 0.133, 0.066)), (0.012, 0.008, 0.012), FRAME, "gun")
    # The slide and its sights.
    box(b, to_rig, Vector((0.0, 0.035, 0.07)), (0.030, 0.19, 0.032), SLIDE, "slide")
    # The rear sight: two ears and the notch between, a dot on each ear's
    # back; the front post, seen through the notch. Their tops make the
    # sight line.
    top, height = SIGHT_LINE, 0.010
    for side in (-1.0, 1.0):
        box(b, to_rig, Vector((side * 0.0075, -0.052, top - height / 2)), (0.007, 0.008, height), SLIDE, "slide")
        box(b, to_rig, Vector((side * 0.0075, -0.0566, top - 0.0045)), (0.0035, 0.0012, 0.0035), DOT, "slide")
    box(b, to_rig, Vector((0.0, 0.12, top - height / 2)), (0.006, 0.006, height), POST, "slide")
    # The magazine: a body inside the grip and the base plate under it.
    box(b, to_rig, Vector((0.0, -0.013, -0.02)), (0.022, 0.034, 0.1), FRAME, "mag", GRIP_TILT)
    box(b, to_rig, Vector((0.0, -0.03, -0.074)), (0.031, 0.05, 0.01), GRIP, "mag", GRIP_TILT)
    flash(b, to_rig, "flash")

    grip_up = (to_rig.to_3x3() @ (Matrix.Rotation(GRIP_TILT, 3, "X") @ Vector((0.0, 0.0, 1.0)))).normalized()
    at = lambda v: to_rig @ v  # noqa: E731
    bones = {
        "gun": (origin, origin + barrel * 0.08, hand),
        "slide": (at(Vector((0.0, -0.06, 0.07))), at(Vector((0.0, 0.04, 0.07))), "gun"),
        "mag": (at(Vector((0.0, -0.03, -0.075))), at(Vector((0.0, -0.03, -0.075))) + grip_up * 0.08, "gun"),
        "flash": (at(MUZZLE), at(MUZZLE) + barrel * 0.04, "gun"),
    }
    return bones, (origin, right, barrel, up)


def animate(rig, gun_rest, left_rest):
    """Every clip, posed and baked; their names."""
    r = poses.Rig(rig, gun_rest, left_rest)
    r.add_ik()
    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.mode_set(mode="POSE")
    finger = poses.trigger_finger(rig)

    # One working action holds the directly keyed bones for every stretch.
    work = bpy.data.actions.new("work")
    rig.animation_data.action = work

    def steady(frame, flash=0.0, slide=0.0, mag=0.0, mag_scale=1.0):
        poses.key_bone(rig, "flash", frame, scale=flash)
        poses.key_bone(rig, "slide", frame, loc=Vector((0, slide, 0)))
        poses.key_bone(rig, "mag", frame, loc=Vector((0, mag, 0)), scale=mag_scale)
        poses.key_bone(rig, "index1.R", frame, rot=finger)
        poses.key_bone(rig, "gun", frame, scale=1.0)

    spans = {}

    # Idle: frames 1..91, a slow breath.
    start = 1
    for f in range(start, start + 91, 5):
        t = (f - start) / 90.0
        lift = Vector((0, 0, 0.004 * math.sin(t * math.tau)))
        g = poses.gun_pose(lift, pitch=0.6 * math.sin(t * math.tau))
        r.key(f, g, poses.support(g))
        steady(f)
    spans["Idle"] = (start, start + 90)

    # Fire: frames 101..107.
    start = 101
    base = poses.gun_pose()
    kick = poses.gun_pose(Vector((0.0, -0.025, 0.012)), pitch=7.0)
    for f, g, slide, fl in ((0, base, 0.0, 1.0), (1, kick, -0.038, 0.0), (3, poses.gun_pose(Vector((0, -0.01, 0.004)), pitch=2.0), 0.0, 0.0), (6, base, 0.0, 0.0)):
        r.key(start + f, g, poses.support(g))
        steady(start + f, flash=fl, slide=slide)
    spans["Fire"] = (start, start + 6)

    # Reload: frames 201..243.
    start = 201
    tilt = poses.gun_pose(Vector((0.01, -0.02, 0.025)), pitch=10.0, roll=25.0)
    low = Vector((-0.20, 0.10, -0.46))
    frames = [
        (0, base, poses.support(base), 0.0, 1.0, 0.0),
        (5, tilt, poses.away(Vector((-0.12, 0.18, -0.30))), 0.0, 1.0, 0.0),
        (9, tilt, poses.away(low), -0.18, 1.0, 0.0),
        (12, tilt, poses.away(low), -0.40, 0.0, 0.0),
        (18, tilt, poses.away(low), -0.40, 0.0, 0.0),
        (20, tilt, poses.away(low), -0.16, 1.0, 0.0),
        (25, tilt, poses.support(tilt, 0.0, 0.0, -0.06), -0.12, 1.0, 0.0),
        (29, tilt, poses.support(tilt, 0.0, 0.0, -0.02), 0.0, 1.0, 0.0),
        (32, tilt, poses.support(tilt, 0.02, -0.06, 0.07), 0.0, 1.0, 0.0),
        (34, tilt, poses.support(tilt, 0.02, -0.12, 0.07), 0.0, 1.0, -0.04),
        (35, tilt, poses.support(tilt, 0.02, -0.12, 0.05), 0.0, 1.0, 0.0),
        (42, base, poses.support(base), 0.0, 1.0, 0.0),
    ]
    for f, g, left, mag, mag_scale, slide in frames:
        r.key(start + f, g, left)
        steady(start + f, slide=slide, mag=mag, mag_scale=mag_scale)
    spans["Reload"] = (start, start + 42)

    # Bash: frames 301..316, the strike landing on 309.
    start = 301
    wind = poses.gun_pose(Vector((0.05, -0.06, 0.08)), pitch=35.0, roll=20.0, yaw=-10.0)
    strike = poses.gun_pose(Vector((-0.06, 0.12, -0.03)), pitch=-55.0, roll=-10.0, yaw=15.0)
    guard = poses.away(Vector((-0.20, 0.16, -0.34)))
    for f, g, left in ((0, base, poses.support(base)), (4, wind, guard), (8, strike, guard), (10, strike, guard), (15, base, poses.support(base))):
        r.key(start + f, g, left)
        steady(start + f)
    spans["Bash"] = (start, start + 15)

    # Aim: frames 401..491, dead still (the game sways it about the eye).
    start = 401
    aimed = poses.aim_pose(SIGHT_LINE, AIM_DISTANCE)
    for f in (start, start + 90):
        r.key(f, aimed, poses.support(aimed))
        steady(f)
    spans["Aim"] = (start, start + 90)

    # AimFire: frames 501..507, a sharper kick back into the eye.
    start = 501
    for f, g, slide, fl in (
        (0, aimed, 0.0, 1.0),
        (1, poses.aim_pose(SIGHT_LINE, AIM_DISTANCE, Vector((0.0, -0.03, 0.008)), pitch=6.0), -0.038, 0.0),
        (3, poses.aim_pose(SIGHT_LINE, AIM_DISTANCE, Vector((0.0, -0.01, 0.002)), pitch=1.5), 0.0, 0.0),
        (6, aimed, 0.0, 0.0),
    ):
        r.key(start + f, g, poses.support(g))
        steady(start + f, flash=fl, slide=slide)
    spans["AimFire"] = (start, start + 6)

    # The flash pops for one frame: no easing into or out of it.
    for fc in work.fcurves if hasattr(work, "fcurves") else []:
        if "flash" in fc.data_path:
            for kp in fc.keyframe_points:
                kp.interpolation = "CONSTANT"

    for name, (a, b) in spans.items():
        rig.animation_data.action = work
        poses.bake(rig, name, a, b)
    r.clear()
    bpy.data.actions.remove(work)
    bpy.ops.object.mode_set(mode="OBJECT")
    return list(spans)
