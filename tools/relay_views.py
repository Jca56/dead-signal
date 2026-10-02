"""Pictures of HOLDOUT's arena as the game builds it, to check it by eye
away from the game: plans cut through at each level, and a few views from
inside. It reads what `holdout::dump` writes (the built mesh, and where
each model stands).

    RELAY_DUMP=/some/dir cargo test --release holdout::dump -- --ignored
    RELAY_DUMP=/some/dir /opt/blender-bin-5.2.1/blender -b --factory-startup --python tools/relay_views.py

Writes PNGs into the same directory (or RELAY_OUT).
"""

import math
import os

import bpy
from mathutils import Vector

HERE = os.path.dirname(os.path.abspath(__file__))
MODELS = os.path.join(HERE, "..", "assets", "models")
DUMP = os.environ["RELAY_DUMP"]
OUT = os.environ.get("RELAY_OUT", DUMP)
ONLY = [v for v in os.environ.get("RELAY_VIEWS", "").split(",") if v]

bpy.ops.wm.read_factory_settings(use_empty=True)
scene = bpy.context.scene

# What the game built.
bpy.ops.wm.ply_import(filepath=os.path.join(DUMP, "world.ply"))

# The models, each kept aside to be stood where the game stands them.
models = {}
for name in ("sites", "furniture", "containers", "scenery"):
    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=os.path.join(MODELS, name + ".glb"))
    for o in set(bpy.data.objects) - before:
        models[o.name] = o
        o.hide_render = True
        o.hide_viewport = True


def mark(colour, size):
    mesh = bpy.data.meshes.new("mark")
    s = size / 2
    verts = [(-s, -s, -s), (s, -s, -s), (s, s, -s), (-s, s, -s), (-s, -s, s), (s, -s, s), (s, s, s), (-s, s, s)]
    faces = [(0, 3, 2, 1), (4, 5, 6, 7), (0, 1, 5, 4), (1, 2, 6, 5), (2, 3, 7, 6), (3, 0, 4, 7)]
    mesh.from_pydata(verts, [], faces)
    col = mesh.color_attributes.new("Col", "BYTE_COLOR", "CORNER")
    for d in col.data:
        d.color = (*colour, 1.0)
    return mesh


MARKS = {"MARK_Buy": mark((0.05, 0.3, 1.0), 0.35), "MARK_Window": mark((1.0, 0.05, 0.1), 0.4), "MARK_From": mark((1.0, 0.3, 0.8), 0.5)}

missing = set()
with open(os.path.join(DUMP, "pieces.txt")) as f:
    for line in f:
        name, x, y, z, yaw = line.split()
        x, y, z, yaw = float(x), float(y), float(z), float(yaw)
        if name in MARKS:
            o = bpy.data.objects.new(name, MARKS[name])
        elif name in models:
            o = models[name].copy()
            o.hide_render = False
            o.hide_viewport = False
        else:
            missing.add(name)
            continue
        o.rotation_mode = "XYZ"
        o.location = (x, -z, y)
        o.rotation_euler = (0.0, 0.0, yaw)
        scene.collection.objects.link(o)
if missing:
    print("relay_views: no model for", sorted(missing))

scene.render.engine = "BLENDER_WORKBENCH"
shading = scene.display.shading
shading.light = "STUDIO"
shading.color_type = "VERTEX"
shading.show_cavity = True
shading.cavity_type = "BOTH"
shading.show_shadows = False
shading.show_backface_culling = False
scene.render.film_transparent = False
world = bpy.data.worlds.new("sky")
world.color = (0.55, 0.6, 0.62)
scene.world = world
scene.view_settings.view_transform = "Standard"
scene.view_settings.exposure = 1.3

cam_data = bpy.data.cameras.new("cam")
cam = bpy.data.objects.new("cam", cam_data)
scene.collection.objects.link(cam)
scene.camera = cam


def shot(name):
    if ONLY and name not in ONLY:
        return False
    scene.render.filepath = os.path.join(OUT, name + ".png")
    return True


def plan(name, cx, cz, wide, deep, cut, px=2000):
    """Looking straight down on (cx, cz), `wide` by `deep` metres, with
    everything over `cut` taken off."""
    if not shot(name):
        return
    cam_data.type = "ORTHO"
    cam_data.ortho_scale = max(wide, deep * 1.0)
    cam_data.clip_start = 0.01
    cam_data.clip_end = 60.0
    cam.location = (cx, -cz, cut)
    cam.rotation_euler = (0.0, 0.0, 0.0)
    scene.render.resolution_x = px
    scene.render.resolution_y = int(px * deep / wide)
    cam_data.ortho_scale = wide
    bpy.ops.render.render(write_still=True)


