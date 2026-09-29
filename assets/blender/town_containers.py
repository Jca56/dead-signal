"""The containers in the town's landmarks: the gun store's glass display
case (CONTAINER_DisplayCase) and its wall rack of long guns
(CONTAINER_GunRack); the fire station's engine (CONTAINER_FireEngine);
a first-aid cabinet (CONTAINER_MedCabinet). See `containers.py`.
"""

import math

from mathutils import Matrix

from container_kit import named, swing

WOOD = (0.36, 0.25, 0.16)
WOOD_DARK = (0.22, 0.15, 0.10)
FRAME = (0.30, 0.30, 0.31)
GLASS_PANE = (0.52, 0.62, 0.66)
FELT = (0.14, 0.20, 0.16)
GUN_METAL = (0.12, 0.12, 0.13)
GUN_WOOD = (0.44, 0.27, 0.14)
BLADE = (0.62, 0.63, 0.62)
AMMO_BOX = [(0.62, 0.18, 0.12), (0.78, 0.66, 0.26), (0.24, 0.34, 0.20)]
PEGBOARD = (0.46, 0.40, 0.30)


def display_case(opened):
    """A glass-topped counter, 1.3 × 0.65 × 1.0: a wooden base, a glass
    box on it, guns and blades lying on green felt inside; its back (the
    clerk's side, -Y) a sliding pane. Opened, the felt bare but for a box
    of rounds, the pane slid aside."""
    p = named("CONTAINER_DisplayCase", opened)
    w, d, h = 1.3, 0.65, 1.0
    base = 0.62
    p.box((-w / 2, -d / 2, 0.0), (w / 2, d / 2, base), WOOD)
    p.box((-w / 2 + 0.03, d / 2, 0.04), (w / 2 - 0.03, d / 2 + 0.01, 0.12), WOOD_DARK)
    p.box((-w / 2 + 0.02, -d / 2 + 0.02, base), (w / 2 - 0.02, d / 2 - 0.02, base + 0.02), FELT)
    # The frame's posts, and the glass: top, front and ends.
    for x in (-w / 2, w / 2 - 0.03):
        for y in (-d / 2, d / 2 - 0.03):
            p.box((x, y, base), (x + 0.03, y + 0.03, h), FRAME)
    p.box((-w / 2, -d / 2, h - 0.02), (w / 2, d / 2, h), GLASS_PANE)
    p.box((-w / 2 + 0.03, d / 2 - 0.01, base), (w / 2 - 0.03, d / 2, h - 0.02), GLASS_PANE)
    for x in (-w / 2, w / 2 - 0.01):
        p.box((x, -d / 2 + 0.03, base), (x + 0.01, d / 2 - 0.03, h - 0.02), GLASS_PANE)
    back = [(-w / 2 + 0.03, -0.02), (0.0, 0.03)] if opened else [(-w / 2 + 0.03, 0.0), (0.0, w / 2 - 0.03)]
    for x0, x1 in back:
        p.box((x0, -d / 2, base), (x1, -d / 2 + 0.01, h - 0.02), GLASS_PANE)
    z = base + 0.02
    if opened:
        p.box((0.25, -0.1, z), (0.4, 0.05, z + 0.06), AMMO_BOX[0])
    else:
        # Two handguns, a knife, boxes of rounds.
        for x in (-0.45, -0.15):
            p.box((x, -0.05, z), (x + 0.2, 0.0, z + 0.025), GUN_METAL)
            p.box((x, -0.05, z), (x + 0.05, 0.1, z + 0.025), GUN_METAL)
        p.box((0.1, 0.05, z), (0.35, 0.08, z + 0.01), BLADE)
        p.box((0.05, 0.04, z), (0.12, 0.09, z + 0.02), GUN_METAL)
        for k, c in enumerate(AMMO_BOX):
            p.box((0.2 + k * 0.12, -0.2, z), (0.3 + k * 0.12, -0.08, z + 0.06), c)
    p.finish()


def long_gun(p, x, z0, stock, barrel):
    """A long gun standing on its stock at `x` against the rack, its
    barrel up to `z0 + barrel`."""
    y = -0.1
    p.box((x - 0.045, y, z0), (x + 0.045, y + 0.06, z0 + 0.42), stock)
    p.box((x - 0.035, y, z0 + 0.42), (x + 0.035, y + 0.06, z0 + 0.7), GUN_METAL)
    p.box((x - 0.015, y + 0.015, z0 + 0.7), (x + 0.015, y + 0.045, z0 + barrel), GUN_METAL)


