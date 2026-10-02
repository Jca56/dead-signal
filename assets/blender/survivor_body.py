"""The living, in parts the game puts together (`survivor.py` writes them;
`src/survivor/looks.rs` picks them): two survivors, each told apart at a
glance even far off. The first in a cap and a field jacket over a tee,
cargo trousers and boots, a belt of pouches, a rucksack with a bedroll;
the second with a ponytail, in a hoodie, jeans and sneakers, a daypack.
What's in the player's colour (the jacket and cap, the hoodie) is the top
region; hair and brows the accent; what's worn under (a tee, drawstrings)
the under.
"""

from survivor_kit import (
    BELT, BEDROLL, BOOT, BUCKLE, DAYPACK, DAYPACK_DARK, EYE, FWD, J, LACE,
    LIP, POUCH, RUCK, RUCK_DARK, SNEAKER, SOLE, STRAP, UP, WHITE,
    accent, bottom, limb, skin, stained, top, under, v,
)

# ---- heads -------------------------------------------------------------------------

NECK = [
    (v(0, 0.004, 1.475), 0.058, {"neck": 1.0}),
    (v(0, 0.012, 1.55), 0.055, {"neck": 0.5, "head": 0.5}),
]


def skull(jaw):
    """Chin to crown, ring by ring: the jaw `jaw` times as broad."""
    return [
        (v(0, 0.036, 1.588), (0.074 * jaw, 0.066 * jaw), {"head": 1.0}),
        (v(0, 0.028, 1.628), (0.097, 0.083 * jaw), {"head": 1.0}),
        (v(0, 0.022, 1.668), (0.106, 0.090), {"head": 1.0}),
        (v(0, 0.018, 1.705), (0.108, 0.092), {"head": 1.0}),
        (v(0, 0.012, 1.742), (0.104, 0.090), {"head": 1.0}),
        (v(0, 0.004, 1.785), (0.086, 0.076), {"head": 1.0}),
        (v(0, -0.004, 1.812), (0.045, 0.040), {"head": 1.0}),
    ]


def face(b, jaw, stubble):
    """A head of the living: eyes, brows, a nose, a mouth, ears; stubble
    darkening the jaw if `stubble`."""
    chin = stained(skin(), skin(0.8), "11000011") if stubble else skin()
    limb(b, NECK + skull(jaw), [skin(), skin(0.9), chin, chin, skin(), skin(), skin(), skin(), skin()], ref=FWD)
    # The nose, its bridge to its tip.
    limb(b, [
        (v(0, 0.124, 1.708), 0.011, {"head": 1.0}),
        (v(0, 0.142, 1.668), 0.016, {"head": 1.0}),
        (v(0, 0.128, 1.656), 0.011, {"head": 1.0}),
    ], [skin(0.92), skin(0.8)], cap_start=True)
    for x in (-0.037, 0.037):
        # The white of the eye, the eye in it; the brow over it.
        limb(b, [(v(x, 0.110, 1.706), (0.018, 0.010), {"head": 1.0}), (v(x, 0.121, 1.706), (0.017, 0.009), {"head": 1.0})], [WHITE], cap_start=True)
        limb(b, [(v(x, 0.118, 1.706), 0.008, {"head": 1.0}), (v(x, 0.1255, 1.706), 0.008, {"head": 1.0})], [EYE], cap_start=True)
        s = 1 if x > 0 else -1
        limb(b, [(v(x - 0.02 * s, 0.121, 1.728), 0.006, {"head": 1.0}), (v(x + 0.022 * s, 0.117, 1.734), 0.006, {"head": 1.0})], [accent(0.8)], cap_start=True, ref=UP)
        # An ear.
        limb(b, [(v(0.094 * s, 0.002, 1.688), (0.028, 0.017), {"head": 1.0}), (v(0.110 * s, 0.0, 1.688), (0.022, 0.012), {"head": 1.0})], [skin(0.85)], cap_start=True, ref=UP)
    limb(b, [(v(-0.023, 0.119, 1.623), 0.0045, {"head": 1.0}), (v(0.023, 0.119, 1.623), 0.0045, {"head": 1.0})], [LIP], cap_start=True, ref=UP)


def head_a(b):
    """The first: a square jaw, a day's stubble."""
    face(b, 1.1, True)


def head_b(b):
    """The second: a narrower jaw."""
    face(b, 0.94, False)


