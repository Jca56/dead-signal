"""What the map is dressed with, each its own object standing on its origin
(the game sets them down by the thousand, turned and scaled):

    SCENE_Pine_0..3     pines, 8-12 m, in tiers of branches
    SCENE_Dead_0..1     dead trees, bare, a few snapped limbs
    SCENE_Rock_0..3     boulders, lumpy, sunk a little
    SCENE_Log           a fallen trunk
    SCENE_Stump         a sawn stump
    SCENE_Pole          a power pole with its crossarm
    SCENE_Tower         the dead radio mast (42 m), its beacon apart as
                        SCENE_Beacon so the game can make it blink

and what the game bumps into for each (SCENE_*_Hull): plain shapes,
never drawn. Pines are solid up their trunk and lowest branches only.

Run headless from the project root:
    /opt/blender-bin-5.2.1/blender -b --factory-startup --python assets/blender/scenery.py

Writes assets/models/scenery.glb.
"""

import math
import os
import random

import bmesh
import bpy
from mathutils import Matrix, Vector

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "..", "models", "scenery.glb")

bpy.ops.wm.read_factory_settings(use_empty=True)
rng = random.Random(23)

# The title scene's own colours, so the map is the same forest.
PINE = (0.10, 0.17, 0.12)
PINE_DARK = (0.07, 0.12, 0.09)
BARK = (0.22, 0.16, 0.12)
DEAD_WOOD = (0.36, 0.33, 0.30)
ROCK = (0.40, 0.40, 0.38)
MOSS = (0.24, 0.27, 0.17)
RUST = (0.36, 0.20, 0.14)
STEEL = (0.30, 0.30, 0.30)
POLE = (0.27, 0.21, 0.16)
SAWN = (0.52, 0.42, 0.30)
GREY = (0.5, 0.5, 0.5)


def jitter(c, amount=0.06):
    k = 1.0 + rng.uniform(-amount, amount)
    return tuple(min(1.0, max(0.0, v * k)) for v in c)


def flat_material():
    mat = bpy.data.materials.new("Flat")
    mat.use_nodes = True
    nodes = mat.node_tree.nodes
    vc = nodes.new("ShaderNodeVertexColor")
    vc.layer_name = "Col"
    mat.node_tree.links.new(vc.outputs["Color"], nodes["Principled BSDF"].inputs["Base Color"])
    return mat


def glow_material():
    mat = bpy.data.materials.new("Beacon")
    mat.use_nodes = True
    b = mat.node_tree.nodes["Principled BSDF"]
    b.inputs["Base Color"].default_value = (0.8, 0.05, 0.03, 1)
    b.inputs["Emission Color"].default_value = (1.0, 0.08, 0.04, 1)
    b.inputs["Emission Strength"].default_value = 4.0
    return mat


class Part:
    def __init__(self, name, seed):
        rng.seed(seed)
        self.name = name
        self.bm = bmesh.new()
        self.col = self.bm.loops.layers.color.new("Col")

    def faces_of(self, verts):
        return sorted({f for v in verts for f in v.link_faces}, key=lambda f: f.index)

    def paint(self, verts, colour, amount=0.08):
        for f in self.faces_of(verts):
            c = jitter(colour, amount)
            for loop in f.loops:
                loop[self.col] = (*c, 1.0)

    def cone(self, base, radius, top_radius, h, sides, colour, spin=0.0):
        base = Vector(base)
        ring_a, ring_b = [], []
        for k in range(sides):
            a = spin + k / sides * math.tau
            ring_a.append(self.bm.verts.new(base + Vector((math.cos(a) * radius, math.sin(a) * radius, 0))))
            if top_radius > 0:
                ring_b.append(self.bm.verts.new(base + Vector((math.cos(a) * top_radius, math.sin(a) * top_radius, h))))
        made = list(ring_a) + list(ring_b)
        if top_radius > 0:
            for k in range(sides):
                n = (k + 1) % sides
                self.bm.faces.new((ring_a[k], ring_a[n], ring_b[n], ring_b[k]))
            self.bm.faces.new(ring_b)
        else:
            tip = self.bm.verts.new(base + Vector((0, 0, h)))
            made.append(tip)
            for k in range(sides):
                self.bm.faces.new((ring_a[k], ring_a[(k + 1) % sides], tip))
        self.bm.faces.new(list(reversed(ring_a)))
        self.paint(made, colour, 0.12)
        return made

    def box(self, lo, hi, colour, turn=None):
        c = Vector(((lo[0] + hi[0]) / 2, (lo[1] + hi[1]) / 2, (lo[2] + hi[2]) / 2))
        s = ((hi[0] - lo[0]) / 2, (hi[1] - lo[1]) / 2, (hi[2] - lo[2]) / 2)
        m = Matrix.Translation(c) @ (turn or Matrix.Identity(4)) @ Matrix.Diagonal((*s, 1.0))
        made = bmesh.ops.create_cube(self.bm, size=2.0, matrix=m)["verts"]
        self.paint(made, colour)
        return list(made)

    def beam(self, a, b, thickness, colour):
        """A square bar from `a` to `b`."""
        a, b = Vector(a), Vector(b)
        d = b - a
        m = Matrix.Translation((a + b) / 2) @ d.to_track_quat("Z", "Y").to_matrix().to_4x4() @ Matrix.Diagonal((thickness / 2, thickness / 2, d.length / 2, 1.0))
        made = bmesh.ops.create_cube(self.bm, size=2.0, matrix=m)["verts"]
        self.paint(made, colour)
        return list(made)

    def rod(self, a, b, radius, colour, sides=6):
        a, b = Vector(a), Vector(b)
        d = b - a
        m = Matrix.Translation((a + b) / 2) @ d.to_track_quat("Z", "Y").to_matrix().to_4x4()
        made = bmesh.ops.create_cone(self.bm, cap_ends=True, segments=sides, radius1=radius, radius2=radius, depth=d.length, matrix=m)["verts"]
        self.paint(made, colour)
        return list(made)

    def lump(self, centre, size, colour, subdivisions=1, rough=0.15):
        m = Matrix.Translation(centre) @ Matrix.Diagonal((*size, 1.0))
        made = bmesh.ops.create_icosphere(self.bm, subdivisions=subdivisions, radius=1.0, matrix=m)["verts"]
        for v in made:
            v.co += Vector([rng.uniform(-rough, rough) * s for s in size])
        self.bm.faces.index_update()
        self.paint(made, colour, 0.12)
        return made

    def finish(self, material):
        mesh = bpy.data.meshes.new(self.name)
        bmesh.ops.recalc_face_normals(self.bm, faces=self.bm.faces)
        self.bm.to_mesh(mesh)
        self.bm.free()
        attrs = mesh.color_attributes
        attrs.active_color = attrs["Col"]
        attrs.render_color_index = attrs.find("Col")
        mesh.materials.append(material)
        bpy.context.scene.collection.objects.link(bpy.data.objects.new(self.name, mesh))


