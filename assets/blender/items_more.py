"""More guns that can be carried, and their rounds (ITEM_Pistol45,
ITEM_Rounds45, ITEM_Magnum, ITEM_Rounds44, ITEM_Uzi, ITEM_Ak,
ITEM_RoundsAk, ITEM_Bullpup, ITEM_Rpk), for `items.py` to write with the
rest: each gun lying on its side, muzzle along +X, its underside towards
the front.
"""

from item_kit import *  # noqa: F403

GUNMETAL = (0.40, 0.40, 0.42)
STAINLESS = (0.56, 0.57, 0.58)
STAINLESS_DARK = (0.40, 0.41, 0.43)
PARKED = (0.15, 0.16, 0.16)
COVER = (0.15, 0.15, 0.16)
WALNUT_DARK = (0.30, 0.15, 0.06)
BAKELITE = (0.44, 0.17, 0.07)
OLIVE = (0.27, 0.30, 0.22)
OLIVE_DARK = (0.19, 0.21, 0.15)
SMOKE = (0.34, 0.33, 0.28)
RED_DOT = (0.85, 0.12, 0.08)
LACQUER = (0.30, 0.33, 0.25)
BOX_RED = (0.42, 0.10, 0.08)


def pistol_45():
    """A .45 lying on its side, barrel along X, its walnut grip to the
    front."""
    p = Part("ITEM_Pistol45")
    p.box((0.03, 0.0, 0.017), (0.24, 0.042, 0.03), GUNMETAL)
    p.box((0.04, 0.03, 0.014), (0.17, 0.024, 0.024), BLACK)
    p.box((-0.07, 0.09, 0.015), (0.05, 0.125, 0.03), WALNUT, turn=-0.26)
    p.box((-0.083, 0.155, 0.015), (0.056, 0.012, 0.032), BLACK, turn=-0.26)
    # The trigger guard and the trigger, the hammer's spur, the sights.
    p.box((0.005, 0.07, 0.014), (0.06, 0.008, 0.018), BLACK)
    p.box((0.032, 0.055, 0.014), (0.008, 0.03, 0.018), BLACK)
    p.box((-0.005, 0.052, 0.014), (0.006, 0.02, 0.01), SILVER)
    p.box((-0.098, -0.004, 0.017), (0.012, 0.016, 0.012), BLACK)
    p.box((-0.085, -0.026, 0.017), (0.01, 0.01, 0.024), BLACK)
    p.box((0.135, -0.026, 0.017), (0.008, 0.008, 0.014), BEAD)
    for k in range(5):
        p.box((-0.07 + k * 0.012, 0.0, 0.033), (0.004, 0.038, 0.004), BLACK)
    p.finish()


def rounds_45():
    """A small carton, its lid off, stubby rounds standing in two rows:
    brass, round lead noses."""
    p = Part("ITEM_Rounds45")
    w, d, h = 0.12, 0.066, 0.045
    p.box((0, 0, h / 2), (w, d, h), CARTON_DARK)
    p.box((0, 0, h * 0.5), (w + 0.004, d + 0.004, 0.018), LABEL)
    for row in (-0.015, 0.015):
        for i in range(5):
            x = -0.044 + i * 0.022
            p.box((x, row, h + 0.008), (0.015, 0.015, 0.016), BRASS)
            p.box((x, row, h + 0.021), (0.012, 0.012, 0.01), LEAD)
    p.finish()


def magnum():
    """A .44 revolver lying on its side, its long barrel along X, its
    walnut grip to the front."""
    p = Part("ITEM_Magnum")
    z = 0.022
    # The barrel, the rib over it, the lug under it; the frame and the
    # cylinder in it.
    p.box((0.165, -0.004, z), (0.17, 0.022, 0.022), STAINLESS)
    p.box((0.165, -0.018, z), (0.17, 0.006, 0.008), STAINLESS_DARK)
    p.box((0.155, 0.014, z), (0.14, 0.014, 0.016), STAINLESS)
    p.box((0.02, 0.0, z), (0.13, 0.06, 0.026), STAINLESS)
    p.box((0.03, 0.0, z), (0.052, 0.046, 0.044), STAINLESS_DARK)
    for y in (-0.014, 0.014):
        p.box((0.03, y, z), (0.036, 0.006, 0.046), BLACK)
    # The grip, the hammer, the trigger in its guard, the front sight.
    p.box((-0.07, 0.08, z), (0.046, 0.11, 0.03), WALNUT, turn=-0.4)
    p.box((-0.05, -0.032, z), (0.014, 0.02, 0.01), STAINLESS_DARK)
    p.box((0.01, 0.046, z), (0.06, 0.008, 0.016), STAINLESS)
    p.box((0.038, 0.036, z), (0.008, 0.024, 0.016), STAINLESS)
    p.box((0.008, 0.034, z), (0.006, 0.016, 0.01), STAINLESS_DARK)
    p.box((0.24, -0.022, z), (0.014, 0.012, 0.006), BEAD)
    p.finish()


