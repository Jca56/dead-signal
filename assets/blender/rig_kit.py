"""What every rigged figure's script shares (`shambler.py`, `survivor.py`):
a skeleton from its rest, parts skinned on it (each a mesh of its own, the
flat vertex-colour material), poses keyed straight onto the bones as turns
about the rig's own axes, each animation an action of its own, and the
export.
"""

import math

import bpy
from mathutils import Matrix, Quaternion, Vector

from kit import Builder


def rig(name, bones):
    """The skeleton, from its rest: `bones` {name: (head, tail, parent)},
    hanging from a root at the origin."""
    data = bpy.data.armatures.new(name)
    obj = bpy.data.objects.new(name, data)
    bpy.context.scene.collection.objects.link(obj)
    bpy.context.view_layer.objects.active = obj
    bpy.ops.object.mode_set(mode="EDIT")
    root = data.edit_bones.new("root")
    root.head, root.tail = (0, 0, 0), (0, 0.15, 0)
    made = {"root": root}
    pending = dict(bones)
    while pending:
        for bone, (head, tail, parent) in list(pending.items()):
            if parent in made:
                e = data.edit_bones.new(bone)
                e.head, e.tail, e.parent = head, tail, made[parent]
                made[bone] = e
                del pending[bone]
    bpy.ops.object.mode_set(mode="OBJECT")
    return obj


def flat_material():
    """Every part's material: its faces' own colours."""
    material = bpy.data.materials.new("Flat")
    material.use_nodes = True
    nodes = material.node_tree.nodes
    vc = nodes.new("ShaderNodeVertexColor")
    vc.layer_name = "Col"
    material.node_tree.links.new(vc.outputs["Color"], nodes["Principled BSDF"].inputs["Base Color"])
    return material


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


def turned(spec):
    """A bone's turn in `pose`, about the rig's own axes: (x, y, z)
    degrees, or a quaternion."""
    if isinstance(spec, Quaternion):
        return spec.to_matrix()
    x, y, z = spec[:3]
    return Matrix.Rotation(math.radians(z), 3, "Z") @ Matrix.Rotation(math.radians(y), 3, "Y") @ Matrix.Rotation(math.radians(x), 3, "X")


def offset(spec):
    """A bone's offset in `pose`, in the rig's own axes."""
    return Vector(spec[3]) if not isinstance(spec, Quaternion) and len(spec) > 3 else Vector()


def key(rig, frame, pose):
    """Key every bone at `frame`: those in `pose` ({bone: (x, y, z) turns,
    or with a fourth item an (dx, dy, dz) offset; or a quaternion turn}) as
    given, the rest at rest, so each action says everything."""
    for pb in rig.pose.bones:
        pb.rotation_mode = "QUATERNION"
        spec = pose.get(pb.name, (0.0, 0.0, 0.0))
        if isinstance(spec, Quaternion):
            m = rig.data.bones[pb.name].matrix_local.to_3x3()
            pb.rotation_quaternion = (m.inverted() @ spec.to_matrix() @ m).to_quaternion()
            pb.location = Vector()
        else:
            pb.rotation_quaternion = turn(rig, pb.name, *spec[:3])
            pb.location = moved(rig, pb.name, *spec[3]) if len(spec) > 3 else Vector()
        pb.keyframe_insert("rotation_quaternion", frame=frame)
        pb.keyframe_insert("location", frame=frame)


def blend(a, b, t):
    """Pose `a` turned `t` of the way to pose `b` (a bone turned by a
    quaternion in either, and offset in neither, turns the shortest way)."""
    out = {}
    for name in set(a) | set(b):
        pa = a.get(name, (0.0, 0.0, 0.0))
        pb = b.get(name, (0.0, 0.0, 0.0))
        if isinstance(pa, Quaternion) or isinstance(pb, Quaternion):
            qa, qb = turned(pa).to_quaternion(), turned(pb).to_quaternion()
            out[name] = qa.slerp(qb, t)
            continue
        rot = tuple(pa[i] + (pb[i] - pa[i]) * t for i in range(3))
        off_a = pa[3] if len(pa) > 3 else (0.0, 0.0, 0.0)
        off_b = pb[3] if len(pb) > 3 else (0.0, 0.0, 0.0)
        out[name] = rot + (tuple(off_a[i] + (off_b[i] - off_a[i]) * t for i in range(3)),)
    return out


def with_(base, **changes):
    """`base` with some bones posed otherwise (`upper_arm__R` for
    `upper_arm.R`)."""
    out = dict(base)
    for name, spec in changes.items():
        out[name.replace("__", ".")] = spec
    return out


class Actions:
    """Animations keyed on `rig`, each cut into an action of its own."""

    def __init__(self, rig):
        self.rig = rig
        self.made = []
        bpy.context.view_layer.objects.active = rig
        bpy.ops.object.mode_set(mode="POSE")

    def action(self, name, keys):
        """An action of `keys`, [(frame, pose)]."""
        act = bpy.data.actions.new(name)
        self.rig.animation_data_create()
        self.rig.animation_data.action = act
        for frame, pose in keys:
            key(self.rig, frame, pose)
        self.rig.animation_data.action = None
        self.made.append(act)

    def done(self):
        bpy.ops.object.mode_set(mode="OBJECT")
        return self.made


def stash(rig, acts):
    for act in acts:
        track = rig.animation_data.nla_tracks.new()
        track.name = act.name
        track.strips.new(act.name, int(act.frame_range[0]), act)


def export(path):
    """Write the scene: the rig, its parts and every action."""
    bpy.ops.export_scene.gltf(
        filepath=path,
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
