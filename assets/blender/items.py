"""Things to pick up: a bandage roll, a medkit and a box of 9mm rounds,
each its own object (ITEM_Bandage, ITEM_Medkit, ITEM_Ammo) sitting on its
origin, for the game to set down wherever it likes.

Run headless from the project root:
    /opt/blender-bin-5.2.1/blender -b --factory-startup --python assets/blender/items.py

Writes assets/models/items.glb. A touch larger than life, so they read on
the ground from standing height.
"""

import math
import os

import bmesh
import bpy
from mathutils import Matrix

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "..", "models", "items.glb")

bpy.ops.wm.read_factory_settings(use_empty=True)

GAUZE = (0.86, 0.84, 0.77)
GAUZE_SHADE = (0.74, 0.72, 0.66)
KIT_RED = (0.66, 0.10, 0.08)
KIT_DARK = (0.36, 0.06, 0.05)
WHITE = (0.92, 0.91, 0.87)
HANDLE = (0.14, 0.13, 0.12)
CARTON = (0.33, 0.34, 0.21)
CARTON_DARK = (0.24, 0.25, 0.15)
LABEL = (0.78, 0.70, 0.48)
BRASS = (0.78, 0.60, 0.26)
LEAD = (0.45, 0.42, 0.40)


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


def bandage():
    p = Part("ITEM_Bandage")
    r, length = 0.055, 0.12
    p.roll((0, 0, r), r, length, GAUZE, GAUZE_SHADE)
    # The loose end, trailing off the roll.
    p.box((0.0, r * 1.6, 0.004), (length * 0.85, 0.09, 0.008), GAUZE_SHADE, turn=0.12)
    p.finish()


def medkit():
    p = Part("ITEM_Medkit")
    w, d, h = 0.34, 0.22, 0.12
    p.box((0, 0, h / 2), (w, d, h), KIT_RED)
    p.box((0, 0, h * 0.52), (w + 0.006, d + 0.006, 0.014), KIT_DARK)
    # The cross on its lid.
    p.box((0, 0, h + 0.004), (0.13, 0.04, 0.008), WHITE)
    p.box((0, 0, h + 0.004), (0.04, 0.13, 0.008), WHITE)
    # A handle on the side facing +Y.
    for x in (-0.06, 0.06):
        p.box((x, d / 2 + 0.02, h * 0.6), (0.015, 0.04, 0.015), HANDLE)
    p.box((0, d / 2 + 0.04, h * 0.6), (0.135, 0.015, 0.02), HANDLE)
    p.finish()


def ammo():
    """An olive carton with its lid flipped open, the rounds standing in
    rows, a tan label round its middle."""
    p = Part("ITEM_Ammo")
    w, d, h = 0.20, 0.12, 0.075
    p.box((0, 0, h / 2), (w, d, h), CARTON)
    p.box((0, 0, h * 0.5), (w + 0.004, d + 0.004, 0.028), LABEL)
    # The lid, open and leaning back past upright.
    p.box((0, -d / 2 - 0.028, h + 0.018), (w, 0.006, 0.07), CARTON_DARK)
    # Rounds: brass cases, dull noses, in three rows.
    for row in (-0.03, 0.0, 0.03):
        for i in range(6):
            x = -0.075 + i * 0.03
            p.box((x, row, h + 0.006), (0.014, 0.014, 0.014), BRASS)
            p.box((x, row, h + 0.017), (0.009, 0.009, 0.01), LEAD)
    p.finish()


def main():
    bandage()
    medkit()
    ammo()
    bpy.ops.export_scene.gltf(filepath=os.path.abspath(OUT), export_format="GLB", export_yup=True, export_apply=False, export_animations=False, export_vertex_color="ACTIVE", export_normals=True)
    print(f"items: bandage, medkit, ammo -> {os.path.abspath(OUT)}")


main()
