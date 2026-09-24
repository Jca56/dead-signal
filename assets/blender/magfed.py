"""What every magazine-fed long gun in the hands shares (`smg.py`, `ar.py`):
held at the hip in the right hand, the left under its handguard; a quick
kick a round (the Fire clip is one round, played again and again for full
auto); a magazine dropped and a fresh one seated, the charging handle
racked; a shove with it; raised to the eye.

A gun module gives its measurements as a `Design` and builds its own parts
on these bones:

    gun     everything that doesn't move; follows the right hand
    mag     the magazine: drops out of the well along its length
    handle  the charging handle: drawn back along the top
    flash   a star at the muzzle, the frame a shot goes off

    Idle     3 s    held at the hip, breathing
    Fire     `fire` the kick, the handle jolting, the flash
    Reload   `reload` tipped, the mag out and gone, a fresh one up and
                    seated, the handle racked
    Bash     0.6 s  a shove with the side of the gun, landing on frame 8
    Aim      3 s    shouldered, the sights on the middle of the view
    AimFire  `fire` the kick from there
"""

import math
from dataclasses import dataclass

import bpy
from mathutils import Matrix, Vector

import poses


@dataclass
class Design:
    """A gun's measurements, in its own frame (x right, y down the barrel,
    z up, the origin in the palm), metres; and its clips' lengths, frames
    at 30 a second."""
    sight_line: float
    aim_distance: float
    hip: Vector
    forend: float
    # The magazine well (where the mag bone sits), and the way it drops out.
    well: Vector
    drop: Vector
    handle: Vector
    handle_back: float
    muzzle: Vector
    fire_frames: int
    kick: float
    reload_frames: int
    left_shoulder: Vector
    right_shoulder: Vector


def bones(d, to_rig, barrel, hand):
    """The gun's bones, as {name: (head, tail, parent)}."""
    at = lambda v: to_rig @ v  # noqa: E731
    down = (to_rig.to_3x3() @ d.drop).normalized()
    origin = to_rig.translation
    return {
        "gun": (origin, origin + barrel * 0.08, hand),
        "mag": (at(d.well), at(d.well) + down * 0.08, "gun"),
        "handle": (at(d.handle), at(d.handle) + barrel * 0.05, "gun"),
        "flash": (at(d.muzzle), at(d.muzzle) + barrel * 0.04, "gun"),
    }