def view(name, eye, at, lens=18.0, px=(1600, 900)):
    """From `eye` towards `at` (the game's x, y, z)."""
    if not shot(name):
        return
    cam_data.type = "PERSP"
    cam_data.lens = lens
    cam_data.clip_start = 0.05
    cam_data.clip_end = 300.0
    e = Vector((eye[0], -eye[2], eye[1]))
    a = Vector((at[0], -at[2], at[1]))
    cam.location = e
    cam.rotation_euler = (a - e).to_track_quat("-Z", "Y").to_euler()
    scene.render.resolution_x, scene.render.resolution_y = px
    bpy.ops.render.render(write_still=True)


# Plans: all of it at each level, then each wing closer.
plan("plan_ground", 2.5, 0.5, 116.0, 46.0, 2.3, 2400)
plan("plan_upper", 2.5, 0.5, 116.0, 46.0, 5.3, 2400)
plan("plan_roofs", 2.5, 0.5, 116.0, 46.0, 40.0, 2400)
plan("station_ground", 39.5, 9.5, 36.0, 26.0, 2.3)
plan("station_upper", 39.5, 9.5, 36.0, 26.0, 5.3)
plan("station_bunker", 39.5, 14.0, 36.0, 34.0, -0.5)
plan("motor_ground", -36.5, 5.0, 34.0, 50.0, 2.3)
plan("motor_upper", -36.5, 8.5, 32.0, 28.0, 5.3)
plan("barracks_ground", 28.0, -10.0, 56.0, 22.0, 2.3)
plan("barracks_upper", 24.5, -12.0, 30.0, 18.0, 5.3)
plan("core_ground", -8.0, -8.0, 40.0, 26.0, 2.3)

# Views from inside, at eye height.
view("in_control_room", (6.5, 1.85, -12.5), (0.0, 1.4, -17.5))
view("in_yard_to_station", (2.0, 1.6, 6.0), (24.0, 2.5, 9.0))
view("in_yard_to_motor", (-2.0, 1.6, 6.0), (-24.0, 2.0, 5.5))
view("in_bay_from_door", (-25.5, 1.85, 6.0), (-49.0, 2.2, 6.0))
view("in_bay_from_mezzanine", (-37.0, 4.85, 13.0), (-37.0, 0.5, 0.0))
view("in_bay_stairs", (-45.0, 1.85, 5.0), (-49.5, 2.0, 11.0))
view("in_lobby", (30.5, 1.85, 9.0), (25.0, 1.2, 16.0))
view("in_lobby_up", (30.5, 1.85, 9.0), (25.0, 1.6, 0.0))
view("in_corridor", (33.0, 1.85, 9.5), (50.0, 1.6, 9.5))
view("in_newsroom", (33.5, 1.85, 12.5), (49.0, 1.2, 18.5))
view("in_studio", (33.5, 4.85, 12.5), (49.0, 4.2, 18.5))
view("in_servers", (49.0, 4.85, 5.5), (42.0, 4.0, 0.5))
view("in_bunker_landing", (30.5, -1.15, 9.5), (25.0, -2.0, 15.0))
view("in_ops", (33.5, -1.15, 6.0), (49.5, -1.8, 12.5))
view("in_plant", (31.0, -1.15, 6.0), (26.0, -2.0, 0.5))
view("in_stores_breach", (43.0, -1.15, 15.0), (46.0, -1.6, 19.5))
view("in_tunnel", (46.0, -1.15, 20.5), (46.0, -1.6, 28.5))
view("in_dorm", (16.5, 4.85, -7.0), (35.0, 4.0, -18.0))
view("in_mess", (16.0, 1.85, -7.5), (25.5, 1.2, -18.0))
view("in_west_lot", (-26.0, 1.6, -5.0), (-48.0, 1.5, -16.0))
view("in_east_court", (37.5, 1.6, -4.5), (52.0, 1.5, -17.0))
view("over_all", (2.0, 60.0, 70.0), (2.0, 0.0, 2.0), lens=28.0, px=(2000, 1100))
print("relay_views: done")
