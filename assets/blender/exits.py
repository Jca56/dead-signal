"""The ways out, each its own object standing on its origin, its front (the
side it's used from) facing +Y:

    EXIT_Radio          a field radio on a folding table, its whip antenna
                        up, a car battery wired to it on the ground
    EXIT_Gate           a road checkpoint across X, its boom raised
    EXIT_Gate_Blocked   the same shut: boom down, barriers across the road
    EXIT_Truck          a pickup along X (its tailgate at -X), hood up on an
                        empty engine bay
    EXIT_Truck_Ready    the same, fixed: hood down

and what the game bumps into for each (EXIT_*_Hull, and the shut road's
barricade alone, EXIT_Gate_Barricade_Hull): plain boxes, never drawn.

Run headless from the project root:
    /opt/blender-bin-5.2.1/blender -b --factory-startup --python assets/blender/exits.py

Writes assets/models/exits.glb.
"""

import math
import os
import random

import bmesh
import bpy
from mathutils import Matrix, Vector

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "..", "models", "exits.glb")

bpy.ops.wm.read_factory_settings(use_empty=True)
rng = random.Random(11)

OLIVE = (0.29, 0.32, 0.20)
OLIVE_DARK = (0.19, 0.21, 0.13)
TABLE = (0.35, 0.33, 0.30)
METAL = (0.42, 0.43, 0.42)
DARK = (0.08, 0.08, 0.08)
DIAL = (0.80, 0.78, 0.66)
LAMP_RED = (0.70, 0.10, 0.08)
BATTERY = (0.11, 0.11, 0.11)
CABLE = (0.13, 0.13, 0.12)
CONCRETE = (0.55, 0.54, 0.50)
CONCRETE_DARK = (0.40, 0.40, 0.37)
STRIPE_RED = (0.62, 0.10, 0.08)
STRIPE_WHITE = (0.85, 0.84, 0.80)
BOOTH = (0.33, 0.36, 0.33)
ROOF = (0.20, 0.20, 0.20)
GLASS = (0.10, 0.12, 0.13)
SAND = (0.55, 0.49, 0.34)
SIGN = (0.20, 0.35, 0.22)
TRUCK = (0.46, 0.16, 0.11)
RUST = (0.40, 0.24, 0.14)
TYRE = (0.09, 0.09, 0.09)
HUB = (0.45, 0.45, 0.43)
CHROME = (0.58, 0.58, 0.55)
LIGHT = (0.80, 0.78, 0.66)
ENGINE = (0.20, 0.20, 0.19)


def jitter(c, amount=0.05):
    k = 1.0 + rng.uniform(-amount, amount)
    return tuple(min(1.0, max(0.0, v * k)) for v in c)


def swing(verts, hinge, axis, degrees):
    m = Matrix.Translation(hinge) @ Matrix.Rotation(math.radians(degrees), 4, axis) @ Matrix.Translation(-Vector(hinge))
    for v in verts:
        v.co = m @ v.co


class Part:
    def __init__(self, name, seed):
        rng.seed(seed)
        self.name = name
        self.bm = bmesh.new()
        self.col = self.bm.loops.layers.color.new("Col")

    def faces_of(self, verts):
        return sorted({f for v in verts for f in v.link_faces}, key=lambda f: f.index)

    def paint(self, verts, colour):
        for f in self.faces_of(verts):
            c = jitter(colour)
            for loop in f.loops:
                loop[self.col] = (*c, 1.0)

    def box(self, lo, hi, colour, turn=None):
        c = Vector(((lo[0] + hi[0]) / 2, (lo[1] + hi[1]) / 2, (lo[2] + hi[2]) / 2))
        s = ((hi[0] - lo[0]) / 2, (hi[1] - lo[1]) / 2, (hi[2] - lo[2]) / 2)
        m = Matrix.Translation(c) @ (turn or Matrix.Identity(4)) @ Matrix.Diagonal((*s, 1.0))
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

    def wheel(self, centre, radius, width):
        self.rod((centre[0], centre[1] - width / 2, centre[2]), (centre[0], centre[1] + width / 2, centre[2]), radius, TYRE, sides=10)
        side = 1 if centre[1] > 0 else -1
        face = centre[1] + side * width / 2
        self.box((centre[0] - radius * 0.45, face - 0.012, centre[2] - radius * 0.45), (centre[0] + radius * 0.45, face + 0.012, centre[2] + radius * 0.45), HUB)

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


def hull(name, boxes):
    p = Part(name + "_Hull", name)
    for lo, hi in boxes:
        p.box(lo, hi, DARK)
    p.finish()


