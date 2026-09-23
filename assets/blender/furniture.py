"""What furnishes a house (to bump into and hide behind, not to search),
each its own object standing on its origin (the middle of its footprint),
its back against a wall at -Y and its front (what faces the room) at +Y:

    FURN_Bed        a double bed, its headboard at the back
    FURN_Sofa       a three-seater
    FURN_Armchair   an armchair
    FURN_Table      a kitchen table and two chairs
    FURN_Counter    a run of kitchen counter, a sink in it
    FURN_Stove      a stove and oven
    FURN_Bathtub    a bath
    FURN_Toilet     a toilet
    FURN_Basin      a washbasin on a pedestal, a mirror over it
    FURN_Bookcase   a tall bookcase
    FURN_Tv         a television on a low stand
    FURN_HayBale    a bale of straw, lying long
    FURN_HayStack   bales stacked three high against a wall
    FURN_WoodStove  a squat cast-iron stove, its pipe up to the ceiling

and what the game bumps into for each (FURN_*_Hull): plain boxes.

Run headless from the project root:
    /opt/blender-bin-5.2.1/blender -b --factory-startup --python assets/blender/furniture.py

Writes assets/models/furniture.glb.
"""

import os
import random

import bmesh
import bpy
from mathutils import Matrix, Vector

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "..", "models", "furniture.glb")

bpy.ops.wm.read_factory_settings(use_empty=True)
rng = random.Random(31)

WOOD = (0.45, 0.32, 0.21)
WOOD_DARK = (0.30, 0.21, 0.14)
WOOD_PALE = (0.58, 0.46, 0.32)
FABRIC = [(0.36, 0.30, 0.40), (0.30, 0.38, 0.32), (0.46, 0.30, 0.24), (0.38, 0.38, 0.36)]
SHEET = (0.78, 0.76, 0.70)
BLANKET = (0.30, 0.34, 0.46)
PILLOW = (0.84, 0.82, 0.76)
PORCELAIN = (0.86, 0.86, 0.83)
STEEL = (0.55, 0.55, 0.53)
DARK = (0.10, 0.10, 0.10)
TOP = (0.40, 0.38, 0.34)
UNIT = (0.66, 0.62, 0.52)
BOOKS = [(0.55, 0.16, 0.14), (0.20, 0.30, 0.50), (0.26, 0.42, 0.26), (0.62, 0.52, 0.28), (0.40, 0.26, 0.40)]
SCREEN = (0.06, 0.08, 0.09)
MIRROR = (0.52, 0.58, 0.60)


def jitter(c, amount=0.05):
    k = 1.0 + rng.uniform(-amount, amount)
    return tuple(min(1.0, max(0.0, v * k)) for v in c)


class Part:
    def __init__(self, name):
        rng.seed(name)
        self.name = name
        self.bm = bmesh.new()
        self.col = self.bm.loops.layers.color.new("Col")

    def box(self, lo, hi, colour):
        c = Vector(((lo[0] + hi[0]) / 2, (lo[1] + hi[1]) / 2, (lo[2] + hi[2]) / 2))
        s = ((hi[0] - lo[0]) / 2, (hi[1] - lo[1]) / 2, (hi[2] - lo[2]) / 2)
        made = bmesh.ops.create_cube(self.bm, size=2.0, matrix=Matrix.Translation(c) @ Matrix.Diagonal((*s, 1.0)))["verts"]
        for f in sorted({f for v in made for f in v.link_faces}, key=lambda f: f.index):
            col = jitter(colour)
            for loop in f.loops:
                loop[self.col] = (*col, 1.0)
        return made

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


def piece(name, hull):
    """Start `name`; its hull (a list of corner pairs) is made alongside."""
    h = Part(name + "_Hull")
    for lo, hi in hull:
        h.box(lo, hi, DARK)
    h.finish()
    return Part(name)


def legs(p, x0, x1, y0, y1, h, t=0.05, colour=WOOD_DARK):
    for x in (x0, x1 - t):
        for y in (y0, y1 - t):
            p.box((x, y, 0.0), (x + t, y + t, h), colour)


