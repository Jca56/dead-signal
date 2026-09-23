"""Things to search: a wooden crate, a metal locker, a wrecked car, a
locked supply cage and a gun cabinet, each its own object (CONTAINER_Crate,
CONTAINER_Locker, CONTAINER_Car, CONTAINER_Cage, CONTAINER_GunCabinet);
and in houses and stores (`indoor_containers.py`) a fridge, a chest
of drawers (CONTAINER_Cabinet), a desk, a wardrobe, a store's shelving
and its till counter (CONTAINER_Register): each standing on its origin with its front (the
side it's searched from) facing +Y, for the game to set down where it likes;
and each again as it's left once searched (CONTAINER_*_Open): the crate's
lid off and leant against it, the locker's door and the cage's swung wide,
the car's boot up. And each as the game bumps into it (CONTAINER_*_Hull): a
few plain boxes, never drawn, so a crowd pressing round one, or a look
across it, meets a dozen faces rather than hundreds.
Every face is solid to the game: the cage's bars stop bodies but let shots
through their gaps.

Run headless from the project root:
    /opt/blender-bin-5.2.1/blender -b --factory-startup --python assets/blender/containers.py

Writes assets/models/containers.glb.
"""

import math
import os
import sys

import bpy
from mathutils import Matrix

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from container_kit import BRASS, CHROME, GLASS, HOLLOW, HUB, KIT_RED, LAMP, LOCKER, LOCKER_DARK, PAINT, PLANK, PLANK_DARK, ROPE, RUST, SLOT, STEEL, STEEL_DARK, STRAW, TAIL, TYRE, hull, named, swing  # noqa: E402
from indoor_containers import cabinet, desk, fridge, register, shelf, wardrobe  # noqa: E402

OUT = os.path.join(HERE, "..", "models", "containers.glb")

bpy.ops.wm.read_factory_settings(use_empty=True)


def crate(opened):
    """A slatted wooden crate, 1.0 × 0.7 × 0.62, rope handles at its ends;
    opened, its lid leans against its end and straw shows inside."""
    p = named("CONTAINER_Crate", opened)
    w, d, h = 1.0, 0.7, 0.62
    p.box((-w / 2 + 0.02, -d / 2 + 0.02, 0.0), (w / 2 - 0.02, d / 2 - 0.02, h - 0.02), PLANK_DARK)
    # Slats round the sides, a gap between each.
    for k in range(4):
        z0 = 0.02 + k * 0.15
        p.box((-w / 2 + 0.03, -d / 2, z0), (w / 2 - 0.03, d / 2, z0 + 0.13), PLANK)
    # The frame at its corners, and the lid's boards.
    for x in (-w / 2, w / 2 - 0.06):
        for y in (-d / 2, d / 2 - 0.06):
            p.box((x, y, 0.0), (x + 0.06, y + 0.06, h), PLANK_DARK)
    lid = []
    for k in range(5):
        x0 = -w / 2 + 0.02 + k * 0.193
        lid += p.box((x0, -d / 2 + 0.02, h - 0.03), (x0 + 0.18, d / 2 - 0.02, h), PLANK)
    if opened:
        p.box((-w / 2 + 0.06, -d / 2 + 0.06, h - 0.035), (w / 2 - 0.06, d / 2 - 0.06, h - 0.015), STRAW)
        # Off, and leant against its end: turned about the top edge until
        # its far edge (0.98 m off) rests on the ground.
        swing(lid, (w / 2, 0.0, h), "Y", -141.0)
        for v in lid:
            v.co.x += 0.05
    # A diagonal brace across its front.
    brace = Matrix.Rotation(math.atan2(h - 0.1, w - 0.1), 4, "Y")
    p.box((-0.55, d / 2, h / 2 - 0.035), (0.55, d / 2 + 0.03, h / 2 + 0.035), PLANK_DARK, brace)
    for x in (-w / 2 - 0.03, w / 2):
        p.box((x, -0.12, h * 0.55), (x + 0.03, 0.12, h * 0.62), ROPE)
    p.finish()


