"""The Shambler: the common dead. Hunched, lopsided, the right arm reaching,
the left hanging; a torn shirt and filthy trousers over grey-green skin.

Run headless from the project root:
    /opt/blender-bin-5.2.1/blender -b --factory-startup --python assets/blender/shambler.py

Writes assets/models/shambler.glb, and the same dead soldier in fatigues,
a helmet, a vest and a pack, on the very same rig and animations, as
assets/models/shambler_soldier.glb: one skinned mesh on a 17-bone rig and
six animations, keyed straight onto the bones:

    Walk    0.8 s  a dragging shamble, one stride a cycle (loops)
    Idle    3 s    swaying where it stands (loops)
    Attack  0.9 s  both arms up and a lunging swipe; lands at 0.4 s
    Flinch  0.33 s snapped back by a hit
    Stumble 0.6 s  knocked reeling by a blow: a step back, arms flung
    Death   1.2 s  knees go, then face down; ends lying still

Built facing Blender's +Y (the game's -Z, the way a yaw of 0 looks), feet
on the ground at the origin. Bones: root > hips > spine > chest > neck >
head; chest > upper_arm > forearm > hand; hips > thigh > shin > foot, each
limb .L and .R (+X is its right).
"""

import math
import os
import sys

import bpy
from mathutils import Matrix, Vector

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from kit import Builder, norm  # noqa: E402

MODELS = os.path.join(HERE, "..", "models")
FPS = 30

SKIN = (0.42, 0.47, 0.38)
SKIN_DARK = (0.30, 0.34, 0.28)
SHIRT = (0.31, 0.34, 0.37)
SHIRT_STAIN = (0.24, 0.22, 0.20)
TROUSERS = (0.27, 0.23, 0.18)
TROUSERS_DARK = (0.21, 0.18, 0.14)
SHOE = (0.12, 0.10, 0.09)
SOCKET = (0.10, 0.09, 0.08)
# A soldier's: fatigues, and the gear over them.
FATIGUES = (0.29, 0.31, 0.22)
FATIGUES_DARK = (0.22, 0.24, 0.17)
BLOOD = (0.26, 0.10, 0.08)
HELMET = (0.24, 0.26, 0.18)
VEST = (0.20, 0.21, 0.16)
PACK = (0.26, 0.25, 0.18)


def v(x, y, z):
    return Vector((x, y, z))


# The skeleton at rest: (head, tail, parent). Right side is +X.
BONES = {
    "hips": (v(0, 0.02, 0.95), v(0, 0.04, 1.15), "root"),
    "spine": (v(0, 0.04, 1.15), v(0, 0.10, 1.33), "hips"),
    "chest": (v(0, 0.10, 1.33), v(0, 0.18, 1.47), "spine"),
    "neck": (v(0, 0.18, 1.47), v(0, 0.24, 1.56), "chest"),
    "head": (v(0, 0.24, 1.56), v(0, 0.30, 1.78), "neck"),
    # The right arm reaches; the left hangs.
    "upper_arm.R": (v(0.20, 0.12, 1.43), v(0.24, 0.22, 1.19), "chest"),
    "forearm.R": (v(0.24, 0.22, 1.19), v(0.24, 0.42, 1.03), "upper_arm.R"),
    "hand.R": (v(0.24, 0.42, 1.03), v(0.24, 0.51, 0.99), "forearm.R"),
    "upper_arm.L": (v(-0.20, 0.12, 1.43), v(-0.23, 0.13, 1.16), "chest"),
    "forearm.L": (v(-0.23, 0.13, 1.16), v(-0.24, 0.18, 0.90), "upper_arm.L"),
    "hand.L": (v(-0.24, 0.18, 0.90), v(-0.24, 0.20, 0.80), "forearm.L"),
}
for side, x in ((".R", 0.10), (".L", -0.10)):
    BONES[f"thigh{side}"] = (v(x, 0.02, 0.93), v(x * 1.1, 0.05, 0.50), "hips")
    BONES[f"shin{side}"] = (v(x * 1.1, 0.05, 0.50), v(x * 1.1, 0.02, 0.08), f"thigh{side}")
    BONES[f"foot{side}"] = (v(x * 1.1, 0.02, 0.08), v(x * 1.1, 0.17, 0.03), f"shin{side}")


def stained(a, b, pattern):
    """A ring of face colours: `b` where `pattern` has a 1."""
    return [b if c == "1" else a for c in pattern]