def bed():
    w, d = 1.6, 2.1
    p = piece("FURN_Bed", [((-w / 2, -d / 2, 0.0), (w / 2, d / 2, 0.6)), ((-w / 2, -d / 2, 0.6), (w / 2, -d / 2 + 0.08, 1.05))])
    p.box((-w / 2, -d / 2, 0.1), (w / 2, d / 2, 0.3), WOOD)
    p.box((-w / 2 + 0.03, -d / 2 + 0.05, 0.3), (w / 2 - 0.03, d / 2 - 0.03, 0.5), SHEET)
    p.box((-w / 2 + 0.01, -d / 2 + 0.6, 0.45), (w / 2 - 0.01, d / 2 - 0.01, 0.56), BLANKET)
    for x in (-0.65, 0.05):
        p.box((x, -d / 2 + 0.12, 0.5), (x + 0.6, -d / 2 + 0.5, 0.62), PILLOW)
    p.box((-w / 2, -d / 2, 0.0), (w / 2, -d / 2 + 0.08, 1.05), WOOD_DARK)
    legs(p, -w / 2, w / 2, -d / 2, d / 2, 0.1)
    p.finish()


def sofa():
    w, d = 2.0, 0.9
    fabric = FABRIC[0]
    p = piece("FURN_Sofa", [((-w / 2, -d / 2, 0.0), (w / 2, d / 2, 0.85))])
    p.box((-w / 2, -d / 2, 0.08), (w / 2, d / 2, 0.42), fabric)
    p.box((-w / 2, -d / 2, 0.42), (w / 2, -d / 2 + 0.22, 0.85), fabric)
    for x in (-w / 2, w / 2 - 0.2):
        p.box((x, -d / 2, 0.42), (x + 0.2, d / 2, 0.62), fabric)
    for k in range(3):
        x = -w / 2 + 0.2 + k * 0.533
        p.box((x + 0.01, -d / 2 + 0.22, 0.42), (x + 0.52, d / 2 - 0.02, 0.52), jitter(fabric, 0.1))
    legs(p, -w / 2 + 0.05, w / 2 - 0.05, -d / 2 + 0.05, d / 2 - 0.05, 0.08)
    p.finish()


def armchair():
    w, d = 0.9, 0.9
    fabric = FABRIC[2]
    p = piece("FURN_Armchair", [((-w / 2, -d / 2, 0.0), (w / 2, d / 2, 0.9))])
    p.box((-w / 2, -d / 2, 0.08), (w / 2, d / 2, 0.42), fabric)
    p.box((-w / 2, -d / 2, 0.42), (w / 2, -d / 2 + 0.2, 0.9), fabric)
    for x in (-w / 2, w / 2 - 0.16):
        p.box((x, -d / 2, 0.42), (x + 0.16, d / 2, 0.64), fabric)
    p.box((-w / 2 + 0.17, -d / 2 + 0.2, 0.42), (w / 2 - 0.17, d / 2 - 0.02, 0.52), jitter(fabric, 0.1))
    legs(p, -w / 2 + 0.05, w / 2 - 0.05, -d / 2 + 0.05, d / 2 - 0.05, 0.08)
    p.finish()


def table():
    """1.2 × 0.8, against the wall on its long side; a chair each end."""
    w, d, h = 1.2, 0.8, 0.75
    p = piece("FURN_Table", [((-w / 2 - 0.45, -d / 2, 0.0), (w / 2 + 0.45, d / 2, h))])
    p.box((-w / 2, -d / 2, h - 0.04), (w / 2, d / 2, h), WOOD_PALE)
    legs(p, -w / 2 + 0.04, w / 2 - 0.04, -d / 2 + 0.04, d / 2 - 0.04, h - 0.04)
    for side in (-1, 1):
        x = side * (w / 2 + 0.22)
        p.box((x - 0.2, -0.2, 0.44), (x + 0.2, 0.2, 0.48), WOOD)
        bx = x + side * 0.18
        p.box((bx - 0.02, -0.2, 0.48), (bx + 0.02, 0.2, 0.92), WOOD)
        legs(p, x - 0.2, x + 0.2, -0.2, 0.2, 0.44, 0.04, WOOD)
    p.box((-0.2, -0.1, h), (0.0, 0.1, h + 0.1), (0.62, 0.60, 0.56))
    p.finish()


def counter():
    w, d, h = 1.8, 0.65, 0.9
    p = piece("FURN_Counter", [((-w / 2, -d / 2, 0.0), (w / 2, d / 2, h + 0.03))])
    p.box((-w / 2, -d / 2, 0.08), (w / 2, d / 2 - 0.03, h), UNIT)
    p.box((-w / 2 + 0.03, -d / 2, 0.0), (w / 2 - 0.03, d / 2 - 0.08, 0.08), DARK)
    p.box((-w / 2, -d / 2, h), (w / 2, d / 2, h + 0.03), TOP)
    p.box((0.1, -0.2, h + 0.005), (0.6, 0.15, h + 0.031), STEEL)
    p.box((0.32, -0.28, h + 0.03), (0.36, -0.24, h + 0.3), STEEL)
    for k in range(3):
        x = -w / 2 + 0.05 + k * 0.6
        p.box((x, d / 2 - 0.03, 0.12), (x + 0.55, d / 2, h - 0.05), jitter(UNIT, 0.08))
        p.box((x + 0.24, d / 2, h - 0.2), (x + 0.32, d / 2 + 0.02, h - 0.17), STEEL)
    p.finish()