def locker(opened):
    """A tall steel locker, 0.6 × 0.5 × 1.9, vents at the top and bottom of
    its door, a dent low on one side; opened, its door swung wide on a bare
    shelf."""
    p = named("CONTAINER_Locker", opened)
    w, d, h = 0.6, 0.5, 1.9
    p.box((-w / 2, -d / 2, 0.08), (w / 2, d / 2, h), LOCKER)
    for x in (-w / 2, w / 2 - 0.05):
        for y in (-d / 2, d / 2 - 0.05):
            p.box((x, y, 0.0), (x + 0.05, y + 0.05, 0.08), LOCKER_DARK)
    # The door, a hair proud of the front, and what's on it.
    door = p.box((-w / 2 + 0.03, d / 2, 0.12), (w / 2 - 0.03, d / 2 + 0.015, h - 0.04), LOCKER)
    for z in (1.62, 1.68, 1.74, 0.24, 0.30, 0.36):
        door += p.box((-0.15, d / 2 + 0.015, z), (0.15, d / 2 + 0.02, z + 0.025), SLOT)
    door += p.box((0.20, d / 2 + 0.015, 0.95), (0.23, d / 2 + 0.05, 1.12), CHROME)
    if opened:
        # Inside: dark, a shelf, a hook.
        p.box((-w / 2 + 0.03, d / 2 - 0.004, 0.12), (w / 2 - 0.03, d / 2 + 0.001, h - 0.04), HOLLOW)
        p.box((-w / 2 + 0.03, d / 2 - 0.004, 1.42), (w / 2 - 0.03, d / 2 + 0.006, 1.45), LOCKER)
        p.box((-0.02, d / 2 - 0.004, 1.25), (0.02, d / 2 + 0.03, 1.28), CHROME)
        swing(door, (-w / 2 + 0.03, d / 2, 0.0), "Z", 110.0)
    p.box((-w / 2 - 0.01, -0.1, 0.4), (-w / 2 + 0.01, 0.15, 0.62), LOCKER_DARK)
    p.box((-w / 2, -d / 2, h), (w / 2, d / 2, h + 0.02), LOCKER_DARK)
    p.finish()


def car(opened):
    """A wrecked sedan along X (its boot at -X), 4.3 × 1.75, on four
    wheels, one of them flat, so it lists; paint gone to rust in patches;
    opened, its boot lid up on a dark hollow."""
    p = named("CONTAINER_Car", opened)
    L, W = 4.3, 1.75
    lift = 0.30
    # Body: the lower hull, then the cabin narrowing to its roof.
    p.box((-L / 2, -W / 2, lift), (L / 2, W / 2, lift + 0.52), PAINT)
    made = p.box((-1.15, -W / 2 + 0.06, lift + 0.52), (1.0, W / 2 - 0.06, lift + 1.02), GLASS)
    for v in made:
        if v.co.z > lift + 0.8:
            v.co.x = -0.85 if v.co.x < 0 else 0.55
            v.co.y *= 0.88
    top = max((f for f in p.faces_of(made)), key=lambda f: f.calc_center_median().z)
    p.paint([top], PAINT)
    # Pillars at the cabin's corners, over the glass.
    for x0 in (-1.16, 0.95):
        for y in (-W / 2 + 0.05, W / 2 - 0.09):
            post = p.box((x0, y, lift + 0.52), (x0 + 0.06, y + 0.04, lift + 1.02), PAINT)
            for v in post:
                if v.co.z > lift + 0.8:
                    v.co.x += (0.3 if x0 < 0 else -0.46) * 1.0
                    v.co.y *= 0.9
    # Bumpers, lamps, the boot's seam, rust.
    for x in (-L / 2 - 0.06, L / 2):
        p.box((x, -W / 2 + 0.05, lift + 0.05), (x + 0.06, W / 2 - 0.05, lift + 0.2), CHROME)
    for y in (-0.62, 0.62):
        p.box((L / 2, y - 0.16, lift + 0.3), (L / 2 + 0.01, y + 0.16, lift + 0.42), LAMP)
        p.box((-L / 2 - 0.01, y - 0.16, lift + 0.3), (-L / 2, y + 0.16, lift + 0.42), TAIL)
    p.box((-1.5, -W / 2 + 0.05, lift + 0.52), (-1.48, W / 2 - 0.05, lift + 0.525), STEEL_DARK)
    if opened:
        p.box((-L / 2 + 0.06, -W / 2 + 0.1, lift + 0.52), (-1.52, W / 2 - 0.1, lift + 0.524), HOLLOW)
        boot = p.box((-L / 2, -W / 2 + 0.05, lift + 0.525), (-1.5, W / 2 - 0.05, lift + 0.555), PAINT)
        swing(boot, (-1.5, 0.0, lift + 0.555), "Y", 72.0)
    for x, y, z, sx, sz in ((1.2, W / 2, 0.45, 0.5, 0.2), (-0.4, W / 2, 0.35, 0.35, 0.25), (-1.7, -W / 2, 0.5, 0.45, 0.22), (0.6, -W / 2, 0.4, 0.3, 0.3)):
        side = 0.005 if y > 0 else -0.005
        p.box((x - sx / 2, y, lift + z - sz / 2), (x + sx / 2, y + side, lift + z + sz / 2), RUST)
    p.box((1.2, -0.5, lift + 0.52), (2.0, 0.3, lift + 0.525), RUST)
    # Wheels; the front left flat.
    for x in (-1.35, 1.35):
        for y in (-W / 2 + 0.05, W / 2 - 0.05):
            flat = x > 0 and y > 0
            r = 0.26 if flat else 0.33
            p.wheel((x, y, r), r, 0.22, TYRE, HUB)
    for v in p.bm.verts:
        v.co = Matrix.Rotation(math.radians(-2.5), 4, "X") @ Matrix.Rotation(math.radians(-1.5), 4, "Y") @ v.co
    p.finish()