def pine(i, flat):
    """Pines in tiers, each variant its own height and fullness."""
    p = Part(f"SCENE_Pine_{i}", f"pine{i}")
    s = (1.0, 1.15, 0.9, 1.3)[i]
    tiers = (3, 4, 3, 4)[i]
    p.cone((0, 0, -0.3), 0.35 * s, 0.25 * s, 2.3 * s, 6, BARK)
    z, r = 1.4 * s, (2.6, 2.3, 2.9, 2.4)[i] * s
    for t in range(tiers):
        p.cone((rng.uniform(-0.1, 0.1), rng.uniform(-0.1, 0.1), z), r, 0, 3.4 * s, 7, PINE if t % 2 == 0 else PINE_DARK, spin=rng.uniform(0, math.tau))
        z += 1.9 * s
        r *= 0.74
    p.finish(flat)
    h = Part(f"SCENE_Pine_{i}_Hull", f"pine{i}hull")
    h.cone((0, 0, -0.3), 0.5, 0.5, 6.3 * s, 8, GREY)
    h.finish(flat)


def dead(i, flat):
    p = Part(f"SCENE_Dead_{i}", f"dead{i}")
    s = (1.0, 1.3)[i]
    top = Vector((rng.uniform(-0.6, 0.6), rng.uniform(-0.6, 0.6), 7.0 * s))
    p.beam((0, 0, -0.3), top, 0.4 * s, DEAD_WOOD)
    for _ in range(3 + i):
        t = rng.uniform(0.4, 0.85)
        start = top * t
        a = rng.uniform(0, math.tau)
        end = start + Vector((math.cos(a) * 2.2 * s, math.sin(a) * 2.2 * s, rng.uniform(0.6, 1.8) * s))
        p.beam(start, end, 0.18 * s, DEAD_WOOD)
    p.finish(flat)
    h = Part(f"SCENE_Dead_{i}_Hull", f"dead{i}hull")
    h.beam((0, 0, -0.3), top, 0.45 * s, GREY)
    h.finish(flat)


def rock(i, flat):
    """A boulder, sunk a fifth of its height; its hull is itself."""
    size = ((1.6, 1.1, 0.8), (1.1, 1.0, 1.0), (2.4, 1.5, 1.1), (0.8, 0.7, 0.5))[i]
    for name in (f"SCENE_Rock_{i}", f"SCENE_Rock_{i}_Hull"):
        p = Part(name, f"rock{i}")
        p.lump(Vector((0, 0, size[2] * 0.6)), size, ROCK)
        # Moss on the top faces of the drawn one.
        if not name.endswith("Hull"):
            for f in p.bm.faces:
                if f.normal.z > 0.8 or f.calc_center_median().z > size[2] * 1.35:
                    c = jitter(MOSS, 0.1)
                    for loop in f.loops:
                        loop[p.col] = (*c, 1.0)
        p.finish(flat)


def log(flat):
    for name in ("SCENE_Log", "SCENE_Log_Hull"):
        p = Part(name, "log")
        p.rod((-3.0, 0, 0.3), (3.0, 0, 0.3), 0.35, BARK if not name.endswith("Hull") else GREY, sides=7)
        if not name.endswith("Hull"):
            p.rod((-3.02, 0, 0.3), (-2.98, 0, 0.3), 0.3, SAWN, sides=7)
            p.beam((1.0, 0, 0.55), (1.6, 0.9, 1.1), 0.12, BARK)
        p.finish(flat)