def limb(b, points, colours, cap_start=False, cap_end=True, ref=Vector((1, 0, 0))):
    """A tube through `points` [(centre, radius, weights)], one starting
    direction for every ring (`ref`, kept square to each)."""
    rings = []
    for i, (centre, radius, weights) in enumerate(points):
        a = points[max(i - 1, 0)][0]
        c = points[min(i + 1, len(points) - 1)][0]
        axis = norm(c - a)
        ra, rb = radius if isinstance(radius, tuple) else (radius, radius)
        rings.append(b.ring(centre, axis, ref, ra, rb, weights))
    b.tube(rings, colours, cap_start=cap_start, cap_end=cap_end)


def body(b, soldier=False):
    j = {name: (h, t) for name, (h, t, _) in BONES.items()}
    fwd = Vector((0, 1, 0))
    # A soldier wears fatigues where the others wear a shirt and trousers.
    SHIRT, SHIRT_STAIN, TROUSERS, TROUSERS_DARK = (FATIGUES, BLOOD, FATIGUES, FATIGUES_DARK) if soldier else (globals()["SHIRT"], globals()["SHIRT_STAIN"], globals()["TROUSERS"], globals()["TROUSERS_DARK"])
    # Torso, hips to neck: trousers, a belt line, a filthy shirt.
    limb(b, [
        (v(0, 0.02, 0.86), (0.16, 0.11), {"hips": 1.0}),
        (v(0, 0.03, 1.00), (0.17, 0.115), {"hips": 1.0}),
        (v(0, 0.05, 1.15), (0.15, 0.10), {"hips": 0.5, "spine": 0.5}),
        (v(0, 0.10, 1.33), (0.19, 0.12), {"spine": 0.5, "chest": 0.5}),
        (v(0, 0.15, 1.44), (0.21, 0.11), {"chest": 1.0}),
        (v(0, 0.19, 1.50), (0.07, 0.065), {"chest": 0.4, "neck": 0.6}),
    ], [TROUSERS, TROUSERS_DARK, stained(SHIRT, SHIRT_STAIN, "10010110"), stained(SHIRT, SHIRT_STAIN, "01000011"), SHIRT], cap_start=True, cap_end=False)
    # Neck and head: jutting forward, dark sockets where the eyes were.
    limb(b, [
        (v(0, 0.19, 1.49), 0.065, {"neck": 1.0}),
        (v(0, 0.24, 1.57), 0.07, {"neck": 0.3, "head": 0.7}),
        (v(0, 0.29, 1.63), (0.095, 0.105), {"head": 1.0}),
        (v(0, 0.30, 1.70), (0.10, 0.11), {"head": 1.0}),
        (v(0, 0.29, 1.78), (0.07, 0.08), {"head": 1.0}),
    ], [SKIN, SKIN_DARK, stained(SKIN, SOCKET, "10000001"), SKIN], cap_start=False, cap_end=True, ref=fwd)
    # Arms: a torn sleeve to the elbow, then skin; a hand like a mitt.
    for side in (".R", ".L"):
        up, fore, hand = j[f"upper_arm{side}"], j[f"forearm{side}"], j[f"hand{side}"]
        u, f, h = f"upper_arm{side}", f"forearm{side}", f"hand{side}"
        limb(b, [
            (up[0], 0.065, {"chest": 0.4, u: 0.6}),
            (up[0].lerp(up[1], 0.5), 0.058, {u: 1.0}),
            (up[1], 0.05, {u: 0.5, f: 0.5}),
            (fore[0].lerp(fore[1], 0.5), 0.042, {f: 1.0}),
            (fore[1], 0.036, {f: 0.5, h: 0.5}),
            (hand[0].lerp(hand[1], 0.6), 0.045, {h: 1.0}),
            (hand[1] + (hand[1] - hand[0]) * 0.4, 0.03, {h: 1.0}),
        ], [SHIRT, stained(SHIRT, SKIN, "10101001"), SKIN, SKIN, SKIN, SKIN_DARK], cap_start=True, cap_end=True)
    # Legs: trousers to the ankle, a shoe.
    for side in (".R", ".L"):
        th, sh, ft = j[f"thigh{side}"], j[f"shin{side}"], j[f"foot{side}"]
        t, s, fo = f"thigh{side}", f"shin{side}", f"foot{side}"
        limb(b, [
            (th[0], 0.085, {"hips": 0.5, t: 0.5}),
            (th[0].lerp(th[1], 0.5), 0.075, {t: 1.0}),
            (th[1], 0.062, {t: 0.5, s: 0.5}),
            (sh[0].lerp(sh[1], 0.5), 0.055, {s: 1.0}),
            (sh[1] + Vector((0, 0, 0.04)), 0.05, {s: 1.0}),
        ], [TROUSERS, TROUSERS, TROUSERS_DARK, TROUSERS], cap_start=False, cap_end=True, ref=fwd)
        limb(b, [
            (ft[0] + Vector((0, -0.03, 0.02)), (0.05, 0.045), {fo: 1.0}),
            (ft[0].lerp(ft[1], 0.5) + Vector((0, 0, 0.0)), (0.055, 0.04), {fo: 1.0}),
            (ft[1] + Vector((0, 0.02, -0.005)), (0.045, 0.028), {fo: 1.0}),
        ], [SHOE, SHOE], cap_start=True, cap_end=True, ref=Vector((0, 0, 1)))
    if soldier:
        gear(b)


