"""What every blade in the hands is made and moved with (`knife.py`,
`machete.py`, `axe.py`). A blade's frame is a gun's (`gun_kit.py`): its
origin in the right fist, x right, y along the knuckles, z up through the
fist; the handle runs along z through the hand, the blade on up above it,
its edge towards +y. It's the "gun" bone, as a gun is.

Its clips are posed as the guns' are (`poses.py`): an Idle breath, then
each swing a list of keys, the weapon's frame at each, baked.
"""

import math

import bpy
from mathutils import Matrix, Vector

import poses

STEEL = (0.52, 0.53, 0.52)
EDGE = (0.82, 0.82, 0.80)
POLYMER = (0.08, 0.08, 0.08)
WOOD = (0.44, 0.30, 0.16)
RED = (0.62, 0.10, 0.08)


def guard(left_rest):
    """The left hand where it rests: up in its guard."""
    return left_rest


def haft(gun, along):
    """The left hand round a haft, `along` it from the right hand (below
    it: negative), fingers wrapped round from the far side."""
    o = gun.translation
    right, knuckles, up = gun.col[0].xyz, gun.col[1].xyz, gun.col[2].xyz
    wrist = o + up * along - right * 0.06 - knuckles * 0.02
    fwd = right * 0.8 + knuckles * 0.2 + up * 0.1
    back = -knuckles * 0.9 - right * 0.2
    return wrist, fwd, back


def animate(rig, gun_rest, left_rest, rest, clips, left=None, shoulders=None):
    """Every clip, posed and baked; their names. `rest` is the weapon's
    frame at rest, `clips` {name: [(frame, weapon's frame)]} (frames from
    0), `left(g)` where the left hand is for a weapon at `g` (none: up in
    its guard), `shoulders` (left, right) moves if they're rolled forward."""
    r = poses.Rig(rig, gun_rest, left_rest)
    r.add_ik()
    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.mode_set(mode="POSE")
    work = bpy.data.actions.new("work")
    rig.animation_data.action = work
    hold = left or (lambda g: guard(left_rest))

    def steady(frame):
        poses.key_bone(rig, "gun", frame, scale=1.0)
        if shoulders:
            poses.shoulder(rig, "upper_arm.L", frame, shoulders[0])
            poses.shoulder(rig, "upper_arm.R", frame, shoulders[1])

    spans = {}
    # Idle: frames 1..91, a slow breath.
    start = 1
    for f in range(start, start + 91, 5):
        t = (f - start) / 90.0
        g = Matrix.Translation(Vector((0, 0, 0.005 * math.sin(t * math.tau)))) @ rest
        r.key(f, g, hold(g))
        steady(f)
    spans["Idle"] = (start, start + 90)

    for k, (name, keys) in enumerate(clips.items()):
        start = 101 + 100 * k
        for f, g in keys:
            r.key(start + f, g, hold(g))
            steady(start + f)
        spans[name] = (start, start + keys[-1][0])

    for name, (a, b) in spans.items():
        rig.animation_data.action = work
        poses.bake(rig, name, a, b)
    r.clear()
    bpy.data.actions.remove(work)
    bpy.ops.object.mode_set(mode="OBJECT")
    return list(spans)
