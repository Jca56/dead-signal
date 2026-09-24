"""The Shambler: the common dead. Hunched, lopsided, the right arm reaching,
the left hanging, grey-green (or grey, or sallow) under what it died in.

Run headless from the project root:
    /opt/blender-bin-5.2.1/blender -b --factory-startup --python assets/blender/shambler.py

Writes assets/models/shambler.glb: a 17-bone rig and six animations, keyed
straight onto the bones, and every part a Shambler is put together from
(`shambler_body.py`, `shambler_extras.py`), each a skinned mesh of its own
on that one rig, named as the game knows it. No two need look alike: the
game picks the parts, and the colours of their regions (`shambler_kit.py`).

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
from kit import Builder  # noqa: E402
import shambler_body  # noqa: E402
import shambler_extras  # noqa: E402
from shambler_kit import BONES  # noqa: E402

MODELS = os.path.join(HERE, "..", "models")
FPS = 30


def rig():
    """The skeleton, from its rest (`BONES`)."""
    data = bpy.data.armatures.new("ShamblerRig")
    obj = bpy.data.objects.new("ShamblerRig", data)
    bpy.context.scene.collection.objects.link(obj)
    bpy.context.view_layer.objects.active = obj
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
    return obj


def part(name, make, armature, material):
    """One part, built by `make(b)`, skinned on `armature`."""
    b = Builder()
    make(b)
    mesh = bpy.data.meshes.new(name)
    b.bm.to_mesh(mesh)
    b.bm.free()
    attrs = mesh.color_attributes
    attrs.active_color = attrs["Col"]
    attrs.render_color_index = attrs.find("Col")
    mesh.materials.append(material)
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.scene.collection.objects.link(obj)
    for group in b.groups:
        obj.vertex_groups.new(name=group)
    obj.parent = armature
    obj.modifiers.new("Rig", "ARMATURE").object = armature
    return obj


def build():
    """The rig, and every part on it."""
    armature = rig()
    material = bpy.data.materials.new("Flat")
    material.use_nodes = True
    nodes = material.node_tree.nodes
    vc = nodes.new("ShaderNodeVertexColor")
    vc.layer_name = "Col"
    material.node_tree.links.new(vc.outputs["Color"], nodes["Principled BSDF"].inputs["Base Color"])
    made = [part(name, make, armature, material) for name, make in {**shambler_body.parts(), **shambler_extras.parts()}.items()]
    return armature, made


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


def main():
    """Build the rig and every part from an empty scene, and write them."""
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.context.scene.render.fps = FPS
    armature, made = build()
    stash(armature, actions(armature))
    out = os.path.abspath(os.path.join(MODELS, "shambler.glb"))
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
    faces = sum(len(o.data.polygons) for o in made)
    print(f"shambler: {len(made)} parts, {faces} faces, {len(armature.data.bones)} bones -> {out}")


main()