def gear(b):
    """A soldier's: a helmet over the skull, a vest round the chest, a pack
    on the back."""
    fwd = Vector((0, 1, 0))
    limb(b, [
        (v(0, 0.29, 1.66), (0.125, 0.135), {"head": 1.0}),
        (v(0, 0.29, 1.74), (0.12, 0.13), {"head": 1.0}),
        (v(0, 0.29, 1.81), (0.07, 0.08), {"head": 1.0}),
    ], [HELMET, HELMET], cap_start=True, cap_end=True, ref=fwd)
    limb(b, [
        (v(0, 0.06, 1.12), (0.18, 0.13), {"hips": 0.4, "spine": 0.6}),
        (v(0, 0.10, 1.30), (0.215, 0.145), {"spine": 0.5, "chest": 0.5}),
        (v(0, 0.14, 1.42), (0.225, 0.135), {"chest": 1.0}),
    ], [VEST, VEST], cap_start=True, cap_end=True)
    limb(b, [
        (v(0, -0.04, 1.14), (0.14, 0.07), {"spine": 1.0}),
        (v(0, 0.0, 1.30), (0.15, 0.08), {"spine": 0.4, "chest": 0.6}),
        (v(0, 0.03, 1.42), (0.13, 0.07), {"chest": 1.0}),
    ], [PACK, PACK], cap_start=True, cap_end=True)


def build(soldier=False):
    b = Builder()
    body(b, soldier)
    mesh = bpy.data.meshes.new("Shambler")
    b.bm.to_mesh(mesh)
    b.bm.free()
    attrs = mesh.color_attributes
    attrs.active_color = attrs["Col"]
    attrs.render_color_index = attrs.find("Col")
    material = bpy.data.materials.new("Flat")
    material.use_nodes = True
    nodes = material.node_tree.nodes
    vc = nodes.new("ShaderNodeVertexColor")
    vc.layer_name = "Col"
    material.node_tree.links.new(vc.outputs["Color"], nodes["Principled BSDF"].inputs["Base Color"])
    mesh.materials.append(material)

    data = bpy.data.armatures.new("ShamblerRig")
    rig = bpy.data.objects.new("ShamblerRig", data)
    bpy.context.scene.collection.objects.link(rig)
    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.mode_set(mode="EDIT")
    root = data.edit_bones.new("root")
    root.head, root.tail = (0, 0, 0), (0, 0.15, 0)
    made = {"root": root}
    pending = dict(BONES)
    while pending:
        for name, (head, tail, parent) in list(pending.items()):
            if parent in made:
                e = data.edit_bones.new(name)
                e.head, e.tail, e.parent = head, tail, made[parent]
                made[name] = e
                del pending[name]
    bpy.ops.object.mode_set(mode="OBJECT")
    obj = bpy.data.objects.new("Shambler", mesh)
    bpy.context.scene.collection.objects.link(obj)
    for name in b.groups:
        obj.vertex_groups.new(name=name)
    obj.parent = rig
    obj.modifiers.new("Rig", "ARMATURE").object = rig
    return rig, obj


# ---- animation ---------------------------------------------------------------------

def turn(rig, bone, x=0.0, y=0.0, z=0.0):
    """A bone's rotation, given as turns about the rig's own X, Y and Z (as
    the bone lies at rest), degrees: +X swings a hanging limb forward and
    tips an upright one back."""
    m = rig.data.bones[bone].matrix_local.to_3x3()
    r = Matrix.Rotation(math.radians(z), 3, "Z") @ Matrix.Rotation(math.radians(y), 3, "Y") @ Matrix.Rotation(math.radians(x), 3, "X")
    return (m.inverted() @ r @ m).to_quaternion()


