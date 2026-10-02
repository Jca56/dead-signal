"""What the relay station (HOLDOUT's arena) is fitted with, each its own
object standing on its origin (the middle of its footprint, on the
ground), its front facing +Y:

    SITE_ArmyTruck   a six-wheeled cargo truck, canvas over its bed
    SITE_Generator   a diesel generator set on its skid, long across
                     (its panel on its front)

and what the game bumps into for each (SITE_*_Hull). See `sites.py`, which
hands in how it makes a part and a hull.
"""

OLIVE = (0.28, 0.30, 0.22)
OLIVE_DARK = (0.20, 0.22, 0.16)
CANVAS = (0.36, 0.36, 0.26)
CANVAS_DARK = (0.28, 0.28, 0.20)
TYRE = (0.07, 0.07, 0.07)
HUB = (0.24, 0.25, 0.21)
DARK = (0.10, 0.10, 0.10)
GLASS = (0.12, 0.16, 0.18)
LAMP = (0.78, 0.76, 0.62)
WHITE = (0.80, 0.79, 0.75)
SKID = (0.22, 0.22, 0.21)
HOUSING = (0.33, 0.37, 0.24)
GRILLE = (0.15, 0.15, 0.14)
PANEL = (0.55, 0.55, 0.52)
RED = (0.62, 0.12, 0.09)


def army_truck(Part, hull):
    """6.4 long, 2.3 wide, 2.6 to the top of its canvas."""
    p = Part("SITE_ArmyTruck")
    hw = 1.15
    # Its wheels (the rear ones in pairs), each with its hub.
    for y in (2.15, -1.25, -2.35):
        for s in (-1, 1):
            x0, x1 = (s * hw - 0.36, s * hw) if s > 0 else (s * hw, s * hw + 0.36)
            p.box((x0, y - 0.47, 0.0), (x1, y + 0.47, 0.94), TYRE)
            face = x1 if s > 0 else x0 - 0.02
            p.box((face, y - 0.2, 0.27), (face + 0.02, y + 0.2, 0.67), HUB)
    p.box((-0.5, -3.1, 0.5), (0.5, 3.0, 0.72), DARK)
    # The bumper, the grille and the lamps either side of it, the hood, a
    # fender over each front wheel.
    p.box((-hw, 3.02, 0.48), (hw, 3.2, 0.72), OLIVE_DARK)
    p.box((-0.5, 3.02, 0.8), (0.5, 3.06, 1.38), DARK)
    for s in (-1, 1):
        p.box((s * 0.72 - 0.11, 3.02, 0.98), (s * 0.72 + 0.11, 3.07, 1.2), LAMP)
        p.box((s * hw - (0.34 if s > 0 else 0.0), 1.55, 0.98), (s * hw + (0.0 if s > 0 else 0.34), 2.75, 1.08), OLIVE_DARK)
    p.box((-0.82, 1.9, 0.72), (0.82, 3.02, 1.45), OLIVE)
    # The cab: its windscreen, a window and a star on each door.
    p.box((-1.08, 0.62, 0.72), (1.08, 1.9, 2.12), OLIVE)
    p.box((-1.1, 0.6, 2.12), (1.1, 1.93, 2.18), OLIVE_DARK)
    p.box((-0.94, 1.9, 1.5), (0.94, 1.915, 2.04), GLASS)
    for s in (-1, 1):
        x = s * 1.08
        p.box((x - 0.012, 0.86, 1.52), (x + 0.012, 1.74, 2.02), GLASS)
        p.box((x - 0.014, 1.12, 1.0), (x + 0.014, 1.42, 1.3), WHITE)
        p.box((x - 0.2 if s < 0 else x, 0.9, 0.55), (x if s < 0 else x + 0.2, 1.6, 0.62), OLIVE_DARK)
    # The bed: its floor and boards, the canvas over its hoops, open at
    # the back over the tailgate.
    p.box((-hw, -3.2, 1.0), (hw, 0.55, 1.14), OLIVE_DARK)
    for s in (-1, 1):
        p.box((s * hw - (0.06 if s > 0 else 0.0), -3.2, 1.14), (s * hw + (0.0 if s > 0 else 0.06), 0.55, 1.62), OLIVE)
    p.box((-hw, 0.49, 1.14), (hw, 0.55, 1.62), OLIVE)
    p.box((-hw, -3.2, 1.14), (hw, -3.14, 1.55), OLIVE)
    p.box((-hw, -3.14, 1.62), (hw, 0.55, 2.42), CANVAS)
    p.box((-hw + 0.22, -3.14, 2.42), (hw - 0.22, 0.55, 2.6), CANVAS_DARK)
    p.box((-hw + 0.12, -3.2, 1.62), (hw - 0.12, -3.14, 2.36), DARK)
    for y in (-2.4, -1.3, -0.2):
        p.box((-hw - 0.012, y, 1.62), (-hw, y + 0.06, 2.42), CANVAS_DARK)
        p.box((hw, y, 1.62), (hw + 0.012, y + 0.06, 2.42), CANVAS_DARK)
    # A fuel tank under one side, a red lamp at each corner of its tail.
    p.box((-hw + 0.02, -0.5, 0.42), (-hw + 0.42, 0.5, 0.82), OLIVE_DARK)
    for s in (-1, 1):
        p.box((s * 0.95 - 0.08, -3.22, 0.86), (s * 0.95 + 0.08, -3.2, 0.98), RED)
    p.finish()
    hull("SITE_ArmyTruck", [((-hw, 1.9, 0.0), (hw, 3.2, 1.45)), ((-hw, 0.6, 0.0), (hw, 1.9, 2.18)), ((-hw, -3.2, 0.0), (hw, 0.6, 2.6))])


