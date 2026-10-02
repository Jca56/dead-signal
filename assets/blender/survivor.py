"""The living, as the others see them playing together: two survivors on
an upright rig named bone for bone as the dead's.

Run headless from the project root:
    /opt/blender-bin-5.2.1/blender -b --factory-startup --python assets/blender/survivor.py

Writes assets/models/survivor.glb: a 17-bone rig and its animations, keyed
straight onto the bones, and every part the two are put together from
(`survivor_body.py`), each a skinned mesh of its own on that one rig. The
game plays the legs' and the body's clips under them and puts the hands
on what's held (a gun where they aim, the arms reaching it); what's
swung and thrown the clips here swing and throw, in the right hand.

The legs' clips, one stride a cycle, the left foot planting at its start:
    Idle        3 s    standing easy, breathing (loops)
    Walk        1 s    1.5 m a stride (loops)
    Jog         0.7 s  4.2 m a stride: the everyday run (loops)
    Sprint      0.63 s 5.7 m a stride, arms pumping (loops)
    Crouch      2 s    down on the haunches (loops)
    CrouchWalk  0.8 s  2.4 m a stride (loops)
    Jump        0.4 s  legs tucked (holds)
    Land        0.3 s  a dip on landing
Down on the ground:
    Down        0.6 s  knees go, sat down (holds)
    Downed      2 s    sat up, leaning back on the left hand (loops)
    Scoot       0.9 s  dragged along on the heels (loops)
    BleedOut    1 s    slumped back, lying still (holds)
    Kneel       1.2 s  down on one knee, hands on a buddy (loops)
The body's, over the legs', each the length of what the game times it by:
    HoldBlade   2 s    a knife or machete at the ready (loops)
    HoldAxe     2 s    an axe on the shoulder (loops)
    Guard       2 s    fists up (loops)
    Stab        0.37 s the knife's thrust; lands at 0.13 s
    Swing       0.57 s the machete across, right to left; lands at 0.23 s
    Swing2      0.57 s and back, left to right
    Chop        0.97 s the axe down; lands at 0.47 s
    Jab, Cross  0.5 s  a right, a left; land at 0.2 s
    Windup      0.33 s arm back to throw (holds)
    Throw       0.5 s  over the top and through

Built facing Blender's +Y (the game's -Z), feet on the ground at the
origin, +X its right.
"""

import math
import os
import sys

import bpy
from mathutils import Matrix, Quaternion, Vector

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import survivor_body  # noqa: E402
from rig_kit import Actions, blend, export, flat_material, offset, part, rig, stash, turned, with_  # noqa: E402
from survivor_kit import ANKLE, BONES, J, legs_to, v  # noqa: E402

MODELS = os.path.join(HERE, "..", "models")
FPS = 30
TAU = math.tau


def build():
    """The rig, and every part on it."""
    armature = rig("SurvivorRig", BONES)
    material = flat_material()
    made = [part(name, make, armature, material) for name, make in survivor_body.parts().items()]
    return armature, made


# ---- where a pose puts the bones ---------------------------------------------------

def fk(pose):
    """Each bone's turn from rest (about the rig's own axes, all its
    parents' with it) and where its head is, posed as `pose` says."""
    turns = {"root": Matrix.Identity(3)}
    heads = {"root": Vector()}
    rest = {"root": Vector()}
    pending = dict(BONES)
    while pending:
        for name, (head, _, parent) in list(pending.items()):
            if parent not in turns:
                continue
            spec = pose.get(name, (0.0, 0.0, 0.0))
            heads[name] = heads[parent] + turns[parent] @ (head - rest[parent] + offset(spec))
            turns[name] = turns[parent] @ turned(spec)
            rest[name] = head
            del pending[name]
    return turns, heads


def chest(pose, x, y, z):
    """The point `x` right, `y` ahead and `z` up of the chest's root, as
    it's turned in `pose`."""
    turns, heads = fk(pose)
    return heads["chest"] + turns["chest"] @ Vector((x, y, z))


