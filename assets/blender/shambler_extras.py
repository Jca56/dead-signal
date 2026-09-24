"""What's put on over a Shambler's body (`shambler_body.py`): hair or
something on its head, a tie, its wounds, and a soldier's gear. Each sits a
little proud of what it's over, so they fit the plain tops (not a hoodie's
or a jacket's bulk: `src/zombie/looks.rs` keeps them apart).
"""

from shambler_kit import (
    BLOOD, BLOOD_DARK, BONE, FWD, GORE, MEAT, PACK, UP, VEST,
    accent, limb, stained, under, v,
)

# ---- on the head -----------------------------------------------------------------

# Over the crown of the skull (`shambler_body.SKULL`), clear of the eyes.
CROWN = [
    (v(0, 0.302, 1.712), (0.106, 0.116), {"head": 1.0}),
    (v(0, 0.295, 1.755), (0.092, 0.102), {"head": 1.0}),
    (v(0, 0.29, 1.795), (0.062, 0.072), {"head": 1.0}),
]


def hair_short(b):
    limb(b, CROWN, [accent(), accent(0.85)], ref=FWD)
    limb(b, [
        (v(0, 0.215, 1.73), (0.085, 0.03), {"head": 1.0}),
        (v(0, 0.205, 1.66), (0.09, 0.03), {"head": 1.0}),
        (v(0, 0.20, 1.61), (0.075, 0.025), {"head": 1.0}),
    ], [accent(0.9), accent(0.8)], cap_start=True)


def hair_long(b):
    """Down over the back of the neck to the shoulders."""
    limb(b, CROWN, [accent(), accent(0.85)], ref=FWD)
    limb(b, [
        (v(0, 0.21, 1.73), (0.10, 0.035), {"head": 1.0}),
        (v(0, 0.19, 1.63), (0.11, 0.04), {"head": 1.0}),
        (v(0, 0.13, 1.52), (0.12, 0.035), {"head": 0.5, "neck": 0.5}),
        (v(0, 0.05, 1.44), (0.14, 0.03), {"neck": 0.3, "chest": 0.7}),
    ], [accent(0.9), accent(0.8), accent(0.72)], cap_start=True)


def cap(b):
    """A peaked cap."""
    limb(b, CROWN, [accent(), accent(0.85)], ref=FWD)
    limb(b, [(v(0, 0.39, 1.717), (0.085, 0.008), {"head": 1.0}), (v(0, 0.48, 1.712), (0.075, 0.008), {"head": 1.0})], [accent(0.7)], cap_start=True)


def beanie(b):
    """A knitted hat, its edge turned up."""
    limb(b, [
        (v(0, 0.302, 1.70), (0.108, 0.118), {"head": 1.0}),
        (v(0, 0.300, 1.735), (0.106, 0.116), {"head": 1.0}),
        (v(0, 0.295, 1.78), (0.09, 0.10), {"head": 1.0}),
        (v(0, 0.29, 1.84), (0.05, 0.055), {"head": 1.0}),
    ], [accent(0.75), accent(), accent()], cap_start=True, ref=FWD)


def helmet(b):
    limb(b, [
        (v(0, 0.29, 1.66), (0.125, 0.135), {"head": 1.0}),
        (v(0, 0.29, 1.74), (0.12, 0.13), {"head": 1.0}),
        (v(0, 0.29, 1.81), (0.07, 0.08), {"head": 1.0}),
    ], [accent(), accent()], cap_start=True, ref=FWD)


def tie(b):
    """Knotted under the collar, down the shirt front: in the colour of
    what's worn under (a collared shirt has nothing under it)."""
    limb(b, [
        (v(0, 0.255, 1.47), (0.022, 0.01), {"chest": 1.0}),
        (v(0, 0.24, 1.41), (0.026, 0.008), {"chest": 1.0}),
        (v(0, 0.21, 1.30), (0.034, 0.008), {"chest": 0.5, "spine": 0.5}),
        (v(0, 0.188, 1.21), (0.018, 0.008), {"spine": 1.0}),
    ], [under(0.8), under(), under()], cap_start=True)


# ---- wounds ----------------------------------------------------------------------

