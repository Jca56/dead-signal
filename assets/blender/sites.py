"""What the places out of town are fitted with, each its own object
standing on its origin (the middle of its footprint, on the ground), its
front facing +Y:

    SITE_Canopy        a gas station's canopy on four posts
    SITE_Pumps         an island of two pumps on a kerb
    SITE_GasSign       the station's sign, high on its pole
    SITE_HeliFront     a downed helicopter's cockpit and cabin, torn open
                       at the back (walk in)
    SITE_HeliRear      its rear cabin, open at the front, its ramp down
    SITE_HeliTail      its tail boom, fin and rotor, lying apart
    SITE_Rotor         one of its blades, bent
    SITE_PlaneFront    a cargo plane's nose and forward hold, torn open at
                       the back (walk in)
    SITE_PlaneRear     its rear hold and tail, open at the front
    SITE_PlaneWing     a wing torn off, an engine on it
    SITE_PlaneEngine   an engine and its propeller, thrown clear
    SITE_Tent          a soldier's ridge tent, open at the front
    SITE_CommandTent   a big walled tent, a doorway in its front
    SITE_Sandbags      a run of sandbag wall
    SITE_Berm          a range's earth backstop
    SITE_Bench         a shooting bench
    SITE_PlateFrame    a frame a steel plate hangs from (at 1.85 m)

and what the game bumps into for each (SITE_*_Hull): plain boxes, the
wrecks' and tents' left open inside to be walked into.

Run headless from the project root:
    /opt/blender-bin-5.2.1/blender -b --factory-startup --python assets/blender/sites.py

Writes assets/models/sites.glb.
"""

import math
import os
import random

import bmesh
import bpy
from mathutils import Matrix, Vector

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "..", "models", "sites.glb")

bpy.ops.wm.read_factory_settings(use_empty=True)
rng = random.Random(41)

CONCRETE = (0.52, 0.51, 0.48)
WHITE = (0.80, 0.79, 0.75)
RED = (0.62, 0.12, 0.09)
STEEL = (0.40, 0.40, 0.39)
DARK = (0.10, 0.10, 0.10)
GLASS = (0.12, 0.16, 0.18)
OLIVE = (0.28, 0.30, 0.22)
OLIVE_DARK = (0.20, 0.22, 0.16)
SCORCH = (0.09, 0.08, 0.07)
PLANE_GREY = (0.58, 0.60, 0.60)
STRIPE = (0.20, 0.30, 0.48)
CANVAS = (0.36, 0.36, 0.26)
CANVAS_DARK = (0.28, 0.28, 0.20)
SAND = (0.58, 0.52, 0.38)
SAND_DARK = (0.48, 0.43, 0.31)
EARTH = (0.34, 0.28, 0.20)
WOOD = (0.44, 0.32, 0.20)


def jitter(c, amount=0.06):
    k = 1.0 + rng.uniform(-amount, amount)
    return tuple(min(1.0, max(0.0, v * k)) for v in c)


class Part:
    """A mesh with a colour a face, made of boxes; the whole of it turned
    by `turn` (a wreck lies askew) as it's finished."""

    def __init__(self, name, turn=None):
        rng.seed(name)
        self.name = name
        self.turn = turn
        self.bm = bmesh.new()
        self.col = self.bm.loops.layers.color.new("Col")

    def box(self, lo, hi, colour, spin=None):
        c = Vector(((lo[0] + hi[0]) / 2, (lo[1] + hi[1]) / 2, (lo[2] + hi[2]) / 2))
        s = ((hi[0] - lo[0]) / 2, (hi[1] - lo[1]) / 2, (hi[2] - lo[2]) / 2)
        m = Matrix.Translation(c) @ (spin or Matrix.Identity(4)) @ Matrix.Diagonal((*s, 1.0))
        made = bmesh.ops.create_cube(self.bm, size=2.0, matrix=m)["verts"]
        for f in sorted({f for v in made for f in v.link_faces}, key=lambda f: f.index):
            k = jitter(colour)
            for loop in f.loops:
                loop[self.col] = (*k, 1.0)
        return list(made)

    def finish(self):
        if self.turn is not None:
            for v in self.bm.verts:
                v.co = self.turn @ v.co
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


def hull(name, boxes, turn=None):
    """`boxes` (corner pairs, or with a spin) as one never-drawn object."""
    p = Part(name + "_Hull", turn)
    for b in boxes:
        p.box(b[0], b[1], DARK, b[2] if len(b) > 2 else None)
    p.finish()


def spin_x(deg):
    return Matrix.Rotation(math.radians(deg), 4, "X")