def frame(along, thumb):
    """A turn taking the rig's down to `along` and its ahead to `thumb`
    (made square to it)."""
    a = along.normalized()
    t = (thumb - a * thumb.dot(a)).normalized()
    to = Matrix((a, t, a.cross(t))).transposed()
    rest_a, rest_t = Vector((0, 0, -1)), Vector((0, 1, 0))
    at_rest = Matrix((rest_a, rest_t, rest_a.cross(rest_t))).transposed()
    return to @ at_rest.inverted()


def arm_to(pose, side, wrist, pole, fingers=None, thumb=None):
    """`pose` with the arm on `side` reaching its wrist to `wrist` (as far
    as it can), the elbow bent towards `pole`; the hand's fingers along
    `fingers` and its thumb along `thumb` if given (else straight on from
    the forearm)."""
    out = dict(pose)
    u, f, h = f"upper_arm{side}", f"forearm{side}", f"hand{side}"
    turns, heads = fk(out)
    shoulder = heads[u]
    a = (J[u][1] - J[u][0]).length
    b = (J[f][1] - J[f][0]).length
    to = wrist - shoulder
    d = min(max(to.length, 0.05), a + b - 1e-4)
    along = to.normalized()
    cos_s = (a * a + d * d - b * b) / (2 * a * d)
    bend = pole - along * pole.dot(along)
    bend = bend.normalized() if bend.length > 1e-6 else Vector((0, 0, -1))
    elbow = shoulder + along * (a * cos_s) + bend * (a * math.sqrt(max(0.0, 1 - cos_s * cos_s)))
    hand = shoulder + along * d
    rest_u = (J[u][1] - J[u][0]).normalized()
    rest_f = (J[f][1] - J[f][0]).normalized()
    upper = rest_u.rotation_difference((elbow - shoulder).normalized()).to_matrix()
    fore = (upper @ rest_f).rotation_difference((hand - elbow).normalized()).to_matrix() @ upper
    out[u] = (turns["chest"].inverted() @ upper).to_quaternion()
    out[f] = (upper.inverted() @ fore).to_quaternion()
    if fingers is not None:
        out[h] = (fore.inverted() @ frame(fingers, thumb if thumb is not None else Vector((0, 1, 0)))).to_quaternion()
    else:
        out[h] = Quaternion()
    return out


def arms_to(pose, right, left):
    """Both arms: `right` and `left` each (wrist, pole[, fingers, thumb]),
    in the chest's frame (see `chest`), the poles in the rig's."""
    out = dict(pose)
    for side, spec in ((".R", right), (".L", left)):
        if spec is None:
            continue
        at, pole = spec[0], spec[1]
        rest = spec[2:] if len(spec) > 2 else ()
        out = arm_to(out, side, chest(out, *at), Vector(pole), *[Vector(r) for r in rest])
    return out


# ---- the legs ----------------------------------------------------------------------

def stand(breath=0.0, sway=0.0):
    """Standing easy, breathing in `breath` (−1–1), swaying `sway`."""
    pose = {
        "spine": (-1 + 0.8 * breath, sway, 0),
        "chest": (1.2 * breath, 0, 0),
        "neck": (-0.5 * breath, 0, 0),
        "head": (0, -sway, 0),
        "upper_arm.R": (2, 5, 0),
        "upper_arm.L": (2, -5, 0),
        "forearm.R": (10 + 2 * breath, 0, 0),
        "forearm.L": (10 + 2 * breath, 0, 0),
    }
    return legs_to(pose, {".L": (0.0, ANKLE, 0), ".R": (0.0, ANKLE, 0)}, hips=(0.01 * sway, 0, -0.01 + 0.004 * breath))