def radio():
    p = Part("EXIT_Radio", "radio")
    # A folding table.
    p.box((-0.6, -0.3, 0.72), (0.6, 0.3, 0.76), TABLE)
    for x in (-0.55, 0.55):
        for y in (-0.25, 0.25):
            p.rod((x, y, 0.0), (x, y, 0.72), 0.02, METAL)
    # The set: a box of dials and a speaker, a handset beside it.
    p.box((-0.3, -0.2, 0.76), (0.2, 0.15, 1.08), OLIVE)
    p.box((-0.28, 0.15, 0.8), (0.18, 0.17, 1.05), OLIVE_DARK)
    for k, x in enumerate((-0.2, -0.08, 0.04)):
        p.rod((x, 0.16, 0.98), (x, 0.2, 0.98), 0.025, DIAL, sides=8)
    for k in range(4):
        p.box((0.07, 0.17, 0.83 + k * 0.03), (0.16, 0.175, 0.845 + k * 0.03), DARK)
    p.box((-0.24, 0.17, 0.86), (-0.2, 0.18, 0.9), LAMP_RED)
    p.box((0.28, -0.12, 0.76), (0.48, -0.04, 0.82), DARK)
    # Its whip antenna, and a battery on the ground wired up to it.
    p.rod((-0.25, -0.15, 1.08), (-0.3, -0.18, 3.3), 0.012, METAL, sides=4)
    p.box((0.5, -0.1, 0.0), (0.76, 0.07, 0.19), BATTERY)
    p.rod((0.55, 0.0, 0.19), (0.18, -0.1, 0.8), 0.012, CABLE, sides=4)
    p.rod((0.7, 0.0, 0.19), (0.19, -0.12, 0.85), 0.012, CABLE, sides=4)
    p.finish()
    hull("EXIT_Radio", [((-0.62, -0.32, 0.0), (0.62, 0.32, 1.1)), ((0.48, -0.12, 0.0), (0.78, 0.09, 0.2))])


def gate(blocked):
    name = "EXIT_Gate_Blocked" if blocked else "EXIT_Gate"
    p = Part(name, "gate")
    # It stands on a concrete slab, sunk into the hillside on its high side
    # and showing its edge on its low one.
    p.box((-6.2, -3.0, -0.8), (4.9, 2.6, 0.04), CONCRETE_DARK)
    # Concrete barriers either side of a 5 m gap, a booth on the left.
    for x0 in (-4.6, 2.6):
        p.box((x0, -0.3, 0.0), (x0 + 2.0, 0.3, 0.2), CONCRETE_DARK)
        made = p.box((x0 + 0.05, -0.25, 0.2), (x0 + 1.95, 0.25, 0.9), CONCRETE)
        for v in made:
            if v.co.z > 0.5:
                v.co.y *= 0.5
    p.box((-4.4, 0.6, 0.0), (-2.9, 2.1, 2.3), BOOTH)
    p.box((-4.55, 0.45, 2.3), (-2.75, 2.25, 2.45), ROOF)
    p.box((-2.91, 0.9, 1.1), (-2.88, 1.8, 1.9), GLASS)
    # The boom: a post on the right, the striped pole across the gap.
    p.box((2.35, -0.12, 0.0), (2.6, 0.12, 1.2), METAL)
    pole = []
    for k in range(10):
        x0 = 2.4 - (k + 1) * 0.5
        pole += p.box((x0, -0.05, 1.0), (x0 + 0.5, 0.05, 1.1), STRIPE_RED if k % 2 else STRIPE_WHITE)
    if blocked:
        # Barriers dragged across, sandbags heaped behind.
        for x0 in (-2.4, -0.4, 1.4):
            made = p.box((x0, -0.9, 0.0), (x0 + 1.8, -0.4, 0.85), CONCRETE)
            swing(made, (x0 + 0.9, -0.65, 0.0), "Z", rng.uniform(-12, 12))
        for k in range(9):
            x = -2.4 + k * 0.55
            for z in (0.0, 0.25):
                p.box((x, -1.6 + (z * 0.4), z), (x + 0.5, -1.05, z + 0.25), SAND)
    else:
        swing(pole, (2.4, 0.0, 1.05), "Y", -80.0)
    # A sign: an arrow pointing on out.
    p.box((-5.2, 0.0, 0.0), (-5.1, 0.1, 2.2), METAL)
    p.box((-5.9, 0.1, 1.7), (-4.5, 0.14, 2.2), SIGN)
    p.box((-5.5, 0.14, 1.9), (-4.9, 0.16, 2.0), STRIPE_WHITE)
    made = p.box((-5.0, 0.14, 1.82), (-4.8, 0.16, 2.08), STRIPE_WHITE)
    for v in made:
        if v.co.x > -4.9:
            v.co.z = 1.95
    p.finish()
    if blocked:
        # Only what's across the road: solid on the runs it's shut.
        hull("EXIT_Gate_Barricade", [((-2.6, -1.6, 0.0), (2.6, -0.4, 1.6))])
    else:
        hull(name, [((-6.2, -3.0, -0.8), (4.9, 2.6, 0.04)), ((-4.6, -0.3, 0.0), (-2.6, 0.3, 0.9)), ((2.6, -0.3, 0.0), (4.6, 0.3, 0.9)), ((-4.4, 0.6, 0.0), (-2.9, 2.1, 2.45)), ((2.35, -0.12, 0.0), (2.6, 0.12, 1.2))])


