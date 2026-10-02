"""What the living are made with (`survivor_body.py`, `survivor.py`): an
upright skeleton at rest, named bone for bone as the dead's (the game hits
and poses them the same way), and how a leg is posed to put its foot where
it's wanted (so a stride plants its feet).

Built facing Blender's +Y (the game's -Z), +X its right, feet on the ground
at the origin. The colours of their regions (skin, top, bottoms, accent,
under) are the dead's (`shambler_kit.py`), each player's own in the game.
"""

import math

from mathutils import Vector

from shambler_kit import accent, bottom, limb, skin, stained, top, under  # noqa: F401


def v(x, y, z):
    return Vector((x, y, z))


FWD = Vector((0, 1, 0))
UP = Vector((0, 0, 1))

# Colours of their own.
BOOT = (0.20, 0.14, 0.09)
SOLE = (0.10, 0.09, 0.08)
SNEAKER = (0.80, 0.80, 0.76)
LACE = (0.60, 0.60, 0.58)
EYE = (0.08, 0.07, 0.07)
WHITE = (0.90, 0.89, 0.85)
LIP = (0.56, 0.32, 0.29)
RUCK = (0.29, 0.30, 0.20)
RUCK_DARK = (0.21, 0.22, 0.15)
BEDROLL = (0.50, 0.42, 0.30)
DAYPACK = (0.20, 0.21, 0.23)
DAYPACK_DARK = (0.13, 0.14, 0.15)
STRAP = (0.12, 0.12, 0.11)
BELT = (0.16, 0.12, 0.08)
POUCH = (0.27, 0.27, 0.19)
BUCKLE = (0.62, 0.60, 0.55)

# The skeleton at rest: (head, tail, parent). Right side is +X. The arms
# hang a little out from the sides.
BONES = {
    "hips": (v(0, 0.0, 0.97), v(0, 0.0, 1.12), "root"),
    "spine": (v(0, 0.0, 1.12), v(0, -0.01, 1.30), "hips"),
    "chest": (v(0, -0.01, 1.30), v(0, 0.0, 1.47), "spine"),
    "neck": (v(0, 0.0, 1.47), v(0, 0.02, 1.57), "chest"),
    "head": (v(0, 0.02, 1.57), v(0, 0.02, 1.80), "neck"),
}
for _side, _s in ((".R", 1.0), (".L", -1.0)):
    BONES[f"upper_arm{_side}"] = (v(0.19 * _s, -0.01, 1.44), v(0.245 * _s, -0.01, 1.155), "chest")
    BONES[f"forearm{_side}"] = (v(0.245 * _s, -0.01, 1.155), v(0.28 * _s, 0.015, 0.905), f"upper_arm{_side}")
    BONES[f"hand{_side}"] = (v(0.28 * _s, 0.015, 0.905), v(0.29 * _s, 0.025, 0.815), f"forearm{_side}")
    BONES[f"thigh{_side}"] = (v(0.10 * _s, 0.0, 0.95), v(0.105 * _s, 0.01, 0.52), "hips")
    BONES[f"shin{_side}"] = (v(0.105 * _s, 0.01, 0.52), v(0.105 * _s, -0.01, 0.09), f"thigh{_side}")
    BONES[f"foot{_side}"] = (v(0.105 * _s, -0.01, 0.09), v(0.105 * _s, 0.15, 0.02), f"shin{_side}")

# Where each bone runs: (head, tail).
J = {name: (h, t) for name, (h, t, _) in BONES.items()}

# ---- legs --------------------------------------------------------------------------

HIP = J["thigh.R"][0]
ANKLE = J["foot.R"][0].z
THIGH = (J["thigh.R"][1] - J["thigh.R"][0]).length
SHIN = (J["shin.R"][1] - J["shin.R"][0]).length


def _sagittal(a, b):
    """How far `b - a` is turned from straight down towards the front,
    radians (seen from the side)."""
    d = b - a
    return math.atan2(d.y, -d.z)


REST_THIGH = _sagittal(*J["thigh.R"])
REST_SHIN = _sagittal(*J["shin.R"])


def leg(dy, dz, pitch=0.0):
    """The thigh's, shin's and foot's turns (degrees about X) that put the
    ankle `dy` ahead of the hip and `dz` above it (below: negative), the
    knee to the front, the foot pitched `pitch` degrees from flat (toe up
    is positive). Out of reach, the leg reaches as far as it can."""
    d = min(max(math.hypot(dy, dz), 0.25), THIGH + SHIN - 1e-4)
    down = math.atan2(dy, -dz)
    at_hip = math.acos(max(-1.0, min(1.0, (THIGH * THIGH + d * d - SHIN * SHIN) / (2 * THIGH * d))))
    at_knee = math.acos(max(-1.0, min(1.0, (THIGH * THIGH + SHIN * SHIN - d * d) / (2 * THIGH * SHIN))))
    thigh = down + at_hip
    shin = thigh - (math.pi - at_knee)
    t = math.degrees(thigh - REST_THIGH)
    s = math.degrees(shin - REST_SHIN) - t
    return t, s, pitch - t - s


def legs_to(pose, feet, hips=(0.0, 0.0, 0.0)):
    """`pose` with both legs put where `feet` says ({".L": (y, z, pitch),
    ".R": ...}, the ankle ahead of the origin and above the ground), the
    hips moved by `hips` (dx, dy, dz) and not turned."""
    out = dict(pose)
    hy, hz = HIP.y + hips[1], HIP.z + hips[2]
    for side, (y, z, pitch) in feet.items():
        t, s, f = leg(y - hy, z - hz, pitch)
        out[f"thigh{side}"] = (t, 0.0, 0.0)
        out[f"shin{side}"] = (s, 0.0, 0.0)
        out[f"foot{side}"] = (f, 0.0, 0.0)
    rot = out.get("hips", (0.0, 0.0, 0.0))
    out["hips"] = (rot[0], rot[1], rot[2], hips)
    return out