def gait(p, stride, duty, reach, lift, low, bob, lean, twist, arms, elbow):
    """A stride at phase `p` (0–1, the left foot planting at 0): each foot
    on the ground for `duty` of it, sliding back under the body (at most
    `reach` ahead or behind), then swung through `lift` high; the hips
    `low`, bobbing `bob` (up at mid-stride walking, down running), leaning
    `lean` degrees, turned `twist` with the stride, the arms swinging
    `arms` degrees against the legs, the elbows bent `elbow`."""
    feet = {}
    a = min(stride * duty / 2, reach)
    for side, off in ((".L", 0.0), (".R", 0.5)):
        q = (p + off) % 1.0
        if q < duty:
            s = q / duty
            heel = max(0.0, (s - 0.65) / 0.35)
            feet[side] = (a * (1 - 2 * s), ANKLE + 0.05 * heel, -30 * heel)
        else:
            s = (q - duty) / (1 - duty)
            e = 0.5 - 0.5 * math.cos(math.pi * s)
            feet[side] = (-a + 2 * a * e, ANKLE + lift * math.sin(math.pi * s), 14 * math.sin(math.pi * s) - 10 * (1 - s))
    c = math.cos(TAU * p)
    pose = {
        "hips": (0, 0, -twist * c),
        "spine": (-lean, 0, 0),
        "chest": (-lean * 0.3, 0, twist * 1.6 * c),
        "neck": (lean * 0.7, 0, -twist * 0.6 * c),
        "head": (lean * 0.6, 0, 0),
        "upper_arm.R": (arms * c, 6, 0),
        "upper_arm.L": (-arms * c, -6, 0),
        "forearm.R": (elbow + 0.3 * arms * max(0.0, c), 0, 0),
        "forearm.L": (elbow + 0.3 * arms * max(0.0, -c), 0, 0),
    }
    return legs_to(pose, feet, hips=(0, 0, low + bob * math.cos(2 * TAU * (p - duty / 2))))


def crouch(breath=0.0):
    """Down on the haunches: the left foot ahead, the right behind on its
    toes, leaning in."""
    pose = {
        "spine": (-14 + breath, 0, 0),
        "chest": (-6 + breath, 0, 0),
        "neck": (12, 0, 0),
        "head": (8, 0, 0),
        "upper_arm.R": (20, 8, 0),
        "upper_arm.L": (20, -8, 0),
        "forearm.R": (35, 0, 0),
        "forearm.L": (35, 0, 0),
    }
    return legs_to(pose, {".L": (0.16, ANKLE, 0), ".R": (-0.15, ANKLE + 0.035, -28)}, hips=(0, 0.02, -0.42 + 0.004 * breath))


def cycle(action, name, frames, pose_at, step=1):
    """A looping clip `frames` long, `pose_at(phase)`, its last key its
    first."""
    action(name, [(f, pose_at(f / frames)) for f in range(0, frames + 1, step)])


def legs(action):
    cycle(action, "Idle", 90, lambda p: stand(math.sin(p * TAU), 0.6 * math.sin(p * TAU * 0.5)), 10)
    cycle(action, "Walk", 30, lambda p: gait(p, 1.5, 0.62, 0.34, 0.10, -0.015, 0.01, 3, 5, 18, 12))
    cycle(action, "Jog", 21, lambda p: gait(p, 4.2, 0.40, 0.38, 0.20, -0.045, -0.02, 10, 8, 35, 75))
    cycle(action, "Sprint", 19, lambda p: gait(p, 5.7, 0.35, 0.45, 0.28, -0.06, -0.025, 17, 10, 55, 88))
    cycle(action, "Crouch", 60, lambda p: crouch(math.sin(p * TAU)), 10)
    cycle(action, "CrouchWalk", 24, lambda p: gait(p, 2.4, 0.60, 0.28, 0.08, -0.38, 0.01, 16, 6, 10, 32))
    rest = stand()
    tuck = legs_to(with_(rest, spine=(-8, 0, 0), upper_arm__R=(25, -12, 0), upper_arm__L=(25, 12, 0), forearm__R=(40, 0, 0), forearm__L=(40, 0, 0)),
                   {".L": (0.12, ANKLE + 0.34, 12), ".R": (-0.06, ANKLE + 0.26, -12)})
    action("Jump", [(0, rest), (6, tuck), (12, tuck)])
    squat = legs_to(with_(rest, spine=(-12, 0, 0), neck=(8, 0, 0), upper_arm__R=(15, -8, 0), upper_arm__L=(15, 8, 0)), {".L": (0.03, ANKLE, 0), ".R": (-0.02, ANKLE, 0)}, hips=(0, 0, -0.20))
    action("Land", [(0, blend(squat, rest, 0.4)), (3, squat), (9, rest)])


