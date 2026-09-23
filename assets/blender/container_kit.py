"""What every container is built from (`containers.py`,
`indoor_containers.py`): a part with a colour per face, set shades of
wood and steel, a hinge to swing a door on, and a plain hull.
"""

import math
import random

import bmesh
import bpy
from mathutils import Matrix, Vector

# One source of luck for every container's shading: `named` seeds it by
# name, so a container shut and opened look alike.
rng = random.Random(7)

PLANK = (0.50, 0.38, 0.24)
PLANK_DARK = (0.34, 0.25, 0.16)
ROPE = (0.60, 0.52, 0.36)
LOCKER = (0.33, 0.38, 0.34)
LOCKER_DARK = (0.20, 0.23, 0.21)
SLOT = (0.08, 0.08, 0.08)
PAINT = (0.30, 0.37, 0.41)
RUST = (0.42, 0.24, 0.14)
GLASS = (0.10, 0.12, 0.13)
TYRE = (0.09, 0.09, 0.09)
HUB = (0.40, 0.40, 0.38)
CHROME = (0.55, 0.55, 0.52)
LAMP = (0.80, 0.78, 0.66)
TAIL = (0.55, 0.08, 0.06)
STEEL = (0.42, 0.43, 0.42)
STEEL_DARK = (0.24, 0.25, 0.25)
BRASS = (0.74, 0.58, 0.24)
KIT_RED = (0.66, 0.10, 0.08)
STRAW = (0.62, 0.53, 0.30)
HOLLOW = (0.06, 0.06, 0.06)


def swing(verts, hinge, axis, degrees):
    """Turn `verts` about the line through `hinge` along `axis`."""
    m = Matrix.Translation(hinge) @ Matrix.Rotation(math.radians(degrees), 4, axis) @ Matrix.Translation(-Vector(hinge))
    for v in verts:
        v.co = m @ v.co


def jitter(c, amount=0.06):
    k = 1.0 + rng.uniform(-amount, amount)
    return tuple(min(1.0, max(0.0, v * k)) for v in c)


class Part:
    def __init__(self, name):
        self.name = name
        self.bm = bmesh.new()
        self.col = self.bm.loops.layers.color.new("Col")

    def paint(self, faces, colour, shade=True):
        for f in faces:
            c = jitter(colour) if shade else colour
            for loop in f.loops:
                loop[self.col] = (*c, 1.0)

    def faces_of(self, verts):
        return sorted({f for v in verts for f in v.link_faces}, key=lambda f: f.index)

    def box(self, lo, hi, colour, turn=None):
        """An axis-lined box between two corners (turned about its middle
        by a matrix if given)."""
        c = Vector(((lo[0] + hi[0]) / 2, (lo[1] + hi[1]) / 2, (lo[2] + hi[2]) / 2))
        s = ((hi[0] - lo[0]) / 2, (hi[1] - lo[1]) / 2, (hi[2] - lo[2]) / 2)
        m = Matrix.Translation(c) @ (turn or Matrix.Identity(4)) @ Matrix.Diagonal((*s, 1.0))
        made = bmesh.ops.create_cube(self.bm, size=2.0, matrix=m)["verts"]
        self.paint(self.faces_of(made), colour)
        return made

    def wheel(self, centre, radius, width, colour, hub):
        m = Matrix.Translation(centre) @ Matrix.Rotation(math.pi / 2, 4, "X")
        made = bmesh.ops.create_cone(self.bm, cap_ends=True, segments=10, radius1=radius, radius2=radius, depth=width, matrix=m)["verts"]
        self.paint(self.faces_of(made), colour)
        face = centre[1] + (width / 2 if centre[1] > 0 else -width / 2)
        self.box((centre[0] - radius * 0.45, face - 0.012, centre[2] - radius * 0.45), (centre[0] + radius * 0.45, face + 0.012, centre[2] + radius * 0.45), hub)

    def finish(self):
        mesh = bpy.data.meshes.new(self.name)
        bmesh.ops.recalc_face_normals(self.bm, faces=self.bm.faces)
        self.bm.to_mesh(mesh)
        self.bm.free()
        attrs = mesh.color_attributes
        attrs.active_color = attrs["Col"]
        attrs.render_color_index = attrs.find("Col")
        mat = bpy.data.materials.get("Flat") or bpy.data.materials.new("Flat")
        mat.use_nodes = True
        nodes = mat.node_tree.nodes
        if "Col" not in [n.name for n in nodes]:
            vc = nodes.new("ShaderNodeVertexColor")
            vc.name = "Col"
            vc.layer_name = "Col"
            mat.node_tree.links.new(vc.outputs["Color"], nodes["Principled BSDF"].inputs["Base Color"])
        mesh.materials.append(mat)
        bpy.context.scene.collection.objects.link(bpy.data.objects.new(self.name, mesh))


def named(base, opened):
    """A part for `base`, shut or opened. Same seed, same colours: the two
    look alike but for what moved."""
    rng.seed(base)
    return Part(base + ("_Open" if opened else ""))


def hull(name, boxes, turn=None):
    """`boxes` (corner pairs) as one object, turned by `turn` if given."""
    p = Part(name + "_Hull")
    for lo, hi in boxes:
        p.box(lo, hi, STEEL_DARK)
    if turn is not None:
        for v in p.bm.verts:
            v.co = turn @ v.co
    p.finish()
