"""The proving ground: a concrete pad behind the spawn with everything the
character controller has to get right, measured.

Run headless from the project root:
    /opt/blender-bin-5.2.1/blender -b --factory-startup --python assets/blender/proving_ground.py

Writes assets/models/proving_ground.glb. Every object is named SOLID_*:
drawn, and stood on or bumped into. Laid out on the pad with +Y towards
the spawn (and the tower beyond it); the way up is a ramp on the east side.

    stairs    0.2 m steps (walk), 0.4 m (the limit), 0.45 m (jump it)
    ramps     30° (walk), 45° (just), 50° (slide back)
    crates    0.8 m and 1.6 m to jump onto
    tunnel    1.3 m inside: crouch through, and no standing up in it
    gap       0.8 m between two walls: just wide enough
    building  a doorway, a room, and stairs outside up to a flat roof
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
from terrain import LANE_FROM, LANE_PLATES, LANE_Y, PAD_HALF, PAD_X, PAD_Y, height  # noqa: E402

OUT = os.path.join(HERE, "..", "models", "proving_ground.glb")

rng = random.Random(4141)
bpy.ops.wm.read_factory_settings(use_empty=True)

CONCRETE = (0.46, 0.46, 0.44)
CONCRETE_DARK = (0.36, 0.36, 0.35)
STEP = (0.54, 0.53, 0.50)
WOOD = (0.42, 0.33, 0.23)
CRATE = (0.47, 0.37, 0.24)
WALL = (0.45, 0.39, 0.34)
ROOF = (0.24, 0.23, 0.22)
RUST = (0.40, 0.23, 0.15)
BURLAP = (0.55, 0.47, 0.33)
POST = (0.33, 0.25, 0.17)
STEEL = (0.52, 0.52, 0.50)
FRAME = (0.20, 0.20, 0.21)

# The pad's top: level with the highest ground under it, so no hill pokes up
# through it.
SAMPLES = [(PAD_X + x, PAD_Y + y) for x in range(-12, 13, 2) for y in range(-12, 13, 2)]
TOP = max(height(x, y) for x, y in SAMPLES) + 0.05
BOTTOM = min(height(x, y) for x, y in SAMPLES) - 1.5


def world(x, y, z):
    """Pad-local (metres from the pad's middle, up from its top) to Blender."""
    return Vector((PAD_X + x, PAD_Y + y, TOP + z))


def jitter(c, amount=0.05):
    k = 1.0 + rng.uniform(-amount, amount)
    return tuple(min(1.0, max(0.0, v * k)) for v in c)


class Part:
    """One named object, closed solids only, a colour per face. A `local`
    part is built about its own origin and set down at `at`, turned `yaw`
    degrees: something the game moves (a target that tips or swings)."""

    def __init__(self, name, at=None, yaw=0.0):
        self.name = name
        self.bm = bmesh.new()
        self.col = self.bm.loops.layers.color.new("Col")
        self.at = at
        self.yaw = yaw

    def paint(self, faces, colour):
        for f in faces:
            c = jitter(colour)
            for loop in f.loops:
                loop[self.col] = (*c, 1.0)

    def block(self, lo, hi, colour):
        """A box between corners `lo` and `hi`: pad-local, or the part's
        own for a local part."""
        place = (lambda x, y, z: Vector((x, y, z))) if self.at is not None else world
        a, b = place(*lo), place(*hi)
        centre = (a + b) / 2
        size = b - a
        m = Matrix.Translation(centre) @ Matrix.Diagonal((size.x / 2, size.y / 2, size.z / 2, 1.0))
        made = bmesh.ops.create_cube(self.bm, size=2.0, matrix=m)
        faces = {f for v in made["verts"] for f in v.link_faces}
        self.paint(sorted(faces, key=lambda f: f.calc_center_median().to_tuple()), colour)

    def wedge(self, x0, x1, y_foot, y_top, rise, colour, z0=0.0):
        """A ramp across x0..x1, from the ground at `y_foot` up to `rise`
        at `y_top`, solid beneath."""
        foot0, foot1 = world(x0, y_foot, z0), world(x1, y_foot, z0)
        base0, base1 = world(x0, y_top, z0), world(x1, y_top, z0)
        top0, top1 = world(x0, y_top, z0 + rise), world(x1, y_top, z0 + rise)
        v = [self.bm.verts.new(p) for p in (foot0, foot1, base0, base1, top0, top1)]
        faces = [
            self.bm.faces.new((v[0], v[1], v[5], v[4])),  # the slope
            self.bm.faces.new((v[2], v[3], v[1], v[0])),  # underneath
            self.bm.faces.new((v[4], v[5], v[3], v[2])),  # the back
            self.bm.faces.new((v[0], v[4], v[2])),
            self.bm.faces.new((v[1], v[3], v[5])),
        ]
        bmesh.ops.recalc_face_normals(self.bm, faces=faces)
        self.paint(faces, colour)

    def finish(self, material):
        mesh = bpy.data.meshes.new(self.name)
        bmesh.ops.recalc_face_normals(self.bm, faces=self.bm.faces)
        self.bm.to_mesh(mesh)
        self.bm.free()
        attrs = mesh.color_attributes
        attrs.active_color = attrs["Col"]
        attrs.render_color_index = attrs.find("Col")
        mesh.materials.append(material)
        obj = bpy.data.objects.new(self.name, mesh)
        if self.at is not None:
            obj.location = self.at
            obj.rotation_euler = (0.0, 0.0, math.radians(self.yaw))
        bpy.context.scene.collection.objects.link(obj)


def flat_material():
    m = bpy.data.materials.new("Flat")
    m.use_nodes = True
    nodes = m.node_tree.nodes
    bsdf = nodes["Principled BSDF"]
    bsdf.inputs["Roughness"].default_value = 1.0
    vc = nodes.new("ShaderNodeVertexColor")
    vc.layer_name = "Col"
    m.node_tree.links.new(vc.outputs["Color"], bsdf.inputs["Base Color"])
    return m


def access_ramp(p):
    """Down the east side: from the pad's edge to the ground, 4 m wide."""
    h = PAD_HALF
    x_end = h + 7.0
    drop = TOP - height(PAD_X + x_end, PAD_Y) + 0.1
    a = world(h, -2.0, 0.0)
    b = world(h, 2.0, 0.0)
    c = world(x_end, -2.0, -drop)
    d = world(x_end, 2.0, -drop)
    low = TOP + (BOTTOM - TOP)
    ua, ub = Vector((a.x, a.y, low)), Vector((b.x, b.y, low))
    uc, ud = Vector((c.x, c.y, low)), Vector((d.x, d.y, low))
    v = [p.bm.verts.new(q) for q in (a, b, c, d, ua, ub, uc, ud)]
    faces = [
        p.bm.faces.new((v[0], v[2], v[3], v[1])),
        p.bm.faces.new((v[4], v[5], v[7], v[6])),
        p.bm.faces.new((v[0], v[1], v[5], v[4])),
        p.bm.faces.new((v[2], v[6], v[7], v[3])),
        p.bm.faces.new((v[0], v[4], v[6], v[2])),
        p.bm.faces.new((v[1], v[3], v[7], v[5])),
    ]
    p.paint(faces, CONCRETE_DARK)
    return math.degrees(math.atan2(drop, x_end - h))


def stairs(p):
    """Three flights up the west side, each to its own landing."""
    for x, rise in ((-8.5, 0.2), (-6.5, 0.4), (-4.5, 0.45)):
        n = 8 if rise == 0.2 else 4
        for k in range(n):
            p.block((x - 0.8, 2.0 - (k + 1) * 0.3, 0.0), (x + 0.8, 2.0 - k * 0.3, (k + 1) * rise), STEP)
        y = 2.0 - n * 0.3
        p.block((x - 0.8, y - 3.0, 0.0), (x + 0.8, y, n * rise), CONCRETE_DARK)


def ramps(p):
    """Three ramps up the east side, 2 m high, each to a landing."""
    for x, degrees in ((3.5, 30.0), (6.0, 45.0), (8.5, 50.0)):
        run = 2.0 / math.tan(math.radians(degrees))
        p.wedge(x - 1.0, x + 1.0, 3.5, 3.5 - run, 2.0, WOOD)
        p.block((x - 1.0, 3.5 - run - 2.0, 0.0), (x + 1.0, 3.5 - run, 2.0), CONCRETE_DARK)


def crates(p):
    p.block((-2.5, 6.0, 0.0), (-1.5, 7.0, 0.8), CRATE)
    p.block((-0.5, 6.0, 0.0), (0.5, 7.0, 0.8), CRATE)
    p.block((-0.45, 6.05, 0.8), (0.45, 6.95, 1.6), CRATE)


def tunnel(p):
    """Along x, 1.2 m wide and 1.3 m high inside."""
    p.block((1.5, 6.6, 0.0), (5.5, 6.9, 1.3), CONCRETE_DARK)
    p.block((1.5, 8.1, 0.0), (5.5, 8.4, 1.3), CONCRETE_DARK)
    p.block((1.5, 6.6, 1.3), (5.5, 8.4, 1.6), CONCRETE)


def gap(p):
    """Two walls 0.8 m apart, 2.5 m long."""
    p.block((7.0, 5.2, 0.0), (9.5, 5.5, 2.5), RUST)
    p.block((7.0, 6.3, 0.0), (9.5, 6.6, 2.5), RUST)


def building(p):
    """A room with a doorway to the north, and stairs outside up to its roof."""
    x0, x1, y0, y1, t, h = -3.0, 3.0, -10.0, -5.0, 0.25, 2.8
    p.block((x0, y0, 0.0), (x0 + t, y1, h), WALL)
    p.block((x1 - t, y0, 0.0), (x1, y1, h), WALL)
    p.block((x0, y0, 0.0), (x1, y0 + t, h), WALL)
    # The north wall, with a 1.2 m doorway 2.2 m high.
    p.block((x0, y1 - t, 0.0), (-0.6, y1, h), WALL)
    p.block((0.6, y1 - t, 0.0), (x1, y1, h), WALL)
    p.block((-0.6, y1 - t, 2.2), (0.6, y1, h), WALL)
    p.block((x0, y0, h), (x1, y1, h + 0.25), ROOF)
    steps = 16
    rise = (h + 0.25) / steps
    for k in range(steps):
        p.block((x1, y1 - (k + 1) * 0.29, 0.0), (x1 + 1.2, y1 - k * 0.29, (k + 1) * rise), STEP)


# ---- targets -------------------------------------------------------------------
#
# The game hit-tests these by the same numbers (src/targets.rs): keep them
# in step.

DUMMIES = ((6.0, -6.0, 0.0), (7.6, -8.6, 15.0), (9.0, -6.2, -10.0))


def dummy(material, n, x, y, yaw):
    """A training dummy standing on its base: a post, a burlap torso
    with a crossbar for arms, a head. It tips over about its foot."""
    d = Part(f"TARGET_Dummy_{n}", at=world(x, y, 0.06), yaw=yaw)
    d.block((-0.04, -0.04, 0.0), (0.04, 0.04, 1.0), POST)
    d.block((-0.21, -0.13, 0.925), (0.21, 0.13, 1.475), BURLAP)
    d.block((-0.40, -0.035, 1.285), (0.40, 0.035, 1.355), POST)
    d.block((-0.11, -0.11, 1.50), (0.11, 0.11, 1.74), BURLAP)
    d.finish(material)


def plate(material, n, distance):
    """A steel plate hung from a hinge on a frame, facing back down the
    lane. It swings about its hinge."""
    x = LANE_FROM + distance
    ground = height(x, LANE_Y)
    hinge = Vector((x, LANE_Y, ground + 1.85))
    p = Part(f"TARGET_Plate_{n}", at=hinge)
    p.block((-0.01, -0.25, -0.55), (0.01, 0.25, -0.05), STEEL)
    for side in (-0.18, 0.18):
        p.block((-0.006, side - 0.006, -0.05), (0.006, side + 0.006, 0.0), FRAME)
    p.finish(material)
    return x, ground


def range_frames(part, spots):
    """The frames the plates hang from: two posts and a bar each."""
    for x, ground in spots:
        for y in (-0.45, 0.45):
            a = Vector((x - 0.04, LANE_Y + y - 0.04, ground - 0.3))
            b = Vector((x + 0.04, LANE_Y + y + 0.04, ground + 1.95))
            block_world(part, a, b, FRAME)
        block_world(part, Vector((x - 0.03, LANE_Y - 0.49, ground + 1.87)), Vector((x + 0.03, LANE_Y + 0.49, ground + 1.93)), FRAME)


def block_world(part, a, b, colour):
    centre = (a + b) / 2
    size = b - a
    m = Matrix.Translation(centre) @ Matrix.Diagonal((size.x / 2, size.y / 2, size.z / 2, 1.0))
    made = bmesh.ops.create_cube(part.bm, size=2.0, matrix=m)
    part.paint(sorted({f for v in made["verts"] for f in v.link_faces}, key=lambda f: f.calc_center_median().to_tuple()), colour)


def main():
    material = flat_material()
    ground = Part("SOLID_Pad")
    h = PAD_HALF
    ground.block((-h, -h, BOTTOM - TOP), (h, h, 0.0), CONCRETE)
    slope = access_ramp(ground)
    ground.finish(material)
    for name, build in (("SOLID_Stairs", stairs), ("SOLID_Ramps", ramps), ("SOLID_Crates", crates), ("SOLID_Tunnel", tunnel), ("SOLID_Gap", gap), ("SOLID_Building", building)):
        part = Part(name)
        build(part)
        part.finish(material)
    stands = Part("SOLID_DummyStands")
    for n, (x, y, yaw) in enumerate(DUMMIES, 1):
        stands.block((x - 0.25, y - 0.25, 0.0), (x + 0.25, y + 0.25, 0.06), POST)
        dummy(material, n, x, y, yaw)
    stands.finish(material)
    spots = [plate(material, n, d) for n, d in enumerate(LANE_PLATES, 1)]
    frames = Part("SOLID_RangeFrames")
    range_frames(frames, spots)
    frames.finish(material)
    bpy.ops.export_scene.gltf(filepath=os.path.abspath(OUT), export_format="GLB", export_yup=True, export_apply=False, export_animations=False, export_vertex_color="ACTIVE", export_normals=True)
    print(f"proving_ground: top {TOP:.2f} m, bottom {BOTTOM:.2f} m, access ramp {slope:.1f} deg -> {os.path.abspath(OUT)}")


main()
