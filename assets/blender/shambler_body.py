"""The Shambler's body, in parts the game puts together (`shambler.py`
writes them; `src/zombie/looks.rs` picks them): a head (whole, its jaw
torn off, or none at all), a torso in one of its tops, each arm (whole in
its sleeve, lost at the elbow or at the shoulder), and the legs.
"""

from shambler_kit import (
    BLOOD, BONE, FWD, GORE, GRIME, J, MEAT, MOUTH, SHOE, SOCKET, UP,
    accent, bottom, limb, skin, stained, top, under, v,
)

# ---- heads -----------------------------------------------------------------------

# The neck and skull, jutting forward.
NECK = [
    (v(0, 0.19, 1.49), 0.065, {"neck": 1.0}),
    (v(0, 0.24, 1.57), 0.07, {"neck": 0.3, "head": 0.7}),
]
SKULL = [
    (v(0, 0.29, 1.63), (0.095, 0.105), {"head": 1.0}),
    (v(0, 0.30, 1.70), (0.10, 0.11), {"head": 1.0}),
    (v(0, 0.29, 1.78), (0.07, 0.08), {"head": 1.0}),
]


def head(b):
    """Whole: a slack mouth, dark sockets where the eyes were."""
    limb(b, NECK + SKULL, [skin(), stained(skin(0.72), MOUTH, "10000001"), stained(skin(), SOCKET, "10000001"), skin()], ref=FWD)


def head_jawless(b):
    """The jaw torn away: raw beneath the upper teeth, the tongue hanging."""
    neck = [NECK[0], (v(0, 0.23, 1.56), 0.062, {"neck": 0.4, "head": 0.6})]
    limb(b, neck + SKULL, [skin(), stained(skin(0.7), GORE, "11000011"), stained(skin(), SOCKET, "10000001"), skin()], ref=FWD)
    limb(b, [
        (v(-0.042, 0.372, 1.626), 0.012, {"head": 1.0}),
        (v(0, 0.382, 1.622), 0.012, {"head": 1.0}),
        (v(0.042, 0.372, 1.626), 0.012, {"head": 1.0}),
    ], [BONE, BONE], cap_start=True, ref=UP)
    limb(b, [
        (v(0, 0.33, 1.60), 0.022, {"head": 1.0}),
        (v(0, 0.35, 1.54), 0.02, {"head": 1.0}),
        (v(0, 0.345, 1.49), 0.015, {"head": 1.0}),
    ], [MEAT, MEAT], cap_start=True, ref=FWD)


def neck_stump(b):
    """No head at all: a ragged neck and the spine's end."""
    limb(b, [
        NECK[0],
        (v(0, 0.215, 1.535), 0.068, {"neck": 1.0}),
        (v(0, 0.228, 1.56), 0.058, {"neck": 1.0}),
        (v(0, 0.232, 1.568), 0.045, {"neck": 1.0}),
    ], [skin(), stained(skin(0.8), BLOOD, "10110101"), GORE], ref=FWD)
    limb(b, [(v(0, 0.225, 1.55), 0.016, {"neck": 1.0}), (v(0, 0.235, 1.595), 0.016, {"neck": 1.0})], [BONE], cap_start=True)


# ---- torsos ----------------------------------------------------------------------

# Hips to neck: rings at the hips, the belt, the waist, the belly, the chest,
# the shoulders and the neck; a torso's `bands` paint the six between.
TORSO = [
    (v(0, 0.02, 0.86), (0.16, 0.11), {"hips": 1.0}),
    (v(0, 0.03, 1.00), (0.17, 0.115), {"hips": 1.0}),
    (v(0, 0.05, 1.15), (0.15, 0.10), {"hips": 0.5, "spine": 0.5}),
    (v(0, 0.075, 1.24), (0.17, 0.11), {"hips": 0.25, "spine": 0.75}),
    (v(0, 0.10, 1.33), (0.19, 0.12), {"spine": 0.5, "chest": 0.5}),
    (v(0, 0.15, 1.44), (0.21, 0.11), {"chest": 1.0}),
    (v(0, 0.19, 1.50), (0.07, 0.065), {"chest": 0.4, "neck": 0.6}),
]


def torso(b, bands, bulk=1.0, hem=False):
    """The torso painted in `bands`, its upper body `bulk` times as broad
    (a thick top), and the belt ring too if the top hangs over it."""
    first = 1 if hem else 2
    points = [(c, (r[0] * bulk, r[1] * bulk) if first <= i <= 5 else r, w) for i, (c, r, w) in enumerate(TORSO)]
    limb(b, points, bands, cap_start=True, cap_end=False)


