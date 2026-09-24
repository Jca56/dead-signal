"""The special dead's own parts, on the Shambler's rig (`shambler.py`
writes them with the rest; `src/zombie/looks` picks them). The Ripper: a
narrow skull split by a wide, torn mouth, and three long claws on each
hand.
"""

from shambler_body import NECK
from shambler_kit import BONE, FWD, GORE, J, MOUTH, SOCKET, UP, limb, skin, stained, v

CLAW = (0.62, 0.58, 0.46)


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


def parts():
    """Every special part by the name the game knows it."""
    return {
        "head_ripper": head_ripper,
        "claws.L": lambda b: claws(b, ".L"),
        "claws.R": lambda b: claws(b, ".R"),
    }
