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
plan("plan_ground", 4.0, 1.0, 174.0, 69.0, 2.3, 2400)
plan("plan_upper", 4.0, 1.0, 174.0, 69.0, 5.3, 2400)
plan("plan_roofs", 4.0, 1.0, 174.0, 69.0, 40.0, 2400)
plan("station_ground", 59.0, 14.0, 54.0, 39.0, 2.3)
plan("station_upper", 59.0, 14.0, 54.0, 39.0, 5.3)
plan("station_bunker", 59.0, 21.0, 54.0, 51.0, -0.5)
plan("motor_ground", -55.0, 7.5, 51.0, 75.0, 2.3)
plan("motor_upper", -55.0, 13.0, 48.0, 42.0, 5.3)
plan("barracks_ground", 42.0, -15.0, 84.0, 33.0, 2.3)
plan("barracks_upper", 37.0, -18.0, 45.0, 27.0, 5.3)
plan("core_ground", -12.0, -12.0, 60.0, 39.0, 2.3)

# Views from inside, at eye height.
view("in_control_room", (10.0, 1.85, -19.0), (0.0, 1.4, -26.0))
view("in_yard_to_station", (3.0, 1.6, 9.0), (36.0, 2.5, 13.5))
view("in_yard_to_motor", (-3.0, 1.6, 9.0), (-36.0, 2.0, 8.0))
view("in_bay_from_door", (-38.0, 1.85, 9.0), (-73.5, 2.2, 9.0))
view("in_bay_from_mezzanine", (-55.5, 4.85, 19.5), (-55.5, 0.5, 0.0))
view("in_bay_stairs", (-67.5, 1.85, 7.5), (-74.0, 2.0, 16.5))
view("in_lobby", (46.0, 1.85, 13.5), (37.5, 1.2, 24.0))
view("in_lobby_up", (46.0, 1.85, 13.5), (37.5, 1.6, 0.0))
view("in_corridor", (49.5, 1.85, 14.0), (75.0, 1.6, 14.0))
view("in_newsroom", (50.0, 1.85, 19.0), (73.5, 1.2, 28.0))
view("in_studio", (50.0, 4.85, 19.0), (73.5, 4.2, 28.0))
view("in_servers", (73.5, 4.85, 8.0), (63.0, 4.0, 1.0))
view("in_bunker_landing", (46.0, -1.15, 14.0), (37.5, -2.0, 22.5))
view("in_ops", (50.0, -1.15, 9.0), (74.0, -1.8, 19.0))
view("in_plant", (46.5, -1.15, 9.0), (39.0, -2.0, 1.0))
view("in_stores_breach", (64.5, -1.15, 22.5), (69.0, -1.6, 29.0))
view("in_tunnel", (69.0, -1.15, 31.0), (69.0, -1.6, 43.0))
view("in_dorm", (25.0, 4.85, -10.5), (52.5, 4.0, -27.0))
view("in_mess", (24.0, 1.85, -11.0), (38.0, 1.2, -27.0))
view("in_west_lot", (-39.0, 1.6, -7.5), (-72.0, 1.5, -24.0))
view("in_east_court", (56.0, 1.6, -7.0), (78.0, 1.5, -25.5))
view("over_all", (3.0, 90.0, 105.0), (3.0, 0.0, 3.0), lens=28.0, px=(2000, 1100))
print("relay_views: done")