def collar(b, shade=0.85, size=1.0, region=top):
    """A collar standing round the neck, open at the throat."""
    limb(b, [
        (v(0, 0.175, 1.465), (0.10 * size, 0.085 * size), {"chest": 0.7, "neck": 0.3}),
        (v(0, 0.20, 1.53), (0.08 * size, 0.07 * size), {"chest": 0.3, "neck": 0.7}),
    ], [region(shade)], cap_end=False)


def torso_plain(b):
    """A tee or a sweater: the top, grimy, over the bottoms."""
    torso(b, [bottom(), bottom(0.8), stained(top(), GRIME, "10010110"), stained(top(), GRIME, "01000011"), stained(top(), GRIME, "00100100"), top()])


def torso_collar(b):
    """A shirt with a collar (or coveralls, a flight suit, fatigues)."""
    torso(b, [bottom(), bottom(0.8), stained(top(), GRIME, "00100100"), top(), stained(top(), GRIME, "10000010"), top()])
    collar(b)


def torso_flannel(b):
    """A checked shirt: two shades of the top, turn and turn about."""
    even = [top() if k % 2 == 0 else top(0.62) for k in range(8)]
    odd = [top(0.62) if k % 2 == 0 else top(0.45) for k in range(8)]
    torso(b, [bottom(), bottom(0.8), even, odd, even, odd])
    collar(b, 0.62)


def torso_tank(b):
    """A vest top: bare shoulders."""
    torso(b, [bottom(), bottom(0.8), stained(top(), GRIME, "00010010"), top(), top(), stained(top(), skin(), "10011001")])


def torso_hoodie(b):
    """A hoodie: bulky, a pouch at the front, the hood down behind."""
    torso(b, [bottom(), top(0.8), top(), stained(top(), top(0.78), "01100000"), top(), top(0.9)], bulk=1.1, hem=True)
    limb(b, [
        (v(0, 0.06, 1.44), (0.13, 0.06), {"chest": 1.0}),
        (v(0, 0.09, 1.53), (0.11, 0.06), {"chest": 0.5, "neck": 0.5}),
        (v(0, 0.14, 1.58), (0.07, 0.04), {"neck": 1.0}),
    ], [top(0.85), top(0.8)], cap_start=True)
    for x in (-0.03, 0.03):
        limb(b, [(v(x, 0.268, 1.47), 0.007, {"chest": 1.0}), (v(x * 1.2, 0.262, 1.37), 0.007, {"chest": 1.0})], [accent()], cap_start=True)


def torso_jacket(b):
    """A jacket hanging open over what's worn under, its collar up."""
    open_front = "01100000"
    torso(b, [bottom(), top(0.85), stained(top(), under(), open_front), stained(top(), under(), open_front), stained(top(), under(0.9), open_front), top(0.9)], bulk=1.12, hem=True)
    collar(b, 0.75, 1.25)


def torso_overalls(b):
    """Overalls over a shirt: the bib at the front, the straps over."""
    torso(b, [bottom(), bottom(0.85), stained(top(), bottom(), "01100000"), stained(top(), bottom(), "01100110"), stained(top(), bottom(0.9), "01100110"), top()])
    collar(b, 0.85)


# ---- arms ------------------------------------------------------------------------

def arm_points(side):
    """Shoulder to fingertips: seven rings."""
    up, fore, hand = J[f"upper_arm{side}"], J[f"forearm{side}"], J[f"hand{side}"]
    u, f, h = f"upper_arm{side}", f"forearm{side}", f"hand{side}"
    return [
        (up[0], 0.065, {"chest": 0.4, u: 0.6}),
        (up[0].lerp(up[1], 0.5), 0.058, {u: 1.0}),
        (up[1], 0.05, {u: 0.5, f: 0.5}),
        (fore[0].lerp(fore[1], 0.5), 0.042, {f: 1.0}),
        (fore[1], 0.036, {f: 0.5, h: 0.5}),
        (hand[0].lerp(hand[1], 0.6), 0.045, {h: 1.0}),
        (hand[1] + (hand[1] - hand[0]) * 0.4, 0.03, {h: 1.0}),
    ]


# Each sleeve's bands, shoulder to fingertips.
SLEEVES = {
    "long": [top(), top(0.9), top(), top(0.8), skin(), skin(0.72)],
    "torn": [top(), stained(top(), skin(), "10101001"), skin(), skin(), skin(), skin(0.72)],
    "short": [top(), skin(), skin(), skin(), skin(), skin(0.72)],
    "bare": [skin(), skin(), skin(), skin(), skin(), skin(0.72)],
}


def arm(b, side, sleeve):
    limb(b, arm_points(side), SLEEVES[sleeve], cap_start=True)


