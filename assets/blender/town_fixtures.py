"""The town's landmarks' signs, each its own object standing on its
origin (the middle of its foot), flat against a wall at -Y, its face to
+Y; its letters raised off a board in a blocky hand, each a 5 × 7 grid of
squares:

    SITE_SignGuns     GUNS, red on black
    SITE_SignPolice   POLICE, white on navy
    SITE_SignFire     FIRE, white on red
    SITE_SignSchool   SCHOOL, cream on green

and what the game bumps into for each (SITE_Sign*_Hull). See `sites.py`,
which hands in how it makes a part and a hull.
"""

# Each letter's rows, top first: a # is a square of it.
FONT = {
    "C": [".###.", "#...#", "#....", "#....", "#....", "#...#", ".###."],
    "E": ["#####", "#....", "#....", "####.", "#....", "#....", "#####"],
    "F": ["#####", "#....", "#....", "####.", "#....", "#....", "#...."],
    "G": [".###.", "#...#", "#....", "#.###", "#...#", "#...#", ".###."],
    "H": ["#...#", "#...#", "#...#", "#####", "#...#", "#...#", "#...#"],
    "I": ["#####", "..#..", "..#..", "..#..", "..#..", "..#..", "#####"],
    "L": ["#....", "#....", "#....", "#....", "#....", "#....", "#####"],
    "N": ["#...#", "##..#", "#.#.#", "#.#.#", "#..##", "#...#", "#...#"],
    "O": [".###.", "#...#", "#...#", "#...#", "#...#", "#...#", ".###."],
    "P": ["####.", "#...#", "#...#", "####.", "#....", "#....", "#...."],
    "R": ["####.", "#...#", "#...#", "####.", "#.#..", "#..#.", "#...#"],
    "S": [".####", "#....", "#....", ".###.", "....#", "....#", "####."],
    "U": ["#...#", "#...#", "#...#", "#...#", "#...#", "#...#", ".###."],
}

# A square of a letter, how far apart letters stand (in squares), and the
# board's margin round the words.
PIXEL = 0.09
SPACING = 1
MARGIN = 0.14
BOARD_DEPTH = 0.08
RAISED = 0.03

NAVY = (0.10, 0.14, 0.32)
SIGN_RED = (0.66, 0.10, 0.08)
BLACK = (0.06, 0.06, 0.07)
GREEN = (0.14, 0.30, 0.18)
CREAM = (0.86, 0.80, 0.62)
WHITE = (0.80, 0.79, 0.75)
FRAME = (0.30, 0.30, 0.31)


def sign(Part, hull, name, words, board, ink):
    """A sign reading `words`, `ink` on a `board`, framed."""
    p = Part(name)
    cols = sum(len(FONT[c][0]) for c in words) + SPACING * (len(words) - 1)
    w = cols * PIXEL + 2 * MARGIN
    h = 7 * PIXEL + 2 * MARGIN
    p.box((-w / 2, -BOARD_DEPTH, 0.0), (w / 2, 0.0, h), board)
    # The frame round its edge.
    for lo, hi in (((-w / 2 - 0.04, -BOARD_DEPTH, -0.04), (w / 2 + 0.04, 0.01, 0.0)), ((-w / 2 - 0.04, -BOARD_DEPTH, h), (w / 2 + 0.04, 0.01, h + 0.04)), ((-w / 2 - 0.04, -BOARD_DEPTH, 0.0), (-w / 2, 0.01, h)), ((w / 2, -BOARD_DEPTH, 0.0), (w / 2 + 0.04, 0.01, h))):
        p.box(lo, hi, FRAME)
    # The letters: each run of squares along a row, one box.
    x = -w / 2 + MARGIN
    for c in words:
        rows = FONT[c]
        for r, row in enumerate(rows):
            z1 = h - MARGIN - r * PIXEL
            k = 0
            while k < len(row):
                if row[k] != "#":
                    k += 1
                    continue
                end = k
                while end < len(row) and row[end] == "#":
                    end += 1
                p.box((x + k * PIXEL, 0.0, z1 - PIXEL), (x + end * PIXEL, RAISED, z1), ink)
                k = end
        x += (len(rows[0]) + SPACING) * PIXEL
    p.finish()
    hull(name, [((-w / 2 - 0.04, -BOARD_DEPTH, -0.04), (w / 2 + 0.04, RAISED, h + 0.04))])


def signs(Part, hull):
    sign(Part, hull, "SITE_SignGuns", "GUNS", BLACK, SIGN_RED)
    sign(Part, hull, "SITE_SignPolice", "POLICE", NAVY, WHITE)
    sign(Part, hull, "SITE_SignFire", "FIRE", SIGN_RED, WHITE)
    sign(Part, hull, "SITE_SignSchool", "SCHOOL", GREEN, CREAM)