def cage(opened):
    """A steel supply cage, 1.6 × 1.0 × 2.0: posts, rails and bars on every
    side, a padlocked door at the front, supplies stacked within; opened,
    the door swung out and the padlock gone."""
    p = named("CONTAINER_Cage", opened)
    w, d, h = 1.6, 1.0, 2.0
    p.box((-w / 2, -d / 2, 0.0), (w / 2, d / 2, 0.05), STEEL_DARK)
    for x in (-w / 2, w / 2 - 0.06):
        for y in (-d / 2, d / 2 - 0.06):
            p.box((x, y, 0.0), (x + 0.06, y + 0.06, h), STEEL)
    door = []
    for z in (0.05, h / 2 - 0.02, h - 0.05):
        p.box((-w / 2, -d / 2, z), (w / 2, -d / 2 + 0.04, z + 0.04), STEEL)
        # The front's rails: either side of the door, and the door's own.
        y = d / 2 - 0.04
        p.box((-w / 2, y, z), (-0.42, y + 0.04, z + 0.04), STEEL)
        p.box((0.42, y, z), (w / 2, y + 0.04, z + 0.04), STEEL)
        door += p.box((-0.40, y + 0.045, z), (0.40, y + 0.075, z + 0.04), STEEL)
        for x in (-w / 2, w / 2 - 0.04):
            p.box((x, -d / 2, z), (x + 0.04, d / 2, z + 0.04), STEEL)
    bar = 0.022
    n = 12
    for k in range(1, n):
        x = -w / 2 + k * w / n
        p.box((x - bar / 2, -d / 2 + 0.01, 0.05), (x + bar / 2, -d / 2 + 0.01 + bar, h - 0.05), STEEL_DARK)
        made = p.box((x - bar / 2, d / 2 - 0.03, 0.05), (x + bar / 2, d / 2 - 0.03 + bar, h - 0.05), STEEL_DARK)
        if -0.42 < x < 0.42:
            door += made
    for k in range(1, 8):
        y = -d / 2 + k * d / 8
        for x in (-w / 2 + 0.01, w / 2 - 0.03):
            p.box((x, y - bar / 2, 0.05), (x + bar, y + bar / 2, h - 0.05), STEEL_DARK)
        p.box((-w / 2, y - bar / 2, h - 0.03), (w / 2, y + bar / 2, h), STEEL_DARK)
    # The door's frame, its padlock.
    p.box((-0.44, d / 2 - 0.02, 0.05), (-0.40, d / 2 + 0.03, h - 0.05), STEEL)
    door += p.box((0.38, d / 2 - 0.02, 0.05), (0.42, d / 2 + 0.03, h - 0.05), STEEL)
    if opened:
        swing(door, (-0.42, d / 2 + 0.03, 0.0), "Z", 100.0)
    else:
        p.box((0.30, d / 2 + 0.03, 1.0), (0.40, d / 2 + 0.06, 1.12), BRASS)
        p.box((0.32, d / 2 + 0.035, 1.12), (0.38, d / 2 + 0.055, 1.19), CHROME)
    # Inside: a crate, a medkit on it, ammo cans.
    p.box((-0.65, -0.35, 0.05), (-0.05, 0.25, 0.5), PLANK)
    p.box((-0.52, -0.2, 0.5), (-0.2, 0.05, 0.62), KIT_RED)
    for k in range(3):
        p.box((0.1 + k * 0.18, -0.35, 0.05), (0.25 + k * 0.18, -0.05, 0.28), (0.29, 0.31, 0.20))
    p.finish()


