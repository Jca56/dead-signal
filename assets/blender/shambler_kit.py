"""What the dead are made with (`shambler_body.py`, `shambler_extras.py`):
the skeleton at rest, tubes along it, and the colours they're painted.

Most of a Shambler isn't painted a colour of its own but a region's: its
skin, its top, its bottoms, an accent (hair, a hat, a tie) or what's worn
under (a jacket's shirt). A region's faces are a grey shade (how light or
dark that band of it is) with the region in the colour's alpha; the game
gives each of the dead its own colour for every region and multiplies it
in. Faces of a colour of their own (blood, bone, shoes) have an alpha of 1.

Built facing Blender's +Y (the game's -Z), +X its right, feet on the ground
at the origin.
"""

from mathutils import Vector

from kit import norm

# The regions, as the game reads them from the alpha (`figures.wgsl`).
SKIN_R, TOP_R, BOTTOM_R, ACCENT_R, UNDER_R = 0.0, 0.2, 0.4, 0.6, 0.8


def skin(shade=1.0):
    return (shade, shade, shade, SKIN_R)


def top(shade=1.0):
    return (shade, shade, shade, TOP_R)


def bottom(shade=1.0):
    return (shade, shade, shade, BOTTOM_R)


def accent(shade=1.0):
    return (shade, shade, shade, ACCENT_R)


def under(shade=1.0):
    return (shade, shade, shade, UNDER_R)


# Colours of their own.
GRIME = (0.24, 0.22, 0.20)
SHOE = (0.12, 0.10, 0.09)
SOCKET = (0.10, 0.09, 0.08)
MOUTH = (0.16, 0.07, 0.06)
BLOOD = (0.26, 0.10, 0.08)
BLOOD_DARK = (0.18, 0.06, 0.05)
GORE = (0.30, 0.06, 0.05)
MEAT = (0.46, 0.16, 0.14)
BONE = (0.76, 0.72, 0.60)
VEST = (0.20, 0.21, 0.16)
PACK = (0.26, 0.25, 0.18)


def v(x, y, z):
    return Vector((x, y, z))


# The skeleton at rest: (head, tail, parent). Right side is +X.
BONES = {
    "hips": (v(0, 0.02, 0.95), v(0, 0.04, 1.15), "root"),
    "spine": (v(0, 0.04, 1.15), v(0, 0.10, 1.33), "hips"),
    "chest": (v(0, 0.10, 1.33), v(0, 0.18, 1.47), "spine"),
    "neck": (v(0, 0.18, 1.47), v(0, 0.24, 1.56), "chest"),
    "head": (v(0, 0.24, 1.56), v(0, 0.30, 1.78), "neck"),
    # The right arm reaches; the left hangs.
    "upper_arm.R": (v(0.20, 0.12, 1.43), v(0.24, 0.22, 1.19), "chest"),
    "forearm.R": (v(0.24, 0.22, 1.19), v(0.24, 0.42, 1.03), "upper_arm.R"),
    "hand.R": (v(0.24, 0.42, 1.03), v(0.24, 0.51, 0.99), "forearm.R"),
    "upper_arm.L": (v(-0.20, 0.12, 1.43), v(-0.23, 0.13, 1.16), "chest"),
    "forearm.L": (v(-0.23, 0.13, 1.16), v(-0.24, 0.18, 0.90), "upper_arm.L"),
    "hand.L": (v(-0.24, 0.18, 0.90), v(-0.24, 0.20, 0.80), "forearm.L"),
}
for _side, _x in ((".R", 0.10), (".L", -0.10)):
    BONES[f"thigh{_side}"] = (v(_x, 0.02, 0.93), v(_x * 1.1, 0.05, 0.50), "hips")
    BONES[f"shin{_side}"] = (v(_x * 1.1, 0.05, 0.50), v(_x * 1.1, 0.02, 0.08), f"thigh{_side}")
    BONES[f"foot{_side}"] = (v(_x * 1.1, 0.02, 0.08), v(_x * 1.1, 0.17, 0.03), f"shin{_side}")

# Where each bone runs: (head, tail).
J = {name: (h, t) for name, (h, t, _) in BONES.items()}
FWD = Vector((0, 1, 0))
UP = Vector((0, 0, 1))


def stained(a, b, pattern):
    """A ring of face colours: `b` where `pattern` has a 1."""
    return [b if c == "1" else a for c in pattern]


def limb(b, points, colours, cap_start=False, cap_end=True, ref=Vector((1, 0, 0))):
    """A tube through `points` [(centre, radius, weights)], one starting
    direction for every ring (`ref`, kept square to each). Round a tube
    along Z with `ref` +X, face 0 is at the right, 1 and 2 at the front,
    3 and 4 at the left, 5 and 6 at the back; with `ref` forward, 0 and 7
    at the front."""
    rings = []
    for i, (centre, radius, weights) in enumerate(points):
        a = points[max(i - 1, 0)][0]
        c = points[min(i + 1, len(points) - 1)][0]
        axis = norm(c - a)
        ra, rb = radius if isinstance(radius, tuple) else (radius, radius)
        rings.append(b.ring(centre, axis, ref, ra, rb, weights))
    b.tube(rings, colours, cap_start=cap_start, cap_end=cap_end)
