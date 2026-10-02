"""The handheld radio, in the right hand: an army walkie-talkie, a chunky
olive brick held the way a pistol's grip is, its face to the left of the
fist (so turned in, it faces the eye): a stubby rubber antenna and two
knobs on top, an amber display with the signal's bars on it, three keys,
a speaker's slats, and the talk button on the edge under the thumb. The
left hand's down out of sight, but to mark where a drop's to come down:
then it comes up with a lit flare, and throws it. Its clips:

    Idle     3 s     held up at the right, its face turned to the eye,
                     breathing
    Key      1 s     brought up and in towards the mouth, the thumb down
                     on the talk button (frame 9) and off it (frame 25),
                     and back
    FlareUp  0.27 s  the left hand up from out of sight, a lit flare in it
    Flare    3 s     the flare held up at the left, spitting, the radio
                     still up at the right
    Throw    0.43 s  the left hand back and up, then flung forward: the
                     flare leaves it on frame 6; and down out of sight

Its frame is a gun's (`gun_kit.py`): its origin in the right fist, x
right, y along the knuckles, z up through the fist. It's the "gun" bone,
as a gun is. The display and the lamp are drawn unlit (an alpha of 0.5),
so they glow in the dark. The flare is the "flare" bone, in the left
hand as a blade is in the right (scaled to nothing but while it's held),
and its flame the "spark" bone at its tip, which flickers.
"""

import math

import bmesh
import bpy
from mathutils import Matrix, Vector

import poses
from gun_kit import box, cylinder, frame_matrix, gun_frame

OLIVE = (0.27, 0.31, 0.17)
OLIVE_DARK = (0.17, 0.20, 0.11)
RUBBER = (0.045, 0.045, 0.045)
SLAT = (0.06, 0.07, 0.05)
KEY = (0.36, 0.37, 0.33)
TALK = (0.50, 0.13, 0.08)
AMBER = (0.95, 0.56, 0.10)
INK = (0.30, 0.15, 0.02)
LAMP = (0.25, 0.95, 0.30)
FLARE = (0.62, 0.10, 0.08)
CAP = (0.82, 0.80, 0.74)
HOT = (1.0, 0.92, 0.85)
BURN = (1.0, 0.22, 0.24)

# Held here: out at the right, well up, so its face clears the fist.
GRIP = Vector((0.225, 0.37, -0.125))
# The body, as the fist holds it (the palm on its back, the fingers round
# its far edge onto its face, the thumb up its near one): how deep (x) and
# how wide (y) and where its middle is, and from how far under the fist
# to how far over it (z).
DEEP, WIDE = 0.036, 0.068
X, Y = -0.005, 0.004
FOOT, TOP = -0.062, 0.105
# Its face (the side the eye sees), and its near edge.
FACE = X - DEEP / 2
NEAR = Y - WIDE / 2
# The thumb, up the near edge: its two joints' ways at rest on the talk
# button, and pressing it.
THUMB = (Vector((-0.47, -0.62, 0.62)), Vector((-0.23, 0.0, 0.97)))
THUMB_DOWN = (Vector((-0.47, -0.56, 0.68)), Vector((-0.2, 0.3, 0.93)))