def moved(rig, bone, dx=0.0, dy=0.0, dz=0.0):
    """A bone's offset, given in the rig's own axes."""
    m = rig.data.bones[bone].matrix_local.to_3x3()
    return m.inverted() @ Vector((dx, dy, dz))


def key(rig, frame, pose):
    """Key every bone at `frame`: those in `pose` ({bone: (x, y, z) turns,
    or with a fourth item an (dx, dy, dz) offset}) as given, the rest at
    rest, so each action says everything."""
    for pb in rig.pose.bones:
        pb.rotation_mode = "QUATERNION"
        spec = pose.get(pb.name, (0.0, 0.0, 0.0))
        pb.rotation_quaternion = turn(rig, pb.name, *spec[:3])
        pb.location = moved(rig, pb.name, *spec[3]) if len(spec) > 3 else Vector()
        pb.keyframe_insert("rotation_quaternion", frame=frame)
        pb.keyframe_insert("location", frame=frame)


def walk_pose(p):
    """The shamble at phase `p` (0–1 of a stride): the left leg swings
    freely, the right drags a shorter step; the body lurches over them."""
    a = p * math.tau
    swing_l = max(0.0, -math.sin(a))
    swing_r = max(0.0, math.sin(a))
    return {
        "thigh.L": (26 * math.cos(a), 0, 0),
        "shin.L": (-(8 + 45 * swing_l), 0, 0),
        "foot.L": (-(12 * math.cos(a)) + 20 * swing_l, 0, 0),
        "thigh.R": (-18 * math.cos(a), 0, 0),
        "shin.R": (-(12 + 22 * swing_r), 0, 0),
        "foot.R": (12 * math.cos(a) + 10 * swing_r, 0, 0),
        "hips": (0, 5 * math.sin(a), 7 * math.cos(a), (0, 0, -0.035 * abs(math.cos(a)))),
        "spine": (-10, 0, -4 * math.cos(a)),
        "chest": (-8, 3 * math.sin(a), -5 * math.cos(a)),
        "neck": (-12, 0, 0),
        "head": (-6, 12 * math.sin(a + 1.0), 6 * math.sin(a)),
        "upper_arm.R": (38 + 8 * math.sin(a), 0, -6),
        "forearm.R": (12, 0, 0),
        "upper_arm.L": (14 * math.cos(a + math.pi), 0, 4),
        "forearm.L": (10 + 6 * swing_r, 0, 0),
    }


def stand(sway=0.0):
    """Standing, swaying `sway` (−1–1) side to side."""
    return {
        "hips": (0, 3 * sway, 2 * sway),
        "spine": (-9 + 2 * sway, 0, 0),
        "chest": (-7, 2 * sway, 0),
        "neck": (-12, 0, 0),
        "head": (-4, 10 * sway, 4 * sway),
        "upper_arm.R": (30 + 4 * sway, 0, -6),
        "forearm.R": (14, 0, 0),
        "upper_arm.L": (4 * sway, 0, 4),
        "forearm.L": (8, 0, 0),
        "thigh.R": (4, 0, 0),
        "shin.R": (-8, 0, 0),
        "foot.R": (4, 0, 0),
        "thigh.L": (-2, 0, 0),
        "shin.L": (-4, 0, 0),
    }


def blend(a, b, t):
    out = {}
    for name in set(a) | set(b):
        pa = a.get(name, (0.0, 0.0, 0.0))
        pb = b.get(name, (0.0, 0.0, 0.0))
        rot = tuple(pa[i] + (pb[i] - pa[i]) * t for i in range(3))
        off_a = pa[3] if len(pa) > 3 else (0.0, 0.0, 0.0)
        off_b = pb[3] if len(pb) > 3 else (0.0, 0.0, 0.0)
        out[name] = rot + (tuple(off_a[i] + (off_b[i] - off_a[i]) * t for i in range(3)),)
    return out


def with_(base, **changes):
    out = dict(base)
    for name, spec in changes.items():
        out[name.replace("__", ".")] = spec
    return out


