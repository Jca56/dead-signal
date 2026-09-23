"""The title screen's backdrop: a clearing in a pine forest, a dead radio
tower with its beacon, a shack at its foot and a line of power poles.

Run headless from the project root:
    /opt/blender-bin-5.2.1/blender -b --factory-startup --python assets/blender/title_scene.py

Writes assets/models/title_scene.glb. Colours live in per-face vertex
colours on one white material, so the whole scene is flat-shaded low poly;
the beacon alone has its own emissive material and is its own object, so
the game can make it blink. Blender is Z-up; the exporter turns it Y-up.
"""

import math
import os
import random

import sys

import bmesh
import bpy
from mathutils import Matrix, Vector

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from terrain import height, in_clearing  # noqa: E402
OUT = os.path.join(HERE, "..", "models", "title_scene.glb")

rng = random.Random(1987)

bpy.ops.wm.read_factory_settings(use_empty=True)


# ---- colour ---------------------------------------------------------------------

def srgb(r, g, b):
    return (r, g, b)


def jitter(c, amount=0.06):
    k = 1.0 + rng.uniform(-amount, amount)
    return tuple(min(1.0, max(0.0, v * k)) for v in c)


def mix(a, b, t):
    return tuple(x + (y - x) * t for x, y in zip(a, b))


GRASS = srgb(0.23, 0.26, 0.15)
DEAD_GRASS = srgb(0.36, 0.33, 0.21)
DIRT = srgb(0.28, 0.22, 0.16)
PINE = srgb(0.10, 0.17, 0.12)
PINE_DARK = srgb(0.07, 0.12, 0.09)
BARK = srgb(0.22, 0.16, 0.12)
DEAD_WOOD = srgb(0.36, 0.33, 0.30)
ROCK = srgb(0.40, 0.40, 0.38)
RUST = srgb(0.36, 0.20, 0.14)
STEEL = srgb(0.30, 0.30, 0.30)
SHACK_WALL = srgb(0.34, 0.37, 0.32)
SHACK_ROOF = srgb(0.20, 0.19, 0.18)
POLE = srgb(0.27, 0.21, 0.16)


# ---- the ground -------------------------------------------------------------------

SIZE = 260.0
CELLS = 52


def face_colours(bm, layer, pick):
    for f in bm.faces:
        c = pick(f)
        for loop in f.loops:
            loop[layer] = (*c, 1.0)


def new_object(name, bm, material):
    mesh = bpy.data.meshes.new(name)
    bm.to_mesh(mesh)
    bm.free()
    # Only our colours, and as the active set, so they export as COLOR_0.
    attrs = mesh.color_attributes
    for a in [a for a in attrs if a.name != "Col"]:
        attrs.remove(a)
    if "Col" in attrs:
        attrs.active_color = attrs["Col"]
        attrs.render_color_index = attrs.find("Col")
    mesh.materials.append(material)
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.scene.collection.objects.link(obj)
    return obj


def colour_layer(bm):
    return bm.loops.layers.color.new("Col")


def make_materials():
    flat = bpy.data.materials.new("Flat")
    flat.use_nodes = True
    nodes = flat.node_tree.nodes
    bsdf = nodes["Principled BSDF"]
    bsdf.inputs["Base Color"].default_value = (1, 1, 1, 1)
    bsdf.inputs["Roughness"].default_value = 1.0
    attr = nodes.new("ShaderNodeVertexColor")
    attr.layer_name = "Col"
    flat.node_tree.links.new(attr.outputs["Color"], bsdf.inputs["Base Color"])

    beacon = bpy.data.materials.new("Beacon")
    beacon.use_nodes = True
    b = beacon.node_tree.nodes["Principled BSDF"]
    b.inputs["Base Color"].default_value = (0.8, 0.05, 0.03, 1)
    b.inputs["Emission Color"].default_value = (1.0, 0.08, 0.04, 1)
    b.inputs["Emission Strength"].default_value = 4.0
    return flat, beacon