# ---- down on the ground ------------------------------------------------------------

def sat(breath=0.0, left=(27, -5), right=(60, -95), rock=0.0):
    """Sat on the ground leaning back on the left hand, the right arm out
    (the game puts the sidearm in it and aims it): the legs' thigh and
    shin turns each `left` and `right`."""
    pose = {
        "hips": (55 + rock, 0, 0, (0, -0.05, -0.76)),
        "spine": (-10 + breath, 0, 0),
        "chest": (-5 + breath, 0, 0),
        "neck": (-24, 0, 0),
        "head": (-16, 0, 0),
        "thigh.L": (left[0], 0, 4),
        "shin.L": (left[1], 0, 0),
        "foot.L": (12, 0, 0),
        "thigh.R": (right[0], 0, -4),
        "shin.R": (right[1], 0, 0),
        "foot.R": (-(55 + right[0] + right[1]) + 5, 0, 0),
    }
    turns, heads = fk(pose)
    pose = arm_to(pose, ".L", heads["hips"] + Vector((-0.27, -0.24, -0.08)), Vector((0, -1, 0.2)), Vector((0.1, 0.4, -1)), Vector((0, 1, 0)))
    return arm_to(pose, ".R", heads["chest"] + Vector((0.22, 0.42, 0.12)), Vector((0.6, 0, -1)))


def lying(t):
    """Slumped back from sitting up, `t` of the way to flat on the back,
    the arms fallen out."""
    pose = {
        "hips": (55 + 33 * t, 0, 0, (0, -0.05 - 0.05 * t, -0.76 - 0.08 * t)),
        "spine": (-10 + 10 * t, 0, 0),
        "chest": (-5 + 8 * t, 0, 0),
        "neck": (-24 + 20 * t, 0, 0),
        "head": (-16 + 26 * t, 22 * t, 0),
        "thigh.L": (27 - 10 * t, 0, 6),
        "shin.L": (-5, 0, 0),
        "foot.L": (12, 0, -10 * t),
        "thigh.R": (60 - 40 * t, 0, -8),
        "shin.R": (-95 + 80 * t, 0, 0),
        "foot.R": (-20 + 30 * t, 0, 10 * t),
    }
    turns, heads = fk(pose)
    pose = arm_to(pose, ".L", heads["hips"] + Vector((-0.27 - 0.2 * t, -0.24 - 0.02 * t, -0.08 + 0.02 * t)), Vector((0, -1, 0.2)))
    return arm_to(pose, ".R", heads["hips"] + Vector((0.28 + 0.2 * t, 0.2 - 0.25 * t, 0.15 - 0.2 * t)), Vector((1, 0, -0.5)))


def kneel(push=0.0):
    """Down on the right knee, the left foot planted, leant over a buddy on
    the ground, both hands on them (pressing, `push` 0–1)."""
    pose = {
        "spine": (-22 - 3 * push, 0, 0),
        "chest": (-16 - 3 * push, 0, 0),
        "neck": (16, 0, 0),
        "head": (12, 0, 0),
        "thigh.R": (-6, 0, 0),
        "shin.R": (-96, 0, 0),
        "foot.R": (-40, 0, 0),
    }
    pose = legs_to(pose, {".L": (0.30, ANKLE, 0)}, hips=(0, 0.02, -0.41))
    pose["thigh.R"], pose["shin.R"], pose["foot.R"] = (-6, 0, 0), (-96, 0, 0), (-40, 0, 0)
    return arms_to(pose, ((0.11, 0.42, -0.52 - 0.05 * push), (0.5, 0, -1)), ((-0.11, 0.42, -0.52 - 0.05 * push), (-0.5, 0, -1)))