def animate(rig, gun_rest, left_rest, d):
    """Every clip, posed and baked; their names."""
    r = poses.Rig(rig, gun_rest, left_rest)
    r.add_ik()
    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.mode_set(mode="POSE")
    finger = poses.trigger_finger(rig)
    work = bpy.data.actions.new("work")
    rig.animation_data.action = work

    def steady(frame, flash=0.0, mag=0.0, mag_scale=1.0, handle=0.0):
        poses.key_bone(rig, "flash", frame, scale=flash)
        # (The mag bone runs down out of the well: along it is out.)
        poses.key_bone(rig, "mag", frame, loc=Vector((0, mag, 0)), scale=mag_scale)
        poses.key_bone(rig, "handle", frame, loc=Vector((0, -handle, 0)))
        poses.key_bone(rig, "index1.R", frame, rot=finger)
        poses.key_bone(rig, "gun", frame, scale=1.0)
        poses.shoulder(rig, "upper_arm.L", frame, d.left_shoulder)
        poses.shoulder(rig, "upper_arm.R", frame, d.right_shoulder)

    def hip(offset=Vector(), pitch=0.0, roll=0.0, yaw=0.0):
        return poses.gun_pose(offset, pitch, roll, yaw, grip=d.hip)

    def aimed(offset=Vector(), pitch=0.0):
        return poses.aim_pose(d.sight_line, d.aim_distance, offset, pitch)

    def held(g, dx=0.0, dy=0.0, dz=0.0):
        return poses.forend(g, d.forend, dx, dy, dz)

    def at_well(g, below=0.0):
        """The left hand at the magazine well, `below` it."""
        o, barrel, up, right = g.translation, g.col[1].xyz, g.col[2].xyz, g.col[0].xyz
        well = o + right * d.well.x + barrel * d.well.y + up * (d.well.z - 0.06 - below)
        return well - right * 0.05 - barrel * 0.03, right * 0.8 + barrel * 0.3 + up * 0.3, -up * 0.9 - right * 0.2

    spans = {}
    base = hip()

    # Idle: frames 1..91, a slow breath.
    start = 1
    for f in range(start, start + 91, 5):
        t = (f - start) / 90.0
        g = hip(Vector((0, 0, 0.004 * math.sin(t * math.tau))), pitch=0.6 * math.sin(t * math.tau))
        r.key(f, g, held(g))
        steady(f)
    spans["Idle"] = (start, start + 90)

    def fire(start, pose, rest, kick):
        """A round: the flash, the kick, back to rest by the last frame."""
        n = d.fire_frames
        for f, g, fl, handle in ((0, rest, 1.0, 0.0), (1, pose(Vector((0.0, -0.022, 0.008)), kick), 0.0, 0.02), (n, rest, 0.0, 0.0)):
            r.key(start + f, g, held(g))
            steady(start + f, flash=fl, handle=handle)
        return (start, start + n)

    spans["Fire"] = fire(101, lambda o, p: hip(o, pitch=p), base, d.kick)

    # Reload: from frame 201, over `reload_frames`.
    start, n = 201, d.reload_frames
    tipped = hip(Vector((-0.02, 0.02, 0.05)), pitch=8.0, roll=-25.0)
    low = poses.away(Vector((-0.18, 0.10, -0.40)))
    keys = [
        (0.0, base, held(base), 0.0, 1.0, 0.0),
        (0.12, tipped, at_well(tipped), 0.0, 1.0, 0.0),
        (0.22, tipped, at_well(tipped, 0.06), 0.10, 1.0, 0.0),
        (0.32, tipped, low, 0.32, 1.0, 0.0),
        (0.36, tipped, low, 0.32, 0.0, 0.0),
        (0.52, tipped, low, 0.32, 0.0, 0.0),
        (0.56, tipped, low, 0.30, 1.0, 0.0),
        (0.66, tipped, at_well(tipped, 0.08), 0.10, 1.0, 0.0),
        (0.74, tipped, at_well(tipped, 0.0), 0.0, 1.0, 0.0),
        (0.82, base, held(base, dz=0.03), 0.0, 1.0, 0.0),
        (0.88, base, held(base), 0.0, 1.0, 0.06),
        (0.92, base, held(base), 0.0, 1.0, 0.0),
        (1.0, base, held(base), 0.0, 1.0, 0.0),
    ]
    for share, g, left, mag, mag_scale, handle in keys:
        f = start + round(share * n)
        r.key(f, g, left)
        steady(f, mag=mag, mag_scale=mag_scale, handle=handle)
    spans["Reload"] = (start, start + n)

    # Bash: frames 401..419, the side of the gun shoved into a face.
    start = 401
    wind = hip(Vector((0.04, -0.08, 0.02)), roll=-10.0, yaw=25.0)
    shove = hip(Vector((-0.10, 0.12, 0.05)), pitch=5.0, yaw=-55.0)
    for f, g in ((0, base), (5, wind), (8, shove), (10, shove), (18, base)):
        r.key(start + f, g, held(g))
        steady(start + f)
    spans["Bash"] = (start, start + 18)

    # Aim: frames 501..591, shouldered and still.
    start = 501
    up = aimed()
    for f in (start, start + 90):
        r.key(f, up, held(up))
        steady(f)
    spans["Aim"] = (start, start + 90)

    spans["AimFire"] = fire(601, lambda o, p: aimed(o, p), up, d.kick * 0.4)

    # The flash pops for one frame, and the mag is there or it isn't.
    for fc in work.fcurves if hasattr(work, "fcurves") else []:
        if "flash" in fc.data_path or ('"mag"' in fc.data_path and "scale" in fc.data_path):
            for kp in fc.keyframe_points:
                kp.interpolation = "CONSTANT"

    for name, (a, b) in spans.items():
        rig.animation_data.action = work
        poses.bake(rig, name, a, b)
    r.clear()
    bpy.data.actions.remove(work)
    bpy.ops.object.mode_set(mode="OBJECT")
    return list(spans)


def turned(to_rig, angle):
    """`to_rig` turned `angle` radians about the gun's x axis (a mag's cant)."""
    return to_rig @ Matrix.Rotation(angle, 4, "X")
