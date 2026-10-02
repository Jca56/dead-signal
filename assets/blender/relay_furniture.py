"""The relay station's own furniture (HOLDOUT's arena), each its own
object standing on its origin (the middle of its footprint), its back
against a wall at -Y and its front (what faces the room) at +Y:

    FURN_Bunk        a steel bunk bed, two high, a ladder at its foot
    FURN_ServerRack  a tall rack of machines, their lamps lit
    FURN_Console     a radio console: a desk, a panel of dials over it,
                     a microphone

and what the game bumps into for each (FURN_*_Hull). See `furniture.py`,
which hands in how it starts a piece.
"""

import random

rng = random.Random(77)

FRAME = (0.30, 0.32, 0.31)
MATTRESS = (0.74, 0.72, 0.64)
ARMY_BLANKET = (0.29, 0.32, 0.22)
PILLOW = (0.82, 0.80, 0.74)
CABINET = (0.13, 0.14, 0.15)
UNIT = [(0.20, 0.21, 0.23), (0.27, 0.28, 0.30)]
LAMP_GREEN = (0.18, 0.80, 0.32)
LAMP_AMBER = (0.90, 0.58, 0.10)
LAMP_RED = (0.85, 0.14, 0.10)
DESK = (0.42, 0.45, 0.42)
DESK_DARK = (0.28, 0.30, 0.29)
PANEL = (0.33, 0.37, 0.31)
DIAL = (0.84, 0.82, 0.72)
BLACK = (0.07, 0.07, 0.07)
PAPER = (0.80, 0.77, 0.66)


def bunk(piece):
    w, d, h = 2.0, 0.95, 1.78
    p = piece("FURN_Bunk", [((-w / 2, -d / 2, 0.0), (w / 2, d / 2, h))])
    for x in (-w / 2, w / 2 - 0.06):
        for y in (-d / 2, d / 2 - 0.06):
            p.box((x, y, 0.0), (x + 0.06, y + 0.06, h), FRAME)
    for z in (0.38, 1.28):
        # The frame, the mattress on it, a blanket over its foot, a pillow.
        p.box((-w / 2, -d / 2, z), (w / 2, d / 2, z + 0.06), FRAME)
        p.box((-w / 2 + 0.05, -d / 2 + 0.04, z + 0.06), (w / 2 - 0.05, d / 2 - 0.04, z + 0.18), MATTRESS)
        p.box((-0.45, -d / 2 + 0.03, z + 0.17), (w / 2 - 0.04, d / 2 - 0.03, z + 0.22), ARMY_BLANKET)
        p.box((-w / 2 + 0.09, -0.3, z + 0.18), (-w / 2 + 0.45, 0.3, z + 0.27), PILLOW)
    # A rail along the top bunk's side, and the ladder at its foot.
    p.box((-w / 2 + 0.06, d / 2 - 0.03, 1.56), (0.45, d / 2, 1.60), FRAME)
    for x in (0.55, 0.9):
        p.box((x, d / 2, 0.0), (x + 0.04, d / 2 + 0.04, 1.5), FRAME)
    for k in range(4):
        z = 0.3 + k * 0.3
        p.box((0.55, d / 2, z), (0.94, d / 2 + 0.04, z + 0.04), FRAME)
    p.finish()


def server_rack(piece):
    w, d, h = 0.7, 0.9, 2.0
    p = piece("FURN_ServerRack", [((-w / 2, -d / 2, 0.0), (w / 2, d / 2, h))])
    p.box((-w / 2, -d / 2, 0.05), (w / 2, d / 2, h), CABINET)
    for x in (-w / 2 + 0.03, w / 2 - 0.11):
        for y in (-d / 2 + 0.03, d / 2 - 0.11):
            p.box((x, y, 0.0), (x + 0.08, y + 0.08, 0.05), BLACK)
    # Its machines, one over another, each with a lamp or three lit.
    for k in range(8):
        z = 0.16 + k * 0.225
        p.box((-w / 2 + 0.04, d / 2, z), (w / 2 - 0.04, d / 2 + 0.012, z + 0.19), UNIT[k % 2])
        p.box((-w / 2 + 0.07, d / 2 + 0.012, z + 0.03), (-0.02, d / 2 + 0.018, z + 0.16), BLACK)
        for j in range(3):
            if rng.random() < 0.75:
                lamp = LAMP_GREEN if rng.random() < 0.7 else (LAMP_AMBER if rng.random() < 0.7 else LAMP_RED)
                x = 0.06 + j * 0.08
                p.box((x, d / 2 + 0.012, z + 0.11), (x + 0.035, d / 2 + 0.02, z + 0.145), lamp)
    # A vent in its top.
    p.box((-w / 2 + 0.1, -d / 2 + 0.15, h), (w / 2 - 0.1, d / 2 - 0.15, h + 0.02), BLACK)
    p.finish()


def console(piece):
    w, d = 1.8, 0.9
    p = piece("FURN_Console", [((-w / 2, -d / 2, 0.0), (w / 2, d / 2, 0.78)), ((-w / 2, -d / 2, 0.78), (w / 2, -0.12, 1.32))])
    # The desk: a pedestal at either end, a board behind, its top.
    for x in (-w / 2, w / 2 - 0.45):
        p.box((x, -d / 2, 0.0), (x + 0.45, d / 2 - 0.06, 0.72), DESK)
        for k in range(3):
            p.box((x + 0.05, d / 2 - 0.06, 0.1 + k * 0.2), (x + 0.4, d / 2 - 0.05, 0.26 + k * 0.2), DESK_DARK)
    p.box((-w / 2, -d / 2, 0.0), (w / 2, -d / 2 + 0.04, 0.72), DESK_DARK)
    p.box((-w / 2, -d / 2, 0.72), (w / 2, d / 2, 0.78), DESK_DARK)
    # The panel over it: dials, rows of switches, a speaker, lamps.
    p.box((-w / 2, -d / 2, 0.78), (w / 2, -0.12, 1.32), PANEL)
    face = -0.12
    for k in range(3):
        x = -0.78 + k * 0.3
        p.box((x, face, 1.0), (x + 0.22, face + 0.012, 1.22), DIAL)
        p.box((x + 0.1, face + 0.012, 1.1), (x + 0.12, face + 0.02, 1.2), BLACK)
    for k in range(9):
        x = -0.78 + k * 0.1
        p.box((x, face, 0.86), (x + 0.04, face + 0.03, 0.93), BLACK)
    p.box((0.3, face, 0.86), (0.8, face + 0.012, 1.24), BLACK)
    for k, lamp in enumerate((LAMP_RED, LAMP_AMBER, LAMP_GREEN, LAMP_GREEN)):
        x = 0.12 + (k % 2) * 0.08
        z = 1.08 + (k // 2) * 0.09
        p.box((x, face, z), (x + 0.04, face + 0.02, z + 0.04), lamp)
    # On the desk: a microphone on its stand, the log.
    p.box((-0.2, 0.05, 0.78), (-0.02, 0.21, 0.81), BLACK)
    p.box((-0.125, 0.115, 0.81), (-0.095, 0.145, 1.02), BLACK)
    p.box((-0.15, 0.09, 1.02), (-0.07, 0.2, 1.1), DESK_DARK)
    p.box((0.2, 0.0, 0.78), (0.62, 0.3, 0.795), PAPER)
    p.finish()


def make(piece):
    bunk(piece)
    server_rack(piece)
    console(piece)
