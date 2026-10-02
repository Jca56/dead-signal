"""A stim's injector, in the right hand: a chunky army auto-injector held
in the fist as the radio is, a glass vial of the stuff standing up out of
the top of it, its needle out of the bottom, and a trigger on the edge
under the thumb. The left forearm's brought up across the bottom of the
view, palm up, and it's jabbed into that. Its one clip:

    Jab  1.5 s  held up at the right; the left forearm comes up; the
                injector's lifted over it, and down into it (frame 14:
                the needle's in from then), the thumb on the trigger
                (frames 15 to 30); out again, and both hands away

Its frame is a gun's (`gun_kit.py`): its origin in the right fist, x
right, y along the knuckles, z up through the fist. It's the "gun" bone,
as a gun is. The vial is drawn unlit (an alpha of 0.5) and grey: the game
gives it the colour of whichever stim's in it.
"""

import bmesh
import bpy
from mathutils import Matrix, Vector

import poses
import radio
from gun_kit import box, frame_matrix, gun_frame

BODY = (0.62, 0.64, 0.58)
BODY_DARK = (0.30, 0.33, 0.28)
RUBBER = (0.05, 0.05, 0.05)
STEEL = (0.55, 0.56, 0.55)
BAND = (0.85, 0.42, 0.08)
TRIGGER = (0.55, 0.12, 0.08)
VIAL = (0.3, 0.3, 0.3)

# Held here at the ready: out at the right, as the radio is.
GRIP = Vector((0.20, 0.37, -0.13))
# The body, as the fist holds it (the radio's own, which the fist's made
# to fit): how deep (x), how wide (y), where its middle is, and from how
# far under the fist to how far over it (z).
DEEP, WIDE = radio.DEEP, radio.WIDE
X, Y = radio.X, radio.Y
FOOT, TOP = -0.058, 0.062
# How far under the fist the needle's point is.
POINT = 0.094


def tube(b, to_rig, low, high, radius, colour, alpha=1.0):
    """A round of eight sides up the injector's own z from `low` to `high`,
    about its middle line."""
    axis = (to_rig.to_3x3() @ Vector((0.0, 0.0, 1.0))).normalized()
    centre = to_rig @ Vector((X, Y, (low + high) / 2))
    m = Matrix.Translation(centre) @ axis.to_track_quat("Z", "Y").to_matrix().to_4x4()
    made = bmesh.ops.create_cone(b.bm, cap_ends=True, segments=8, radius1=radius, radius2=radius, depth=high - low, matrix=m)
    group = b.group("gun")
    for v in made["verts"]:
        v[b.deform][group] = 1.0
    for f in {f for v in made["verts"] for f in v.link_faces}:
        for loop in f.loops:
            loop[b.col] = (*colour, alpha)


def build(b, wrist, fwd, back, across, hand, left=None):
    """Add the injector to builder `b` in the right hand; its bone, and its
    frame."""
    origin, right, barrel, up = gun_frame(wrist, fwd, back, across)
    to_rig = frame_matrix(origin, right, barrel, up)

    def part(centre, size, colour):
        box(b, to_rig, Vector(centre), size, colour, "gun")

    # The body, a rubber foot and shoulder, a dark panel on its face and
    # an orange band round it.
    part((X, Y, (FOOT + TOP) / 2), (DEEP, WIDE, TOP - FOOT), BODY)
    part((X, Y, FOOT + 0.005), (DEEP + 0.004, WIDE + 0.004, 0.01), RUBBER)
    part((X, Y, TOP - 0.004), (DEEP + 0.004, WIDE + 0.004, 0.008), RUBBER)
    part((X - DEEP / 2 - 0.0005, Y, 0.012), (0.002, WIDE - 0.016, 0.05), BODY_DARK)
    part((X, Y, TOP - 0.018), (DEEP + 0.002, WIDE + 0.002, 0.008), BAND)
    # The trigger, on the edge nearest the eye, under the thumb.
    part((X - 0.002, Y - WIDE / 2 - 0.002, 0.03), (0.022, 0.005, 0.034), TRIGGER)
    # Out of the top: a steel collar, the vial, its cap.
    tube(b, to_rig, TOP, TOP + 0.012, 0.021, STEEL)
    tube(b, to_rig, TOP + 0.012, TOP + 0.074, 0.0165, VIAL, 0.5)
    tube(b, to_rig, TOP + 0.074, TOP + 0.086, 0.0195, STEEL)
    # Out of the bottom: the nozzle, a guard ring, the needle.
    tube(b, to_rig, FOOT - 0.02, FOOT, 0.012, STEEL)
    tube(b, to_rig, FOOT - 0.006, FOOT, 0.016, BAND)
    tube(b, to_rig, -POINT, FOOT - 0.02, 0.0028, STEEL)
    return {"gun": (origin, origin + barrel * 0.08, hand)}, (origin, right, barrel, up)