def spin_y(deg):
    return Matrix.Rotation(math.radians(deg), 4, "Y")


def spin_z(deg):
    return Matrix.Rotation(math.radians(deg), 4, "Z")


# ---- the gas station ---------------------------------------------------------

def canopy():
    p = Part("SITE_Canopy")
    posts = [(x, y) for x in (-4.5, 4.5) for y in (-2.5, 2.5)]
    for x, y in posts:
        p.box((x - 0.15, y - 0.15, 0.0), (x + 0.15, y + 0.15, 4.6), WHITE)
    p.box((-6.0, -4.0, 4.6), (6.0, 4.0, 5.0), WHITE)
    for y in (-4.02, 3.98):
        p.box((-6.02, y, 4.62), (6.02, y + 0.04, 4.98), RED)
    for x in (-6.02, 5.98):
        p.box((x, -4.0, 4.62), (x + 0.04, 4.0, 4.98), RED)
    p.box((-5.0, -3.0, 4.58), (5.0, 3.0, 4.6), (0.86, 0.85, 0.80))
    p.finish()
    hull("SITE_Canopy", [((x - 0.15, y - 0.15, 0.0), (x + 0.15, y + 0.15, 4.6)) for x, y in posts] + [((-6.0, -4.0, 4.6), (6.0, 4.0, 5.0))])


def pumps():
    p = Part("SITE_Pumps")
    p.box((-1.8, -0.6, 0.0), (1.8, 0.6, 0.18), CONCRETE)
    for x in (-0.9, 0.9):
        p.box((x - 0.35, -0.25, 0.18), (x + 0.35, 0.25, 1.75), WHITE)
        p.box((x - 0.36, -0.26, 1.35), (x + 0.36, 0.26, 1.75), RED)
        p.box((x - 0.22, 0.25, 0.9), (x + 0.22, 0.27, 1.25), GLASS)
        p.box((x + 0.36, -0.05, 0.5), (x + 0.42, 0.05, 1.2), DARK)
    p.box((-0.1, -0.1, 0.18), (0.1, 0.1, 1.0), (0.85, 0.66, 0.12))
    p.finish()
    hull("SITE_Pumps", [((-1.8, -0.6, 0.0), (1.8, 0.6, 0.18))] + [((x - 0.35, -0.25, 0.18), (x + 0.35, 0.25, 1.75)) for x in (-0.9, 0.9)])


def gas_sign():
    p = Part("SITE_GasSign")
    p.box((-0.15, -0.15, 0.0), (0.15, 0.15, 6.0), STEEL)
    p.box((-1.3, -0.2, 6.0), (1.3, 0.2, 7.6), RED)
    p.box((-1.2, -0.22, 6.1), (1.2, 0.22, 6.7), WHITE)
    p.box((-1.2, -0.22, 6.9), (1.2, 0.22, 7.5), (0.95, 0.72, 0.15))
    p.finish()
    hull("SITE_GasSign", [((-0.15, -0.15, 0.0), (0.15, 0.15, 6.0))])


# ---- the wrecks --------------------------------------------------------------

def hold(p, w, h, y0, y1, floor, colour, dark, windows=True):
    """A fuselage's hollow section from `y0` to `y1`: its floor, sides (a
    row of windows in them), roof; open at both ends. Its boxes, for the
    hull."""
    t = 0.12
    boxes = [
        ((-w / 2, y0, floor - 0.2), (w / 2, y1, floor)),
        ((-w / 2, y0, floor), (-w / 2 + t, y1, floor + h)),
        ((w / 2 - t, y0, floor), (w / 2, y1, floor + h)),
        ((-w / 2, y0, floor + h), (w / 2, y1, floor + h + t)),
    ]
    for lo, hi in boxes:
        p.box(lo, hi, colour)
    p.box((-w / 2 + t, y0, floor - 0.01), (w / 2 - t, y1, floor + 0.01), dark)
    if windows:
        y = y0 + 0.8
        while y < y1 - 0.6:
            for x in (-w / 2 - 0.01, w / 2 - 0.01):
                p.box((x, y, floor + h * 0.55), (x + 0.02, y + 0.4, floor + h * 0.8), GLASS)
            y += 1.2
    return boxes


