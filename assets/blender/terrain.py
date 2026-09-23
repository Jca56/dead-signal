"""What both scenes agree on: the shape of the ground, and where the
proving ground sits in it. Blender coordinates (Z up; +Y is towards the
tower, which the game calls -Z)."""

import math


def height(x, y):
    """Metres above zero at a point: gentle hills, flat in the clearing,
    rising to ridges at the edges so the horizon is never a straight line."""
    h = 1.6 * math.sin(x * 0.045 + 0.7) * math.cos(y * 0.038 - 0.3)
    h += 0.8 * math.sin(x * 0.11 + y * 0.07)
    h += 0.4 * math.sin(x * 0.23 - y * 0.19 + 1.3)
    r = math.hypot(x, y - 20.0)
    h *= min(1.0, 0.25 + r / 60.0)
    edge = max(0.0, r - 70.0)
    return h + (edge / 25.0) ** 2 * 3.0


# The proving ground: a 20 m pad behind the spawn, out of the title
# camera's sight (it looks towards +Y from about y = -22).
PAD_X = 0.0
PAD_Y = -38.0
PAD_HALF = 10.0
# Where no tree or rock may stand: the pad, its access ramp to the east,
# the shooting lane running on east from it, and a margin. Everything here
# is behind the title camera.
CLEAR = [(-13.0, 19.0, -52.0, -25.0), (19.0, 66.0, -37.0, -26.0)]
# The lane: shot along +X from the pad's east edge at y = LANE_Y, with
# steel plates LANE_PLATES metres out.
LANE_Y = PAD_Y + 6.0
LANE_FROM = PAD_X + PAD_HALF
LANE_PLATES = (10.0, 25.0, 50.0)


def in_clearing(x, y):
    return any(x0 <= x <= x1 and y0 <= y <= y1 for x0, x1, y0, y1 in CLEAR)
