"""What the radio calls down, and what marks where:

    Flare   a road flare, 0.22 m long, lying along +Y from its foot at the
            origin: a red stick, a pale cap at its foot; what burns at its
            tip is the game's
    Crate   a supply crate, 0.9 by 0.6 by 0.55 m, on the ground at its
            middle: olive boards, darker bands and corners, a pale stripe
            round it, rope handles
    Open    the same with its lid off, leaning against its side: what it
            held is gone
    Chute   its parachute, from where its cords meet (the origin: the top
            of the crate) up 4.6 m: a round canopy of olive and sand
            gores, eight cords down to the crate
    Plane   the strafing run's: a ground-attack plane, 14 m across its
            straight wings and 13 m long, its nose along +Y, about its
            middle: a grey fuselage, a gun under its nose, a dark canopy,
            two engines high on its back, a tailplane with a fin at each
            end

Run headless from the project root:
    /opt/blender-bin-5.2.1/blender -b --factory-startup --python assets/blender/airdrop.py

Writes assets/models/airdrop.glb.
"""

import math
import os

import bmesh
import bpy
from mathutils import Matrix, Vector

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "..", "models", "airdrop.glb")

RED = (0.62, 0.10, 0.08)
CAP = (0.82, 0.80, 0.74)
OLIVE = (0.30, 0.33, 0.19)
OLIVE_DARK = (0.19, 0.21, 0.12)
BAND = (0.12, 0.13, 0.09)
STRIPE = (0.80, 0.78, 0.66)
INSIDE = (0.34, 0.26, 0.15)
ROPE = (0.55, 0.47, 0.30)
SAND = (0.62, 0.56, 0.40)
CORD = (0.70, 0.68, 0.60)

# The crate: half its width, half its depth, its height.
W, D, H = 0.45, 0.30, 0.55


def material():
    mat = bpy.data.materials.new("Flat")
    mat.use_nodes = True
    nodes = mat.node_tree.nodes
    nodes["Principled BSDF"].inputs["Roughness"].default_value = 1.0
    vc = nodes.new("ShaderNodeVertexColor")
    vc.layer_name = "Col"
    mat.node_tree.links.new(vc.outputs["Color"], nodes["Principled BSDF"].inputs["Base Color"])
    return mat


class Shape:
    """A mesh being made: boxes and faces, each its colour."""

    def __init__(self):
        self.bm = bmesh.new()
        self.col = self.bm.loops.layers.color.new("Col")

    def paint(self, faces, colour):
        for f in faces:
            for loop in f.loops:
                loop[self.col] = (*colour, 1.0)

    def box(self, centre, size, colour, turn=None):
        m = Matrix.Translation(centre) @ (turn or Matrix.Identity(4)) @ Matrix.Diagonal((size[0] / 2, size[1] / 2, size[2] / 2, 1.0))
        verts = bmesh.ops.create_cube(self.bm, size=2.0, matrix=m)["verts"]
        self.paint({f for v in verts for f in v.link_faces}, colour)

    def face(self, points, colour):
        """A face seen from both sides (cloth)."""
        for order in (points, list(reversed(points))):
            self.paint([self.bm.faces.new([self.bm.verts.new(p) for p in order])], colour)

    def finish(self, name, mat):
        mesh = bpy.data.meshes.new(name)
        self.bm.to_mesh(mesh)
        self.bm.free()
        attrs = mesh.color_attributes
        attrs.active_color = attrs["Col"]
        attrs.render_color_index = attrs.find("Col")
        mesh.materials.append(mat)
        bpy.context.scene.collection.objects.link(bpy.data.objects.new(name, mesh))


def flare(mat):
    s = Shape()
    made = bmesh.ops.create_cone(s.bm, cap_ends=True, segments=8, radius1=0.014, radius2=0.014, depth=0.2, matrix=Matrix.Translation((0, 0.12, 0)) @ Matrix.Rotation(math.pi / 2, 4, "X"))["verts"]
    s.paint({f for v in made for f in v.link_faces}, RED)
    made = bmesh.ops.create_cone(s.bm, cap_ends=True, segments=8, radius1=0.016, radius2=0.016, depth=0.02, matrix=Matrix.Translation((0, 0.01, 0)) @ Matrix.Rotation(math.pi / 2, 4, "X"))["verts"]
    s.paint({f for v in made for f in v.link_faces}, CAP)
    s.finish("Flare", mat)


def body(s, lid):
    """The crate's boards, bands, corners, stripe and handles; with its
    `lid` on, or open to its inside."""
    wall = 0.03
    # Four sides (boards, a darker one every other) and the floor.
    for k in range(5):
        z = H * (k + 0.5) / 5
        shade = OLIVE if k % 2 == 0 else OLIVE_DARK
        for y in (-D + wall / 2, D - wall / 2):
            s.box((0, y, z), (2 * W, wall, H / 5), shade)
        for x in (-W + wall / 2, W - wall / 2):
            s.box((x, 0, z), (wall, 2 * D - 2 * wall, H / 5), shade)
    s.box((0, 0, wall / 2), (2 * W - 2 * wall, 2 * D - 2 * wall, wall), INSIDE)
    # Bands round it, corner posts, the pale stripe, and a rope at each end.
    for x in (-W * 0.55, W * 0.55):
        s.box((x, 0, H / 2), (0.07, 2 * D + 0.02, H + 0.01), BAND)
    for x in (-W, W):
        for y in (-D, D):
            s.box((x * 0.985, y * 0.98, H / 2), (0.05, 0.05, H + 0.02), BAND)
    for y in (-D - 0.004, D + 0.004):
        s.box((0, y, H * 0.62), (W * 0.9, 0.006, 0.07), STRIPE)
    for x in (-W - 0.02, W + 0.02):
        s.box((x, 0, H * 0.7), (0.03, 0.22, 0.03), ROPE)
    if lid:
        s.box((0, 0, H + 0.02), (2 * W + 0.04, 2 * D + 0.04, 0.04), OLIVE)
        for x in (-W * 0.55, W * 0.55):
            s.box((x, 0, H + 0.045), (0.07, 2 * D + 0.05, 0.012), BAND)
    else:
        # The lid off, leaning against the long side.
        lean = Matrix.Rotation(math.radians(68), 4, "X")
        s.box((0, -D - 0.2, 0.28), (2 * W + 0.04, 2 * D + 0.04, 0.04), OLIVE, lean)


