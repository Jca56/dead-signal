"""Pictures of a rigged figure (the dead, a hound) in its poses, painted
as the game would paint it, to check it away from the game.

    /opt/blender-bin-5.2.1/blender -b --factory-startup --python tools/figure_views.py -- <out dir> <model> <hide part,...> <Clip:share,...>

`model` is a file in assets/models (no .glb); the parts named are left
out (a figure wears only some of its parts); each clip is drawn `share`
(0–1) of the way through, from its side and from in front of it.
"""

import math
import os
import sys

import bpy
from mathutils import Vector

HERE = os.path.dirname(os.path.abspath(__file__))
MODELS = os.path.join(HERE, "..", "assets", "models")
out, model, hidden, shots = sys.argv[sys.argv.index("--") + 1:][:4]
# A region's colour, by the alpha that names it (skin, top, bottoms,
# accent, under): a charred hide and its darker saddle.
PALETTE = [(0.19, 0.16, 0.14), (0.10, 0.10, 0.11), (0.13, 0.11, 0.10), (0.08, 0.07, 0.07), (0.24, 0.19, 0.15)]

bpy.ops.wm.read_factory_settings(use_empty=True)
scene = bpy.context.scene
bpy.ops.import_scene.gltf(filepath=os.path.join(MODELS, f"{model}.glb"))
rig = next(o for o in bpy.data.objects if o.type == "ARMATURE")
for o in bpy.data.objects:
    if o.type != "MESH":
        continue
    if o.name in hidden.split(","):
        o.hide_render = True
    for attr in o.data.color_attributes:
        for d in attr.data:
            r, g, b, a = d.color
            k = round(a * 5)
            if k < 5:
                p = PALETTE[k]
                d.color = (r * p[0], g * p[1], b * p[2], 1.0)
scene.render.engine = "BLENDER_WORKBENCH"
shading = scene.display.shading
shading.light = "STUDIO"
shading.color_type = "VERTEX"
shading.show_cavity = True
world = bpy.data.worlds.new("sky")
world.color = (0.47, 0.50, 0.49)
scene.world = world
scene.view_settings.view_transform = "Standard"
cam_data = bpy.data.cameras.new("cam")
cam_data.sensor_fit = "VERTICAL"
cam_data.angle_y = math.radians(35.0)
cam = bpy.data.objects.new("cam", cam_data)
scene.collection.objects.link(cam)
scene.camera = cam
scene.render.resolution_x, scene.render.resolution_y = 900, 640
if rig.animation_data:
    for track in rig.animation_data.nla_tracks:
        track.mute = True


def look(eye, at):
    cam.location = eye
    cam.rotation_euler = (Vector(at) - Vector(eye)).to_track_quat("-Z", "Y").to_euler()


for shot in shots.split(","):
    clip, share = shot.split(":")
    action = next((a for a in bpy.data.actions if a.name.split(".")[0] == clip), None)
    if action is None:
        print("no clip", clip, [a.name for a in bpy.data.actions])
        continue
    rig.animation_data.action = action
    a, b = action.frame_range
    scene.frame_set(int(round(a + (b - a) * float(share))))
    # (The glTF's -Z is the way it faces: in Blender, +Y.)
    for name, eye in (("side", (3.4, 0.2, 0.9)), ("front", (1.6, 2.6, 1.1))):
        look(eye, (0.0, 0.1, 0.5))
        scene.render.filepath = os.path.join(out, f"{model}_{clip.lower()}_{int(float(share) * 100)}_{name}.png")
        bpy.ops.render.render(write_still=True)
print("figure_views: done")