def gun_rack(opened):
    """A rack for long guns against a wall, 1.9 × 0.35 × 2.0: a pegboard
    back, a shelf low down with a notched rail over it, five long guns
    stood on it; opened, all but one gone."""
    p = named("CONTAINER_GunRack", opened)
    w, d, h = 1.9, 0.35, 2.0
    p.box((-w / 2, -d / 2, 0.0), (w / 2, -d / 2 + 0.04, h), PEGBOARD)
    p.box((-w / 2, -d / 2, 0.0), (w / 2, d / 2, 0.35), WOOD_DARK)
    p.box((-w / 2, -d / 2 + 0.04, 0.35), (w / 2, d / 2, 0.38), WOOD)
    p.box((-w / 2, -d / 2 + 0.04, 1.3), (w / 2, -d / 2 + 0.12, 1.36), WOOD)
    for x in (-w / 2, w / 2 - 0.05):
        p.box((x, -d / 2, 0.0), (x + 0.05, d / 2, h), WOOD_DARK)
    guns = [(-0.7, GUN_WOOD, 1.45), (-0.35, GUN_METAL, 1.35), (0.0, GUN_WOOD, 1.5), (0.35, GUN_WOOD, 1.4), (0.7, GUN_METAL, 1.3)]
    for k, (x, stock, barrel) in enumerate(guns):
        if opened and k != 3:
            continue
        long_gun(p, x, 0.38, stock, barrel)
    p.finish()


ENGINE_RED = (0.66, 0.08, 0.06)
ENGINE_RED_DARK = (0.46, 0.05, 0.04)
ALLOY = (0.66, 0.67, 0.66)
WINDSCREEN = (0.10, 0.12, 0.13)
TYRE = (0.09, 0.09, 0.09)
HUB = (0.60, 0.60, 0.58)
AMBER = (0.90, 0.60, 0.10)
LIGHT_RED = (0.85, 0.08, 0.06)
DARK_INSIDE = (0.06, 0.06, 0.06)


def fire_engine(opened):
    """A fire engine, 8.2 long × 2.5 × 2.6, its cab to +Y: the cab and its
    crew seats behind glass, the body behind lined with roller-doored
    lockers down both sides, a ladder along its roof, six wheels, a light
    bar. Built along X (its cab at +X) and turned. Opened, a locker's
    door rolled up on an empty shelf."""
    p = named("CONTAINER_FireEngine", opened)
    L, W = 8.2, 2.5
    lift, top = 0.55, 2.3
    cab = L / 2 - 2.4
    # The body, the cab (a step taller at the front), the bumper.
    p.box((-L / 2, -W / 2, lift), (cab, W / 2, top), ENGINE_RED)
    p.box((cab, -W / 2, lift), (L / 2, W / 2, top + 0.25), ENGINE_RED)
    p.box((L / 2 - 0.02, -W / 2 + 0.1, top - 0.8), (L / 2 + 0.01, W / 2 - 0.1, top + 0.1), WINDSCREEN)
    for y in (-W / 2 - 0.01, W / 2 - 0.01):
        p.box((cab + 0.3, y, top - 0.75), (L / 2 - 0.3, y + 0.02, top + 0.1), WINDSCREEN)
    p.box((L / 2, -W / 2, lift - 0.1), (L / 2 + 0.2, W / 2, lift + 0.25), ALLOY)
    p.box((-L / 2, -W / 2 + 0.05, lift - 0.25), (L / 2, W / 2 - 0.05, lift), ENGINE_RED_DARK)
    # The lockers down each side: alloy roller doors, one rolled up if
    # searched.
    for side in (-1, 1):
        y = side * W / 2
        for k in range(3):
            x0 = -L / 2 + 0.25 + k * 1.75
            if opened and side == 1 and k == 1:
                p.box((x0, y - 0.3 * side, lift + 0.3), (x0 + 1.55, y - 0.28 * side, top - 0.2), DARK_INSIDE)
                p.box((x0, y - 0.3 * side, lift + 1.0), (x0 + 1.55, y, lift + 1.03), ALLOY)
                p.box((x0, y - 0.02 * side, top - 0.3), (x0 + 1.55, y + 0.02 * side, top - 0.15), ALLOY)
            else:
                p.box((x0, y - 0.01 * side, lift + 0.3), (x0 + 1.55, y + 0.015 * side, top - 0.2), ALLOY)
        p.box((-L / 2 + 0.1, y - 0.01 * side, lift + 0.15), (cab - 0.1, y + 0.02 * side, lift + 0.22), ALLOY)
    # The ladder on the roof: two rails and rungs.
    for y in (-0.55, 0.55):
        p.box((-L / 2 - 0.2, y - 0.04, top + 0.05), (cab + 0.6, y + 0.04, top + 0.15), ALLOY)
    x = -L / 2
    while x < cab + 0.5:
        p.box((x, -0.55, top + 0.08), (x + 0.05, 0.55, top + 0.12), ALLOY)
        x += 0.4
    for y in (-0.6, -0.3):
        p.box((-L / 2 + 0.3, y, top), (-L / 2 + 0.6, y + 0.25, top + 0.05), ALLOY)
    # The light bar on the cab, the amber at the back.
    p.box((L / 2 - 1.4, -0.8, top + 0.25), (L / 2 - 1.1, 0.8, top + 0.36), LIGHT_RED)
    for y in (-W / 2 + 0.1, W / 2 - 0.3):
        p.box((-L / 2 - 0.01, y, top - 0.3), (-L / 2, y + 0.2, top - 0.15), AMBER)
    # Six wheels: the front axle under the cab, two at the back.
    for x in (L / 2 - 1.3, -L / 2 + 2.0, -L / 2 + 3.0):
        for y in (-W / 2 + 0.2, W / 2 - 0.2):
            p.wheel((x, y, 0.5), 0.5, 0.35, TYRE, HUB)
    turn = Matrix.Rotation(math.pi / 2, 4, "Z")
    for v in p.bm.verts:
        v.co = turn @ v.co
    p.finish()