def down(action):
    rest = stand()
    buckle = legs_to(with_(rest, spine=(-12, 0, 0), head=(10, 0, 0), upper_arm__R=(30, -15, 0), upper_arm__L=(30, 15, 0)), {".L": (0.08, ANKLE, 0), ".R": (-0.04, ANKLE, 0)}, hips=(0, 0.02, -0.30))
    landing = with_(sat(), hips=(35, 0, 0, (0, -0.02, -0.66)), spine=(-18, 0, 0), chest=(-10, 0, 0))
    action("Down", [(0, rest), (6, buckle), (12, landing), (18, sat())])
    cycle(action, "Downed", 60, lambda p: sat(math.sin(p * TAU)), 10)

    def scoot(p):
        c = math.cos(p * TAU)
        k, j = 0.5 + 0.5 * c, 0.5 - 0.5 * c
        return sat(0.0, (27 + 30 * k, -5 - 80 * k), (27 + 30 * j, -5 - 80 * j), rock=3 * math.sin(p * TAU * 2))

    cycle(action, "Scoot", 27, scoot, 3)
    action("BleedOut", [(0, sat()), (9, lying(0.6)), (15, lying(1.05)), (19, lying(1.0)), (30, lying(1.0))])
    cycle(action, "Kneel", 36, lambda p: kneel(0.5 - 0.5 * math.cos(p * TAU * 2)), 3)


# ---- in the hands ------------------------------------------------------------------
# Wrists in the chest's frame (right, ahead, up of its root); poles, fingers
# and thumbs in the rig's (+Y ahead). What's held lies along the thumb, its
# edge along the fingers.

R_POLE = (0.6, -0.2, -1)
L_POLE = (-0.6, -0.2, -1)


def ready(**body):
    """Stood square, the body turned as `body` says."""
    return with_(stand(), **body)