def heli_front():
    lie = spin_y(-7.0)
    p = Part("SITE_HeliFront", lie)
    boxes = hold(p, 2.6, 2.3, -3.0, 2.0, 0.35, OLIVE, OLIVE_DARK)
    # The cockpit: a nose narrowing forward, its glass, a scorched break
    # at the back.
    p.box((-1.2, 2.0, 0.15), (1.2, 3.2, 2.4), OLIVE)
    p.box((-1.0, 3.2, 0.3), (1.0, 3.9, 1.9), OLIVE)
    p.box((-0.95, 3.25, 1.2), (0.95, 3.95, 1.85), GLASS)
    p.box((-1.21, 2.2, 1.3), (1.21, 3.0, 2.2), GLASS)
    for x in (-1.3, 1.3):
        p.box((x - 0.06, -2.4, 0.0), (x + 0.06, 2.6, 0.12), DARK)
    p.box((-1.31, -3.05, 0.1), (1.31, -2.9, 2.8), SCORCH)
    p.box((-0.3, -0.5, 2.8), (0.3, 0.5, 3.3), OLIVE_DARK)
    p.finish()
    hull("SITE_HeliFront", [(lo, hi) for lo, hi in boxes] + [((-1.2, 2.0, 0.15), (1.2, 3.9, 2.4))], lie)


def heli_rear():
    lie = spin_y(10.0) @ spin_z(8.0)
    p = Part("SITE_HeliRear", lie)
    boxes = hold(p, 2.6, 2.3, -2.0, 2.0, 0.35, OLIVE, OLIVE_DARK)
    # The ramp down behind it, and the stub of the boom over.
    p.box((-1.1, -3.6, 0.0), (1.1, -2.0, 0.12), OLIVE_DARK)
    p.box((-0.5, -2.8, 2.2), (0.5, -2.0, 2.8), OLIVE)
    p.box((-1.31, 1.95, 0.1), (1.31, 2.1, 2.8), SCORCH)
    p.finish()
    hull("SITE_HeliRear", [(lo, hi) for lo, hi in boxes] + [((-1.1, -3.6, 0.0), (1.1, -2.0, 0.12))], lie)


def heli_tail():
    lie = spin_y(18.0)
    p = Part("SITE_HeliTail", lie)
    p.box((-0.35, -3.5, 0.0), (0.35, 3.0, 0.7), OLIVE)
    p.box((-0.1, -3.4, 0.6), (0.1, -2.2, 2.2), OLIVE_DARK)
    for k in range(4):
        a = spin_x(k * 45.0)
        p.box((0.15, -2.8 - 0.06, 1.0), (0.2, -2.8 + 0.06, 2.0), DARK, a)
    p.box((-0.36, 2.9, 0.0), (0.36, 3.05, 0.72), SCORCH)
    p.finish()
    hull("SITE_HeliTail", [((-0.35, -3.5, 0.0), (0.35, 3.0, 0.7)), ((-0.1, -3.4, 0.6), (0.1, -2.2, 2.2))], lie)


def rotor():
    p = Part("SITE_Rotor")
    p.box((-0.25, -3.0, 0.0), (0.25, 0.4, 0.08), DARK)
    p.box((-0.25, 0.4, 0.0), (0.25, 3.2, 0.08), DARK, spin_z(18.0))
    p.box((-0.3, -3.1, 0.0), (0.3, -2.9, 0.12), STEEL)
    p.finish()
    hull("SITE_Rotor", [((-0.25, -3.0, 0.0), (0.25, 3.2, 0.08))])


def plane_front():
    lie = spin_y(-5.0)
    p = Part("SITE_PlaneFront", lie)
    boxes = hold(p, 3.2, 2.6, -4.0, 3.0, 0.3, PLANE_GREY, (0.30, 0.30, 0.29))
    for x in (-1.61, 1.59):
        p.box((x, -4.0, 1.1), (x + 0.02, 3.0, 1.35), STRIPE)
    p.box((-1.5, 3.0, 0.15), (1.5, 4.6, 2.8), PLANE_GREY)
    p.box((-1.2, 4.6, 0.4), (1.2, 5.6, 2.3), PLANE_GREY)
    p.box((-1.1, 4.9, 1.8), (1.1, 5.62, 2.3), GLASS)
    p.box((-1.61, -4.05, 0.1), (1.61, -3.9, 3.0), SCORCH)
    p.finish()
    hull("SITE_PlaneFront", [(lo, hi) for lo, hi in boxes] + [((-1.5, 3.0, 0.15), (1.5, 5.6, 2.8))], lie)