def cap(b):
    """A peaked cap, in the player's colour, a short back and sides under
    it."""
    limb(b, [
        (v(0, 0.012, 1.742), (0.115, 0.100), {"head": 1.0}),
        (v(0, 0.004, 1.790), (0.098, 0.087), {"head": 1.0}),
        (v(0, -0.004, 1.828), (0.055, 0.050), {"head": 1.0}),
    ], [top(0.9), top(0.8)], cap_start=True, ref=FWD)
    limb(b, [(v(0, 0.104, 1.748), (0.088, 0.008), {"head": 1.0}), (v(0, 0.198, 1.738), (0.076, 0.007), {"head": 1.0})], [top(0.7)], cap_start=True)
    limb(b, [
        (v(0, -0.030, 1.745), (0.080, 0.106), {"head": 1.0}),
        (v(0, -0.035, 1.690), (0.078, 0.101), {"head": 1.0}),
        (v(0, -0.030, 1.640), (0.062, 0.086), {"head": 1.0}),
    ], [accent(), accent(0.85)], cap_start=True, ref=FWD)


def hair_b(b):
    """A crown of hair, swept across the brow, down the back of the head,
    and tied up in a ponytail."""
    limb(b, [
        (v(0, 0.010, 1.738), (0.114, 0.098), {"head": 1.0}),
        (v(0, 0.003, 1.786), (0.095, 0.084), {"head": 1.0}),
        (v(0, -0.005, 1.824), (0.050, 0.046), {"head": 1.0}),
    ], [accent(), accent(0.85)], cap_start=True, ref=FWD)
    limb(b, [
        (v(0, -0.050, 1.760), (0.096, 0.056), {"head": 1.0}),
        (v(0, -0.066, 1.690), (0.100, 0.052), {"head": 1.0}),
        (v(0, -0.058, 1.625), (0.086, 0.040), {"head": 1.0}),
    ], [accent(0.92), accent(0.8)], cap_start=True)
    limb(b, [(v(-0.075, 0.098, 1.768), (0.020, 0.013), {"head": 1.0}), (v(0.055, 0.112, 1.748), (0.016, 0.010), {"head": 1.0})], [accent(0.9)], cap_start=True, ref=UP)
    limb(b, [
        (v(0, -0.098, 1.748), 0.030, {"head": 1.0}),
        (v(0, -0.118, 1.742), 0.024, {"head": 1.0}),
        (v(0, -0.150, 1.680), 0.033, {"head": 1.0}),
        (v(0, -0.160, 1.590), 0.027, {"head": 0.7, "neck": 0.3}),
        (v(0, -0.148, 1.505), 0.010, {"head": 0.5, "neck": 0.5}),
    ], [(0.10, 0.08, 0.07), accent(0.9), accent(0.8), accent(0.7)], cap_start=True)


# ---- torsos ------------------------------------------------------------------------

# Hips to neck: rings at the hips, the belt, the waist, the belly, the chest,
# the shoulders and the neck; a torso's `bands` paint the six between.
TORSO = [
    (v(0, 0.0, 0.86), (0.155, 0.105), {"hips": 1.0}),
    (v(0, 0.0, 0.99), (0.165, 0.110), {"hips": 1.0}),
    (v(0, 0.0, 1.12), (0.148, 0.098), {"hips": 0.5, "spine": 0.5}),
    (v(0, -0.005, 1.23), (0.160, 0.102), {"hips": 0.2, "spine": 0.8}),
    (v(0, -0.010, 1.33), (0.180, 0.112), {"spine": 0.5, "chest": 0.5}),
    (v(0, -0.005, 1.425), (0.200, 0.108), {"chest": 1.0}),
    (v(0, 0.0, 1.49), (0.070, 0.062), {"chest": 0.4, "neck": 0.6}),
]


def torso(b, bands, bulk=1.0):
    """The torso painted in `bands`, from the belt up `bulk` times as broad
    (a top worn over it)."""
    points = [(c, (r[0] * bulk, r[1] * bulk) if 1 <= i <= 5 else r, w) for i, (c, r, w) in enumerate(TORSO)]
    limb(b, points, bands, cap_start=True, cap_end=False)


def pocket(b, at, bone, colour, w=0.036, h=0.06):
    """A flap pocket on the front, `w` across and `h` down from `at`."""
    limb(b, [(at, (w, 0.009), {bone: 1.0}), (at - v(0, 0, h), (w, 0.009), {bone: 1.0})], [colour], cap_start=True)


def torso_jacket(b):
    """A field jacket, open down a tee, its collar up, two pockets on the
    chest."""
    open_front = "01100000"
    torso(b, [bottom(), top(0.85), stained(top(), under(), open_front), stained(top(), under(), open_front), stained(top(), under(0.9), open_front), top(0.9)], bulk=1.1)
    limb(b, [
        (v(0, 0.0, 1.46), (0.105, 0.085), {"chest": 0.7, "neck": 0.3}),
        (v(0, 0.012, 1.53), (0.083, 0.07), {"chest": 0.3, "neck": 0.7}),
    ], [top(0.72)], cap_end=False)
    for x in (-0.09, 0.09):
        pocket(b, v(x, 0.118, 1.40), "chest", top(0.78))