def hands(action):
    blade = arms_to(ready(), ((0.21, 0.30, 0.0), R_POLE, (0, 0.8, -0.6), (0, 0.6, 0.8)), ((-0.16, 0.26, 0.08), L_POLE, (0, 0.3, 1)))
    cycle(action, "HoldBlade", 60, lambda p: with_(blade, chest=(1.2 * math.sin(p * TAU), 0, 0)), 10)
    axe = arms_to(ready(), ((0.23, 0.16, 0.10), R_POLE, (0, 0.6, -0.8), (0, -0.35, 1)), ((-0.14, 0.24, 0.02), L_POLE))
    cycle(action, "HoldAxe", 60, lambda p: with_(axe, chest=(1.2 * math.sin(p * TAU), 0, 0)), 10)
    guard = arms_to(ready(), ((0.14, 0.26, 0.20), R_POLE, (0, 0.3, 1)), ((-0.14, 0.30, 0.22), L_POLE, (0, 0.3, 1)))
    cycle(action, "Guard", 60, lambda p: with_(guard, chest=(1.2 * math.sin(p * TAU), 0, 0)), 10)

    back = arms_to(ready(), ((0.22, 0.10, 0.06), R_POLE, (0, 0.6, -0.8), (0, 0.5, 0.9)), ((-0.16, 0.26, 0.08), L_POLE, (0, 0.3, 1)))
    stab = arms_to(ready(chest=(0, 0, 16), spine=(-6, 0, 0)), ((0.10, 0.62, 0.10), R_POLE, (0, 0.1, -1), (0, 1, 0.1)), ((-0.18, 0.20, 0.04), L_POLE))
    action("Stab", [(0, blade), (2, back), (4, stab), (7, blend(stab, blade, 0.5)), (11, blade)])

    def swing(dir):
        """Across and through: right to left (`dir` 1), or back again."""
        s = dir
        wind = arms_to(ready(chest=(4, 0, -30 * s), spine=(0, 0, -8 * s)), ((0.36 * s, 0.05, 0.34), R_POLE, (s * 0.5, 0.2, -0.3), (0, -0.2, 1)), ((-0.14, 0.24, 0.04), L_POLE))
        hit = arms_to(ready(chest=(-8, 0, 26 * s), spine=(-6, 0, 8 * s)), ((-0.02 * s, 0.56, 0.02), R_POLE, (0, 1, -0.1), (-s, 0.6, 0)), ((-0.20, 0.14, -0.02), L_POLE))
        through = arms_to(ready(chest=(-6, 0, 36 * s), spine=(-4, 0, 10 * s)), ((-0.28 * s, 0.32, -0.08), R_POLE, (0, 0.6, -0.6), (-s, 0.1, 0)), ((-0.22, 0.10, -0.04), L_POLE))
        return [(0, blade), (3, wind), (7, hit), (10, through), (17, blade)]

    action("Swing", swing(1))
    action("Swing2", swing(-1))
    lifted = arms_to(ready(chest=(10, 0, -8), spine=(4, 0, 0)), ((0.16, -0.02, 0.46), R_POLE, (0, -0.3, 0.2), (0, -0.8, 0.6)), ((-0.12, 0.20, 0.08), L_POLE))
    chopped = arms_to(ready(chest=(-22, 0, 6), spine=(-14, 0, 0)), ((0.08, 0.54, -0.14), R_POLE, (0, 0.4, -0.6), (0, 0.6, -0.8)), ((-0.16, 0.26, -0.06), L_POLE))
    action("Chop", [(0, axe), (8, lifted), (11, lifted), (14, chopped), (20, blend(chopped, axe, 0.4)), (29, axe)])
    jab = arms_to(ready(chest=(0, 0, 14)), ((0.08, 0.62, 0.22), R_POLE, (0, 1, 0)), ((-0.14, 0.28, 0.20), L_POLE, (0, 0.3, 1)))
    cross = arms_to(ready(chest=(0, 0, -18)), ((0.14, 0.24, 0.20), R_POLE, (0, 0.3, 1)), ((-0.06, 0.62, 0.22), L_POLE, (0, 1, 0)))
    action("Jab", [(0, guard), (3, blend(guard, jab, 0.6)), (6, jab), (9, blend(jab, guard, 0.5)), (15, guard)])
    action("Cross", [(0, guard), (3, blend(guard, cross, 0.6)), (6, cross), (9, blend(cross, guard, 0.5)), (15, guard)])

    wound = arms_to(ready(chest=(6, 0, -26), spine=(4, 0, -8)), ((0.30, -0.16, 0.40), R_POLE, (0, 0.2, 1), (0, 1, 0)), ((-0.08, 0.46, 0.14), L_POLE, (0, 1, 0.3)))
    action("Windup", [(0, ready()), (6, blend(ready(), wound, 0.8)), (10, wound)])
    release = arms_to(ready(chest=(-10, 0, 20), spine=(-8, 0, 6)), ((0.18, 0.44, 0.34), R_POLE, (0, 1, 0.6), (0, 0.4, 1)), ((-0.20, 0.10, 0.00), L_POLE))
    through = arms_to(ready(chest=(-16, 0, 30), spine=(-12, 0, 8)), ((-0.04, 0.40, -0.16), R_POLE, (0, 0.6, -1)), ((-0.22, 0.04, -0.06), L_POLE))
    action("Throw", [(0, wound), (4, release), (9, through), (15, ready())])


def actions(armature):
    acts = Actions(armature)
    legs(acts.action)
    down(acts.action)
    hands(acts.action)
    return acts.done()


def main():
    """Build the rig and every part from an empty scene, and write them."""
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.context.scene.render.fps = FPS
    armature, made = build()
    stash(armature, actions(armature))
    out = os.path.abspath(os.path.join(MODELS, "survivor.glb"))
    export(out)
    faces = sum(len(o.data.polygons) for o in made)
    print(f"survivor: {len(made)} parts, {faces} faces, {len(armature.data.bones)} bones -> {out}")


main()