def stove():
    w, d, h = 0.7, 0.65, 0.9
    p = piece("FURN_Stove", [((-w / 2, -d / 2, 0.0), (w / 2, d / 2, h))])
    p.box((-w / 2, -d / 2, 0.0), (w / 2, d / 2 - 0.02, h), PORCELAIN)
    p.box((-w / 2 + 0.05, d / 2 - 0.02, 0.1), (w / 2 - 0.05, d / 2, 0.6), DARK)
    p.box((-w / 2 + 0.08, d / 2, 0.55), (w / 2 - 0.08, d / 2 + 0.03, 0.58), STEEL)
    for x in (-0.16, 0.16):
        for y in (-0.15, 0.15):
            p.box((x - 0.1, y - 0.1, h), (x + 0.1, y + 0.1, h + 0.015), DARK)
    p.finish()


def bathtub():
    w, d, h = 1.7, 0.75, 0.55
    p = piece("FURN_Bathtub", [((-w / 2, -d / 2, 0.0), (w / 2, d / 2, h))])
    p.box((-w / 2, -d / 2, 0.0), (w / 2, d / 2, 0.1), PORCELAIN)
    p.box((-w / 2, -d / 2, 0.1), (w / 2, -d / 2 + 0.08, h), PORCELAIN)
    p.box((-w / 2, d / 2 - 0.08, 0.1), (w / 2, d / 2, h), PORCELAIN)
    p.box((-w / 2, -d / 2 + 0.08, 0.1), (-w / 2 + 0.08, d / 2 - 0.08, h), PORCELAIN)
    p.box((w / 2 - 0.08, -d / 2 + 0.08, 0.1), (w / 2, d / 2 - 0.08, h), PORCELAIN)
    p.box((-w / 2 + 0.08, -d / 2 + 0.08, 0.1), (w / 2 - 0.08, d / 2 - 0.08, 0.14), (0.38, 0.36, 0.30))
    p.box((w / 2 - 0.2, -d / 2 + 0.02, h), (w / 2 - 0.14, -d / 2 + 0.2, h + 0.18), STEEL)
    p.finish()


def toilet():
    p = piece("FURN_Toilet", [((-0.22, -0.35, 0.0), (0.22, 0.35, 0.8))])
    p.box((-0.2, -0.35, 0.4), (0.2, -0.17, 0.8), PORCELAIN)
    p.box((-0.14, -0.2, 0.0), (0.14, 0.1, 0.38), PORCELAIN)
    p.box((-0.2, -0.2, 0.36), (0.2, 0.33, 0.42), PORCELAIN)
    p.box((-0.18, -0.15, 0.42), (0.18, 0.3, 0.44), (0.76, 0.76, 0.72))
    p.finish()


def basin():
    p = piece("FURN_Basin", [((-0.3, -0.25, 0.0), (0.3, 0.25, 0.88))])
    p.box((-0.08, -0.2, 0.0), (0.08, -0.05, 0.75), PORCELAIN)
    p.box((-0.3, -0.25, 0.75), (0.3, 0.25, 0.88), PORCELAIN)
    p.box((-0.2, -0.12, 0.84), (0.2, 0.18, 0.881), (0.40, 0.40, 0.38))
    p.box((-0.3, -0.25, 1.2), (0.3, -0.23, 1.75), MIRROR)
    p.finish()


def bookcase():
    w, d, h = 1.0, 0.35, 1.9
    p = piece("FURN_Bookcase", [((-w / 2, -d / 2, 0.0), (w / 2, d / 2, h))])
    p.box((-w / 2, -d / 2, 0.0), (w / 2, -d / 2 + 0.03, h), WOOD_DARK)
    for x in (-w / 2, w / 2 - 0.03):
        p.box((x, -d / 2, 0.0), (x + 0.03, d / 2, h), WOOD_DARK)
    for k in range(5):
        z = k * 0.45
        p.box((-w / 2, -d / 2, z), (w / 2, d / 2, z + 0.03), WOOD_DARK)
        if k == 4:
            break
        x = -w / 2 + 0.05
        while x < w / 2 - 0.12:
            bw = rng.uniform(0.03, 0.07)
            p.box((x, -d / 2 + 0.05, z + 0.03), (x + bw, d / 2 - 0.04, z + 0.03 + rng.uniform(0.22, 0.36)), rng.choice(BOOKS))
            x += bw + 0.005
    p.box((-w / 2, -d / 2, h - 0.03), (w / 2, d / 2, h), WOOD_DARK)
    p.finish()