def plane_rear():
    lie = spin_z(12.0) @ spin_y(8.0)
    p = Part("SITE_PlaneRear", lie)
    boxes = hold(p, 3.2, 2.6, -2.0, 3.0, 0.3, PLANE_GREY, (0.30, 0.30, 0.29))
    p.box((-1.2, -4.5, 0.6), (1.2, -2.0, 2.9), PLANE_GREY)
    p.box((-0.12, -4.8, 2.9), (0.12, -2.8, 5.4), PLANE_GREY)
    p.box((-2.6, -4.6, 2.2), (2.6, -3.6, 2.35), PLANE_GREY)
    p.box((-0.13, -4.4, 4.2), (0.13, -3.4, 5.0), STRIPE)
    p.box((-1.61, 2.95, 0.1), (1.61, 3.1, 3.0), SCORCH)
    p.finish()
    hull("SITE_PlaneRear", [(lo, hi) for lo, hi in boxes] + [((-1.2, -4.5, 0.6), (1.2, -2.0, 2.9)), ((-0.12, -4.8, 2.9), (0.12, -2.8, 5.4))], lie)


def plane_wing():
    lie = spin_y(6.0)
    p = Part("SITE_PlaneWing", lie)
    p.box((-5.5, -1.2, 0.0), (5.5, 1.2, 0.3), PLANE_GREY)
    p.box((-5.5, 1.0, 0.05), (5.5, 1.3, 0.25), (0.46, 0.47, 0.47))
    p.box((1.0, -2.2, -0.2), (2.4, 1.6, 1.0), PLANE_GREY)
    p.box((1.2, 1.6, 0.0), (2.2, 1.9, 0.8), DARK)
    p.box((-5.52, -1.25, -0.02), (-5.3, 1.25, 0.32), SCORCH)
    p.finish()
    hull("SITE_PlaneWing", [((-5.5, -1.2, 0.0), (5.5, 1.2, 0.3)), ((1.0, -2.2, -0.2), (2.4, 1.6, 1.0))], lie)


def plane_engine():
    lie = spin_x(-12.0)
    p = Part("SITE_PlaneEngine", lie)
    p.box((-0.7, -1.4, 0.0), (0.7, 1.4, 1.3), PLANE_GREY)
    p.box((-0.55, 1.4, 0.15), (0.55, 1.6, 1.15), DARK)
    for k in range(3):
        p.box((-0.12, 1.6, 0.55), (0.12, 1.7, 2.1), DARK, spin_y(k * 120.0 + 10.0))
    p.box((-0.72, -1.45, 0.0), (0.72, -1.3, 1.32), SCORCH)
    p.finish()
    hull("SITE_PlaneEngine", [((-0.7, -1.4, 0.0), (0.7, 1.6, 1.3))], lie)


# ---- the camp ----------------------------------------------------------------

def tent():
    """A ridge tent, 3 × 4, 2.4 at the ridge, open at the front."""
    p = Part("SITE_Tent")
    w, d, h = 3.0, 4.0, 2.4
    slope = math.degrees(math.atan2(h, w / 2))
    side = math.hypot(w / 2, h)
    boxes = []
    for s in (-1, 1):
        lean = spin_y(-s * (90.0 - slope))
        lo = (s * w / 4 - 0.04, -d / 2, h / 2 - side / 2)
        hi = (s * w / 4 + 0.04, d / 2, h / 2 + side / 2)
        p.box(lo, hi, CANVAS if s < 0 else CANVAS_DARK, lean)
        boxes.append((lo, hi, lean))
    p.box((-w / 2 + 0.3, -d / 2, 0.0), (w / 2 - 0.3, -d / 2 + 0.06, h * 0.7), CANVAS_DARK)
    boxes.append(((-w / 2 + 0.3, -d / 2, 0.0), (w / 2 - 0.3, -d / 2 + 0.06, h * 0.7)))
    for y in (-d / 2 + 0.05, d / 2 - 0.05):
        p.box((-0.04, y - 0.04, 0.0), (0.04, y + 0.04, h), WOOD)
    p.box((-w / 2 + 0.4, -d / 2 + 0.1, 0.0), (w / 2 - 0.4, d / 2 - 0.1, 0.02), OLIVE_DARK)
    p.finish()
    hull("SITE_Tent", boxes)


