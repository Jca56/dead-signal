"""The Hellhound: a big dog long dead and still burning. Charred hide
cracked open on embers, its eyes two coals, torn ears, a rag of a tail;
some of them open to the ribs down one side.

Run headless from the project root:
    /opt/blender-bin-5.2.1/blender -b --factory-startup --python assets/blender/hound.py

Writes assets/models/hound.glb: a rig of its own, its parts (each a skinned
mesh, named as the game knows it: `src/zombie/looks`), and its animations,
keyed straight onto the bones:

    Run     0.4 s  flat out, a bound a cycle: gathered, then stretched (loops)
    Idle    2 s    standing, head low, its sides heaving (loops)
    Attack  0.5 s  a crouch and a leap, the jaws shut at 0.2 s
    Flinch  0.33 s its head snapped aside by a hit
    Stumble 0.6 s  knocked off its feet behind, scrabbling up
    Death   1.2 s  its legs go and it falls on its side; ends lying still

Built facing Blender's +Y (the game's -Z), paws on the ground at the
origin. Its bones are named as the dead's are, so the game hits and draws
it the same way (`src/zombie/figure.rs`): root > hips > spine > chest >
neck > head > jaw; its forelegs are its arms (chest > upper_arm > forearm >
hand), its hind legs its legs (hips > thigh > shin > foot), each .L and .R
(+X is its right); and hips > tail.
"""

import math
import os
import sys

import bpy

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from rig_kit import Actions, blend, export, flat_material, part, rig, stash, with_  # noqa: E402
from shambler_kit import BONE, FWD, GORE, MOUTH, SOCKET, UP, limb, skin, top, v  # noqa: E402

MODELS = os.path.join(HERE, "..", "models")
FPS = 30

# Colours of its own: what glows through the cracks, and its eyes.
# (An alpha of 0.9: a colour of its own that glows, as the game draws it.)
EMBER = (1.0, 0.33, 0.04, 0.9)
EMBER_HOT = (1.0, 0.62, 0.12, 0.9)
MEAT = (0.34, 0.09, 0.07)

# The skeleton at rest: (head, tail, parent). Right side is +X.
BONES = {
    "hips": (v(0, -0.38, 0.72), v(0, -0.20, 0.74), "root"),
    "spine": (v(0, -0.20, 0.74), v(0, 0.05, 0.74), "hips"),
    "chest": (v(0, 0.05, 0.74), v(0, 0.30, 0.72), "spine"),
    "neck": (v(0, 0.30, 0.72), v(0, 0.46, 0.86), "chest"),
    "head": (v(0, 0.46, 0.86), v(0, 0.72, 0.80), "neck"),
    "jaw": (v(0, 0.50, 0.80), v(0, 0.72, 0.76), "head"),
    "tail": (v(0, -0.42, 0.73), v(0, -0.70, 0.60), "hips"),
}
for _side, _s in ((".R", 1.0), (".L", -1.0)):
    BONES[f"upper_arm{_side}"] = (v(0.12 * _s, 0.26, 0.66), v(0.13 * _s, 0.22, 0.38), "chest")
    BONES[f"forearm{_side}"] = (v(0.13 * _s, 0.22, 0.38), v(0.13 * _s, 0.25, 0.09), f"upper_arm{_side}")
    BONES[f"hand{_side}"] = (v(0.13 * _s, 0.25, 0.09), v(0.13 * _s, 0.34, 0.02), f"forearm{_side}")
    BONES[f"thigh{_side}"] = (v(0.11 * _s, -0.34, 0.68), v(0.12 * _s, -0.26, 0.40), "hips")
    BONES[f"shin{_side}"] = (v(0.12 * _s, -0.26, 0.40), v(0.12 * _s, -0.42, 0.15), f"thigh{_side}")
    BONES[f"foot{_side}"] = (v(0.12 * _s, -0.42, 0.15), v(0.12 * _s, -0.36, 0.02), f"shin{_side}")

J = {name: (h, t) for name, (h, t, _) in BONES.items()}


def faces(base, **others):
    """A ring of face colours, `base` but for the faces named: round a
    tube along its length with its rings begun at the top, `top` is its
    two top faces, `right`, `under` and `left` the pairs round from there,
    and `f0`..`f7` a face each."""
    out = [base] * 8
    pairs = {"top": (7, 0), "right": (1, 2), "under": (3, 4), "left": (5, 6)}
    for name, colour in others.items():
        for k in pairs.get(name) or (int(name[1]),):
            out[k] = colour
    return out


# ---- the body --------------------------------------------------------------------

