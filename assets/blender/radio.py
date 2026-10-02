"""The handheld radio, in the right hand: an army walkie-talkie, a chunky
olive brick held the way a pistol's grip is, its face to the left of the
fist (so turned in, it faces the eye): a stubby rubber antenna and two
knobs on top, an amber display with the signal's bars on it, three keys,
a speaker's slats, and the talk button on the edge under the thumb. The
left hand's down out of sight. Its clips:

    Idle  3 s    held up at the right, its face turned to the eye,
                 breathing
    Key   1 s    brought up and in towards the mouth, the thumb down on
                 the talk button (frame 9) and off it (frame 25), and back

Its frame is a gun's (`gun_kit.py`): its origin in the right fist, x
right, y along the knuckles, z up through the fist. It's the "gun" bone,
as a gun is. The display and the lamp are drawn unlit (an alpha of 0.5),
so they glow in the dark.
"""

from mathutils import Matrix, Vector

import melee_kit
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
    return {"gun": (origin, origin + barrel * 0.08, hand)}, (origin, right, barrel, up)


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
    def at(offset=Vector(), pitch=0.0, roll=0.0, yaw=0.0):
        return poses.gun_pose(offset, pitch, roll, yaw, grip=GRIP)

    # Turned in so its face is to the eye, its top tipped back a little.
    rest = at(pitch=6.0, roll=-5.0, yaw=44.0)
    spoken = at(Vector((-0.07, -0.07, 0.045)), pitch=20.0, roll=-16.0, yaw=58.0)
    held = spoken @ Matrix.Translation(Vector((0.0, 0.0, 0.004)))
    talk = [(0, rest), (6, spoken), (9, spoken), (22, held), (25, held), (30, rest)]
    low = poses.away(Vector((-0.22, 0.10, -0.44)))

    up, down = thumb_pose(rig, gun_rest, THUMB), thumb_pose(rig, gun_rest, THUMB_DOWN)

    def thumb(name, frame, f):
        """The thumb up the near edge, on the talk button; down on it
        while it's keyed."""
        for bone, q in down if name == "Key" and 9 <= f <= 22 else up:
            poses.key_bone(rig, bone, frame, rot=q)

    return melee_kit.animate(rig, gun_rest, left_rest, rest, {"Key": talk}, left=lambda g: low, also=thumb)