def rounds_44():
    """A box, its lid off, six big rounds standing in it: long brass,
    flat lead noses."""
    p = Part("ITEM_Rounds44")
    w, d, h = 0.10, 0.07, 0.05
    p.box((0, 0, h / 2), (w, d, h), BOX_RED)
    p.box((0, 0, h * 0.5), (w + 0.004, d + 0.004, 0.02), LABEL)
    for row in (-0.016, 0.016):
        for i in range(3):
            x = -0.028 + i * 0.028
            p.box((x, row, h + 0.014), (0.017, 0.017, 0.028), BRASS)
            p.box((x, row, h + 0.033), (0.014, 0.014, 0.01), LEAD)
    p.finish()


def uzi():
    """A Mini Uzi lying on its side, muzzle along +X, its grip (in the
    middle of it) and the magazine through it towards the front."""
    p = Part("ITEM_Uzi")
    z = 0.024
    p.box((0.03, 0.0, z), (0.25, 0.056, 0.044), PARKED)
    for x in (-0.05, 0.0, 0.05, 0.10):
        p.box((x, 0.0, z), (0.012, 0.03, 0.047), BLACK)
    p.box((0.158, 0.0, z), (0.02, 0.034, 0.034), SILVER)
    p.box((0.185, 0.0, z), (0.06, 0.02, 0.02), BLACK)
    # The grip and the magazine out of the bottom of it, the guard.
    p.box((0.0, 0.075, z), (0.05, 0.10, 0.034), BLACK, turn=-0.14)
    p.box((-0.014, 0.17, z), (0.034, 0.10, 0.024), BLACK, turn=-0.14)
    p.box((-0.021, 0.221, z), (0.04, 0.008, 0.028), SILVER, turn=-0.14)
    p.box((0.05, 0.05, z), (0.05, 0.008, 0.016), BLACK)
    p.box((0.072, 0.04, z), (0.008, 0.024, 0.016), BLACK)
    # The folded stock, the knob on top, the sights.
    p.box((-0.105, 0.0, z), (0.016, 0.07, 0.03), SILVER)
    p.box((-0.06, 0.036, z), (0.10, 0.006, 0.006), SILVER)
    p.box((0.03, -0.032, z), (0.03, 0.01, 0.016), SILVER)
    p.box((-0.07, -0.036, z), (0.012, 0.02, 0.03), BLACK)
    p.box((0.13, -0.036, z), (0.012, 0.02, 0.028), BLACK)
    p.box((0.13, -0.048, z), (0.006, 0.006, 0.006), BEAD)
    p.finish()


def ak_body(p, z, barrel_to):
    """What an AK and an RPK share: the receiver and its cover, the wrist,
    the handguard and the gas tube, the barrel out to `barrel_to`, the
    grip, the guard, the sights."""
    p.box((-0.2, -0.004, z), (0.10, 0.04, 0.04), WALNUT)
    p.box((0.0, 0.0, z), (0.30, 0.056, 0.046), BLACK)
    p.box((-0.01, -0.03, z), (0.22, 0.012, 0.04), COVER)
    p.box((0.24, 0.008, z), (0.19, 0.044, 0.05), WALNUT)
    p.box((0.24, -0.026, z), (0.17, 0.022, 0.03), WALNUT_DARK)
    p.box((0.35, -0.02, z), (0.03, 0.034, 0.024), BLUED)
    length = barrel_to - 0.335
    p.box((0.335 + length / 2, -0.002, z), (length, 0.018, 0.018), BLACK)
    p.box((-0.07, 0.07, z), (0.046, 0.10, 0.03), WALNUT_DARK, turn=-0.3)
    p.box((-0.01, 0.05, z), (0.07, 0.008, 0.016), BLACK)
    p.box((0.12, -0.04, z), (0.05, 0.012, 0.022), BLACK)
    p.box((barrel_to - 0.06, -0.03, z), (0.02, 0.04, 0.02), BLACK)
    p.box((0.02, -0.01, z + 0.026), (0.02, 0.02, 0.012), SILVER)