def ground(flat):
    bm = bmesh.new()
    step = SIZE / CELLS
    grid = []
    for j in range(CELLS + 1):
        row = []
        for i in range(CELLS + 1):
            x = -SIZE / 2 + i * step
            y = -SIZE / 2 + j * step
            if 0 < i < CELLS and 0 < j < CELLS:
                x += rng.uniform(-0.3, 0.3) * step
                y += rng.uniform(-0.3, 0.3) * step
            row.append(bm.verts.new((x, y, height(x, y))))
        grid.append(row)
    for j in range(CELLS):
        for i in range(CELLS):
            a, b, c, d = grid[j][i], grid[j][i + 1], grid[j + 1][i + 1], grid[j + 1][i]
            # Alternate the diagonal so the facets don't line up in stripes.
            if (i + j) % 2:
                bm.faces.new((a, b, c))
                bm.faces.new((a, c, d))
            else:
                bm.faces.new((a, b, d))
                bm.faces.new((b, c, d))
    layer = colour_layer(bm)

    def pick(f):
        centre = f.calc_center_median()
        t = 0.5 + 0.5 * math.sin(centre.x * 0.08 + math.cos(centre.y * 0.06) * 2.0)
        c = mix(GRASS, DEAD_GRASS, t * 0.8)
        # The worn track up to the tower.
        if abs(centre.x - (centre.y - 20.0) * 0.18 - 6.0) < 3.5 and -20 < centre.y < 48:
            c = mix(c, DIRT, 0.75)
        return jitter(c, 0.1)

    face_colours(bm, layer, pick)
    return new_object("Ground", bm, flat)


# ---- props ----------------------------------------------------------------------

def add_cone(bm, layer, base, radius, top_radius, h, sides, colour, spin=0.0):
    ring_a, ring_b = [], []
    for k in range(sides):
        a = spin + k / sides * math.tau
        ring_a.append(bm.verts.new(base + Vector((math.cos(a) * radius, math.sin(a) * radius, 0))))
        if top_radius > 0:
            ring_b.append(bm.verts.new(base + Vector((math.cos(a) * top_radius, math.sin(a) * top_radius, h))))
    faces = []
    if top_radius > 0:
        for k in range(sides):
            n = (k + 1) % sides
            faces.append(bm.faces.new((ring_a[k], ring_a[n], ring_b[n], ring_b[k])))
        faces.append(bm.faces.new(ring_b))
    else:
        tip = bm.verts.new(base + Vector((0, 0, h)))
        for k in range(sides):
            faces.append(bm.faces.new((ring_a[k], ring_a[(k + 1) % sides], tip)))
    faces.append(bm.faces.new(list(reversed(ring_a))))
    for f in faces:
        c = jitter(colour, 0.12)
        for loop in f.loops:
            loop[layer] = (*c, 1.0)


def add_box(bm, layer, matrix, size, colour):
    sx, sy, sz = (s / 2 for s in size)
    corners = [Vector((x, y, z)) for z in (-sz, sz) for y in (-sy, sy) for x in (-sx, sx)]
    v = [bm.verts.new(matrix @ c) for c in corners]
    # Each face's corners anticlockwise seen from outside, so it faces out.
    quads = [(0, 2, 3, 1), (4, 5, 7, 6), (0, 1, 5, 4), (2, 6, 7, 3), (0, 4, 6, 2), (1, 3, 7, 5)]
    for q in quads:
        f = bm.faces.new([v[i] for i in q])
        c = jitter(colour, 0.08)
        for loop in f.loops:
            loop[layer] = (*c, 1.0)


def beam(bm, layer, a, b, thickness, colour):
    """A square bar from `a` to `b`."""
    d = b - a
    length = d.length
    rot = d.to_track_quat("Z", "Y").to_matrix().to_4x4()
    m = Matrix.Translation((a + b) / 2) @ rot
    add_box(bm, layer, m, (thickness, thickness, length), colour)


def clear_of(x, y, spots):
    return all(math.hypot(x - sx, y - sy) > r for sx, sy, r in spots)


