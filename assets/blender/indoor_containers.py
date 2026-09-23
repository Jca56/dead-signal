"""The containers that stand in houses and stores: a fridge, a chest of
drawers (CONTAINER_Cabinet), a desk, a wardrobe, a store's shelving and
its till counter (CONTAINER_Register). See `containers.py`.
"""

from container_kit import CHROME, HOLLOW, named, rng, swing

WHITE_GOODS = (0.80, 0.80, 0.76)
FRIDGE_DARK = (0.30, 0.31, 0.31)
WOOD = (0.45, 0.32, 0.21)
WOOD_DARK = (0.30, 0.21, 0.14)
WOOD_PALE = (0.58, 0.46, 0.32)
KNOB = (0.66, 0.60, 0.40)
SHELF_METAL = (0.55, 0.56, 0.56)
GOODS = [(0.70, 0.20, 0.16), (0.85, 0.72, 0.30), (0.24, 0.45, 0.62), (0.30, 0.56, 0.30), (0.82, 0.80, 0.74), (0.62, 0.36, 0.18)]
COUNTER = (0.52, 0.47, 0.40)
COUNTER_TOP = (0.30, 0.30, 0.30)
TILL = (0.20, 0.21, 0.22)


def fridge(opened):
    """A fridge, 0.75 × 0.7 × 1.8, its door hinged at its left; opened,
    swung wide on a lit-less inside and bare shelves."""
    p = named("CONTAINER_Fridge", opened)
    w, d, h = 0.75, 0.7, 1.8
    p.box((-w / 2, -d / 2, 0.0), (w / 2, d / 2 - 0.04, h), WHITE_GOODS)
    door = p.box((-w / 2, d / 2 - 0.04, 0.02), (w / 2, d / 2, h - 0.02), WHITE_GOODS)
    door += p.box((w / 2 - 0.1, d / 2, 0.9), (w / 2 - 0.07, d / 2 + 0.04, 1.35), CHROME)
    door += p.box((-w / 2, d / 2 - 0.041, 1.2), (w / 2, d / 2 + 0.001, 1.22), FRIDGE_DARK)
    if opened:
        p.box((-w / 2 + 0.04, d / 2 - 0.045, 0.06), (w / 2 - 0.04, d / 2 - 0.041, h - 0.06), HOLLOW)
        for z in (0.5, 0.95, 1.4):
            p.box((-w / 2 + 0.04, -d / 2 + 0.05, z), (w / 2 - 0.04, d / 2 - 0.045, z + 0.02), FRIDGE_DARK)
        swing(door, (-w / 2, d / 2, 0.0), "Z", 105.0)
    p.finish()


def cabinet(opened):
    """A chest of three drawers, 1.0 × 0.5 × 0.9; opened, its top drawer
    pulled out."""
    p = named("CONTAINER_Cabinet", opened)
    w, d, h = 1.0, 0.5, 0.9
    p.box((-w / 2, -d / 2, 0.05), (w / 2, d / 2 - 0.02, h), WOOD)
    p.box((-w / 2 - 0.02, -d / 2, h), (w / 2 + 0.02, d / 2 + 0.02, h + 0.03), WOOD_DARK)
    for k in range(3):
        z0 = 0.1 + k * 0.27
        drawer = p.box((-w / 2 + 0.04, d / 2 - 0.02, z0), (w / 2 - 0.04, d / 2, z0 + 0.24), WOOD_PALE)
        drawer += p.box((-0.08, d / 2, z0 + 0.1), (0.08, d / 2 + 0.03, z0 + 0.14), KNOB)
        if opened and k == 2:
            drawer += p.box((-w / 2 + 0.06, -d / 2 + 0.05, z0), (w / 2 - 0.06, d / 2 - 0.02, z0 + 0.02), WOOD_PALE)
            p.box((-w / 2 + 0.06, d / 2 - 0.03, z0 + 0.02), (w / 2 - 0.06, d / 2 - 0.021, z0 + 0.22), HOLLOW)
            for v in drawer:
                v.co.y += 0.32
    p.finish()


def desk(opened):
    """A desk, 1.3 × 0.65 × 0.76: a top on a panel at its left and a
    pedestal of drawers at its right, a chair tucked in; opened, a drawer
    out."""
    p = named("CONTAINER_Desk", opened)
    w, d, h = 1.3, 0.65, 0.76
    p.box((-w / 2, -d / 2, h - 0.04), (w / 2, d / 2, h), WOOD)
    p.box((-w / 2, -d / 2, 0.0), (-w / 2 + 0.04, d / 2, h - 0.04), WOOD_DARK)
    p.box((w / 2 - 0.42, -d / 2, 0.0), (w / 2, d / 2 - 0.02, h - 0.04), WOOD_DARK)
    p.box((-w / 2 + 0.04, -d / 2, 0.3), (w / 2 - 0.42, -d / 2 + 0.03, h - 0.04), WOOD_DARK)
    for k in range(3):
        z0 = 0.06 + k * 0.22
        drawer = p.box((w / 2 - 0.4, d / 2 - 0.02, z0), (w / 2 - 0.02, d / 2, z0 + 0.19), WOOD_PALE)
        drawer += p.box((w / 2 - 0.25, d / 2, z0 + 0.08), (w / 2 - 0.17, d / 2 + 0.03, z0 + 0.11), KNOB)
        if opened and k == 2:
            p.box((w / 2 - 0.39, d / 2 - 0.03, z0 + 0.02), (w / 2 - 0.03, d / 2 - 0.021, z0 + 0.17), HOLLOW)
            for v in drawer:
                v.co.y += 0.28
    # Papers, and a lamp.
    p.box((-0.4, -0.1, h), (-0.1, 0.12, h + 0.01), (0.85, 0.83, 0.76))
    p.box((0.2, -0.25, h), (0.3, -0.15, h + 0.35), FRIDGE_DARK)
    # The chair, pushed in.
    p.box((-0.45, d / 2 - 0.1, 0.44), (0.0, d / 2 + 0.3, 0.48), WOOD_DARK)
    p.box((-0.45, d / 2 + 0.26, 0.48), (0.0, d / 2 + 0.3, 0.9), WOOD_DARK)
    for x in (-0.43, -0.04):
        for y in (d / 2 - 0.08, d / 2 + 0.26):
            p.box((x, y, 0.0), (x + 0.03, y + 0.03, 0.44), WOOD_DARK)
    p.finish()


