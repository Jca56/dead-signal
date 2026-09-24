"""What a Spitter throws: the glob in the air, and the puddle it leaves
(scaled by the game to how wide it spread), bile green and a little
aglow so they read in the fog.

    Glob    a lumpy ball, 0.26 m across, on its middle
    Puddle  a ragged pool 2 m across on the ground at its middle, the
            thick of it dark in the middle, a few bubbles

Run headless from the project root:
    /opt/blender-bin-5.2.1/blender -b --factory-startup --python assets/blender/spit.py

Writes assets/models/spit.glb.
"""

import math
import os
import random

import bmesh
import bpy
from mathutils import Matrix, Vector

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "..", "models", "spit.glb")

BILE = (0.52, 0.72, 0.14)
BILE_DARK = (0.26, 0.40, 0.06)
BILE_PALE = (0.74, 0.86, 0.36)

rng = random.Random(41)


def material():
    mat = bpy.data.materials.new("Flat")
    mat.use_nodes = True
    nodes = mat.node_tree.nodes
    vc = nodes.new("ShaderNodeVertexColor")
    vc.layer_name = "Col"
    mat.node_tree.links.new(vc.outputs["Color"], nodes["Principled BSDF"].inputs["Base Color"])
    return mat


def paint(bm, col, faces, colour, amount=0.1):
    for f in faces:
        k = 1.0 + rng.uniform(-amount, amount)
        c = tuple(min(1.0, v * k) for v in colour)
        for loop in f.loops:
            loop[col] = (*c, 1.0)


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


def glob(mat):
    bm = bmesh.new()
    col = bm.loops.layers.color.new("Col")
    made = bmesh.ops.create_icosphere(bm, subdivisions=1, radius=0.13)["verts"]
    for v in made:
        v.co *= 1.0 + rng.uniform(-0.18, 0.18)
    paint(bm, col, bm.faces, BILE, 0.2)
    # A drip trailing off it.
    drip = bmesh.ops.create_icosphere(bm, subdivisions=1, radius=0.06, matrix=Matrix.Translation((0.0, -0.13, 0.03)))["verts"]
    paint(bm, col, {f for v in drip for f in v.link_faces}, BILE_PALE, 0.1)
    finish("Glob", bm, mat)


def puddle(mat):
    """Rings of points, ragged at the edge, on the ground; a fan of faces."""
    bm = bmesh.new()
    col = bm.loops.layers.color.new("Col")
    sides = 14
    centre = bm.verts.new((0, 0, 0.03))
    inner, outer = [], []
    for k in range(sides):
        a = k / sides * math.tau
        r_in = 0.5 * (1.0 + rng.uniform(-0.15, 0.15))
        r_out = 1.0 * (1.0 + rng.uniform(-0.2, 0.12))
        inner.append(bm.verts.new((math.cos(a) * r_in, math.sin(a) * r_in, 0.025)))
        outer.append(bm.verts.new((math.cos(a) * r_out, math.sin(a) * r_out, 0.012)))
    faces_in, faces_out = [], []
    for k in range(sides):
        n = (k + 1) % sides
        faces_in.append(bm.faces.new((centre, inner[k], inner[n])))
        faces_out.append(bm.faces.new((inner[k], outer[k], outer[n], inner[n])))
    paint(bm, col, faces_in, BILE_DARK, 0.12)
    paint(bm, col, faces_out, BILE, 0.15)
    # Bubbles.
    for _ in range(5):
        a, r = rng.uniform(0, math.tau), rng.uniform(0.1, 0.7)
        size = rng.uniform(0.04, 0.08)
        m = Matrix.Translation((math.cos(a) * r, math.sin(a) * r, 0.03)) @ Matrix.Diagonal((size, size, size * 0.6, 1.0))
        made = bmesh.ops.create_icosphere(bm, subdivisions=1, radius=1.0, matrix=m)["verts"]
        paint(bm, col, {f for v in made for f in v.link_faces}, BILE_PALE, 0.1)
    finish("Puddle", bm, mat)


def main():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    mat = material()
    glob(mat)
    puddle(mat)
    bpy.ops.export_scene.gltf(filepath=os.path.abspath(OUT), export_format="GLB", export_yup=True, export_apply=False, export_animations=False, export_vertex_color="ACTIVE", export_normals=True)
    tris = sum(len(o.data.polygons) for o in bpy.data.objects if o.type == "MESH")
    print(f"spit: {len(bpy.data.objects)} objects, {tris} faces -> {os.path.abspath(OUT)}")


main()