def crate(mat):
    for name, lid in (("Crate", True), ("Open", False)):
        s = Shape()
        body(s, lid)
        s.finish(name, mat)


def chute(mat):
    s = Shape()
    gores, rim, top, high = 12, 2.3, 0.35, 4.6
    # The canopy: gores from a small vent at the top, bulging, down to a
    # rim that's scalloped between the cords.
    rings = [(top, high), (rim * 0.62, high - 0.35), (rim * 0.92, high - 0.85), (rim, high - 1.3)]

    def at(k, ring, pull=1.0):
        a = k / gores * math.tau
        r, z = rings[ring]
        return Vector((math.cos(a) * r * pull, math.sin(a) * r * pull, z))

    for k in range(gores):
        colour = OLIVE if k % 2 == 0 else SAND
        for ring in range(len(rings) - 1):
            mid = Vector((0, 0, 0))
            a, b, c, d = at(k, ring), at(k + 1, ring), at(k + 1, ring + 1), at(k, ring + 1)
            # (Each panel puffed out a little at its middle.)
            mid = (a + b + c + d) / 4
            mid += Vector((mid.x, mid.y, 0)).normalized() * 0.12
            for tri in ((a, b, mid), (b, c, mid), (c, d, mid), (d, a, mid)):
                s.face(list(tri), colour)
    # The cords: from the rim, every gore and a half, down to the crate.
    for k in range(0, gores, 1):
        if k % 3 == 2:
            continue
        foot = Vector((math.cos(k / gores * math.tau) * 0.25, math.sin(k / gores * math.tau) * 0.18, 0))
        head = at(k, len(rings) - 1)
        along = head - foot
        m = Matrix.Translation((foot + head) / 2) @ along.to_track_quat("Z", "Y").to_matrix().to_4x4() @ Matrix.Diagonal((0.012, 0.012, along.length / 2, 1.0))
        verts = bmesh.ops.create_cube(s.bm, size=2.0, matrix=m)["verts"]
        s.paint({f for v in verts for f in v.link_faces}, CORD)
    s.finish("Chute", mat)


GREY = (0.24, 0.27, 0.27)
BELLY = (0.34, 0.37, 0.38)
GLASS = (0.05, 0.07, 0.09)
GUN = (0.06, 0.06, 0.06)
INTAKE = (0.10, 0.10, 0.11)


def plane(mat):
    s = Shape()
    # The fuselage: a long body, a nose tapering down from it, a belly.
    s.box((0, 0.5, 0), (1.3, 9.0, 1.4), GREY)
    s.box((0, 5.6, -0.12), (1.0, 1.6, 1.05), GREY)
    s.box((0, 6.7, -0.25), (0.6, 0.9, 0.7), GREY)
    s.box((0, 0.5, -0.72), (1.1, 8.6, 0.12), BELLY)
    # The gun, out under the nose; the canopy, up behind it.
    s.box((0.12, 6.6, -0.62), (0.22, 2.2, 0.22), GUN)
    s.box((0, 3.6, 0.82), (0.8, 1.9, 0.5), GLASS)
    # Straight wings, a little up at the tips, a pod under each.
    for side in (-1.0, 1.0):
        s.box((side * 3.6, 0.7, -0.2), (6.4, 2.3, 0.22), GREY, Matrix.Rotation(side * math.radians(-3.0), 4, "Y"))
        s.box((side * 6.7, 0.5, 0.02), (0.5, 1.5, 0.16), BELLY)
        s.box((side * 2.6, 0.9, -0.55), (0.3, 1.6, 0.3), GUN)
    # The engines, high on the back, each side.
    for side in (-1.0, 1.0):
        s.box((side * 1.25, -2.2, 0.95), (1.05, 2.8, 1.05), GREY)
        s.box((side * 1.25, -0.75, 0.95), (0.85, 0.12, 0.85), INTAKE)
        s.box((side * 0.6, -2.2, 0.6), (0.5, 1.4, 0.3), GREY)
    # The tail: its boom, its plane, and a fin at each end of that.
    s.box((0, -4.9, 0.1), (0.8, 2.4, 0.9), GREY)
    s.box((0, -5.6, 0.3), (5.2, 1.3, 0.16), GREY)
    for side in (-1.0, 1.0):
        s.box((side * 2.6, -5.7, 0.95), (0.16, 1.4, 1.5), GREY)
    s.finish("Plane", mat)


def main():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    mat = material()
    flare(mat)
    crate(mat)
    chute(mat)
    plane(mat)
    bpy.ops.export_scene.gltf(filepath=os.path.abspath(OUT), export_format="GLB", export_yup=True, export_apply=False, export_animations=False, export_vertex_color="ACTIVE", export_normals=True)
    print(f"airdrop: {len(bpy.data.objects)} objects -> {os.path.abspath(OUT)}")


main()