def wardrobe(opened):
    """A wardrobe, 1.2 × 0.6 × 2.0, two doors; opened, both swung out on a
    rail of dark clothes."""
    p = named("CONTAINER_Wardrobe", opened)
    w, d, h = 1.2, 0.6, 2.0
    p.box((-w / 2, -d / 2, 0.05), (w / 2, d / 2 - 0.03, h), WOOD_DARK)
    p.box((-w / 2 - 0.02, -d / 2, h), (w / 2 + 0.02, d / 2 + 0.02, h + 0.05), WOOD_DARK)
    left = p.box((-w / 2 + 0.02, d / 2 - 0.03, 0.08), (-0.005, d / 2, h - 0.03), WOOD)
    left += p.box((-0.07, d / 2, 0.95), (-0.04, d / 2 + 0.03, 1.15), KNOB)
    right = p.box((0.005, d / 2 - 0.03, 0.08), (w / 2 - 0.02, d / 2, h - 0.03), WOOD)
    right += p.box((0.04, d / 2, 0.95), (0.07, d / 2 + 0.03, 1.15), KNOB)
    if opened:
        p.box((-w / 2 + 0.03, d / 2 - 0.035, 0.08), (w / 2 - 0.03, d / 2 - 0.031, h - 0.03), HOLLOW)
        p.box((-w / 2 + 0.05, 0.0, 1.75), (w / 2 - 0.05, 0.02, 1.77), CHROME)
        for k in range(4):
            x = -0.45 + k * 0.28
            p.box((x, -0.18, 0.9), (x + 0.2, 0.2, 1.72), [(0.25, 0.27, 0.35), (0.40, 0.22, 0.20), (0.30, 0.30, 0.28)][k % 3])
        swing(left, (-w / 2 + 0.02, d / 2, 0.0), "Z", -100.0)
        swing(right, (w / 2 - 0.02, d / 2, 0.0), "Z", 100.0)
    p.finish()


def shelf(opened):
    """A store's shelving, 1.8 × 0.6 × 1.6, three shelves of goods, both
    faces; opened (searched), most of it gone."""
    p = named("CONTAINER_Shelf", opened)
    w, d, h = 1.8, 0.6, 1.6
    p.box((-w / 2, -0.03, 0.0), (w / 2, 0.03, h), SHELF_METAL)
    for x in (-w / 2, w / 2 - 0.04):
        p.box((x, -d / 2, 0.0), (x + 0.04, d / 2, h), SHELF_METAL)
    left = 0
    for z in (0.08, 0.6, 1.1):
        p.box((-w / 2, -d / 2, z), (w / 2, d / 2, z + 0.03), SHELF_METAL)
        for side in (-1, 1):
            x = -w / 2 + 0.08
            while x < w / 2 - 0.2:
                bw = rng.uniform(0.12, 0.24)
                bh = rng.uniform(0.15, 0.36)
                keep = not opened or rng.random() < 0.15
                if keep:
                    y0, y1 = (0.05, d / 2 - 0.03) if side > 0 else (-d / 2 + 0.03, -0.05)
                    p.box((x, y0, z + 0.03), (x + bw, y1, z + 0.03 + bh), GOODS[left % len(GOODS)])
                left += 1
                x += bw + 0.03
    p.finish()


def register(opened):
    """A shop counter, 2.0 × 0.8 × 1.0, a till on it; opened, its drawer
    out and empty."""
    p = named("CONTAINER_Register", opened)
    w, d, h = 2.0, 0.8, 1.0
    p.box((-w / 2, -d / 2, 0.0), (w / 2, d / 2, h - 0.04), COUNTER)
    p.box((-w / 2 - 0.03, -d / 2 - 0.03, h - 0.04), (w / 2 + 0.03, d / 2 + 0.03, h), COUNTER_TOP)
    p.box((-w / 2 + 0.05, d / 2, 0.05), (w / 2 - 0.05, d / 2 + 0.01, 0.12), WOOD_DARK)
    # The till, facing the customer's side (+Y).
    p.box((0.3, -0.2, h), (0.75, 0.2, h + 0.12), TILL)
    p.box((0.35, -0.15, h + 0.12), (0.7, 0.0, h + 0.32), TILL)
    p.box((0.37, -0.012, h + 0.15), (0.68, 0.0, h + 0.3), (0.25, 0.42, 0.30))
    drawer = p.box((0.32, 0.12, h + 0.01), (0.73, 0.2, h + 0.1), TILL)
    if opened:
        for v in drawer:
            v.co.y += 0.25
        p.box((0.34, 0.2, h + 0.02), (0.71, 0.45, h + 0.03), HOLLOW)
    p.finish()