def held(origin, up, knuckles):
    """The injector's frame with its origin at `origin`, its vial along
    `up`, the fist's knuckles as near `knuckles` as that leaves them."""
    up = up.normalized()
    barrel = (knuckles - up * knuckles.dot(up)).normalized()
    return poses.frame_to(origin, barrel.cross(up).normalized(), barrel, up)


def animate(rig, gun_rest, left_rest):
    """The clip, posed and baked; its name."""
    ready = poses.gun_pose(Vector(), 6.0, -5.0, 34.0, grip=GRIP)
    # The left forearm, brought up across the bottom of the view, palm up
    # (the shoulder reaching in for it): its wrist, its knuckles' way, the
    # back of its hand.
    low = poses.away(Vector((-0.22, 0.10, -0.44)))
    shown = (Vector((-0.04, 0.50, -0.17)), Vector((0.75, 0.62, 0.22)), Vector((0.1, -0.35, -0.93)))
    reach = Vector((0.08, 0.16, 0.04))
    # Where the needle goes in (on the forearm, by the cuff), and the way
    # the injector lies then: leant to the right and back to the eye.
    into = Vector((-0.062, 0.426, -0.161))
    lies = Vector((0.505, -0.53, 0.68)).normalized()
    knuckles = Vector((-0.6, 0.8, 0.0))

    # (Lifted off it, it's drawn back to the right and away, not up at
    # the eye.)
    back = Vector((0.55, 0.35, 0.45)).normalized()

    def over(lifted):
        return held(into + lies * POINT + (back * lifted if lifted > 0.0 else lies * lifted), lies, knuckles)

    up, down = radio.thumb_pose(rig, gun_rest, radio.THUMB), radio.thumb_pose(rig, gun_rest, radio.THUMB_DOWN)

    r = poses.Rig(rig, gun_rest, left_rest)
    r.add_ik()
    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.mode_set(mode="POSE")
    work = bpy.data.actions.new("work")
    rig.animation_data.action = work

    # (frame, the injector, the left hand, the left shoulder reaching, the
    # thumb down on the trigger)
    keys = [
        (0, ready, low, 0.0, False),
        (7, over(0.05), shown, 1.0, False),
        (11, over(0.075), shown, 1.0, False),
        (14, over(-0.012), shown, 1.0, False),
        (15, over(-0.012), shown, 1.0, True),
        (22, over(-0.017), shown, 1.0, True),
        (30, over(-0.014), shown, 1.0, True),
        (31, over(-0.014), shown, 1.0, False),
        (35, over(0.055), shown, 1.0, False),
        (41, ready, low, 0.0, False),
        (45, ready, low, 0.0, False),
    ]
    start = 1
    for f, gun, left, reached, pressed in keys:
        r.key(start + f, gun, left)
        poses.key_bone(rig, "gun", start + f, scale=1.0)
        poses.shoulder(rig, "upper_arm.L", start + f, reach * reached)
        for bone, q in down if pressed else up:
            poses.key_bone(rig, bone, start + f, rot=q)
    rig.animation_data.action = work
    poses.bake(rig, "Jab", start, start + keys[-1][0])
    r.clear()
    bpy.data.actions.remove(work)
    bpy.ops.object.mode_set(mode="OBJECT")
    return ["Jab"]