CABINET_WOOD = (0.36, 0.22, 0.13)
CABINET_DARK = (0.22, 0.13, 0.08)
BAIZE = (0.20, 0.28, 0.17)
GUN_WOOD = (0.44, 0.27, 0.14)
GUN_METAL = (0.15, 0.15, 0.16)
GLINT = (0.62, 0.68, 0.70)


def gun_cabinet(opened):
    """A gun cabinet, 0.8 × 0.45 × 1.9, dark wood, a glass door at its
    front, long guns racked in it on green baize and a drawer under them;
    opened, the door swung wide and the rack bare."""
    p = named("CONTAINER_GunCabinet", opened)
    w, d, h = 0.8, 0.45, 1.9
    # The carcass: back, sides, top, base, and the drawer under the rack.
    p.box((-w / 2, -d / 2, 0.0), (w / 2, -d / 2 + 0.03, h), CABINET_DARK)
    for x in (-w / 2, w / 2 - 0.03):
        p.box((x, -d / 2, 0.0), (x + 0.03, d / 2 - 0.02, h), CABINET_WOOD)
    p.box((-w / 2 - 0.02, -d / 2, h), (w / 2 + 0.02, d / 2 + 0.01, h + 0.05), CABINET_DARK)
    p.box((-w / 2, -d / 2, 0.0), (w / 2, d / 2 - 0.02, 0.08), CABINET_DARK)
    p.box((-w / 2 + 0.03, -d / 2 + 0.03, 0.08), (w / 2 - 0.03, d / 2 - 0.02, 0.4), CABINET_WOOD)
    p.box((-0.08, d / 2 - 0.02, 0.22), (0.08, d / 2 + 0.01, 0.25), BRASS)
    p.box((-w / 2 + 0.03, -d / 2 + 0.03, 0.4), (w / 2 - 0.03, -d / 2 + 0.035, h), BAIZE)
    p.box((-w / 2 + 0.03, -d / 2 + 0.03, 0.4), (w / 2 - 0.03, d / 2 - 0.05, 0.43), CABINET_DARK)
    # The rack: a notched bar high up, and the guns stood in it.
    p.box((-w / 2 + 0.03, -d / 2 + 0.03, 1.45), (w / 2 - 0.03, -d / 2 + 0.12, 1.5), CABINET_DARK)
    if not opened:
        for x in (-0.24, 0.0, 0.24):
            p.box((x - 0.035, -d / 2 + 0.06, 0.43), (x + 0.035, -d / 2 + 0.14, 0.95), GUN_WOOD)
            p.box((x - 0.018, -d / 2 + 0.07, 0.95), (x + 0.018, -d / 2 + 0.12, 1.72), GUN_METAL)
    # The door: a frame round a pane of glass, hinged at the left.
    y0, y1 = d / 2 - 0.02, d / 2 + 0.01
    door = []
    door += p.box((-w / 2, y0, 0.42), (-w / 2 + 0.06, y1, h), CABINET_WOOD)
    door += p.box((w / 2 - 0.06, y0, 0.42), (w / 2, y1, h), CABINET_WOOD)
    door += p.box((-w / 2, y0, 0.42), (w / 2, y1, 0.5), CABINET_WOOD)
    door += p.box((-w / 2, y0, h - 0.08), (w / 2, y1, h), CABINET_WOOD)
    # The glass isn't drawn (nothing in the world is see-through), only a
    # glint or two on it, so what's racked behind shows.
    glint = Matrix.Rotation(math.radians(35), 4, "Y")
    for x, z, length in ((-0.18, 1.35, 0.35), (-0.08, 1.2, 0.2)):
        door += p.box((x - 0.006, y0 + 0.012, z - length / 2), (x + 0.006, y0 + 0.016, z + length / 2), GLINT, glint)
    door += p.box((w / 2 - 0.1, y1, 1.05), (w / 2 - 0.08, y1 + 0.03, 1.2), BRASS)
    if opened:
        swing(door, (-w / 2, y1, 0.0), "Z", 105.0)
    p.finish()


