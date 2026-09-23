"""Things to search: a wooden crate, a metal locker, a wrecked car and a
locked supply cage, each its own object (CONTAINER_Crate, CONTAINER_Locker,
CONTAINER_Car, CONTAINER_Cage) standing on its origin with its front (the
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
import random

import bmesh
import bpy
from mathutils import Matrix, Vector

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "..", "models", "containers.glb")

bpy.ops.wm.read_factory_settings(use_empty=True)
rng = random.Random(7)

PLANK = (0.50, 0.38, 0.24)
PLANK_DARK = (0.34, 0.25, 0.16)
ROPE = (0.60, 0.52, 0.36)
LOCKER = (0.33, 0.38, 0.34)
LOCKER_DARK = (0.20, 0.23, 0.21)
SLOT = (0.08, 0.08, 0.08)
PAINT = (0.30, 0.37, 0.41)
RUST = (0.42, 0.24, 0.14)
GLASS = (0.10, 0.12, 0.13)
TYRE = (0.09, 0.09, 0.09)
HUB = (0.40, 0.40, 0.38)
CHROME = (0.55, 0.55, 0.52)
LAMP = (0.80, 0.78, 0.66)
TAIL = (0.55, 0.08, 0.06)
STEEL = (0.42, 0.43, 0.42)
STEEL_DARK = (0.24, 0.25, 0.25)
BRASS = (0.74, 0.58, 0.24)
KIT_RED = (0.66, 0.10, 0.08)
STRAW = (0.62, 0.53, 0.30)
HOLLOW = (0.06, 0.06, 0.06)


def swing(verts, hinge, axis, degrees):
    """Turn `verts` about the line through `hinge` along `axis`."""
    m = Matrix.Translation(hinge) @ Matrix.Rotation(math.radians(degrees), 4, axis) @ Matrix.Translation(-Vector(hinge))
    for v in verts:
        v.co = m @ v.co


def jitter(c, amount=0.06):
    k = 1.0 + rng.uniform(-amount, amount)
    return tuple(min(1.0, max(0.0, v * k)) for v in c)


class Part:
    def __init__(self, name):
        self.name = name
        self.bm = bmesh.new()
        self.col = self.bm.loops.layers.color.new("Col")

    def paint(self, faces, colour, shade=True):
        for f in faces:
            c = jitter(colour) if shade else colour
            for loop in f.loops:
                loop[self.col] = (*c, 1.0)

    def faces_of(self, verts):
        return sorted({f for v in verts for f in v.link_faces}, key=lambda f: f.index)

    def box(self, lo, hi, colour, turn=None):
        """An axis-lined box between two corners (turned about its middle
        by a matrix if given)."""
        c = Vector(((lo[0] + hi[0]) / 2, (lo[1] + hi[1]) / 2, (lo[2] + hi[2]) / 2))
        s = ((hi[0] - lo[0]) / 2, (hi[1] - lo[1]) / 2, (hi[2] - lo[2]) / 2)
        m = Matrix.Translation(c) @ (turn or Matrix.Identity(4)) @ Matrix.Diagonal((*s, 1.0))
        made = bmesh.ops.create_cube(self.bm, size=2.0, matrix=m)["verts"]
        self.paint(self.faces_of(made), colour)
        return made

    def wheel(self, centre, radius, width, colour, hub):
        m = Matrix.Translation(centre) @ Matrix.Rotation(math.pi / 2, 4, "X")
        made = bmesh.ops.create_cone(self.bm, cap_ends=True, segments=10, radius1=radius, radius2=radius, depth=width, matrix=m)["verts"]
        self.paint(self.faces_of(made), colour)
        face = centre[1] + (width / 2 if centre[1] > 0 else -width / 2)
        self.box((centre[0] - radius * 0.45, face - 0.012, centre[2] - radius * 0.45), (centre[0] + radius * 0.45, face + 0.012, centre[2] + radius * 0.45), hub)

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


def named(base, opened):
    """A part for `base`, shut or opened. Same seed, same colours: the two
    look alike but for what moved."""
    rng.seed(base)
    return Part(base + ("_Open" if opened else ""))


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


def hull(name, boxes, turn=None):
    """`boxes` (corner pairs) as one object, turned by `turn` if given."""
    p = Part(name + "_Hull")
    for lo, hi in boxes:
        p.box(lo, hi, STEEL_DARK)
    if turn is not None:
        for v in p.bm.verts:
            v.co = turn @ v.co
    p.finish()


def hulls():
    hull("CONTAINER_Crate", [((-0.5, -0.35, 0.0), (0.5, 0.35, 0.62))])
    hull("CONTAINER_Locker", [((-0.3, -0.25, 0.0), (0.3, 0.27, 1.92))])
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
    hulls()
    bpy.ops.export_scene.gltf(filepath=os.path.abspath(OUT), export_format="GLB", export_yup=True, export_apply=False, export_animations=False, export_vertex_color="ACTIVE", export_normals=True)
    print(f"containers: {len(bpy.data.objects)} -> {os.path.abspath(OUT)}")


main()
