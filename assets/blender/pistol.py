"""The pistol, built into the arms' mesh in the right hand: a boxy low poly
service pistol (slide, frame, grip, trigger guard, magazine) and a muzzle
flash, each part on a bone of its own under the hand:

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
from mathutils import Matrix, Vector

SLIDE = (0.17, 0.17, 0.18)
FRAME = (0.10, 0.10, 0.10)
GRIP = (0.13, 0.12, 0.11)
SIGHT = (0.75, 0.73, 0.66)
FLASH = (1.0, 0.78, 0.35)

# Where things are, in the gun's frame, metres.
MUZZLE = Vector((0.0, 0.14, 0.07))
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
    box(b, to_rig, Vector((0.0, -0.052, 0.09)), (0.022, 0.008, 0.008), SLIDE, "slide")
    box(b, to_rig, Vector((0.0, 0.12, 0.09)), (0.006, 0.006, 0.009), SIGHT, "slide")
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