def generator(Part, hull):
    """3.2 long (across), 1.5 deep, 1.6 high; its exhaust up to 2.4."""
    p = Part("SITE_Generator")
    hl, hw = 1.6, 0.75
    p.box((-hl, -hw, 0.0), (hl, hw, 0.18), SKID)
    p.box((-hl + 0.1, -hw + 0.08, 0.18), (hl - 0.35, hw - 0.08, 1.6), HOUSING)
    p.box((hl - 0.35, -hw + 0.08, 0.18), (hl - 0.1, hw - 0.08, 1.5), GRILLE)
    # Its doors' seams, its panel and what's on it.
    for x in (-0.75, 0.35):
        p.box((x, hw - 0.08, 0.3), (x + 0.02, hw - 0.07, 1.5), GRILLE)
        p.box((x, -hw + 0.07, 0.3), (x + 0.02, -hw + 0.08, 1.5), GRILLE)
    p.box((-0.6, hw - 0.08, 0.7), (0.2, hw + 0.02, 1.3), PANEL)
    for k in range(3):
        p.box((-0.52 + k * 0.24, hw + 0.02, 1.02), (-0.36 + k * 0.24, hw + 0.03, 1.18), DARK)
    p.box((-0.5, hw + 0.02, 0.8), (-0.42, hw + 0.04, 0.88), RED)
    p.box((-0.3, hw + 0.02, 0.8), (0.1, hw + 0.03, 0.9), GRILLE)
    # The exhaust up off its top, a cap on it; the filler; a lifting eye.
    p.box((-hl + 0.5, -0.12, 1.6), (-hl + 0.74, 0.12, 2.4), GRILLE)
    p.box((-hl + 0.44, -0.18, 2.4), (-hl + 0.8, 0.18, 2.45), SKID)
    p.box((0.6, -0.3, 1.6), (0.78, -0.12, 1.7), SKID)
    p.box((-0.1, -0.06, 1.6), (0.1, 0.06, 1.72), SKID)
    p.finish()
    hull("SITE_Generator", [((-hl, -hw, 0.0), (hl, hw, 1.6)), ((-hl + 0.5, -0.12, 1.6), (-hl + 0.74, 0.12, 2.4))])


def make(Part, hull):
    army_truck(Part, hull)
    generator(Part, hull)