def stump(flat):
    p = Part("SCENE_Stump", "stump")
    p.cone((0, 0, -0.2), 0.5, 0.42, 0.75, 7, BARK)
    p.cone((0, 0, 0.55), 0.42, 0.40, 0.02, 7, SAWN)
    p.finish(flat)
    h = Part("SCENE_Stump_Hull", "stumphull")
    h.cone((0, 0, -0.2), 0.5, 0.45, 0.77, 7, GREY)
    h.finish(flat)


def pole(flat):
    """A leaning power pole, its crossarm across X (along the line)."""
    p = Part("SCENE_Pole", "pole")
    top = Vector((0.15, 0.1, 8.8))
    p.beam((0, 0, -0.4), top, 0.28, POLE)
    arm = top * 0.93
    p.beam(arm + Vector((0.1, -1.2, 0)), arm + Vector((-0.1, 1.2, 0)), 0.16, POLE)
    for y in (-1.0, 0.0, 1.0):
        p.box((arm.x - 0.05, arm.y + y - 0.05, arm.z + 0.08), (arm.x + 0.05, arm.y + y + 0.05, arm.z + 0.25), (0.55, 0.55, 0.5))
    p.finish(flat)
    h = Part("SCENE_Pole_Hull", "polehull")
    h.beam((0, 0, -0.4), top, 0.32, GREY)
    h.finish(flat)


TOWER_HEIGHT = 42.0


def tower(flat, glow):
    """A four-legged lattice mast, tapering, braced in X panels, a dish
    bent off true near the top: the title's tower."""
    p = Part("SCENE_Tower", "tower")
    levels = 10
    half = lambda z: 2.6 - 1.9 * (z / TOWER_HEIGHT)  # noqa: E731
    corners = [(1, 1), (-1, 1), (-1, -1), (1, -1)]
    rings = []
    for k in range(levels + 1):
        z = TOWER_HEIGHT * k / levels
        h = half(z)
        rings.append([Vector((cx * h, cy * h, z - 0.2)) for cx, cy in corners])
    for k in range(levels):
        colour = RUST if k % 3 else STEEL
        for c in range(4):
            p.beam(rings[k][c], rings[k + 1][c], 0.28, colour)
            n = (c + 1) % 4
            p.beam(rings[k + 1][c], rings[k + 1][n], 0.16, colour)
            p.beam(rings[k][c], rings[k + 1][n], 0.1, colour)
            p.beam(rings[k][n], rings[k + 1][c], 0.1, colour)
    top = Vector((0, 0, TOWER_HEIGHT - 0.2))
    p.beam(top, top + Vector((0, 0, 4.0)), 0.14, STEEL)
    dish = Matrix.Translation(top + Vector((0.9, -0.2, -4.0))) @ Matrix.Rotation(math.radians(70), 4, "Y") @ Matrix.Rotation(math.radians(20), 4, "X")
    ring = [p.bm.verts.new(dish @ Vector((math.cos(k / 8 * math.tau) * 1.3, math.sin(k / 8 * math.tau) * 1.3, 0.5))) for k in range(8)]
    centre = p.bm.verts.new(dish @ Vector((0, 0, 0)))
    for k in range(8):
        p.bm.faces.new((ring[k], ring[(k + 1) % 8], centre))
    p.paint(ring + [centre], STEEL, 0.05)
    p.finish(flat)
    b = Part("SCENE_Beacon", "beacon")
    b.lump(top + Vector((0, 0, 4.2)), (0.45, 0.45, 0.45), (0.8, 0.05, 0.03), rough=0.0)
    b.finish(glow)
    # Solid: the legs, and the X braces of the lowest panels (room to walk
    # in under the braces and stand inside it).
    h = Part("SCENE_Tower_Hull", "towerhull")
    low = [[Vector((cx * half(z), cy * half(z), z - 0.2)) for cx, cy in corners] for z in (0.0, TOWER_HEIGHT / 10, TOWER_HEIGHT * 0.3)]
    for c in range(4):
        n = (c + 1) % 4
        h.beam(low[0][c], low[2][c], 0.4, GREY)
        h.beam(low[0][c], low[1][n], 0.18, GREY)
        h.beam(low[0][n], low[1][c], 0.18, GREY)
    h.finish(flat)


def main():
    flat, glow = flat_material(), glow_material()
    for i in range(4):
        pine(i, flat)
    for i in range(2):
        dead(i, flat)
    for i in range(4):
        rock(i, flat)
    log(flat)
    stump(flat)
    pole(flat)
    tower(flat, glow)
    bpy.ops.export_scene.gltf(filepath=os.path.abspath(OUT), export_format="GLB", export_yup=True, export_apply=False, export_animations=False, export_vertex_color="ACTIVE", export_normals=True)
    tris = sum(len(o.data.polygons) for o in bpy.data.objects if o.type == "MESH")
    print(f"scenery: {len(bpy.data.objects)} objects, {tris} faces -> {os.path.abspath(OUT)}")


main()
