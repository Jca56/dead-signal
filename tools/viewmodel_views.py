"""Pictures of a weapon's viewmodel as the eye sees it, to check it away
from the game: its hip hold, down its sights, and midway through a reload.

    /opt/blender-bin-5.2.1/blender -b --factory-startup --python tools/viewmodel_views.py -- <out dir> lmg flamethrower
"""

import math
import os
import sys

import bpy

HERE = os.path.dirname(os.path.abspath(__file__))
MODELS = os.path.join(HERE, "..", "assets", "models")
args = sys.argv[sys.argv.index("--") + 1:]
out, names = args[0], args[1:]

for name in names:
    bpy.ops.wm.read_factory_settings(use_empty=True)
    scene = bpy.context.scene
    bpy.ops.import_scene.gltf(filepath=os.path.join(MODELS, f"viewmodel_{name}.glb"))
    rig = next(o for o in bpy.data.objects if o.type == "ARMATURE")
    scene.render.engine = "BLENDER_WORKBENCH"
    shading = scene.display.shading
    shading.light = "STUDIO"
    shading.color_type = "VERTEX"
    shading.show_cavity = True
    world = bpy.data.worlds.new("sky")
    world.color = (0.35, 0.38, 0.36)
    scene.world = world
    scene.view_settings.view_transform = "Standard"
    scene.view_settings.exposure = 0.8
    cam_data = bpy.data.cameras.new("cam")
    cam_data.sensor_fit = "VERTICAL"
    cam_data.angle_y = math.radians(55.0)
    cam_data.clip_start = 0.01
    cam = bpy.data.objects.new("cam", cam_data)
    cam.rotation_euler = (math.radians(90.0), 0.0, 0.0)
    scene.collection.objects.link(cam)
    scene.camera = cam
    scene.render.resolution_x, scene.render.resolution_y = 1280, 720
    if rig.animation_data:
        for track in rig.animation_data.nla_tracks:
            track.mute = True
    for clip, share in (("Idle", 0.0), ("Aim", 0.0), ("Reload", 0.3), ("Reload", 0.6), ("Fire", 0.0)):
        action = next((a for a in bpy.data.actions if a.name.split(".")[0] == clip), None)
        if action is None:
            print("no clip", clip, [a.name for a in bpy.data.actions])
            continue
        rig.animation_data.action = action
        a, b = action.frame_range
        scene.frame_set(int(round(a + (b - a) * share)))
        scene.render.filepath = os.path.join(out, f"vm_{name}_{clip.lower()}_{int(share * 100)}.png")
        bpy.ops.render.render(write_still=True)
print("viewmodel_views: done")