def ak():
    """An AK-47 lying on its side, muzzle along +X, its grip and its
    curved bakelite magazine towards the front."""
    p = Part("ITEM_Ak")
    z = 0.026
    ak_body(p, z, 0.62)
    p.box((0.625, -0.002, z), (0.036, 0.024, 0.024), SILVER)
    # The stock and its plate, the magazine in three canted pieces.
    p.box((-0.33, 0.012, z), (0.20, 0.085, 0.044), WALNUT, turn=0.06)
    p.box((-0.435, 0.018, z), (0.012, 0.095, 0.046), BLUED, turn=0.06)
    p.box((0.045, 0.085, z), (0.058, 0.10, 0.026), BAKELITE, turn=0.2)
    p.box((0.075, 0.165, z), (0.056, 0.085, 0.025), BAKELITE, turn=0.48)
    p.box((0.125, 0.225, z), (0.054, 0.07, 0.024), BAKELITE, turn=0.76)
    p.finish()


def rounds_ak():
    """A small paper-wrapped box, its top torn off, rounds standing in two
    rows: lacquered steel, copper tips."""
    p = Part("ITEM_RoundsAk")
    w, d, h = 0.13, 0.065, 0.052
    p.box((0, 0, h / 2), (w, d, h), CARTON)
    p.box((0, 0, h * 0.42), (w + 0.004, d + 0.004, 0.012), BLACK)
    for row in (-0.015, 0.015):
        for i in range(6):
            x = -0.05 + i * 0.02
            p.box((x, row, h + 0.016), (0.011, 0.011, 0.032), LACQUER)
            p.box((x, row, h + 0.038), (0.007, 0.007, 0.014), COPPER)
    p.finish()


def bullpup():
    """A bullpup rifle lying on its side, muzzle along +X: one olive body,
    its grip, its magazine (behind the grip) and its folding foregrip
    towards the front, the sight away from the eye."""
    p = Part("ITEM_Bullpup")
    z = 0.028
    p.box((-0.11, 0.0, z), (0.44, 0.074, 0.05), OLIVE)
    p.box((-0.335, 0.004, z), (0.016, 0.10, 0.052), BLACK)
    p.box((0.15, 0.002, z), (0.10, 0.06, 0.046), OLIVE)
    p.box((0.31, -0.008, z), (0.26, 0.02, 0.02), BLACK)
    p.box((0.445, -0.008, z), (0.05, 0.026, 0.026), SILVER)
    # The grip in its big guard, the foregrip, the magazine.
    p.box((-0.01, 0.075, z), (0.046, 0.10, 0.03), OLIVE_DARK, turn=-0.24)
    p.box((0.03, 0.105, z), (0.085, 0.01, 0.014), OLIVE_DARK)
    p.box((0.066, 0.07, z), (0.012, 0.075, 0.014), OLIVE_DARK)
    p.box((0.20, 0.065, z), (0.03, 0.085, 0.028), BLACK)
    p.box((-0.155, 0.085, z), (0.052, 0.12, 0.026), SMOKE, turn=0.1)
    # The sight: its rail on two posts, the frame on it, the dot.
    for x in (0.06, 0.19):
        p.box((x, -0.045, z), (0.022, 0.018, 0.02), OLIVE_DARK)
    p.box((0.125, -0.056, z), (0.17, 0.006, 0.026), BLACK)
    p.box((0.13, -0.08, z), (0.036, 0.046, 0.04), BLACK)
    p.box((0.149, -0.08, z), (0.002, 0.02, 0.02), RED_DOT)
    p.finish()


def rpk():
    """An RPK lying on its side, muzzle along +X: the AK's body, a longer
    barrel with its bipod folded back, a club-footed stock, and the drum
    towards the front."""
    p = Part("ITEM_Rpk")
    z = 0.034
    ak_body(p, z, 0.78)
    p.box((0.50, -0.002, z), (0.20, 0.024, 0.024), BLACK)
    p.box((0.775, -0.002, z), (0.03, 0.024, 0.024), SILVER)
    for dz in (-0.014, 0.014):
        p.box((0.58, 0.02, z + dz), (0.26, 0.007, 0.007), SILVER)
    # The stock: the butt, its club foot, the plate.
    p.box((-0.31, 0.006, z), (0.14, 0.07, 0.044), WALNUT)
    p.box((-0.36, 0.045, z), (0.07, 0.07, 0.044), WALNUT)
    p.box((-0.40, 0.025, z), (0.012, 0.115, 0.046), BLUED)
    # The drum on its neck.
    p.box((0.06, 0.05, z), (0.05, 0.05, 0.026), BLACK)
    p.box((0.075, 0.125, z), (0.13, 0.13, 0.062), COVER)
    p.box((0.075, 0.125, z), (0.094, 0.094, 0.064), BLACK, turn=0.785)
    p.box((0.075, 0.125, z), (0.03, 0.03, 0.068), SILVER)
    p.finish()
