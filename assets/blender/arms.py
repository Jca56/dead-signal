"""The survivor's arms, as the camera sees them: both forearms and hands
up in a loose guard, in a worn field jacket and fingerless gloves; and in
them, one weapon at a time.

Run headless from the project root:
    /opt/blender-bin-5.2.1/blender -b --factory-startup --python assets/blender/arms.py

Writes one viewmodel a weapon, assets/models/viewmodel_<weapon>.glb: the
arms and that weapon as one skinned mesh on one armature, and the
weapon's clips (every one has Idle and Bash; a gun has Fire and Reload
too). Each weapon is a module (`fists.py`, `pistol.py`) that adds its
parts and bones to the arms (`build`, none for bare fists) and makes its
clips (`animate`). Built in camera space: the eye at the origin,
looking down Blender's +Y (the exporter makes that glTF's -Z), +Z up, +X
right. The mesh is modelled in the guard pose, which is the rest pose, so
nothing moves until an animation says so.

Bones per side (.R / .L): upper_arm > forearm > hand > thumb1 > thumb2,
index1 > index2, fingers1 > fingers2 (middle, ring and little finger
move as one). All under "root".
"""

import math
import os

import sys

import bmesh
import bpy
from mathutils import Matrix, Vector

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import fists  # noqa: E402
import pistol  # noqa: E402
from kit import Builder, banded, norm, rotate  # noqa: E402
MODELS = os.path.join(HERE, "..", "models")

# Every weapon's viewmodel: its name in the file's, and its module.
WEAPONS = [("fists", fists), ("pistol", pistol)]

# Colours (sRGB, as in the title scene).
SLEEVE = (0.30, 0.29, 0.20)
SLEEVE_SHADE = (0.25, 0.24, 0.17)
CUFF = (0.36, 0.34, 0.24)
GLOVE = (0.10, 0.09, 0.08)
GLOVE_SEAM = (0.16, 0.14, 0.12)
SKIN = (0.62, 0.46, 0.37)

# ---- the guard pose, right side (the left is its mirror) ----------------------------

SHOULDER = Vector((0.29, -0.22, -0.26))
ELBOW = Vector((0.29, 0.10, -0.31))
WRIST = Vector((0.21, 0.33, -0.10))
# The left hand sits a little further forward, a boxer's lead.
LEAD = Vector((0.0, 0.035, 0.012))


def hand_frame(wrist, elbow, side):
    """Forward (to the knuckles), back (the back of the hand's normal) and
    thumb (towards the thumb) for a hand at `wrist`."""
    fwd = norm((wrist - elbow).normalized() + Vector((0, 0.1, 0.35)))
    # The back of the hand faces up (a touch out): the fingers curl away
    # underneath, and the eye sees knuckles and thumbs, as in a guard.
    back = Vector((0.25 * side, -0.1, 1.0))
    back = norm(back - fwd * back.dot(fwd))
    thumb = norm(fwd.cross(back)) * -side
    return fwd, back, thumb


