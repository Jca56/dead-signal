"""Everything that can be carried: supplies (ITEM_Bandage, ITEM_Medkit,
ITEM_Ammo), food and water (ITEM_Beans, ITEM_Water), valuables (ITEM_Pills,
ITEM_Cash, ITEM_Watch, ITEM_Ring, ITEM_Chain, ITEM_Radio, ITEM_Battery,
ITEM_Fuel, ITEM_GoldBar), the cage's key (ITEM_Key) and the armory's (ITEM_ArmoryKey), the weapons
(ITEM_Pistol, ITEM_Shotgun, ITEM_Rifle, ITEM_Knife, ITEM_Machete, ITEM_Axe)
and the guns' rounds (ITEM_Shells, ITEM_RifleRounds). Each its own object
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
import sys

import bmesh
import bpy
from mathutils import Matrix

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
OUT = os.path.join(HERE, "..", "models", "items.glb")

bpy.ops.wm.read_factory_settings(use_empty=True)

from item_kit import *  # noqa: E402,F403
import items_guns  # noqa: E402
import items_throw  # noqa: E402


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


STEEL_BLADE = (0.55, 0.56, 0.55)
EDGE = (0.82, 0.82, 0.80)
AXE_RED = (0.62, 0.10, 0.08)


def knife():
    """A tactical knife lying flat, point along +X."""
    p = Part("ITEM_Knife")
    z = 0.012
    p.box((-0.08, 0.0, z), (0.11, 0.028, 0.022), BLACK)
    p.box((-0.02, 0.0, z), (0.012, 0.06, 0.02), BLACK)
    p.box((0.075, -0.002, z - 0.004), (0.15, 0.026, 0.006), STEEL_BLADE)
    p.box((0.07, 0.011, z - 0.004), (0.13, 0.006, 0.007), EDGE)
    p.finish()


def machete():
    """A machete lying flat, its blade along +X, the edge to the front."""
    p = Part("ITEM_Machete")
    z = 0.012
    p.box((-0.24, 0.0, z), (0.13, 0.034, 0.026), WALNUT)
    p.box((0.1, 0.0, z - 0.006), (0.46, 0.05, 0.006), STEEL_BLADE)
    p.box((0.1, 0.027, z - 0.006), (0.44, 0.008, 0.007), EDGE)
    p.finish()


def axe():
    """A fire axe lying flat, the haft along X, the head at +X, its blade
    to the front."""
    p = Part("ITEM_Axe")
    z = 0.02
    p.box((-0.05, 0.0, z), (0.72, 0.045, 0.035), WALNUT)
    p.box((0.34, 0.02, z), (0.1, 0.22, 0.03), AXE_RED)
    p.box((0.34, 0.14, z), (0.14, 0.03, 0.02), STEEL_BLADE)
    p.box((0.34, -0.1, z), (0.03, 0.08, 0.02), AXE_RED)
    p.finish()


def armory_key():
    """A heavy steel key, its ring, a red tag."""
    p = Part("ITEM_ArmoryKey")
    p.hoop((-0.045, 0, 0.008), 0.026, 0.01, SILVER, segments=10)
    p.box((0.025, 0, 0.008), (0.09, 0.018, 0.016), SILVER)
    for x, h in ((0.05, 0.024), (0.066, 0.032), (0.082, 0.02)):
        p.box((x, -0.012 - h / 2, 0.008), (0.014, h, 0.016), SILVER)
    p.box((-0.08, 0.04, 0.005), (0.06, 0.05, 0.01), AXE_RED, turn=0.4)
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
    items_guns.pistol()
    items_guns.shotgun()
    items_guns.shells()
    items_guns.rifle()
    items_guns.rifle_rounds()
    items_guns.smg()
    items_guns.assault_rifle()
    items_guns.rounds_556()
    items_throw.molotov()
    items_throw.pipe_bomb()
    knife()
    machete()
    axe()
    armory_key()
    bpy.ops.export_scene.gltf(filepath=os.path.abspath(OUT), export_format="GLB", export_yup=True, export_apply=False, export_animations=False, export_vertex_color="ACTIVE", export_normals=True)
    print(f"items: {len(bpy.data.objects)} -> {os.path.abspath(OUT)}")


main()