def gore_ribs(b):
    """The chest torn open, the ribs showing."""
    limb(b, [
        (v(0, 0.165, 1.22), (0.075, 0.02), {"spine": 1.0}),
        (v(0, 0.196, 1.30), (0.085, 0.02), {"spine": 0.5, "chest": 0.5}),
        (v(0, 0.226, 1.38), (0.07, 0.02), {"chest": 1.0}),
    ], [GORE, BLOOD_DARK], cap_start=True)
    for z, front in ((1.25, 0.189), (1.30, 0.208), (1.35, 0.227)):
        w = {"spine": 0.5, "chest": 0.5}
        limb(b, [(v(-0.06, front + 0.004, z), 0.01, w), (v(0, front + 0.012, z), 0.01, w), (v(0.06, front + 0.004, z), 0.01, w)], [BONE, BONE], cap_start=True, ref=UP)


def gore_belly(b):
    """The belly opened, and what was in it hanging out."""
    limb(b, [
        (v(0, 0.137, 1.02), (0.06, 0.018), {"hips": 1.0}),
        (v(0, 0.14, 1.08), (0.07, 0.02), {"hips": 0.5, "spine": 0.5}),
        (v(0, 0.142, 1.13), (0.05, 0.018), {"spine": 1.0}),
    ], [GORE, GORE], cap_start=True)
    limb(b, [(v(0.02, 0.155, 1.07), 0.024, {"hips": 1.0}), (v(0.035, 0.175, 0.98), 0.022, {"hips": 1.0}), (v(0.02, 0.17, 0.90), 0.018, {"hips": 1.0})], [MEAT, MEAT], cap_start=True, ref=FWD)
    limb(b, [(v(-0.02, 0.155, 1.06), 0.02, {"hips": 1.0}), (v(-0.03, 0.165, 1.00), 0.02, {"hips": 1.0})], [MEAT], cap_start=True, ref=FWD)


def gore_front(b):
    """Soaked in blood down the front: a shell over the torso's (`TORSO`),
    forward of it, so only its front shows."""
    limb(b, [
        (v(0, 0.083, 1.16), (0.146, 0.083), {"hips": 0.5, "spine": 0.5}),
        (v(0, 0.105, 1.24), (0.164, 0.092), {"hips": 0.25, "spine": 0.75}),
        (v(0, 0.13, 1.33), (0.184, 0.102), {"spine": 0.5, "chest": 0.5}),
        (v(0, 0.171, 1.42), (0.20, 0.094), {"chest": 1.0}),
    ], [stained(BLOOD, BLOOD_DARK, "01000100"), stained(BLOOD, BLOOD_DARK, "00100010"), BLOOD], cap_end=False)


def gore_bite(b):
    """A bite out of the side of the neck."""
    limb(b, [(v(-0.05, 0.17, 1.495), 0.028, {"neck": 1.0}), (v(-0.052, 0.215, 1.515), 0.026, {"neck": 1.0})], [GORE], cap_start=True, ref=UP)


# ---- a soldier's gear ------------------------------------------------------------

def vest(b):
    limb(b, [
        (v(0, 0.06, 1.12), (0.18, 0.13), {"hips": 0.4, "spine": 0.6}),
        (v(0, 0.10, 1.30), (0.215, 0.145), {"spine": 0.5, "chest": 0.5}),
        (v(0, 0.14, 1.42), (0.225, 0.135), {"chest": 1.0}),
    ], [VEST, VEST], cap_start=True)


def pack(b):
    limb(b, [
        (v(0, -0.04, 1.14), (0.14, 0.07), {"spine": 1.0}),
        (v(0, 0.0, 1.30), (0.15, 0.08), {"spine": 0.4, "chest": 0.6}),
        (v(0, 0.03, 1.42), (0.13, 0.07), {"chest": 1.0}),
    ], [PACK, PACK], cap_start=True)


def parts():
    """Every extra by the name the game knows it: {name: build(b)}."""
    return {
        "hair_short": hair_short,
        "hair_long": hair_long,
        "cap": cap,
        "beanie": beanie,
        "helmet": helmet,
        "tie": tie,
        "gore_ribs": gore_ribs,
        "gore_belly": gore_belly,
        "gore_front": gore_front,
        "gore_bite": gore_bite,
        "vest": vest,
        "pack": pack,
    }
