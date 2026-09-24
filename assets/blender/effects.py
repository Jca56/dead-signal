"""What fire and throwing look like, apart from the things themselves:

    Flame   a tongue of flame 0.5 m tall on its base: an orange outer
            cone, a yellow core; drawn a little aglow, scaled and flickered
            by the game
    Dot     a small ball, 6 cm across, white: the throw's arc, dot by dot
    Ring    a thin ring 1 m across, flat on the ground at its middle: where
            the throw lands

Run headless from the project root:
    /opt/blender-bin-5.2.1/blender -b --factory-startup --python assets/blender/effects.py

Writes assets/models/effects.glb.
"""

import math
import os

import bmesh
import bpy
from mathutils import Matrix

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "..", "models", "effects.glb")

ORANGE = (1.0, 0.45, 0.08)
RED = (0.85, 0.18, 0.05)
YELLOW = (1.0, 0.85, 0.30)
WHITE = (0.95, 0.95, 0.92)


def material():
    mat = bpy.data.materials.new("Flat")
    mat.use_nodes = True
    nodes = mat.node_tree.nodes
    vc = nodes.new("ShaderNodeVertexColor")
    vc.layer_name = "Col"
    mat.node_tree.links.new(vc.outputs["Color"], nodes["Principled BSDF"].inputs["Base Color"])
    return mat


def paint(bm, col, verts, colour):
    for f in {f for v in verts for f in v.link_faces}:
        for loop in f.loops:
            loop[col] = (*colour, 1.0)


def finish(name, bm, mat):
    mesh = bpy.data.meshes.new(name)
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    bm.to_mesh(mesh)
    bm.free()
    attrs = mesh.color_attributes
    attrs.active_color = attrs["Col"]
    attrs.render_color_index = attrs.find("Col")
    mesh.materials.append(mat)
    bpy.context.scene.collection.objects.link(bpy.data.objects.new(name, mesh))


def flame(mat):
    bm = bmesh.new()
    col = bm.loops.layers.color.new("Col")
    outer = bmesh.ops.create_cone(bm, cap_ends=True, segments=5, radius1=0.16, radius2=0.0, depth=0.5, matrix=Matrix.Translation((0, 0, 0.25)))["verts"]
    paint(bm, col, outer, ORANGE)
    base = [v for v in outer if v.co.z < 0.01]
    paint(bm, col, base, RED)
    core = bmesh.ops.create_cone(bm, cap_ends=True, segments=5, radius1=0.09, radius2=0.0, depth=0.32, matrix=Matrix.Translation((0.02, 0.01, 0.16)) @ Matrix.Rotation(0.3, 4, "Z"))["verts"]
    paint(bm, col, core, YELLOW)
    finish("Flame", bm, mat)


def dot(mat):
    bm = bmesh.new()
    col = bm.loops.layers.color.new("Col")
    paint(bm, col, bmesh.ops.create_icosphere(bm, subdivisions=1, radius=0.03)["verts"], WHITE)
    finish("Dot", bm, mat)


def ring(mat):
    bm = bmesh.new()
    col = bm.loops.layers.color.new("Col")
    sides = 24
    for k in range(sides):
        a = k / sides * math.tau
        m = Matrix.Translation((math.cos(a) * 0.5, math.sin(a) * 0.5, 0.02)) @ Matrix.Rotation(a, 4, "Z") @ Matrix.Diagonal((0.02, 0.07, 0.01, 1.0))
        paint(bm, col, bmesh.ops.create_cube(bm, size=2.0, matrix=m)["verts"], WHITE)
    finish("Ring", bm, mat)


def main():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    mat = material()
    flame(mat)
    dot(mat)
    ring(mat)
    bpy.ops.export_scene.gltf(filepath=os.path.abspath(OUT), export_format="GLB", export_yup=True, export_apply=False, export_animations=False, export_vertex_color="ACTIVE", export_normals=True)
    print(f"effects: {len(bpy.data.objects)} objects -> {os.path.abspath(OUT)}")


main()
