"""The special dead's own parts, on the Shambler's rig (`shambler.py`
writes them with the rest; `src/zombie/looks` picks them). The Ripper: a
narrow skull split by a wide, torn mouth, and three long claws on each
hand. The Spitter: a belly swollen fit to burst, blistered with bile, and
a head over a bulging throat sac, drooling.
"""

from shambler_body import NECK, SKULL
from shambler_kit import BONE, FWD, GORE, J, MOUTH, SOCKET, UP, bottom, limb, skin, stained, top, v

CLAW = (0.62, 0.58, 0.46)
BILE = (0.52, 0.72, 0.14)
BILE_DARK = (0.30, 0.44, 0.08)


def head_ripper(b):
    """Long and narrow, the jaw hanging open wide, rows of teeth, the eyes
    sunk deep."""
    limb(b, NECK + [
        (v(0, 0.29, 1.62), (0.082, 0.10), {"head": 1.0}),
        (v(0, 0.305, 1.70), (0.086, 0.112), {"head": 1.0}),
        (v(0, 0.30, 1.79), (0.06, 0.085), {"head": 1.0}),
    ], [skin(), stained(skin(0.6), MOUTH, "11000011"), stained(skin(), SOCKET, "10000001"), skin(0.85)], ref=FWD)
    # The jaw, dropped: a flap hanging from under the skull, raw inside.
    limb(b, [
        (v(0, 0.30, 1.585), (0.06, 0.02), {"head": 1.0}),
        (v(0, 0.36, 1.53), (0.055, 0.018), {"head": 1.0}),
        (v(0, 0.385, 1.50), (0.04, 0.014), {"head": 1.0}),
    ], [stained(skin(0.7), GORE, "11100000"), skin(0.6)], cap_start=True)
    # Teeth: upper and lower rows.
    for z, y, w in ((1.625, 0.37, 0.05), (1.525, 0.37, 0.045)):
        limb(b, [(v(-w, y - 0.012, z), 0.009, {"head": 1.0}), (v(0, y, z), 0.009, {"head": 1.0}), (v(w, y - 0.012, z), 0.009, {"head": 1.0})], [BONE, BONE], cap_start=True, ref=UP)


def claws(b, side):
    """Three long hooked blades out of the fingertips."""
    hand = J[f"hand{side}"]
    h = f"hand{side}"
    along = (hand[1] - hand[0]).normalized()
    tip = hand[1] + along * 0.02
    across = along.cross(UP).normalized()
    for k in (-1, 0, 1):
        root = tip + across * (0.022 * k)
        mid = root + along * 0.11 + across * (0.01 * k) - UP * 0.015
        end = mid + along * 0.10 + across * (0.012 * k) - UP * 0.05
        limb(b, [(root, 0.011, {h: 1.0}), (mid, 0.008, {h: 1.0}), (end, 0.002, {h: 1.0})], [CLAW, CLAW], cap_start=True, ref=UP)


def torso_bloated(b):
    """Hips to neck, the belly and chest blown out round and tight, the
    shirt split and hanging in strips over blisters of bile."""
    blister = stained(skin(), BILE, "01010010")
    blister2 = stained(skin(0.85), BILE_DARK, "00101001")
    limb(b, [
        (v(0, 0.02, 0.86), (0.16, 0.11), {"hips": 1.0}),
        (v(0, 0.04, 1.00), (0.19, 0.15), {"hips": 1.0}),
        (v(0, 0.10, 1.10), (0.26, 0.25), {"hips": 0.6, "spine": 0.4}),
        (v(0, 0.13, 1.22), (0.29, 0.29), {"hips": 0.3, "spine": 0.7}),
        (v(0, 0.14, 1.33), (0.27, 0.26), {"spine": 0.5, "chest": 0.5}),
        (v(0, 0.15, 1.43), (0.23, 0.17), {"chest": 1.0}),
        (v(0, 0.19, 1.50), (0.08, 0.075), {"chest": 0.4, "neck": 0.6}),
    ], [bottom(), stained(bottom(0.8), skin(), "01100000"), blister, blister2, stained(top(), skin(), "01101100"), top(0.9)], cap_start=True, cap_end=False)


def head_spitter(b):
    """A skull over a throat swollen into a sac, the mouth hanging open and
    dribbling bile."""
    limb(b, NECK + SKULL, [skin(), stained(skin(0.7), MOUTH, "10000001"), stained(skin(), SOCKET, "10000001"), skin()], ref=FWD)
    # The sac, bulging under the jaw and down the throat.
    limb(b, [
        (v(0, 0.22, 1.49), (0.07, 0.06), {"neck": 1.0}),
        (v(0, 0.28, 1.53), (0.11, 0.10), {"neck": 0.5, "head": 0.5}),
        (v(0, 0.31, 1.58), (0.08, 0.075), {"head": 1.0}),
    ], [stained(skin(0.9), BILE, "10010001"), stained(skin(), BILE_DARK, "01000010")], cap_start=True, ref=FWD)
    # Bile running from the mouth.
    limb(b, [(v(0, 0.39, 1.62), 0.018, {"head": 1.0}), (v(0.01, 0.40, 1.56), 0.012, {"head": 1.0}), (v(0.0, 0.395, 1.50), 0.007, {"head": 1.0})], [BILE, BILE], cap_start=True, ref=FWD)


def parts():
    """Every special part by the name the game knows it."""
    return {
        "torso_bloated": torso_bloated,
        "head_spitter": head_spitter,
        "head_ripper": head_ripper,
        "claws.L": lambda b: claws(b, ".L"),
        "claws.R": lambda b: claws(b, ".R"),
    }