TOWER = Vector((12.0, 46.0, 0.0))
TOWER.z = height(TOWER.x, TOWER.y) - 0.2
SHACK = Vector((4.0, 40.0, 0.0))
SHACK.z = height(SHACK.x, SHACK.y)
# Places trees keep out of: the clearing, the tower, the shack, the track.
KEEP_OUT = [(0.0, 5.0, 22.0), (TOWER.x, TOWER.y, 10.0), (SHACK.x, SHACK.y, 8.0), (6.0, 25.0, 9.0)]


# Where the trees and poles ended up, for their collision shapes.
PINE_SPOTS = []
DEAD_SPOTS = []
POLE_SPOTS = []


def pines(flat):
    bm = bmesh.new()
    layer = colour_layer(bm)
    # A tree on the proving ground is still grown (so it draws the same
    # random numbers and every other tree stays put), into a mesh thrown away.
    junk = bmesh.new()
    junk_layer = colour_layer(junk)
    placed = 0
    while placed < 170:
        x = rng.uniform(-120, 120)
        y = rng.uniform(-60, 125)
        if not clear_of(x, y, KEEP_OUT):
            continue
        placed += 1
        s = rng.uniform(0.8, 1.5)
        base = Vector((x, y, height(x, y) - 0.2))
        into, into_layer = (junk, junk_layer) if in_clearing(x, y) else (bm, layer)
        if into is bm:
            PINE_SPOTS.append((base, s))
        add_cone(into, into_layer, base, 0.35 * s, 0.25 * s, 2.0 * s, 6, BARK)
        tiers = rng.choice((3, 3, 4))
        z = 1.4 * s
        r = 2.6 * s
        for t in range(tiers):
            colour = PINE if t % 2 == 0 else PINE_DARK
            add_cone(into, into_layer, base + Vector((0, 0, z)), r, 0, 3.4 * s, 7, colour, spin=rng.uniform(0, math.tau))
            z += 1.9 * s
            r *= 0.74
    junk.free()
    return new_object("Pines", bm, flat)


def dead_trees(flat):
    bm = bmesh.new()
    layer = colour_layer(bm)
    placed = 0
    while placed < 14:
        x = rng.uniform(-60, 60)
        y = rng.uniform(-20, 80)
        if not clear_of(x, y, KEEP_OUT[1:]) or math.hypot(x, y - 5) < 12:
            continue
        placed += 1
        s = rng.uniform(0.9, 1.4)
        base = Vector((x, y, height(x, y) - 0.2))
        top = base + Vector((rng.uniform(-0.6, 0.6), rng.uniform(-0.6, 0.6), 7.0 * s))
        DEAD_SPOTS.append((base, top, s))
        beam(bm, layer, base, top, 0.4 * s, DEAD_WOOD)
        for _ in range(3):
            t = rng.uniform(0.4, 0.85)
            start = base.lerp(top, t)
            a = rng.uniform(0, math.tau)
            end = start + Vector((math.cos(a) * 2.2 * s, math.sin(a) * 2.2 * s, rng.uniform(0.6, 1.8) * s))
            beam(bm, layer, start, end, 0.18 * s, DEAD_WOOD)
    return new_object("DeadTrees", bm, flat)


def rocks(flat):
    bm = bmesh.new()
    layer = colour_layer(bm)
    junk = bmesh.new()
    junk_layer = colour_layer(junk)
    for _ in range(45):
        x = rng.uniform(-90, 90)
        y = rng.uniform(-40, 110)
        if not clear_of(x, y, KEEP_OUT[1:]):
            continue
        s = rng.uniform(0.5, 1.8)
        m = Matrix.Translation((x, y, height(x, y) - 0.2 * s)) @ Matrix.Rotation(rng.uniform(0, math.tau), 4, "Z")
        m = m @ Matrix.Diagonal((s * rng.uniform(1.0, 1.6), s, s * rng.uniform(0.5, 0.8), 1.0))
        into, into_layer = (junk, junk_layer) if in_clearing(x, y) else (bm, layer)
        rock = bmesh.ops.create_icosphere(into, subdivisions=1, radius=1.0, matrix=m)
        for v in rock["verts"]:
            v.co += Vector((rng.uniform(-0.15, 0.15) for _ in range(3))) * s
        # In a fixed order (a set's is not), so the shading is the same
        # every time the scene is built.
        into.faces.index_update()
        faces = sorted({f for v in rock["verts"] for f in v.link_faces}, key=lambda f: f.index)
        for f in faces:
            c = jitter(ROCK, 0.12)
            for loop in f.loops:
                loop[into_layer] = (*c, 1.0)
    junk.free()
    return new_object("SOLID_Rocks", bm, flat)