def torso_hoodie(b):
    """A hoodie: a pouch across the front, the hood down behind, its
    drawstrings hanging."""
    torso(b, [bottom(), top(0.82), stained(top(), top(0.76), "01100000"), top(), top(), top(0.9)], bulk=1.1)
    limb(b, [
        (v(0, -0.07, 1.44), (0.13, 0.06), {"chest": 1.0}),
        (v(0, -0.06, 1.53), (0.11, 0.06), {"chest": 0.5, "neck": 0.5}),
        (v(0, -0.02, 1.575), (0.07, 0.04), {"neck": 1.0}),
    ], [top(0.85), top(0.78)], cap_start=True)
    for x in (-0.028, 0.028):
        limb(b, [(v(x, 0.118, 1.46), 0.006, {"chest": 1.0}), (v(x * 1.2, 0.124, 1.36), 0.006, {"chest": 1.0})], [under()], cap_start=True)


# ---- arms --------------------------------------------------------------------------

def arm(b, side):
    """Shoulder to fingertips, sleeved to the wrist, a cuff there."""
    up, fore, hand = J[f"upper_arm{side}"], J[f"forearm{side}"], J[f"hand{side}"]
    u, f, h = f"upper_arm{side}", f"forearm{side}", f"hand{side}"
    limb(b, [
        (up[0], 0.064, {"chest": 0.4, u: 0.6}),
        (up[0].lerp(up[1], 0.5), 0.056, {u: 1.0}),
        (up[1], 0.048, {u: 0.5, f: 0.5}),
        (fore[0].lerp(fore[1], 0.5), 0.044, {f: 1.0}),
        (fore[0].lerp(fore[1], 0.9), 0.043, {f: 1.0}),
        (hand[0], 0.031, {f: 0.4, h: 0.6}),
        (hand[0].lerp(hand[1], 0.55), (0.02, 0.042), {h: 1.0}),
        (hand[1] + (hand[1] - hand[0]) * 0.3, (0.016, 0.03), {h: 1.0}),
    ], [top(), top(0.9), top(), top(0.88), top(0.72), skin(), skin(0.85)], cap_start=True)


# ---- legs --------------------------------------------------------------------------

def leg(b, side, bands):
    """Hip to ankle, in `bands`."""
    th, sh = J[f"thigh{side}"], J[f"shin{side}"]
    t, s = f"thigh{side}", f"shin{side}"
    limb(b, [
        (th[0], 0.088, {"hips": 0.5, t: 0.5}),
        (th[0].lerp(th[1], 0.5), 0.078, {t: 1.0}),
        (th[1], 0.063, {t: 0.5, s: 0.5}),
        (sh[0].lerp(sh[1], 0.5), 0.056, {s: 1.0}),
        (sh[1] + v(0, 0, 0.05), 0.051, {s: 1.0}),
    ], bands, ref=FWD)


def shoe(b, side, upper, sole, high):
    """A shoe: `upper` over its `sole`, up the ankle if `high` (a boot)."""
    sh, ft = J[f"shin{side}"], J[f"foot{side}"]
    s, fo = f"shin{side}", f"foot{side}"
    if high:
        limb(b, [(sh[1] + v(0, 0, 0.13), 0.058, {s: 1.0}), (sh[1] + v(0, 0, 0.02), 0.056, {s: 0.5, fo: 0.5})], [upper], cap_end=False, ref=FWD)
    underside = stained(upper, sole, "00011000")
    limb(b, [
        (ft[0] + v(0, -0.045, -0.01), (0.052, 0.048), {fo: 1.0}),
        (ft[0].lerp(ft[1], 0.5) + v(0, 0, 0.005), (0.058, 0.046), {fo: 1.0}),
        (ft[1] + v(0, 0.028, 0.0), (0.048, 0.028), {fo: 1.0}),
    ], [underside, underside], cap_start=True, ref=UP)


def legs_cargo(b):
    """Cargo trousers, a pocket on each thigh, tucked into boots."""
    for side, s in ((".R", 1), (".L", -1)):
        leg(b, side, [bottom(), bottom(0.9), bottom(), bottom(0.85)])
        th = J[f"thigh{side}"]
        mid = th[0].lerp(th[1], 0.55)
        limb(b, [(v(mid.x + 0.075 * s, 0.012, mid.z + 0.07), (0.012, 0.046), {f"thigh{side}": 1.0}), (v(mid.x + 0.072 * s, 0.012, mid.z - 0.07), (0.012, 0.046), {f"thigh{side}": 1.0})], [bottom(0.78)], cap_start=True)
        shoe(b, side, BOOT, SOLE, True)