def command_tent():
    """A walled tent, 6 × 5, a doorway 2 m wide in the middle of its front,
    its roof pitched to 3 m."""
    p = Part("SITE_CommandTent")
    w, d, h, t = 6.0, 5.0, 2.2, 0.06
    boxes = [
        ((-w / 2, -d / 2, 0.0), (w / 2, -d / 2 + t, h)),
        ((-w / 2, -d / 2, 0.0), (-w / 2 + t, d / 2, h)),
        ((w / 2 - t, -d / 2, 0.0), (w / 2, d / 2, h)),
        ((-w / 2, d / 2 - t, 0.0), (-1.0, d / 2, h)),
        ((1.0, d / 2 - t, 0.0), (w / 2, d / 2, h)),
        ((-1.0, d / 2 - t, 2.05), (1.0, d / 2, h)),
    ]
    for lo, hi in boxes:
        p.box(lo, hi, CANVAS)
    # Each half of the roof high at the middle, down to its wall (turned
    # about Y, a positive turn takes +X down).
    for s in (-1, 1):
        lean = spin_y(s * 18.0)
        lo, hi = (s * w / 4 - 1.58, -d / 2 - 0.1, h + 0.35), (s * w / 4 + 1.58, d / 2 + 0.1, h + 0.43)
        p.box(lo, hi, CANVAS_DARK, lean)
        boxes.append((lo, hi, lean))
    p.box((-w / 2 + 0.1, -d / 2 + 0.1, 0.0), (w / 2 - 0.1, d / 2 - 0.1, 0.02), OLIVE_DARK)
    # A map table in the middle.
    p.box((-0.9, -1.4, 0.8), (0.9, -0.5, 0.86), WOOD)
    for x in (-0.8, 0.8):
        for y in (-1.3, -0.6):
            p.box((x - 0.03, y - 0.03, 0.0), (x + 0.03, y + 0.03, 0.8), WOOD)
    p.box((-0.6, -1.2, 0.86), (0.4, -0.7, 0.87), (0.80, 0.76, 0.62))
    p.finish()
    boxes.append(((-0.9, -1.4, 0.0), (0.9, -0.5, 0.86)))
    hull("SITE_CommandTent", boxes)


def sandbags():
    """A run of sandbag wall, 4 m long, 0.9 high."""
    p = Part("SITE_Sandbags")
    for row in range(3):
        z = row * 0.3
        off = 0.25 if row % 2 else 0.0
        x = -2.0 + off
        while x < 2.0 - 0.1:
            x1 = min(x + 0.5, 2.0)
            p.box((x + 0.02, -0.35 + row * 0.05, z), (x1 - 0.02, 0.35 - row * 0.05, z + 0.3), SAND if (row + int(x * 2)) % 2 else SAND_DARK)
            x = x1
    p.finish()
    hull("SITE_Sandbags", [((-2.0, -0.35, 0.0), (2.0, 0.35, 0.9))])


# ---- the proving ground ------------------------------------------------------

def berm():
    """Earth banked up behind the targets, 18 m long, 3 high."""
    p = Part("SITE_Berm")
    for k, (d, z) in enumerate(((2.4, 1.0), (1.8, 2.0), (1.2, 3.0))):
        p.box((-9.0 + k * 0.3, -d / 2, 0.0), (9.0 - k * 0.3, d / 2, z), EARTH)
    p.finish()
    hull("SITE_Berm", [((-9.0, -1.2, 0.0), (9.0, 1.2, 3.0))])


def bench():
    p = Part("SITE_Bench")
    p.box((-0.7, -0.4, 0.85), (0.7, 0.4, 0.92), WOOD)
    for x in (-0.6, 0.6):
        for y in (-0.3, 0.3):
            p.box((x - 0.04, y - 0.04, 0.0), (x + 0.04, y + 0.04, 0.85), WOOD)
    p.box((-0.7, -0.95, 0.42), (0.7, -0.65, 0.48), WOOD)
    for x in (-0.6, 0.6):
        p.box((x - 0.04, -0.84, 0.0), (x + 0.04, -0.76, 0.42), WOOD)
    p.finish()
    hull("SITE_Bench", [((-0.7, -0.95, 0.0), (0.7, 0.4, 0.92))])


def plate_frame():
    p = Part("SITE_PlateFrame")
    for x in (-0.45, 0.45):
        p.box((x - 0.04, -0.04, 0.0), (x + 0.04, 0.04, 1.95), STEEL)
    p.box((-0.49, -0.03, 1.87), (0.49, 0.03, 1.93), STEEL)
    p.finish()
    hull("SITE_PlateFrame", [((x - 0.04, -0.04, 0.0), (x + 0.04, 0.04, 1.95)) for x in (-0.45, 0.45)])


def main():
    canopy()
    pumps()
    gas_sign()
    heli_front()
    heli_rear()
    heli_tail()
    rotor()
    plane_front()
    plane_rear()
    plane_wing()
    plane_engine()
    tent()
    command_tent()
    sandbags()
    berm()
    bench()
    plate_frame()
    bpy.ops.export_scene.gltf(filepath=os.path.abspath(OUT), export_format="GLB", export_yup=True, export_apply=False, export_animations=False, export_vertex_color="ACTIVE", export_normals=True)
    print(f"sites: {len(bpy.data.objects)} objects -> {os.path.abspath(OUT)}")


main()