def truck(ready):
    """A pickup, 5.2 × 2.0, along X (its tailgate at -X), cab forward."""
    name = "EXIT_Truck_Ready" if ready else "EXIT_Truck"
    p = Part(name, "truck")
    L, W, lift = 5.2, 2.0, 0.42
    # Chassis, cab, bed.
    p.box((-L / 2, -W / 2, lift), (L / 2, W / 2, lift + 0.5), TRUCK)
    p.box((0.2, -W / 2 + 0.05, lift + 0.5), (1.6, W / 2 - 0.05, lift + 1.35), TRUCK)
    for y0, y1 in ((-W / 2 + 0.04, -W / 2 + 0.06), (W / 2 - 0.06, W / 2 - 0.04)):
        p.box((0.35, y0, lift + 0.85), (1.45, y1, lift + 1.25), GLASS)
    made = p.box((1.55, -W / 2 + 0.12, lift + 0.8), (1.62, W / 2 - 0.12, lift + 1.28), GLASS)
    p.box((0.18, -W / 2 + 0.12, lift + 0.85), (0.22, W / 2 - 0.12, lift + 1.25), GLASS)
    # The bed's sides and tailgate, dark inside.
    p.box((-L / 2 + 0.05, -W / 2 + 0.08, lift + 0.5), (0.15, W / 2 - 0.08, lift + 0.52), DARK)
    for y0 in (-W / 2, W / 2 - 0.08):
        p.box((-L / 2, y0, lift + 0.5), (0.2, y0 + 0.08, lift + 0.95), TRUCK)
    p.box((-L / 2, -W / 2, lift + 0.5), (-L / 2 + 0.08, W / 2, lift + 0.95), TRUCK)
    # Front: grille, lamps, bumpers; rust on the flanks.
    p.box((L / 2, -0.6, lift + 0.1), (L / 2 + 0.03, 0.6, lift + 0.45), CHROME)
    for y in (-0.78, 0.78):
        p.box((L / 2, y - 0.12, lift + 0.25), (L / 2 + 0.02, y + 0.12, lift + 0.42), LIGHT)
    for x in (-L / 2 - 0.08, L / 2 + 0.03):
        p.box((x, -W / 2, lift - 0.05), (x + 0.08, W / 2, lift + 0.12), CHROME)
    for x, sx, z in ((-1.4, 0.7, 0.3), (1.9, 0.5, 0.25), (-0.4, 0.4, 0.35)):
        p.box((x - sx / 2, W / 2, lift + z - 0.12), (x + sx / 2, W / 2 + 0.005, lift + z + 0.12), RUST)
        p.box((x - sx / 2 + 0.2, -W / 2 - 0.005, lift + z - 0.1), (x + sx / 2 + 0.2, -W / 2, lift + z + 0.1), RUST)
    # The engine bay: the block, and the empty tray where a battery goes.
    p.box((1.75, -0.55, lift + 0.2), (2.45, 0.35, lift + 0.52), ENGINE)
    p.box((1.8, 0.45, lift + 0.4), (2.15, 0.8, lift + 0.5), DARK if not ready else BATTERY)
    hood = p.box((1.6, -W / 2 + 0.05, lift + 0.5), (L / 2, W / 2 - 0.05, lift + 0.56), TRUCK)
    if not ready:
        swing(hood, (1.6, 0.0, lift + 0.56), "Y", 58.0)
    for x in (-1.65, 1.75):
        for y in (-W / 2 + 0.1, W / 2 - 0.1):
            p.wheel((x, y, 0.42), 0.42, 0.3)
    p.finish()
    if not ready:
        hull("EXIT_Truck", [((-L / 2 - 0.08, -W / 2, 0.05), (L / 2 + 0.05, W / 2, lift + 0.95)), ((0.2, -W / 2, lift + 0.95), (1.6, W / 2, lift + 1.35))])


def main():
    radio()
    gate(False)
    gate(True)
    truck(False)
    truck(True)
    bpy.ops.export_scene.gltf(filepath=os.path.abspath(OUT), export_format="GLB", export_yup=True, export_apply=False, export_animations=False, export_vertex_color="ACTIVE", export_normals=True)
    print(f"exits: {len(bpy.data.objects)} -> {os.path.abspath(OUT)}")


main()