def actions(rig):
    """Each animation keyed on its own stretch of timeline, then cut into
    an action of its own."""
    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.mode_set(mode="POSE")
    made = []

    def action(name, keys):
        act = bpy.data.actions.new(name)
        rig.animation_data_create()
        rig.animation_data.action = act
        for frame, pose in keys:
            key(rig, frame, pose)
        rig.animation_data.action = None
        made.append(act)

    action("Walk", [(f, walk_pose(f / 24)) for f in range(0, 25, 2)])
    action("Idle", [(f, stand(math.sin(f / 90 * math.tau))) for f in range(0, 91, 10)])
    rest = stand()
    windup = with_(rest, upper_arm__R=(150, 0, -10), forearm__R=(30, 0, 0), upper_arm__L=(115, 0, 10), forearm__L=(25, 0, 0), chest=(10, 0, 8), spine=(4, 0, 0), head=(8, 0, 0))
    swipe = with_(rest, upper_arm__R=(45, 0, 25), forearm__R=(5, 0, 0), upper_arm__L=(55, 0, -20), forearm__L=(10, 0, 0), chest=(-28, 0, -12), spine=(-14, 0, 0), hips=(0, 0, 0, (0, 0.18, -0.03)), head=(-10, 0, 0), thigh__R=(28, 0, 0), shin__R=(-20, 0, 0))
    action("Attack", [(0, rest), (7, windup), (12, swipe), (18, blend(swipe, rest, 0.5)), (27, rest)])
    struck = with_(rest, chest=(18, 0, 6), spine=(8, 0, 0), head=(22, 8, 0), upper_arm__R=(15, 0, -10))
    action("Flinch", [(0, rest), (3, struck), (10, rest)])
    reel = with_(rest, chest=(30, 0, 10), spine=(14, 0, 0), head=(30, 12, 0), upper_arm__R=(60, 0, -40), forearm__R=(30, 0, 0), upper_arm__L=(50, 0, 40), forearm__L=(25, 0, 0), hips=(0, 0, 0, (0, -0.06, -0.02)))
    back = with_(reel, chest=(22, 0, 6), spine=(10, 0, 0), thigh__L=(-28, 0, 0), shin__L=(-12, 0, 0), thigh__R=(12, 0, 0), shin__R=(-18, 0, 0), hips=(0, 0, 0, (0, -0.14, -0.05)))
    action("Stumble", [(0, rest), (3, reel), (9, back), (13, blend(back, rest, 0.5)), (18, rest)])
    buckle = with_(rest, thigh__R=(45, 0, 0), thigh__L=(40, 0, 0), shin__R=(-85, 0, 0), shin__L=(-80, 0, 0), foot__R=(40, 0, 0), foot__L=(40, 0, 0), hips=(0, 0, 0, (0, 0.05, -0.32)), head=(15, 10, 0))
    tip = with_(buckle, hips=(-45, 5, 0, (0, 0.30, -0.52)), spine=(-10, 0, 0), upper_arm__R=(90, 0, 0), upper_arm__L=(80, 0, 0))
    down = {"hips": (-88, 6, 5, (0, 0.72, -0.80)), "head": (-15, 25, 0), "upper_arm.R": (165, 0, -25), "upper_arm.L": (150, 0, 30), "forearm.R": (20, 0, 0), "forearm.L": (15, 0, 0), "thigh.L": (-6, 0, 4), "shin.L": (-12, 0, 0), "foot.L": (-40, 0, 0), "foot.R": (-40, 0, 0)}
    bounce = blend(down, {**down, "hips": (-84, 6, 5, (0, 0.72, -0.76))}, 1.0)
    action("Death", [(0, rest), (8, buckle), (16, tip), (24, down), (28, bounce), (36, down)])
    bpy.ops.object.mode_set(mode="OBJECT")
    return made


def stash(rig, acts):
    for act in acts:
        track = rig.animation_data.nla_tracks.new()
        track.name = act.name
        track.strips.new(act.name, int(act.frame_range[0]), act)


def model(name, soldier):
    """Build and write one of the dead, from an empty scene."""
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.context.scene.render.fps = FPS
    rig, obj = build(soldier)
    stash(rig, actions(rig))
    out = os.path.abspath(os.path.join(MODELS, f"{name}.glb"))
    bpy.ops.export_scene.gltf(
        filepath=out,
        export_format="GLB",
        export_yup=True,
        export_apply=False,
        export_animations=True,
        export_animation_mode="ACTIONS",
        export_anim_slide_to_zero=True,
        export_skins=True,
        export_vertex_color="ACTIVE",
        export_normals=True,
    )
    print(f"{name}: {len(obj.data.polygons)} faces, {len(rig.data.bones)} bones -> {out}")


def main():
    model("shambler", False)
    model("shambler_soldier", True)


main()