TOWER_HEIGHT = 42.0


def tower(flat):
    """A four-legged lattice mast, tapering, braced in X panels."""
    bm = bmesh.new()
    layer = colour_layer(bm)
    levels = 10
    half = lambda z: 2.6 - 1.9 * (z / TOWER_HEIGHT)  # noqa: E731
    corners = [(1, 1), (-1, 1), (-1, -1), (1, -1)]
    rings = []
    for k in range(levels + 1):
        z = TOWER_HEIGHT * k / levels
        h = half(z)
        rings.append([TOWER + Vector((cx * h, cy * h, z)) for cx, cy in corners])
    for k in range(levels):
        colour = RUST if k % 3 else STEEL
        for c in range(4):
            beam(bm, layer, rings[k][c], rings[k + 1][c], 0.28, colour)
            n = (c + 1) % 4
            beam(bm, layer, rings[k + 1][c], rings[k + 1][n], 0.16, colour)
            beam(bm, layer, rings[k][c], rings[k + 1][n], 0.1, colour)
            beam(bm, layer, rings[k][n], rings[k + 1][c], 0.1, colour)
    top = TOWER + Vector((0, 0, TOWER_HEIGHT))
    beam(bm, layer, top, top + Vector((0, 0, 4.0)), 0.14, STEEL)
    # A dish, bent off true: something broke up here.
    dish = Matrix.Translation(top + Vector((0.9, -0.2, -4.0))) @ Matrix.Rotation(math.radians(70), 4, "Y") @ Matrix.Rotation(math.radians(20), 4, "X")
    ring = []
    for k in range(8):
        a = k / 8 * math.tau
        ring.append(bm.verts.new(dish @ Vector((math.cos(a) * 1.3, math.sin(a) * 1.3, 0.5))))
    centre = bm.verts.new(dish @ Vector((0, 0, 0)))
    for k in range(8):
        f = bm.faces.new((ring[k], ring[(k + 1) % 8], centre))
        for loop in f.loops:
            loop[layer] = (*jitter(STEEL, 0.05), 1.0)
    return new_object("Tower", bm, flat)


def beacon(material):
    bm = bmesh.new()
    top = TOWER + Vector((0, 0, TOWER_HEIGHT + 4.2))
    bmesh.ops.create_icosphere(bm, subdivisions=1, radius=0.45, matrix=Matrix.Translation(top))
    return new_object("Beacon", bm, material)


def shack(flat):
    bm = bmesh.new()
    layer = colour_layer(bm)
    turn = Matrix.Rotation(math.radians(18), 4, "Z")
    base = Matrix.Translation(SHACK) @ turn
    add_box(bm, layer, base @ Matrix.Translation((0, 0, 1.3)), (4.2, 3.2, 2.8), SHACK_WALL)
    roof = base @ Matrix.Translation((0, 0, 2.95)) @ Matrix.Rotation(math.radians(7), 4, "X")
    add_box(bm, layer, roof, (4.8, 3.9, 0.22), SHACK_ROOF)
    add_box(bm, layer, base @ Matrix.Translation((0.8, -1.62, 1.0)), (0.9, 0.06, 1.9), SHACK_ROOF)
    # A drum and a crate by the door.
    add_cone(bm, layer, (base @ Vector((-1.5, -2.3, 0))), 0.45, 0.45, 0.95, 8, RUST)
    add_box(bm, layer, base @ Matrix.Translation((2.6, -1.4, 0.4)) @ Matrix.Rotation(0.4, 4, "Z"), (0.9, 0.9, 0.8), BARK)
    return new_object("Shack", bm, flat)