def arm(b, side, suffix):
    """One arm: sleeve, cuff, wrist, gloved hand, fingers, thumb."""
    shoulder = Vector((SHOULDER.x * side, SHOULDER.y, SHOULDER.z))
    elbow = Vector((ELBOW.x * side, ELBOW.y, ELBOW.z))
    wrist = Vector((WRIST.x * side, WRIST.y, WRIST.z))
    if side < 0:
        elbow += LEAD
        wrist += LEAD
    up = f"upper_arm{suffix}"
    fore = f"forearm{suffix}"
    hand = f"hand{suffix}"
    bones = {}

    # Every ring from the shoulder to the knuckles starts from the same
    # direction (the back of the hand), so point k of one ring meets point
    # k of the next: no corkscrew.
    fwd, back, thumb_dir = hand_frame(wrist, elbow, side)
    across = thumb_dir
    ref = back

    # Sleeve: upper arm into forearm, bending at the elbow.
    d_up = norm(elbow - shoulder)
    d_fore = norm(wrist - elbow)
    bend = norm(d_up + d_fore)
    rings = [
        b.ring(shoulder, d_up, ref, 0.058, 0.062, {up: 1.0}),
        b.ring(shoulder.lerp(elbow, 0.6), d_up, ref, 0.056, 0.060, {up: 1.0}),
        b.ring(elbow, bend, ref, 0.055, 0.057, {up: 0.5, fore: 0.5}),
        b.ring(elbow.lerp(wrist, 0.45), d_fore, ref, 0.046, 0.050, {fore: 1.0}),
        b.ring(elbow.lerp(wrist, 0.80), d_fore, ref, 0.042, 0.045, {fore: 1.0}),
        # The rolled cuff: a thick band.
        b.ring(elbow.lerp(wrist, 0.82), d_fore, ref, 0.048, 0.051, {fore: 1.0}),
        b.ring(elbow.lerp(wrist, 0.92), d_fore, ref, 0.047, 0.050, {fore: 1.0}),
        b.ring(elbow.lerp(wrist, 0.93), d_fore, ref, 0.031, 0.034, {fore: 1.0}),
    ]
    sleeve = banded(SLEEVE, SLEEVE_SHADE)
    b.tube(rings, [sleeve, sleeve, sleeve, sleeve, CUFF, CUFF, CUFF], cap_start=True, cap_end=True)

    # The wrist: a sliver of skin between cuff and glove.
    wrist_rings = [
        b.ring(elbow.lerp(wrist, 0.92), d_fore, ref, 0.027, 0.030, {fore: 1.0}),
        b.ring(wrist, fwd, ref, 0.024, 0.031, {fore: 0.4, hand: 0.6}),
    ]
    b.tube(wrist_rings, [SKIN], cap_start=False, cap_end=False)

    # The palm: a flattened block from wrist to knuckles, the glove's
    # mouth a little over the wrist.
    knuckles = wrist + fwd * 0.085
    palm = [
        b.ring(wrist - fwd * 0.012, fwd, ref, 0.026, 0.035, {fore: 0.4, hand: 0.6}),
        b.ring(wrist + fwd * 0.05, fwd, ref, 0.021, 0.043, {hand: 1.0}),
        b.ring(knuckles, fwd, ref, 0.019, 0.044, {hand: 1.0}),
    ]
    b.tube(palm, [GLOVE_SEAM, GLOVE], cap_start=True, cap_end=True)

    # Fingers, half curled: the first joint turns down 60°, the next 70° more.
    palm_side = -back
    curl_axis = norm(fwd.cross(palm_side))

    def finger(base, lengths, radius, first, second, name1, name2):
        d1 = rotate(fwd, curl_axis, first)
        d2 = rotate(fwd, curl_axis, first + second)
        mid = base + d1 * lengths[0]
        tip = mid + d2 * lengths[1]
        rings = [
            b.ring(base, d1, across, radius, radius * 0.9, {name1: 1.0}),
            b.ring(mid, norm(d1 + d2), across, radius * 0.95, radius * 0.85, {name1: 0.5, name2: 0.5}),
            b.ring(mid + d2 * lengths[1] * 0.35, d2, across, radius * 0.9, radius * 0.8, {name2: 1.0}),
            b.ring(tip, d2, across, radius * 0.75, radius * 0.7, {name2: 1.0}),
        ]
        # Glove to the middle joint, bare skin beyond: fingerless.
        b.tube(rings, [GLOVE, GLOVE, SKIN], cap_start=False, cap_end=True)
        return base, mid, tip

    i1, i2 = f"index1{suffix}", f"index2{suffix}"
    f1, f2 = f"fingers1{suffix}", f"fingers2{suffix}"
    index = finger(knuckles + across * 0.028 + back * 0.002, (0.042, 0.038), 0.0105, 62, 72, i1, i2)
    bones[i1] = (index[0], index[1], hand)
    bones[i2] = (index[1], index[2], i1)
    first = None
    for k, (offset, length) in enumerate(((0.008, 0.046), (-0.012, 0.043), (-0.030, 0.036))):
        base = knuckles + across * offset - fwd * (0.004 * k)
        f = finger(base, (length, length * 0.85), 0.0105 - 0.0008 * k, 60 + 4 * k, 72, f1, f2)
        if first is None:
            first = f
    bones[f1] = (first[0], first[1], hand)
    bones[f2] = (first[1], first[2], f1)

    # The thumb: from the heel of the hand, across the front of the fist.
    t1, t2 = f"thumb1{suffix}", f"thumb2{suffix}"
    t_base = wrist + fwd * 0.028 + across * 0.036 + palm_side * 0.008
    t_d1 = norm(fwd * 0.75 + across * 0.35 + palm_side * 0.55)
    t_d2 = norm(fwd * 0.2 - across * 0.75 + palm_side * 0.55)
    t_mid = t_base + t_d1 * 0.040
    t_tip = t_mid + t_d2 * 0.034
    # One starting direction for all its rings, square to its bend.
    t_ref = norm(t_d1.cross(t_d2))
    rings = [
        b.ring(t_base, t_d1, t_ref, 0.014, 0.012, {hand: 0.5, t1: 0.5}),
        b.ring(t_mid, norm(t_d1 + t_d2), t_ref, 0.012, 0.011, {t1: 0.5, t2: 0.5}),
        b.ring(t_mid + t_d2 * 0.012, t_d2, t_ref, 0.0115, 0.0105, {t2: 1.0}),
        b.ring(t_tip, t_d2, t_ref, 0.009, 0.008, {t2: 1.0}),
    ]
    b.tube(rings, [GLOVE, GLOVE, SKIN], cap_start=True, cap_end=True)
    bones[t1] = (t_base, t_mid, hand)
    bones[t2] = (t_mid, t_tip, t1)

    bones[up] = (shoulder, elbow, "root")
    bones[fore] = (elbow, wrist, up)
    bones[hand] = (wrist, knuckles, fore)
    return bones, (wrist, fwd, back, across)