# Rump to breast: (centre, (high, wide), weights). Deep in the chest,
# tucked up behind it.
RUMP = (v(0, -0.45, 0.70), (0.10, 0.09), {"hips": 1.0})
HAUNCH = (v(0, -0.34, 0.69), (0.17, 0.145), {"hips": 1.0})
LOIN = (v(0, -0.18, 0.72), (0.135, 0.115), {"hips": 0.5, "spine": 0.5})
BELLY = (v(0, -0.02, 0.69), (0.165, 0.135), {"spine": 1.0})
RIBS = (v(0, 0.14, 0.65), (0.225, 0.17), {"spine": 0.4, "chest": 0.6})
SHOULDERS = (v(0, 0.27, 0.67), (0.225, 0.18), {"chest": 1.0})
BREAST = (v(0, 0.36, 0.71), (0.15, 0.125), {"chest": 0.6, "neck": 0.4})
PROFILE = [RUMP, HAUNCH, LOIN, BELLY, RIBS, SHOULDERS, BREAST]


def between(a, c, t):
    """A ring `t` of the way from ring `a` to ring `c`, weighted as the
    nearer."""
    return (a[0].lerp(c[0], t), tuple(a[1][i] + (c[1][i] - a[1][i]) * t for i in (0, 1)), a[2] if t < 0.5 else c[2])


def cracked(profile, hide, cracks):
    """A tube's rings and bands: `profile`'s, painted `hide` (a band
    each), split where `cracks` {band: (how far along it, its colours)}
    says by a narrow band of its own: a fissure, glowing."""
    points, bands = [profile[0]], []
    for i, (a, c) in enumerate(zip(profile, profile[1:])):
        if i in cracks:
            t, glow = cracks[i]
            points += [between(a, c, t - 0.07), between(a, c, t + 0.07)]
            bands += [hide[i], glow, hide[i]]
        else:
            bands.append(hide[i])
        points.append(c)
    return points, bands


def hide(shade=1.0):
    """A band of its coat: charred, a darker saddle along the back, the
    belly darker still."""
    return faces(skin(shade), top=top(0.8), under=skin(shade * 0.7))


def hackles(b):
    """A ridge of burnt spines from its shoulders down its back."""
    for y, z, bone in ((0.25, 0.895, "chest"), (0.17, 0.875, "chest"), (0.08, 0.855, "spine"), (-0.02, 0.855, "spine"), (-0.12, 0.85, "spine")):
        limb(b, [(v(0, y, z - 0.02), (0.035, 0.016), {bone: 1.0}), (v(0, y - 0.05, z + 0.055), 0.003, {bone: 1.0})], [top(0.55)], cap_start=True, ref=FWD)


def body(b):
    """Whole: charred hide, cracked across on embers."""
    points, bands = cracked(PROFILE, [skin(0.8), hide(), hide(), hide(0.9), hide(), hide(0.9)], {
        1: (0.5, faces(skin(), f0=EMBER, f1=EMBER_HOT, f2=EMBER)),
        3: (0.4, faces(skin(), f5=EMBER, f6=EMBER_HOT, f7=EMBER)),
        4: (0.6, faces(skin(), f0=EMBER, f1=EMBER, f2=EMBER_HOT, f3=EMBER)),
    })
    limb(b, points, bands, cap_start=True, cap_end=False, ref=UP)
    hackles(b)


def body_flayed(b):
    """Open down its left side: the ribs bare, raw between them."""
    cage = [between(BELLY, SHOULDERS, k / 6) for k in range(1, 6)]
    rib = faces(skin(), top=top(0.8), under=skin(0.7), left=BONE)
    raw = faces(skin(0.9), top=top(0.8), under=skin(0.7), left=MEAT)
    points, bands = cracked([RUMP, HAUNCH, LOIN, BELLY, *cage, SHOULDERS, BREAST], [skin(0.8), hide(), faces(skin(), top=top(0.8), f6=GORE), raw, rib, raw, rib, raw, rib, hide(0.9)], {
        1: (0.5, faces(skin(), f0=EMBER, f1=EMBER_HOT, f2=EMBER)),
        9: (0.5, faces(skin(), f0=EMBER, f1=EMBER, f2=EMBER_HOT)),
    })
    limb(b, points, bands, cap_start=True, cap_end=False, ref=UP)
    hackles(b)


# ---- the head --------------------------------------------------------------------