def bone_end(b, a, c, bone, r=0.015):
    limb(b, [(a, r, {bone: 1.0}), (c, r, {bone: 1.0})], [BONE], cap_start=True)


def elbow_stump(b, side, sleeve):
    """Lost below the elbow: the upper arm, ragged at its end, a bone
    showing. `sleeve`: "sleeve" (to the elbow), "short" or "bare"."""
    up = J[f"upper_arm{side}"]
    u = f"upper_arm{side}"
    points = arm_points(side)[:2] + [(up[0].lerp(up[1], 0.88), 0.048, {u: 1.0}), (up[0].lerp(up[1], 0.95), 0.03, {u: 1.0})]
    bands = {
        "sleeve": [top(), stained(top(0.9), GORE, "01001000"), GORE],
        "short": [top(), skin(), GORE],
        "bare": [skin(), stained(skin(), BLOOD, "01001000"), GORE],
    }[sleeve]
    limb(b, points, bands, cap_start=True)
    bone_end(b, up[0].lerp(up[1], 0.9), up[0].lerp(up[1], 1.02), u)


def shoulder_stump(b, side, sleeve):
    """The whole arm gone: a torn stub at the shoulder. `sleeve`: "sleeve"
    or "bare"."""
    up = J[f"upper_arm{side}"]
    u = f"upper_arm{side}"
    points = [
        (up[0], 0.065, {"chest": 0.4, u: 0.6}),
        (up[0].lerp(up[1], 0.18), 0.06, {"chest": 0.2, u: 0.8}),
        (up[0].lerp(up[1], 0.26), 0.04, {u: 1.0}),
    ]
    first = stained(top(), GORE, "01010110") if sleeve == "sleeve" else stained(skin(), BLOOD, "01010110")
    limb(b, points, [first, GORE], cap_start=True)
    bone_end(b, up[0].lerp(up[1], 0.15), up[0].lerp(up[1], 0.32), u, 0.018)


# ---- legs ------------------------------------------------------------------------

LEGS = {
    "trousers": [bottom(), bottom(), bottom(0.8), bottom()],
    "shorts": [bottom(), skin(), skin(0.8), skin()],
    "boots": [bottom(), bottom(), bottom(0.8), SHOE],
}


def legs(b, kind):
    """Both legs, to the ankle in `kind`, and the shoes."""
    for side in (".R", ".L"):
        th, sh, ft = J[f"thigh{side}"], J[f"shin{side}"], J[f"foot{side}"]
        t, s, fo = f"thigh{side}", f"shin{side}", f"foot{side}"
        limb(b, [
            (th[0], 0.085, {"hips": 0.5, t: 0.5}),
            (th[0].lerp(th[1], 0.5), 0.075, {t: 1.0}),
            (th[1], 0.062, {t: 0.5, s: 0.5}),
            (sh[0].lerp(sh[1], 0.5), 0.055, {s: 1.0}),
            (sh[1] + v(0, 0, 0.04), 0.05, {s: 1.0}),
        ], LEGS[kind], ref=FWD)
        limb(b, [
            (ft[0] + v(0, -0.03, 0.02), (0.05, 0.045), {fo: 1.0}),
            (ft[0].lerp(ft[1], 0.5), (0.055, 0.04), {fo: 1.0}),
            (ft[1] + v(0, 0.02, -0.005), (0.045, 0.028), {fo: 1.0}),
        ], [SHOE, SHOE], cap_start=True, ref=UP)


def parts():
    """Every body part by the name the game knows it: {name: build(b)}."""
    out = {
        "head": head,
        "head_jawless": head_jawless,
        "neck_stump": neck_stump,
        "torso_plain": torso_plain,
        "torso_collar": torso_collar,
        "torso_flannel": torso_flannel,
        "torso_tank": torso_tank,
        "torso_hoodie": torso_hoodie,
        "torso_jacket": torso_jacket,
        "torso_overalls": torso_overalls,
    }
    for side in (".L", ".R"):
        for sleeve in SLEEVES:
            out[f"arm_{sleeve}{side}"] = lambda b, s=side, k=sleeve: arm(b, s, k)
        for sleeve in ("sleeve", "short", "bare"):
            out[f"elbow_{sleeve}{side}"] = lambda b, s=side, k=sleeve: elbow_stump(b, s, k)
        for sleeve in ("sleeve", "bare"):
            out[f"shoulder_{sleeve}{side}"] = lambda b, s=side, k=sleeve: shoulder_stump(b, s, k)
    for kind in LEGS:
        out[f"legs_{kind}"] = lambda b, k=kind: legs(b, k)
    return out