# ---- armature, mesh, animation -----------------------------------------------------

def build(weapon):
    """The arms with `weapon`'s parts in them; the rig, the weapon's rest
    frame (what its `build` gave back) and the left hand's."""
    b = Builder()
    bones = {}
    right_bones, (wrist, fwd, back, across) = arm(b, 1.0, ".R")
    bones.update(right_bones)
    left_bones, (l_wrist, l_fwd, l_back, _) = arm(b, -1.0, ".L")
    bones.update(left_bones)
    weapon_bones, weapon_rest = weapon.build(b, wrist, fwd, back, across, "hand.R")
    bones.update(weapon_bones)

    # Every ring runs round its axis the same way whichever side it is on,
    # so every face already winds outward.
    mesh = bpy.data.meshes.new("Arms")
    b.bm.to_mesh(mesh)
    b.bm.free()
    attrs = mesh.color_attributes
    attrs.active_color = attrs["Col"]
    attrs.render_color_index = attrs.find("Col")

    material = bpy.data.materials.new("Flat")
    material.use_nodes = True
    nodes = material.node_tree.nodes
    bsdf = nodes["Principled BSDF"]
    bsdf.inputs["Roughness"].default_value = 1.0
    vc = nodes.new("ShaderNodeVertexColor")
    vc.layer_name = "Col"
    material.node_tree.links.new(vc.outputs["Color"], bsdf.inputs["Base Color"])
    mesh.materials.append(material)

    arm_data = bpy.data.armatures.new("Rig")
    rig = bpy.data.objects.new("Rig", arm_data)
    bpy.context.scene.collection.objects.link(rig)
    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.mode_set(mode="EDIT")
    root = arm_data.edit_bones.new("root")
    root.head = (0, 0, 0)
    root.tail = (0, 0.05, 0)
    made = {"root": root}
    # Parents before children.
    pending = dict(bones)
    while pending:
        for name, (head, tail, parent) in list(pending.items()):
            if parent in made:
                e = arm_data.edit_bones.new(name)
                e.head = head
                e.tail = tail
                e.parent = made[parent]
                made[name] = e
                del pending[name]
    bpy.ops.object.mode_set(mode="OBJECT")

    obj = bpy.data.objects.new("Arms", mesh)
    bpy.context.scene.collection.objects.link(obj)
    for name in b.groups:
        obj.vertex_groups.new(name=name)
    obj.parent = rig
    mod = obj.modifiers.new("Rig", "ARMATURE")
    mod.object = rig
    return rig, weapon_rest, (l_wrist, l_fwd, l_back)


def stash(rig, actions):
    """Put every action on an NLA track of its own: what the exporter
    writes, one glTF animation each."""
    for action in actions:
        track = rig.animation_data.nla_tracks.new()
        track.name = action.name
        track.strips.new(action.name, int(action.frame_range[0]), action)


def viewmodel(name, weapon):
    """Build and write `weapon`'s viewmodel, from an empty scene."""
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.context.scene.render.fps = 30
    rig, weapon_rest, left_rest = build(weapon)
    rig.animation_data_create()
    names = weapon.animate(rig, weapon_rest, left_rest)
    stash(rig, [bpy.data.actions[n] for n in names])
    rig.animation_data.action = None
    out = os.path.abspath(os.path.join(MODELS, f"viewmodel_{name}.glb"))
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
    mesh = bpy.data.objects["Arms"].data
    print(f"{name}: {len(mesh.polygons)} faces, {len(rig.data.bones)} bones, clips {names} -> {out}")


def main():
    only = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    for name, weapon in WEAPONS:
        if not only or name in only:
            viewmodel(name, weapon)


main()