def build(b, wrist, fwd, back, across, hand, left=None):
    """Add the radio to builder `b` in the right hand; its bone, and its
    frame."""
    origin, right, barrel, up = gun_frame(wrist, fwd, back, across)
    to_rig = frame_matrix(origin, right, barrel, up)

    def part(centre, size, colour, alpha=1.0):
        box(b, to_rig, Vector(centre), size, colour, "gun", alpha=alpha)

    def post(centre, radius, length, colour):
        cylinder(b, to_rig @ Vector(centre), to_rig.to_3x3() @ Vector((0.0, 0.0, 1.0)), radius, length, colour, "gun")

    # The body, a rubber foot and cap, the clip on its back.
    part((X, Y, (FOOT + TOP) / 2), (DEEP, WIDE, TOP - FOOT), OLIVE)
    part((X, Y, FOOT + 0.006), (DEEP + 0.004, WIDE + 0.004, 0.012), RUBBER)
    part((X, Y, TOP + 0.004), (DEEP + 0.004, WIDE + 0.004, 0.008), RUBBER)
    part((X + DEEP / 2 + 0.002, Y, 0.07), (0.005, 0.024, 0.06), RUBBER)
    # On top: the antenna (far from the eye), and the two knobs.
    post((X, Y + 0.022, TOP + 0.017), 0.0105, 0.018, RUBBER)
    post((X, Y + 0.022, TOP + 0.068), 0.0068, 0.085, RUBBER)
    post((X, Y - 0.003, TOP + 0.014), 0.006, 0.012, KEY)
    post((X, Y - 0.02, TOP + 0.016), 0.0085, 0.016, KEY)
    # The face: the display in its bezel, the bars of signal and three
    # lines of words on it, the lamp over it.
    part((FACE - 0.001, Y, 0.077), (0.004, WIDE - 0.01, 0.034), RUBBER)
    part((FACE - 0.0032, Y, 0.077), (0.001, WIDE - 0.018, 0.026), AMBER, 0.5)
    for k, tall in enumerate((0.005, 0.009, 0.014, 0.019)):
        part((FACE - 0.0039, Y + 0.02 - 0.0058 * k, 0.067 + tall / 2), (0.0006, 0.0036, tall), INK, 0.5)
    for z, wide in ((0.084, 0.018), (0.077, 0.013), (0.070, 0.016)):
        part((FACE - 0.0039, Y - 0.004 - wide / 2, z), (0.0006, wide, 0.003), INK, 0.5)
    part((FACE - 0.001, Y + 0.024, 0.0995), (0.003, 0.005, 0.005), LAMP, 0.5)
    # Three keys under it, then the speaker's slats down to the fist.
    for y in (-0.019, 0.0, 0.019):
        part((FACE - 0.0015, Y + y, 0.051), (0.004, 0.014, 0.008), KEY)
    part((FACE - 0.0003, Y, 0.0), (0.002, WIDE - 0.012, 0.084), OLIVE_DARK)
    for k in range(8):
        part((FACE - 0.001, Y, 0.036 - 0.0095 * k), (0.003, WIDE - 0.02, 0.0045), SLAT)
    # The talk button, on the edge nearest the eye, under the thumb.
    part((X - 0.002, NEAR - 0.002, 0.08), (0.024, 0.005, 0.04), TALK)
    bones = {"gun": (origin, origin + barrel * 0.08, hand)}
    bones.update(flare(b, *left))
    return bones, (origin, right, barrel, up)


def flare(b, wrist, fwd, back):
    """The flare in the left fist, as a blade's in the right: a red stick
    up through it, a pale cap at its foot, and at its tip the flame (a
    white-hot heart in a red tongue, unlit). Its bones."""
    up = fwd.cross(back).normalized()
    origin = wrist + fwd * 0.045 - back * 0.03
    to_rig = frame_matrix(origin, back.normalized(), fwd.normalized(), up)
    cylinder(b, to_rig @ Vector((0.0, 0.0, 0.035)), up, 0.012, 0.19, FLARE, "flare")
    cylinder(b, to_rig @ Vector((0.0, 0.0, -0.066)), up, 0.0135, 0.014, CAP, "flare")
    tip = to_rig @ Vector((0.0, 0.0, 0.13))
    # Its flame: a white-hot bead at the tip, and red tongues spat up and
    # out of it.
    group = b.group("spark")

    def burning(made, colour):
        for v in made["verts"]:
            v[b.deform][group] = 1.0
        for f in {f for v in made["verts"] for f in v.link_faces}:
            for loop in f.loops:
                loop[b.col] = (*colour, 0.5)

    burning(bmesh.ops.create_icosphere(b.bm, subdivisions=1, radius=0.013, matrix=Matrix.Translation(tip + up * 0.006)), HOT)
    for lean, turn, tall in ((0.0, 0.0, 0.06), (0.55, 0.0, 0.04), (0.55, 2.1, 0.045), (0.55, 4.2, 0.035)):
        way = (Matrix.Rotation(turn, 3, up) @ (Matrix.Rotation(lean, 3, fwd.normalized()) @ up)).normalized()
        m = Matrix.Translation(tip + way * (0.008 + tall / 2)) @ way.to_track_quat("Z", "Y").to_matrix().to_4x4()
        burning(bmesh.ops.create_cone(b.bm, cap_ends=True, segments=5, radius1=0.009, radius2=0.0, depth=tall, matrix=m), BURN)
    return {"flare": (origin, origin + up * 0.08, "hand.L"), "spark": (tip, tip + up * 0.05, "flare")}


def thumb_pose(rig, gun_rest, ways):
    """The pose of the right thumb's two bones that lays them along `ways`
    (in the radio's frame): each turned, in its own rest frame, the least
    it takes."""
    o, r, b, u = gun_rest
    to_rig = frame_matrix(o, r, b, u).to_3x3()
    turned = Matrix.Identity(3)
    out = []
    for name, way in zip(("thumb1.R", "thumb2.R"), ways):
        bone = rig.data.bones[name]
        rest = bone.matrix_local.to_3x3()
        lies = (bone.tail_local - bone.head_local).normalized()
        # (What its parent's turn has done already is undone first.)
        turn = lies.rotation_difference(turned.inverted() @ (to_rig @ way).normalized()).to_matrix()
        out.append((name, (rest.inverted() @ turn @ rest).to_quaternion()))
        turned = turned @ turn
    return out