def poles(flat):
    bm = bmesh.new()
    layer = colour_layer(bm)
    for k in range(7):
        y = SHACK.y - 8.0 - k * 16.0
        x = SHACK.x - 10.0 - k * 3.5
        base = Vector((x, y, height(x, y) - 0.3))
        lean = Vector((rng.uniform(-0.4, 0.4), rng.uniform(-0.4, 0.4), 8.5))
        POLE_SPOTS.append((base, base + lean))
        beam(bm, layer, base, base + lean, 0.28, POLE)
        top = base + lean * 0.93
        beam(bm, layer, top + Vector((-1.2, 0.2, 0)), top + Vector((1.2, -0.2, 0)), 0.16, POLE)
    return new_object("Poles", bm, flat)


# ---- collision ---------------------------------------------------------------
#
# Invisible shapes the game stands on and bumps into, named COL_*. Built
# after everything seen, so nothing here can change the random numbers
# the visible scene was made from.

def solids(flat):
    """Two objects: COL_Wood (trees, poles, the shack and its crate) and
    COL_Metal (the tower and the drum), so a bullet knows what it hit."""
    bm = bmesh.new()
    layer = colour_layer(bm)
    metal = bmesh.new()
    metal_layer = colour_layer(metal)
    grey = (0.5, 0.5, 0.5)
    # Pines: a metre-wide column through the trunk and lower branches.
    for base, s in PINE_SPOTS:
        add_cone(bm, layer, base, 0.5, 0.5, 6.0 * s, 8, grey)
    for base, top, s in DEAD_SPOTS:
        beam(bm, layer, base, top, 0.45 * s, grey)
    for base, top in POLE_SPOTS:
        beam(bm, layer, base, top, 0.32, grey)
    # The shack as a box, with its drum and crate.
    turn = Matrix.Rotation(math.radians(18), 4, "Z")
    at = Matrix.Translation(SHACK) @ turn
    add_box(bm, layer, at @ Matrix.Translation((0, 0, 1.5)), (4.4, 3.4, 3.2), grey)
    add_cone(metal, metal_layer, at @ Vector((-1.5, -2.3, 0)), 0.45, 0.45, 0.95, 8, grey)
    add_box(bm, layer, at @ Matrix.Translation((2.6, -1.4, 0.4)) @ Matrix.Rotation(0.4, 4, "Z"), (0.9, 0.9, 0.8), grey)
    # The tower's legs, and the X braces of its lowest panels: room to walk
    # in under the braces and stand inside it.
    half = lambda z: 2.6 - 1.9 * (z / TOWER_HEIGHT)  # noqa: E731
    corners = [(1, 1), (-1, 1), (-1, -1), (1, -1)]
    rings = [[TOWER + Vector((cx * half(z), cy * half(z), z)) for cx, cy in corners] for z in (0.0, TOWER_HEIGHT / 10, TOWER_HEIGHT * 0.3)]
    for c in range(4):
        n = (c + 1) % 4
        beam(metal, metal_layer, rings[0][c], rings[2][c], 0.4, grey)
        beam(metal, metal_layer, rings[0][c], rings[1][n], 0.18, grey)
        beam(metal, metal_layer, rings[0][n], rings[1][c], 0.18, grey)
    new_object("COL_Metal", metal, flat)
    return new_object("COL_Wood", bm, flat)


def main():
    flat, glow = make_materials()
    ground(flat)
    pines(flat)
    dead_trees(flat)
    rocks(flat)
    tower(flat)
    beacon(glow)
    shack(flat)
    poles(flat)
    solids(flat)
    bpy.ops.export_scene.gltf(
        filepath=os.path.abspath(OUT),
        export_format="GLB",
        export_yup=True,
        export_apply=False,
        export_animations=False,
        export_vertex_color="ACTIVE",
        export_normals=True,
    )
    tris = sum(len(o.data.polygons) for o in bpy.data.objects if o.type == "MESH")
    print(f"title_scene: {len(bpy.data.objects)} objects, {tris} faces -> {os.path.abspath(OUT)}")


main()