def head(b):
    """A thick neck, a broad skull, its eyes two coals under a heavy brow;
    a long muzzle, the jaw hanging a little open on its teeth; torn ears."""
    limb(b, [
        (v(0, 0.31, 0.71), (0.15, 0.125), {"chest": 0.5, "neck": 0.5}),
        (v(0, 0.39, 0.79), (0.125, 0.105), {"neck": 1.0}),
        (v(0, 0.46, 0.855), (0.115, 0.10), {"neck": 0.4, "head": 0.6}),
        (v(0, 0.535, 0.875), (0.115, 0.115), {"head": 1.0}),
        (v(0, 0.61, 0.855), (0.08, 0.085), {"head": 1.0}),
        (v(0, 0.71, 0.835), (0.05, 0.06), {"head": 1.0}),
        (v(0, 0.775, 0.825), (0.036, 0.045), {"head": 1.0}),
    ], [
        hide(),
        hide(0.9),
        faces(skin(), top=top(0.8)),
        faces(skin(0.85), top=top(0.7), under=MOUTH),
        faces(skin(0.8), under=MOUTH),
        faces(SOCKET, under=MOUTH),
    ], cap_start=False, ref=UP)
    # A fissure round the back of the neck.
    limb(b, [(v(0, 0.355, 0.755), (0.142, 0.118), {"neck": 1.0}), (v(0, 0.37, 0.77), (0.138, 0.115), {"neck": 1.0})], [faces(skin(0.9), f5=EMBER, f6=EMBER_HOT, f7=EMBER)], cap_start=False, cap_end=False, ref=UP)
    # The lower jaw, its inside up.
    limb(b, [
        (v(0, 0.52, 0.79), (0.03, 0.065), {"jaw": 1.0}),
        (v(0, 0.65, 0.775), (0.025, 0.052), {"jaw": 1.0}),
        (v(0, 0.755, 0.765), (0.016, 0.036), {"jaw": 1.0}),
    ], [faces(skin(0.7), top=MOUTH), faces(skin(0.7), top=MOUTH)], cap_start=True, ref=UP)
    for x in (-1.0, 1.0):
        # Fangs: one down from the muzzle, one up from the jaw, and a
        # row behind.
        limb(b, [(v(0.036 * x, 0.745, 0.812), 0.011, {"head": 1.0}), (v(0.036 * x, 0.75, 0.765), 0.002, {"head": 1.0})], [BONE], cap_start=True, ref=FWD)
        limb(b, [(v(0.028 * x, 0.735, 0.772), 0.009, {"jaw": 1.0}), (v(0.028 * x, 0.74, 0.808), 0.002, {"jaw": 1.0})], [BONE], cap_start=True, ref=FWD)
        limb(b, [(v(0.048 * x, 0.63, 0.818), 0.009, {"head": 1.0}), (v(0.042 * x, 0.71, 0.806), 0.008, {"head": 1.0})], [BONE], cap_start=True, ref=UP)
        # An eye: a coal, sunk under the brow.
        limb(b, [(v(0.068 * x, 0.588, 0.905), 0.02, {"head": 1.0}), (v(0.082 * x, 0.612, 0.898), 0.017, {"head": 1.0})], [EMBER_HOT], cap_start=True, ref=UP)
        limb(b, [(v(0.05 * x, 0.565, 0.935), (0.014, 0.03), {"head": 1.0}), (v(0.075 * x, 0.625, 0.918), (0.01, 0.026), {"head": 1.0})], [top(0.6)], cap_start=True, ref=UP)
        # A torn ear.
        limb(b, [(v(0.07 * x, 0.50, 0.955), (0.035, 0.02), {"head": 1.0}), (v(0.085 * x, 0.47, 1.02), (0.018, 0.009), {"head": 1.0}), (v(0.09 * x, 0.455, 1.055), 0.003, {"head": 1.0})], [skin(0.7), skin(0.6)], cap_start=True, ref=FWD)


# ---- legs and tail ---------------------------------------------------------------

