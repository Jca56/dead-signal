"""What every sidearm after the pistol shares (`pistol45.py`, `uzi.py`,
`magnum.py`): held out in both hands, the left cupped under the right; a
kick a round; tipped to reload, what holds its rounds out of it, the left
hand away for more and back, shut, and racked; a whip with it; raised to
the eye. (The pistol itself, `pistol.py`, is as it was made: these are
its clips, measured out.)

A gun module gives its measurements as a `Design` and builds its own parts
on these bones:

    gun    everything that doesn't move; follows the right hand
    slide  what goes back as it fires, and is racked: a slide, a bolt's
           knob, a hammer
    mag    what holds its rounds: a magazine dropped out of the grip along
           its bone and a fresh one brought, or (not `swapped`) a cylinder
           swung out along its bone, filled where it is, and shut
    flash  a star at the muzzle, the frame a shot goes off

    Idle     3 s      a two-handed grip, breathing
    Fire     `fire`   the kick, the slide back and home, the flash
    Reload   `reload` as above
    Bash     0.5 s    a whip with it, landing on frame 8
    Aim      3 s      raised to the eye, the sights on the middle of the view
    AimFire  `fire`   the kick from there
"""

import math
from dataclasses import dataclass

import bpy
from mathutils import Vector

import poses


@dataclass
class Design:
    """A sidearm's measurements, in its own frame (x right, y down the
    barrel, z up, the origin in the palm), metres; its clips' lengths,
    frames at 30 a second."""
    sight_line: float
    aim_distance: float
    muzzle: Vector
    fire_frames: int
    # How hard it kicks (degrees up), and how far its slide goes back.
    kick: float
    slide_back: float
    reload_frames: int
    # How far out its magazine goes, and whether that's dropped and a
    # fresh one brought (else it's a cylinder: out, filled, and shut).
    mag_out: float
    swapped: bool
    # How far the slide's drawn back to rack it.
    rack: float


def animate(rig, gun_rest, left_rest, d):
    """Every clip, posed and baked; their names."""
    r = poses.Rig(rig, gun_rest, left_rest)
    r.add_ik()
    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.mode_set(mode="POSE")
    finger = poses.trigger_finger(rig)
    work = bpy.data.actions.new("work")
    rig.animation_data.action = work

    def steady(frame, flash=0.0, slide=0.0, mag=0.0, mag_scale=1.0):
        poses.key_bone(rig, "flash", frame, scale=flash)
        poses.key_bone(rig, "slide", frame, loc=Vector((0, -slide, 0)))
        poses.key_bone(rig, "mag", frame, loc=Vector((0, -mag, 0)), scale=mag_scale)
        poses.key_bone(rig, "index1.R", frame, rot=finger)
        poses.key_bone(rig, "gun", frame, scale=1.0)

    spans = {}
    base = poses.gun_pose()

    # Idle: frames 1..91, a slow breath.
    start = 1
    for f in range(start, start + 91, 5):
        t = (f - start) / 90.0
        g = poses.gun_pose(Vector((0, 0, 0.004 * math.sin(t * math.tau))), pitch=0.6 * math.sin(t * math.tau))
        r.key(f, g, poses.support(g))
        steady(f)
    spans["Idle"] = (start, start + 90)

    def fire(start, pose, rest, kick):
        """A round: the flash, the kick and the slide back, home by the
        last frame (a long clip settles on the way)."""
        n = d.fire_frames
        keys = [(0, rest, 0.0, 1.0), (1, pose(Vector((0.0, -0.025, 0.012)), kick), d.slide_back, 0.0)]
        if n >= 5:
            keys.append((n // 2, pose(Vector((0.0, -0.01, 0.004)), kick * 0.3), 0.0, 0.0))
        keys.append((n, rest, 0.0, 0.0))
        for f, g, slide, fl in keys:
            r.key(start + f, g, poses.support(g))
            steady(start + f, flash=fl, slide=slide)
        return (start, start + n)

    spans["Fire"] = fire(101, lambda o, p: poses.gun_pose(o, pitch=p), base, d.kick)

    # Reload: from frame 201, over `reload_frames` (the pistol's own, by
    # its shares).
    start, n = 201, d.reload_frames
    tilt = poses.gun_pose(Vector((0.01, -0.02, 0.025)), pitch=10.0, roll=25.0)
    low = poses.away(Vector((-0.20, 0.10, -0.46)))
    near = poses.away(Vector((-0.12, 0.18, -0.30)))
    out, gone = d.mag_out, 0.0 if d.swapped else 1.0
    keys = [
        (0, base, poses.support(base), 0.0, 1.0, 0.0),
        (5, tilt, near, 0.0, 1.0, 0.0),
        (9, tilt, low, out * 0.45 if d.swapped else out, 1.0, 0.0),
        (12, tilt, low, out, gone, 0.0),
        (18, tilt, low, out, gone, 0.0),
        (20, tilt, low, out * 0.4 if d.swapped else out, 1.0, 0.0),
        (25, tilt, poses.support(tilt, 0.0, 0.0, -0.06), out * 0.3 if d.swapped else out, 1.0, 0.0),
        (29, tilt, poses.support(tilt, 0.0, 0.0, -0.02), 0.0, 1.0, 0.0),
        (32, tilt, poses.support(tilt, 0.02, -0.06, 0.07), 0.0, 1.0, 0.0),
        (34, tilt, poses.support(tilt, 0.02, -0.12, 0.07), 0.0, 1.0, d.rack),
        (35, tilt, poses.support(tilt, 0.02, -0.12, 0.05), 0.0, 1.0, 0.0),
        (42, base, poses.support(base), 0.0, 1.0, 0.0),
    ]
    for f42, g, left, mag, mag_scale, slide in keys:
        f = start + round(f42 / 42.0 * n)
        r.key(f, g, left)
        steady(f, slide=slide, mag=mag, mag_scale=mag_scale)
    spans["Reload"] = (start, start + n)

    # Bash: frames 401..416, the strike landing on 409.
    start = 401
    wind = poses.gun_pose(Vector((0.05, -0.06, 0.08)), pitch=35.0, roll=20.0, yaw=-10.0)
    strike = poses.gun_pose(Vector((-0.06, 0.12, -0.03)), pitch=-55.0, roll=-10.0, yaw=15.0)
    guard = poses.away(Vector((-0.20, 0.16, -0.34)))
    for f, g, left in ((0, base, poses.support(base)), (4, wind, guard), (8, strike, guard), (10, strike, guard), (15, base, poses.support(base))):
        r.key(start + f, g, left)
        steady(start + f)
    spans["Bash"] = (start, start + 15)

    # Aim: frames 501..591, dead still (the game sways it about the eye).
    start = 501
    aimed = poses.aim_pose(d.sight_line, d.aim_distance)
    for f in (start, start + 90):
        r.key(f, aimed, poses.support(aimed))
        steady(f)
    spans["Aim"] = (start, start + 90)

    spans["AimFire"] = fire(601, lambda o, p: poses.aim_pose(d.sight_line, d.aim_distance, Vector((o.x, o.y, o.z * 0.6)), pitch=p), aimed, d.kick * 0.8)

    # The flash pops for one frame, and a magazine's there or it isn't.
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