CABINET_WHITE = (0.82, 0.82, 0.78)
CABINET_GREY = (0.60, 0.60, 0.58)
CROSS_RED = (0.72, 0.10, 0.08)
SUPPLIES = [(0.85, 0.83, 0.76), (0.66, 0.10, 0.08), (0.30, 0.52, 0.70)]


def med_cabinet(opened):
    """A tall steel medical cabinet, 0.8 × 0.4 × 1.8, two doors, a red
    cross on them; opened, the doors swung wide on shelves of supplies
    picked over."""
    p = named("CONTAINER_MedCabinet", opened)
    w, d, h = 0.8, 0.4, 1.8
    p.box((-w / 2, -d / 2, 0.05), (w / 2, d / 2 - 0.02, h), CABINET_WHITE)
    p.box((-w / 2, -d / 2, 0.0), (w / 2, d / 2 - 0.04, 0.05), CABINET_GREY)
    left = p.box((-w / 2 + 0.02, d / 2 - 0.02, 0.08), (-0.005, d / 2, h - 0.03), CABINET_WHITE)
    right = p.box((0.005, d / 2 - 0.02, 0.08), (w / 2 - 0.02, d / 2, h - 0.03), CABINET_WHITE)
    # The cross, across both doors, and the handles.
    for lo, hi, half in (((-0.2, 1.3), (0.0, 1.38), "l"), ((0.0, 1.3), (0.2, 1.38), "r"), ((-0.04, 1.14), (0.0, 1.54), "l"), ((0.0, 1.14), (0.04, 1.54), "r")):
        made = p.box((lo[0], d / 2, lo[1]), (hi[0], d / 2 + 0.006, hi[1]), CROSS_RED)
        (left if half == "l" else right).extend(made)
    left += p.box((-0.06, d / 2, 0.8), (-0.03, d / 2 + 0.04, 1.0), CABINET_GREY)
    right += p.box((0.03, d / 2, 0.8), (0.06, d / 2 + 0.04, 1.0), CABINET_GREY)
    if opened:
        p.box((-w / 2 + 0.02, d / 2 - 0.025, 0.08), (w / 2 - 0.02, d / 2 - 0.021, h - 0.03), (0.30, 0.30, 0.30))
        for z in (0.5, 0.95, 1.4):
            p.box((-w / 2 + 0.02, -d / 2 + 0.02, z), (w / 2 - 0.02, d / 2 - 0.02, z + 0.02), CABINET_GREY)
        p.box((-0.25, -0.1, 0.97), (-0.1, 0.05, 1.07), SUPPLIES[0])
        p.box((0.1, -0.1, 1.42), (0.22, 0.02, 1.5), SUPPLIES[2])
        swing(left, (-w / 2 + 0.02, d / 2, 0.0), "Z", -100.0)
        swing(right, (w / 2 - 0.02, d / 2, 0.0), "Z", 100.0)
    else:
        p.box((-0.3, -0.1, 1.42), (-0.15, 0.05, 1.52), SUPPLIES[1])
    p.finish()
