"""What's thrown (ITEM_Molotov, ITEM_PipeBomb), for `items.py` to write
with the rest: each lying on its side along X. The game throws these same
objects, turning in the air.
"""

from item_kit import *  # noqa: F403

GLASS_GREEN = (0.24, 0.40, 0.22)
GLASS_DARK = (0.16, 0.28, 0.15)
RAG = (0.78, 0.72, 0.58)
RAG_DIRTY = (0.52, 0.44, 0.32)
PIPE = (0.46, 0.46, 0.44)
PIPE_DARK = (0.30, 0.30, 0.29)
TAPE = (0.16, 0.16, 0.16)
WIRE_RED = (0.72, 0.12, 0.08)
LAMP = (0.95, 0.20, 0.10)


def molotov():
    """A bottle of something that burns, a rag stuffed in its neck."""
    p = Part("ITEM_Molotov")
    r = 0.04
    p.roll((0.0, 0.0, r), r, 0.16, GLASS_GREEN, GLASS_DARK)
    p.roll((0.105, 0.0, r), r * 0.45, 0.05, GLASS_GREEN, GLASS_DARK)
    p.box((0.0, 0.0, r), (0.08, r * 2.05, r * 2.05), LABEL)
    # The rag, out of the neck and hanging.
    p.box((0.145, 0.0, r), (0.04, 0.03, 0.03), RAG)
    p.box((0.18, 0.012, r - 0.012), (0.05, 0.028, 0.012), RAG_DIRTY, turn=0.4)
    p.finish()


def pipe_bomb():
    """A length of pipe, capped both ends, taped round with a little box
    and its red light, wires over it."""
    p = Part("ITEM_PipeBomb")
    r = 0.028
    p.roll((0.0, 0.0, r), r, 0.16, PIPE, PIPE_DARK)
    for x in (-0.085, 0.085):
        p.roll((x, 0.0, r), r * 1.18, 0.022, PIPE_DARK, PIPE_DARK)
    p.box((0.0, 0.0, r), (0.05, r * 2.1, r * 2.1), TAPE)
    p.box((0.0, 0.0, r * 2.1), (0.04, 0.03, 0.018), BLACK)
    p.box((0.012, -0.008, r * 2.1 + 0.011), (0.008, 0.008, 0.006), LAMP)
    p.box((-0.04, 0.0, r * 2.05), (0.06, 0.004, 0.004), WIRE_RED)
    p.finish()