def animate(rig, gun_rest, left_rest):
    """Every clip, posed and baked; their names."""

    def at(offset=Vector(), pitch=0.0, roll=0.0, yaw=0.0):
        return poses.gun_pose(offset, pitch, roll, yaw, grip=GRIP)

    # Turned in so its face is to the eye, its top tipped back a little.
    rest = at(pitch=6.0, roll=-5.0, yaw=44.0)
    spoken = at(Vector((-0.07, -0.07, 0.045)), pitch=20.0, roll=-16.0, yaw=58.0)
    held = spoken @ Matrix.Translation(Vector((0.0, 0.0, 0.004)))
    # The left hand (wrist, knuckles' way, the back of it): out of sight;
    # up with the flare, its tip leaning out; wound back; flung; and
    # followed through.
    low = poses.away(Vector((-0.22, 0.10, -0.44)))
    lit = (Vector((-0.21, 0.37, -0.20)), Vector((0.0, 1.0, -0.15)), Vector((-1.0, 0.0, -0.2)))
    wound = (Vector((-0.24, 0.22, -0.07)), Vector((0.05, 0.50, 0.90)), Vector((-1.0, 0.0, 0.25)))
    flung = (Vector((-0.08, 0.48, -0.05)), Vector((0.20, 1.0, -0.45)), Vector((-1.0, 0.2, 0.0)))
    spent = (Vector((-0.09, 0.45, -0.26)), Vector((0.30, 0.8, -0.70)), Vector((-0.9, 0.1, -0.3)))
    up, down = thumb_pose(rig, gun_rest, THUMB), thumb_pose(rig, gun_rest, THUMB_DOWN)

    r = poses.Rig(rig, gun_rest, left_rest)
    r.add_ik()
    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.mode_set(mode="POSE")
    work = bpy.data.actions.new("work")
    rig.animation_data.action = work

    def key(frame, gun, left, pressed=False, flare=0.0, spark=1.0):
        """Both hands for a frame: the radio at `gun` (the thumb down on
        the talk button if `pressed`), the left hand at `left` with the
        flare `flare` of its size, its flame `spark` of its own."""
        r.key(frame, gun, left)
        poses.key_bone(rig, "gun", frame, scale=1.0)
        poses.key_bone(rig, "flare", frame, scale=flare)
        poses.key_bone(rig, "spark", frame, scale=spark)
        for bone, q in down if pressed else up:
            poses.key_bone(rig, bone, frame, rot=q)

    def breath(f, start):
        t = (f - start) / 90.0
        return Matrix.Translation(Vector((0, 0, 0.005 * math.sin(t * math.tau)))) @ rest

    spans = {}
    # Idle: frames 1..91, a slow breath.
    for f in range(1, 92, 5):
        key(f, breath(f, 1), low)
    spans["Idle"] = (1, 91)
    # Flare: frames 501..591, the same breath, the flare swaying and its
    # flame spitting (the same at its last frame as its first).
    for f in range(501, 592, 3):
        t = (f - 501) / 90.0
        sway = Vector((0.004 * math.sin(t * math.tau * 2), 0.0, 0.006 * math.sin(t * math.tau)))
        spit = 1.0 + 0.3 * math.sin(t * math.tau * 7) + 0.15 * math.sin(t * math.tau * 13)
        key(f, breath(f, 501), (lit[0] + sway, lit[1], lit[2]), flare=1.0, spark=spit)
    spans["Flare"] = (501, 591)
    clips = {
        "Key": [(0, rest, low, False, 0.0), (6, spoken, low, False, 0.0), (9, spoken, low, True, 0.0), (22, held, low, True, 0.0), (25, held, low, False, 0.0), (30, rest, low, False, 0.0)],
        "FlareUp": [(0, rest, low, False, 1.0), (8, rest, lit, False, 1.0)],
        "Throw": [(0, rest, lit, False, 1.0), (3, rest, wound, False, 1.0), (5, rest, flung, False, 1.0), (6, rest, flung, False, 0.0), (9, rest, spent, False, 0.0), (13, rest, low, False, 0.0)],
    }
    for k, (name, keys) in enumerate(clips.items()):
        start = 101 + 100 * k
        for f, gun, left, pressed, flare in keys:
            key(start + f, gun, left, pressed, flare)
        spans[name] = (start, start + keys[-1][0])

    for name, (a, b) in spans.items():
        rig.animation_data.action = work
        poses.bake(rig, name, a, b)
    r.clear()
    bpy.data.actions.remove(work)
    bpy.ops.object.mode_set(mode="OBJECT")
    return list(spans)
