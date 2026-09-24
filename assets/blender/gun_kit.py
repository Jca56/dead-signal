"""What every gun in the hands is built from (`pistol.py`, `shotgun.py`,
`rifle.py`): its frame in a right hand, boxes in that frame weighted to a
bone, cylinders (rounds, shells) weighted to one, and the muzzle flash. A gun's frame: x right, y down the barrel, z up, its origin
where the grip sits in the palm.
"""

import math

import bmesh
from mathutils import Matrix, Vector

FLASH = (1.0, 0.78, 0.35)


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


def box(b, to_rig, centre, size, colour, bone, tilt=0.0, alpha=1.0):
    """A box in the gun's frame (turned `tilt` about its x axis), weighted
    whole to `bone`; an `alpha` of 0.5 draws it unlit (a glowing dot)."""
    m = to_rig @ Matrix.Translation(centre) @ Matrix.Rotation(tilt, 4, "X") @ Matrix.Diagonal((size[0] / 2, size[1] / 2, size[2] / 2, 1.0))
    made = bmesh.ops.create_cube(b.bm, size=2.0, matrix=m)
    group = b.group(bone)
    for v in made["verts"]:
        v[b.deform][group] = 1.0
    faces = sorted({f for v in made["verts"] for f in v.link_faces}, key=lambda f: f.index if f.index >= 0 else 0)
    for f in faces:
        for loop in f.loops:
            loop[b.col] = (*colour, alpha)


def flash(b, to_rig, bone, muzzle, size=1.0):
    """A six-pointed star of crossed diamonds just out of `muzzle` (in the
    gun's frame), seen from either side, `size` times a pistol's; its
    colour's alpha of 0.5 tells the game to draw it unlit."""
    group = b.group(bone)
    centre = to_rig @ (muzzle + Vector((0.0, 0.035 * size, 0.0)))
    axis = (to_rig.to_3x3() @ Vector((0.0, 1.0, 0.0))).normalized()
    side = (to_rig.to_3x3() @ Vector((1.0, 0.0, 0.0))).normalized()
    for k in range(3):
        across = Matrix.Rotation(k * math.pi / 3, 3, axis) @ side
        tip_a, tip_b = centre + across * 0.045 * size, centre - across * 0.045 * size
        back, front = centre - axis * 0.02 * size, centre + axis * 0.07 * size
        # Front and back faces each on their own corners (one set of
        # corners can't carry two faces).
        for order in ((back, tip_a, front, tip_b), (tip_b, front, tip_a, back)):
            verts = [b.bm.verts.new(p) for p in order]
            for v in verts:
                v[b.deform][group] = 1.0
            f = b.bm.faces.new(verts)
            for loop in f.loops:
                loop[b.col] = (*FLASH, 0.5)


def cylinder(b, centre, axis, radius, length, colour, bone):
    """A cylinder of eight sides along `axis` about `centre` (rig space),
    weighted whole to `bone`."""
    m = Matrix.Translation(centre) @ axis.to_track_quat("Z", "Y").to_matrix().to_4x4()
    made = bmesh.ops.create_cone(b.bm, cap_ends=True, segments=8, radius1=radius, radius2=radius, depth=length, matrix=m)
    group = b.group(bone)
    for v in made["verts"]:
        v[b.deform][group] = 1.0
    for f in {f for v in made["verts"] for f in v.link_faces}:
        for loop in f.loops:
            loop[b.col] = (*colour, 1.0)