def legs(b):
    """Four of them, heavy in the shoulder and the haunch, down to big
    paws."""
    for side in (".R", ".L"):
        ua, fa, ha = J[f"upper_arm{side}"], J[f"forearm{side}"], J[f"hand{side}"]
        u, f, h = f"upper_arm{side}", f"forearm{side}", f"hand{side}"
        limb(b, [
            (ua[0] + v(0, 0, 0.05), (0.11, 0.075), {"chest": 0.5, u: 0.5}),
            (ua[0].lerp(ua[1], 0.5), (0.085, 0.065), {u: 1.0}),
            (ua[1], 0.055, {u: 0.5, f: 0.5}),
            (fa[0].lerp(fa[1], 0.5), 0.046, {f: 1.0}),
            (fa[1], 0.04, {f: 0.5, h: 0.5}),
            (ha[0] + v(0, 0.01, -0.045), 0.038, {h: 1.0}),
        ], [skin(), skin(0.9), skin(0.8), skin(0.8), skin(0.6)], cap_start=True, ref=FWD)
        th, sh, ft = J[f"thigh{side}"], J[f"shin{side}"], J[f"foot{side}"]
        t, s, fo = f"thigh{side}", f"shin{side}", f"foot{side}"
        limb(b, [
            (th[0] + v(0, 0, 0.04), (0.14, 0.08), {"hips": 0.5, t: 0.5}),
            (th[0].lerp(th[1], 0.5), (0.115, 0.07), {t: 1.0}),
            (th[1], (0.065, 0.055), {t: 0.5, s: 0.5}),
            (sh[0].lerp(sh[1], 0.5), 0.046, {s: 1.0}),
            (sh[1], 0.038, {s: 0.5, fo: 0.5}),
            (ft[0].lerp(ft[1], 0.75), 0.036, {fo: 1.0}),
        ], [skin(), skin(0.9), skin(0.8), skin(0.8), skin(0.6)], cap_start=True, ref=FWD)
        # The paws, claws out in front.
        for at, bone in ((ha[1], h), (ft[1], fo)):
            limb(b, [
                (at + v(0, -0.08, 0.02), (0.035, 0.045), {bone: 1.0}),
                (at + v(0, -0.02, 0.015), (0.038, 0.058), {bone: 1.0}),
                (at + v(0, 0.045, 0.0), (0.022, 0.05), {bone: 1.0}),
            ], [skin(0.6), skin(0.5)], cap_start=True, ref=UP)
            for k in (-1, 0, 1):
                toe = at + v(0.03 * k, 0.045, 0.0)
                limb(b, [(toe, 0.009, {bone: 1.0}), (toe + v(0, 0.04, -0.012), 0.002, {bone: 1.0})], [BONE], cap_start=True, ref=UP)


def tail(b):
    """A rag of one, its end still alight."""
    limb(b, [
        (v(0, -0.43, 0.725), 0.045, {"hips": 0.5, "tail": 0.5}),
        (v(0, -0.55, 0.675), 0.034, {"tail": 1.0}),
        (v(0, -0.65, 0.625), 0.02, {"tail": 1.0}),
        (v(0, -0.72, 0.59), 0.004, {"tail": 1.0}),
    ], [skin(0.8), skin(0.7), EMBER], cap_start=True, ref=UP)


def build():
    """The rig, and every part on it."""
    armature = rig("HoundRig", BONES)
    material = flat_material()
    parts = {"hound_body": body, "hound_body_flayed": body_flayed, "hound_head": head, "hound_legs": legs, "hound_tail": tail}
    return armature, [part(name, make, armature, material) for name, make in parts.items()]


# ---- animation ---------------------------------------------------------------------

def stand(breath=0.0):
    """Standing, head low, `breath` (−1–1) through a breath."""
    return {
        "neck": (-14, 0, 0),
        "head": (-10 + 2 * breath, 0, 0),
        "jaw": (-8 - 4 * breath, 0, 0),
        "chest": (1.5 * breath, 0, 0),
        "tail": (-10, 0, 6 * breath),
    }


def bound(p):
    """Flat out at phase `p` (0–1 of a bound): the hind legs drive as the
    forelegs reach, then all four gather under it; its back flexes with
    them, and the right leads a little."""
    a = p * math.tau
    out = {
        "hips": (-4 * math.cos(a), 0, 0, (0, 0, 0.05 * math.sin(a) - 0.03)),
        "spine": (9 * math.cos(a), 0, 0),
        "chest": (-7 * math.cos(a), 0, 0),
        "neck": (-10 + 5 * math.cos(a), 0, 0),
        "head": (-4, 0, 0),
        "jaw": (-14, 0, 0),
        "tail": (18 + 10 * math.sin(a), 0, 0),
    }
    for side, lead in ((".R", 0.0), (".L", 0.5)):
        f = a + lead
        gathered = max(0.0, -math.cos(f))
        out[f"upper_arm{side}"] = (48 * math.cos(f), 0, 0)
        out[f"forearm{side}"] = (-(10 + 55 * gathered), 0, 0)
        out[f"hand{side}"] = (25 * gathered, 0, 0)
        out[f"thigh{side}"] = (-42 * math.cos(f) + 6, 0, 0)
        out[f"shin{side}"] = (20 * math.cos(f) + 30 * gathered, 0, 0)
        out[f"foot{side}"] = (-25 * gathered, 0, 0)
    return out