def tv():
    w, d = 1.2, 0.45
    p = piece("FURN_Tv", [((-w / 2, -d / 2, 0.0), (w / 2, d / 2, 0.5)), ((-0.45, -0.2, 0.5), (0.45, 0.05, 1.05))])
    p.box((-w / 2, -d / 2, 0.0), (w / 2, d / 2, 0.5), WOOD)
    p.box((-w / 2 + 0.05, d / 2, 0.08), (w / 2 - 0.05, d / 2 + 0.01, 0.42), WOOD_DARK)
    p.box((-0.45, -0.2, 0.5), (0.45, 0.05, 1.05), DARK)
    p.box((-0.4, 0.05, 0.55), (0.4, 0.055, 1.0), SCREEN)
    p.finish()


STRAW = (0.66, 0.56, 0.30)
STRAW_DARK = (0.52, 0.43, 0.22)
TWINE = (0.36, 0.30, 0.18)
IRON = (0.14, 0.14, 0.14)
EMBER = (0.55, 0.20, 0.06)


def bale(p, x, y, z, turned=False):
    """A bale of straw, 1.1 × 0.5 × 0.45, its middle at (x, y) on `z`
    (turned: long along y), two bands of twine round it."""
    (hw, hd) = (0.25, 0.55) if turned else (0.55, 0.25)
    p.box((x - hw, y - hd, z), (x + hw, y + hd, z + 0.45), STRAW if rng.random() < 0.6 else STRAW_DARK)
    for k in (-0.25, 0.25):
        if turned:
            p.box((x - hw - 0.01, y + k - 0.02, z - 0.005), (x + hw + 0.01, y + k + 0.02, z + 0.455), TWINE)
        else:
            p.box((x + k - 0.02, y - hd - 0.01, z - 0.005), (x + k + 0.02, y + hd + 0.01, z + 0.455), TWINE)


def hay_bale():
    p = piece("FURN_HayBale", [((-0.55, -0.25, 0.0), (0.55, 0.25, 0.45))])
    bale(p, 0.0, 0.0, 0.0)
    p.finish()


def hay_stack():
    """Two bales side by side, two across them, one on top: against the
    wall at the back."""
    w, d = 1.1, 1.0
    p = piece("FURN_HayStack", [((-w / 2, -d / 2, 0.0), (w / 2, d / 2, 1.35))])
    for y in (-0.25, 0.25):
        bale(p, 0.0, y, 0.0)
    for x in (-0.28, 0.28):
        bale(p, x, 0.0, 0.45, turned=True)
    bale(p, 0.0, -0.22, 0.9)
    p.finish()


def wood_stove():
    w, d, h = 0.6, 0.55, 0.75
    p = piece("FURN_WoodStove", [((-w / 2, -d / 2, 0.0), (w / 2, d / 2, h))])
    legs(p, -w / 2 + 0.02, w / 2 - 0.02, -d / 2 + 0.02, d / 2 - 0.02, 0.15, t=0.06, colour=IRON)
    p.box((-w / 2, -d / 2, 0.15), (w / 2, d / 2, h), IRON)
    p.box((-0.16, d / 2, 0.28), (0.16, d / 2 + 0.02, 0.55), EMBER)
    p.box((-0.2, d / 2 + 0.02, 0.26), (0.2, d / 2 + 0.03, 0.29), STEEL)
    # The pipe, up from its back to the ceiling.
    p.box((-0.07, -d / 2 + 0.05, h), (0.07, -d / 2 + 0.19, 2.8), IRON)
    p.finish()


def main():
    bed()
    sofa()
    armchair()
    table()
    counter()
    stove()
    bathtub()
    toilet()
    basin()
    bookcase()
    tv()
    hay_bale()
    hay_stack()
    wood_stove()
    bpy.ops.export_scene.gltf(filepath=os.path.abspath(OUT), export_format="GLB", export_yup=True, export_apply=False, export_animations=False, export_vertex_color="ACTIVE", export_normals=True)
    tris = sum(len(o.data.polygons) for o in bpy.data.objects if o.type == "MESH")
    print(f"furniture: {len(bpy.data.objects)} objects, {tris} faces -> {os.path.abspath(OUT)}")


main()
