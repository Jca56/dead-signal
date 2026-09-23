"""Everything that can be carried: supplies (ITEM_Bandage, ITEM_Medkit,
ITEM_Ammo), food and water (ITEM_Beans, ITEM_Water), valuables (ITEM_Pills,
ITEM_Cash, ITEM_Watch, ITEM_Ring, ITEM_Chain, ITEM_Radio, ITEM_Battery,
ITEM_Fuel, ITEM_GoldBar), the cage's key (ITEM_Key) and the weapons
(ITEM_Pistol). Each its own object
sitting on its origin, for the game to set down wherever it likes and to
draw its icon from (seen from the front, +Y, a little above: an item's
long side runs along X, a tall one stands up Z).

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


def beans():
    p = Part("ITEM_Beans")
    p.can((0, 0, 0), 0.05, 0.12, TIN)
    p.can((0, 0, 0.025), 0.052, 0.07, TIN_LABEL)
    p.finish()


def water():
    p = Part("ITEM_Water")
    p.can((0, 0, 0), 0.05, 0.2, BOTTLE)
    p.can((0, 0, 0.2), 0.035, 0.03, BOTTLE)
    p.can((0, 0, 0.23), 0.02, 0.025, CAP_BLUE, segments=8)
    p.can((0, 0, 0.07), 0.052, 0.06, (0.85, 0.85, 0.82))
    p.finish()


def pills():
    p = Part("ITEM_Pills")
    p.can((0, 0, 0), 0.035, 0.08, PILL, segments=8)
    p.can((0, 0, 0.08), 0.038, 0.02, (0.92, 0.91, 0.87), segments=8)
    p.can((0, 0, 0.02), 0.036, 0.035, (0.92, 0.91, 0.87), segments=8)
    p.finish()


def cash():
    p = Part("ITEM_Cash")
    for k in range(3):
        p.box((0.004 * k, 0.003 * k, 0.008 + k * 0.016), (0.16, 0.075, 0.016), BILL if k % 2 else BILL_DARK, turn=0.06 * k)
    p.box((0, 0, 0.026), (0.03, 0.078, 0.05), BAND)
    p.finish()


def watch():
    p = Part("ITEM_Watch")
    p.box((0, 0, 0.005), (0.15, 0.034, 0.01), LEATHER)
    p.can((0, 0, 0.004), 0.042, 0.018, SILVER, segments=12)
    p.can((0, 0, 0.022), 0.035, 0.003, (0.90, 0.88, 0.80), segments=12)
    p.box((0.01, 0.0, 0.026), (0.026, 0.004, 0.002), BLACK, turn=0.5)
    p.finish()


def ring():
    p = Part("ITEM_Ring")
    p.hoop((0, 0, 0.03), 0.028, 0.007, GOLD, segments=14, turn=Matrix.Rotation(math.pi / 2, 4, "X"))
    p.box((0, 0, 0.066), (0.016, 0.016, 0.012), GEM, turn=math.pi / 4)
    p.finish()


def chain():
    p = Part("ITEM_Chain")
    for k in range(16):
        a = k / 16 * math.tau
        x, y = math.cos(a) * 0.07, math.sin(a) * 0.045
        p.box((x, y, 0.006), (0.018, 0.009, 0.009), GOLD if k % 2 else GOLD_DARK, turn=a + math.pi / 2)
    p.box((0, -0.06, 0.008), (0.03, 0.036, 0.012), GOLD)
    p.finish()


def radio():
    p = Part("ITEM_Radio")
    p.box((0, 0, 0.09), (0.075, 0.04, 0.18), PLASTIC)
    p.box((0, 0.02, 0.06), (0.05, 0.004, 0.06), BLACK)
    p.box((0, 0.02, 0.125), (0.05, 0.004, 0.03), (0.40, 0.52, 0.40))
    p.can((0.022, 0, 0.18), 0.008, 0.11, BLACK, segments=6)
    p.can((-0.02, 0, 0.18), 0.011, 0.016, (0.55, 0.10, 0.08), segments=8)
    p.finish()


def battery():
    p = Part("ITEM_Battery")
    p.box((0, 0, 0.09), (0.26, 0.17, 0.18), BLACK)
    p.box((0, 0, 0.185), (0.25, 0.16, 0.012), PLASTIC)
    p.box((0, 0.086, 0.1), (0.14, 0.004, 0.06), (0.80, 0.72, 0.30))
    p.can((0.08, 0.03, 0.19), 0.015, 0.025, (0.70, 0.12, 0.08), segments=8)
    p.can((-0.08, 0.03, 0.19), 0.015, 0.025, (0.25, 0.25, 0.25), segments=8)
    p.finish()


def fuel():
    p = Part("ITEM_Fuel")
    p.box((0, 0, 0.16), (0.26, 0.13, 0.32), FUEL_RED)
    p.box((0, 0.066, 0.16), (0.2, 0.004, 0.24), (0.48, 0.09, 0.06))
    for x in (-0.06, 0.0, 0.06):
        p.box((x, 0, 0.345), (0.02, 0.05, 0.05), FUEL_RED)
    p.box((0, 0, 0.37), (0.14, 0.05, 0.02), FUEL_RED)
    p.can((0.1, 0, 0.32), 0.02, 0.05, BLACK, segments=8)
    p.finish()


def key():
    p = Part("ITEM_Key")
    p.hoop((-0.035, 0, 0.006), 0.02, 0.008, SILVER, segments=10)
    p.box((0.02, 0, 0.006), (0.07, 0.014, 0.012), SILVER)
    for x, h in ((0.035, 0.018), (0.047, 0.026), (0.06, 0.016)):
        p.box((x, -0.01 - h / 2, 0.006), (0.012, h, 0.012), SILVER)
    p.box((-0.06, 0.03, 0.004), (0.05, 0.04, 0.008), TAG, turn=0.4)
    p.finish()


def pistol():
    """A service pistol lying on its side, barrel along X, grip to the
    front (towards the icon's eye, so it reads as a pistol)."""
    p = Part("ITEM_Pistol")
    p.box((0.02, 0.0, 0.017), (0.22, 0.04, 0.03), PLASTIC)
    p.box((0.03, 0.03, 0.014), (0.16, 0.024, 0.024), BLACK)
    p.box((-0.07, 0.09, 0.015), (0.048, 0.12, 0.028), HANDLE, turn=-0.26)
    p.box((-0.082, 0.152, 0.015), (0.056, 0.014, 0.032), BLACK, turn=-0.26)
    # The trigger guard, and the trigger in it.
    p.box((0.005, 0.07, 0.014), (0.06, 0.008, 0.018), BLACK)
    p.box((0.032, 0.055, 0.014), (0.008, 0.03, 0.018), BLACK)
    p.box((-0.005, 0.052, 0.014), (0.006, 0.02, 0.01), SILVER)
    # The sights, and the slide's grip lines.
    p.box((-0.078, -0.024, 0.017), (0.01, 0.01, 0.024), BLACK)
    p.box((0.12, -0.024, 0.017), (0.008, 0.008, 0.014), SILVER)
    for k in range(4):
        p.box((-0.06 + k * 0.012, 0.0, 0.033), (0.004, 0.036, 0.004), BLACK)
    p.finish()


def gold_bar():
    p = Part("ITEM_GoldBar")
    m = Matrix.Translation((0, 0, 0.03)) @ Matrix.Diagonal((0.1, 0.045, 0.03, 1.0))
    verts = bmesh.ops.create_cube(p.bm, size=2.0, matrix=m)["verts"]
    for v in verts:
        if v.co.z > 0.03:
            v.co.x *= 0.82
            v.co.y *= 0.75
    p.paint(verts, GOLD, GOLD_DARK)
    p.finish()


def main():
    bandage()
    medkit()
    ammo()
    beans()
    water()
    pills()
    cash()
    watch()
    ring()
    chain()
    radio()
    battery()
    fuel()
    key()
    gold_bar()
    pistol()
    bpy.ops.export_scene.gltf(filepath=os.path.abspath(OUT), export_format="GLB", export_yup=True, export_apply=False, export_animations=False, export_vertex_color="ACTIVE", export_normals=True)
    print(f"items: {len(bpy.data.objects)} -> {os.path.abspath(OUT)}")


main()