def actions(armature):
    """Each animation keyed on its own stretch of timeline, then cut into
    an action of its own."""
    acts = Actions(armature)
    action = acts.action
    action("Run", [(f, bound(f / 12)) for f in range(0, 13)])
    action("Idle", [(f, stand(math.sin(f / 60 * math.tau))) for f in range(0, 61, 6)])
    rest = stand()
    # Down on its haunches, then up and out, the jaws wide, then shut.
    crouch = with_(rest, hips=(-6, 0, 0, (0, -0.06, -0.10)), thigh__R=(28, 0, 0), thigh__L=(28, 0, 0), shin__R=(-30, 0, 0), shin__L=(-30, 0, 0), upper_arm__R=(-12, 0, 0), upper_arm__L=(-12, 0, 0), forearm__R=(14, 0, 0), forearm__L=(14, 0, 0), neck=(4, 0, 0), head=(6, 0, 0), jaw=(-30, 0, 0))
    leap = with_(rest, hips=(12, 0, 0, (0, 0.22, 0.08)), spine=(6, 0, 0), chest=(8, 0, 0), neck=(6, 0, 0), head=(10, 0, 0), jaw=(-4, 0, 0), thigh__R=(-40, 0, 0), thigh__L=(-40, 0, 0), shin__R=(18, 0, 0), shin__L=(18, 0, 0), upper_arm__R=(55, 0, 0), upper_arm__L=(55, 0, 0), forearm__R=(-35, 0, 0), forearm__L=(-35, 0, 0), tail=(25, 0, 0))
    landed = with_(rest, hips=(-4, 0, 0, (0, 0.08, -0.04)), upper_arm__R=(15, 0, 0), upper_arm__L=(15, 0, 0), forearm__R=(-20, 0, 0), forearm__L=(-20, 0, 0), head=(-14, 0, 0), jaw=(-2, 0, 0))
    action("Attack", [(0, rest), (3, crouch), (6, leap), (10, landed), (15, rest)])
    struck = with_(rest, neck=(-6, 0, 22), head=(4, 14, 18), chest=(0, 0, 6), jaw=(-22, 0, 0))
    action("Flinch", [(0, rest), (3, struck), (10, rest)])
    # Its hindquarters go out from under it; it scrabbles back up.
    down_behind = with_(rest, hips=(14, 6, 10, (0.05, -0.08, -0.26)), thigh__R=(60, 0, 0), thigh__L=(50, 0, 0), shin__R=(-70, 0, 0), shin__L=(-60, 0, 0), spine=(-8, 0, -6), chest=(-8, 0, -4), head=(8, 0, -10), jaw=(-20, 0, 0), tail=(-30, 0, 20))
    action("Stumble", [(0, rest), (4, down_behind), (10, down_behind), (14, blend(down_behind, rest, 0.6)), (18, rest)])
    # Its legs fold, and it goes over on its right side.
    buckle = with_(rest, hips=(0, 0, 0, (0, 0.03, -0.22)), thigh__R=(45, 0, 0), thigh__L=(45, 0, 0), shin__R=(-60, 0, 0), shin__L=(-60, 0, 0), upper_arm__R=(-25, 0, 0), upper_arm__L=(-25, 0, 0), forearm__R=(60, 0, 0), forearm__L=(60, 0, 0), head=(-20, 0, 0), jaw=(-25, 0, 0))
    lying = {
        "hips": (0, 82, 0, (0.10, 0.03, -0.56)),
        "spine": (0, 0, -6),
        "chest": (0, 4, -6),
        "neck": (-6, 0, -10),
        "head": (-8, 6, -8),
        "jaw": (-30, 0, 0),
        "tail": (-20, 0, 10),
        "thigh.R": (20, 0, 0), "shin.R": (-25, 0, 0),
        "thigh.L": (-15, 0, 0), "shin.L": (-10, 0, 0),
        "upper_arm.R": (25, 0, 0), "forearm.R": (-20, 0, 0),
        "upper_arm.L": (-10, 0, 0), "forearm.L": (10, 0, 0),
    }
    settle = with_(lying, hips=(0, 78, 0, (0.10, 0.03, -0.53)))
    action("Death", [(0, rest), (7, buckle), (16, settle), (20, lying), (36, lying)])
    return acts.done()


def main():
    """Build the rig and every part from an empty scene, and write them."""
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.context.scene.render.fps = FPS
    armature, made = build()
    stash(armature, actions(armature))
    out = os.path.abspath(os.path.join(MODELS, "hound.glb"))
    export(out)
    count = sum(len(o.data.polygons) for o in made)
    print(f"hound: {len(made)} parts, {count} faces, {len(armature.data.bones)} bones -> {out}")


main()