def hulls():
    hull("CONTAINER_Crate", [((-0.5, -0.35, 0.0), (0.5, 0.35, 0.62))])
    hull("CONTAINER_Locker", [((-0.3, -0.25, 0.0), (0.3, 0.27, 1.92))])
    hull("CONTAINER_Fridge", [((-0.375, -0.35, 0.0), (0.375, 0.35, 1.8))])
    hull("CONTAINER_Cabinet", [((-0.52, -0.25, 0.0), (0.52, 0.27, 0.93))])
    hull("CONTAINER_Desk", [((-0.65, -0.325, 0.0), (0.65, 0.325, 0.78))])
    hull("CONTAINER_Wardrobe", [((-0.62, -0.3, 0.0), (0.62, 0.3, 2.05))])
    hull("CONTAINER_Shelf", [((-0.9, -0.3, 0.0), (0.9, 0.3, 1.6))])
    hull("CONTAINER_Register", [((-1.03, -0.43, 0.0), (1.03, 0.43, 1.0))])
    hull("CONTAINER_GunCabinet", [((-0.42, -0.24, 0.0), (0.42, 0.25, 1.95))])
    lean = Matrix.Rotation(math.radians(-2.5), 4, "X") @ Matrix.Rotation(math.radians(-1.5), 4, "Y")
    hull("CONTAINER_Car", [((-2.2, -0.875, 0.05), (2.2, 0.875, 0.82)), ((-1.0, -0.78, 0.82), (0.75, 0.78, 1.32))], lean)
    w, d, h, t = 1.6, 1.0, 2.0, 0.05
    hull("CONTAINER_Cage", [
        ((-w / 2, -d / 2, 0.0), (w / 2, d / 2, t)),
        ((-w / 2, -d / 2, 0.0), (w / 2, -d / 2 + t, h)),
        ((-w / 2, d / 2 - t, 0.0), (w / 2, d / 2 + 0.03, h)),
        ((-w / 2, -d / 2, 0.0), (-w / 2 + t, d / 2, h)),
        ((w / 2 - t, -d / 2, 0.0), (w / 2, d / 2, h)),
        ((-w / 2, -d / 2, h - t), (w / 2, d / 2, h)),
    ])


def main():
    for opened in (False, True):
        crate(opened)
        locker(opened)
        car(opened)
        cage(opened)
        fridge(opened)
        cabinet(opened)
        desk(opened)
        wardrobe(opened)
        shelf(opened)
        register(opened)
        gun_cabinet(opened)
    hulls()
    bpy.ops.export_scene.gltf(filepath=os.path.abspath(OUT), export_format="GLB", export_yup=True, export_apply=False, export_animations=False, export_vertex_color="ACTIVE", export_normals=True)
    print(f"containers: {len(bpy.data.objects)} -> {os.path.abspath(OUT)}")


main()
