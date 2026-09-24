"""What every carried thing is built from (`items.py`, `items_guns.py`):
a part (a bmesh with a colour per face) made of boxes, rolls, cans and
hoops, and the colours they're painted.
"""

import math

import bmesh
import bpy
from mathutils import Matrix

GAUZE = (0.86, 0.84, 0.77)
GAUZE_SHADE = (0.74, 0.72, 0.66)
KIT_RED = (0.66, 0.10, 0.08)
KIT_DARK = (0.36, 0.06, 0.05)
WHITE = (0.92, 0.91, 0.87)
HANDLE = (0.14, 0.13, 0.12)
GOLD = (0.85, 0.63, 0.20)
GOLD_DARK = (0.62, 0.43, 0.12)
SILVER = (0.74, 0.74, 0.71)
LEATHER = (0.33, 0.21, 0.12)
TIN = (0.62, 0.62, 0.58)
TIN_LABEL = (0.66, 0.30, 0.14)
BOTTLE = (0.52, 0.66, 0.72)
CAP_BLUE = (0.18, 0.30, 0.55)
PILL = (0.80, 0.45, 0.12)
BILL = (0.46, 0.56, 0.38)
BILL_DARK = (0.34, 0.43, 0.28)
BAND = (0.86, 0.80, 0.62)
BLACK = (0.11, 0.11, 0.11)
PLASTIC = (0.20, 0.21, 0.20)
FUEL_RED = (0.58, 0.12, 0.08)
GEM = (0.70, 0.08, 0.10)
TAG = (0.80, 0.62, 0.14)
CARTON = (0.33, 0.34, 0.21)
CARTON_DARK = (0.24, 0.25, 0.15)
LABEL = (0.78, 0.70, 0.48)
BRASS = (0.78, 0.60, 0.26)
LEAD = (0.45, 0.42, 0.40)

WALNUT = (0.40, 0.23, 0.12)
BLUED = (0.12, 0.13, 0.15)
SHELL_RED = (0.62, 0.10, 0.08)
BEAD = (0.95, 0.45, 0.08)
SCOPE_BLACK = (0.09, 0.09, 0.10)
LENS = (0.22, 0.42, 0.52)
COPPER = (0.66, 0.36, 0.20)

class Part:
    def __init__(self, name):
        self.name = name
        self.bm = bmesh.new()
        self.col = self.bm.loops.layers.color.new("Col")

    def paint(self, verts, colour, alt=None):
        faces = sorted({f for v in verts for f in v.link_faces}, key=lambda f: f.calc_center_median().to_tuple())
        for i, f in enumerate(faces):
            c = alt if alt and i % 2 else colour
            for loop in f.loops:
                loop[self.col] = (*c, 1.0)

    def box(self, centre, size, colour, turn=0.0):
        m = Matrix.Translation(centre) @ Matrix.Rotation(turn, 4, "Z") @ Matrix.Diagonal((size[0] / 2, size[1] / 2, size[2] / 2, 1.0))
        self.paint(bmesh.ops.create_cube(self.bm, size=2.0, matrix=m)["verts"], colour)

    def roll(self, centre, radius, length, colour, alt):
        """A cylinder lying along X."""
        m = Matrix.Translation(centre) @ Matrix.Rotation(math.pi / 2, 4, "Y")
        made = bmesh.ops.create_cone(self.bm, cap_ends=True, segments=10, radius1=radius, radius2=radius, depth=length, matrix=m)
        self.paint(made["verts"], colour, alt)

    def can(self, centre, radius, height, colour, segments=10):
        """A cylinder standing up Z from `centre`."""
        m = Matrix.Translation((centre[0], centre[1], centre[2] + height / 2))
        made = bmesh.ops.create_cone(self.bm, cap_ends=True, segments=segments, radius1=radius, radius2=radius, depth=height, matrix=m)
        self.paint(made["verts"], colour)

    def hoop(self, centre, big, small, colour, segments=12, turn=None):
        """A ring (a torus of boxes round Z, or turned by `turn`)."""
        for k in range(segments):
            a = k / segments * math.tau
            at = Matrix.Translation(centre) @ (turn or Matrix.Identity(4)) @ Matrix.Translation((math.cos(a) * big, math.sin(a) * big, 0.0))
            m = at @ Matrix.Rotation(a, 4, "Z") @ Matrix.Diagonal((small, big * math.pi / segments * 1.15, small, 1.0))
            self.paint(bmesh.ops.create_cube(self.bm, size=2.0, matrix=m)["verts"], colour)

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