def legs_jeans(b):
    """Jeans, turned up at the ankle, and sneakers."""
    for side in (".R", ".L"):
        leg(b, side, [bottom(), bottom(0.9), bottom(), bottom(0.72)])
        shoe(b, side, SNEAKER, WHITE, False)
        ft = J[f"foot{side}"]
        limb(b, [(ft[0].lerp(ft[1], 0.25) + v(0, 0, 0.042), (0.024, 0.02), {f"foot{side}": 1.0}), (ft[0].lerp(ft[1], 0.7) + v(0, 0, 0.03), (0.022, 0.018), {f"foot{side}": 1.0})], [LACE], cap_start=True, ref=UP)


# ---- what they carry ---------------------------------------------------------------

def straps(b, front_z):
    """A pack's straps, up the back, over each shoulder, down the front to
    `front_z`."""
    for x in (-0.1, 0.1):
        limb(b, [
            (v(x, -0.13, 1.28), 0.013, {"spine": 0.3, "chest": 0.7}),
            (v(x * 1.05, -0.08, 1.455), 0.013, {"chest": 1.0}),
            (v(x * 1.05, 0.05, 1.47), 0.013, {"chest": 1.0}),
            (v(x, 0.118, 1.38), 0.013, {"chest": 1.0}),
            (v(x, 0.122, front_z), 0.013, {"chest": 0.6, "spine": 0.4}),
        ], [STRAP] * 4, cap_start=True)


def belt(b):
    """A belt with a buckle, pouches on it at the back."""
    limb(b, [(v(0, 0.0, 1.022), (0.176, 0.120), {"hips": 1.0}), (v(0, 0.0, 0.988), (0.176, 0.120), {"hips": 1.0})], [BELT], cap_start=False, cap_end=False)
    limb(b, [(v(0, 0.118, 1.005), (0.022, 0.018), {"hips": 1.0}), (v(0, 0.126, 1.005), (0.022, 0.018), {"hips": 1.0})], [BUCKLE], cap_start=True, ref=UP)
    for x in (-0.13, 0.13):
        limb(b, [(v(x, -0.085, 1.03), (0.036, 0.03), {"hips": 1.0}), (v(x, -0.09, 0.95), (0.034, 0.028), {"hips": 1.0})], [POUCH], cap_start=True)


def ruck(b):
    """A rucksack, a pocket on it, a bedroll across its top."""
    limb(b, [
        (v(0, -0.200, 1.06), (0.14, 0.075), {"spine": 1.0}),
        (v(0, -0.215, 1.22), (0.16, 0.090), {"spine": 0.6, "chest": 0.4}),
        (v(0, -0.210, 1.40), (0.15, 0.085), {"chest": 1.0}),
        (v(0, -0.190, 1.46), (0.10, 0.060), {"chest": 1.0}),
    ], [RUCK, RUCK, RUCK_DARK], cap_start=True)
    limb(b, [(v(0, -0.300, 1.25), (0.10, 0.035), {"spine": 0.5, "chest": 0.5}), (v(0, -0.308, 1.11), (0.10, 0.03), {"spine": 1.0})], [RUCK_DARK], cap_start=True)
    limb(b, [(v(-0.16, -0.205, 1.50), 0.046, {"chest": 1.0}), (v(0.16, -0.205, 1.50), 0.046, {"chest": 1.0})], [BEDROLL], cap_start=True, ref=UP)
    straps(b, 1.22)


def daypack(b):
    """A small daypack."""
    limb(b, [
        (v(0, -0.165, 1.15), (0.12, 0.050), {"spine": 1.0}),
        (v(0, -0.180, 1.27), (0.13, 0.065), {"spine": 0.5, "chest": 0.5}),
        (v(0, -0.172, 1.39), (0.11, 0.055), {"chest": 1.0}),
    ], [DAYPACK, DAYPACK_DARK], cap_start=True)
    straps(b, 1.25)


def parts():
    """Every part by the name the game knows it: {name: build(b)}."""
    out = {
        "head_a": head_a,
        "head_b": head_b,
        "cap": cap,
        "hair_b": hair_b,
        "torso_jacket": torso_jacket,
        "torso_hoodie": torso_hoodie,
        "legs_cargo": legs_cargo,
        "legs_jeans": legs_jeans,
        "belt": belt,
        "ruck": ruck,
        "daypack": daypack,
    }
    for side in (".L", ".R"):
        out[f"arm{side}"] = lambda b, s=side: arm(b, s)
    return out
