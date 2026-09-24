"""The guns that can be carried, and their rounds (ITEM_Pistol,
ITEM_Shotgun, ITEM_Shells, ITEM_Rifle, ITEM_RifleRounds, ITEM_Smg,
ITEM_AssaultRifle, ITEM_Rounds556), for `items.py` to write with the rest:
each lying on its side, muzzle along +X, its underside towards the front.
"""

from item_kit import *  # noqa: F403

def pistol():
    """A service pistol lying on its side, barrel along X, grip to the
    front (towards the icon's eye, so it reads as a pistol)."""
    p = Part("ITEM_Pistol")
    p.box((0.02, 0.0, 0.017), (0.22, 0.04, 0.03), PLASTIC)
    p.box((0.03, 0.03, 0.014), (0.16, 0.024, 0.024), BLACK)
    p.box((-0.07, 0.09, 0.015), (0.048, 0.12, 0.028), HANDLE, turn=-0.26)
    p.box((-0.082, 0.152, 0.015), (0.056, 0.014, 0.032), BLACK, turn=-0.26)
    # The trigger guard, and the trigger in it.
    p.box((0.005, 0.07, 0.014), (0.06, 0.008, 0.018), BLACK)
    p.box((0.032, 0.055, 0.014), (0.008, 0.03, 0.018), BLACK)
    p.box((-0.005, 0.052, 0.014), (0.006, 0.02, 0.01), SILVER)
    # The sights, and the slide's grip lines.
    p.box((-0.078, -0.024, 0.017), (0.01, 0.01, 0.024), BLACK)
    p.box((0.12, -0.024, 0.017), (0.008, 0.008, 0.014), SILVER)
    for k in range(4):
        p.box((-0.06 + k * 0.012, 0.0, 0.033), (0.004, 0.036, 0.004), BLACK)
    p.finish()


def shotgun():
    """A pump shotgun lying on its side, muzzle along +X, its underside
    (forend, trigger guard) towards the front."""
    p = Part("ITEM_Shotgun")
    z = 0.024
    # (Each box by its middle and its size.) The stock and its pad, the
    # wrist, the receiver.
    p.box((-0.33, 0.02, z), (0.32, 0.07, 0.044), WALNUT, turn=0.12)
    p.box((-0.495, 0.04, z), (0.02, 0.10, 0.046), BLACK, turn=0.12)
    p.box((-0.08, 0.0, z), (0.16, 0.05, 0.04), WALNUT, turn=0.2)
    p.box((0.10, 0.0, z), (0.24, 0.07, 0.048), BLUED)
    p.box((0.52, -0.012, z), (0.60, 0.028, 0.03), BLUED)
    p.box((0.45, 0.028, z), (0.46, 0.026, 0.026), BLUED)
    p.box((0.38, 0.034, z), (0.20, 0.05, 0.05), WALNUT)
    p.box((0.04, 0.045, z), (0.06, 0.02, 0.012), BLUED)
    p.box((0.815, -0.03, z), (0.01, 0.01, 0.01), BEAD)
    p.finish()


def shells():
    """A small carton, its lid off, shells standing in two rows, red hulls
    and brass bases."""
    p = Part("ITEM_Shells")
    w, d, h = 0.16, 0.08, 0.07
    p.box((0, 0, h / 2), (w, d, h), CARTON)
    p.box((0, 0, h * 0.5), (w + 0.004, d + 0.004, 0.026), LABEL)
    for row in (-0.018, 0.018):
        for i in range(5):
            x = -0.06 + i * 0.03
            p.box((x, row, h + 0.012), (0.022, 0.022, 0.024), SHELL_RED)
            p.box((x, row, h + 0.001), (0.024, 0.024, 0.004), BRASS)
    p.finish()


def rifle():
    """A scoped bolt-action rifle lying on its side, muzzle along +X, the
    scope away from the eye (up, in the gun's frame, is -Y here)."""
    p = Part("ITEM_Rifle")
    z = 0.024
    # (Each box by its middle and its size.) The stock, the wrist, the
    # receiver, the forestock, the barrel.
    p.box((-0.40, 0.03, z), (0.36, 0.08, 0.044), WALNUT, turn=0.1)
    p.box((-0.585, 0.045, z), (0.02, 0.11, 0.046), BLACK, turn=0.1)
    p.box((-0.15, 0.01, z), (0.14, 0.05, 0.04), WALNUT, turn=0.18)
    p.box((0.02, 0.0, z), (0.22, 0.05, 0.042), BLUED)
    p.box((0.32, 0.012, z), (0.36, 0.045, 0.044), WALNUT)
    p.box((0.62, -0.012, z), (0.62, 0.022, 0.024), BLUED)
    # The bolt's handle, out and down.
    p.box((-0.03, 0.04, z + 0.01), (0.014, 0.06, 0.012), SILVER)
    # The scope on its rings, its lens at the front.
    for x in (-0.05, 0.08):
        p.box((x, -0.04, z), (0.02, 0.03, 0.03), BLACK)
    p.box((0.02, -0.07, z), (0.34, 0.032, 0.034), SCOPE_BLACK)
    p.box((0.19, -0.07, z), (0.06, 0.046, 0.046), SCOPE_BLACK)
    p.box((0.222, -0.07, z), (0.004, 0.038, 0.038), LENS)
    p.finish()


def rifle_rounds():
    """A small box, lid off, rounds standing in two rows: brass, copper
    tips."""
    p = Part("ITEM_RifleRounds")
    w, d, h = 0.14, 0.07, 0.06
    p.box((0, 0, h / 2), (w, d, h), CARTON_DARK)
    p.box((0, 0, h * 0.5), (w + 0.004, d + 0.004, 0.022), LABEL)
    for row in (-0.016, 0.016):
        for i in range(5):
            x = -0.052 + i * 0.026
            p.box((x, row, h + 0.02), (0.012, 0.012, 0.04), BRASS)
            p.box((x, row, h + 0.046), (0.008, 0.008, 0.014), COPPER)
    p.finish()


TAN = (0.52, 0.46, 0.34)
RED_DOT = (0.85, 0.12, 0.08)


def smg():
    """A compact SMG lying on its side, muzzle along +X, its grip and
    stick magazine towards the front, the wire stock folded along it."""
    p = Part("ITEM_Smg")
    z = 0.022
    p.box((0.02, 0.0, z), (0.26, 0.05, 0.042), BLACK)
    p.box((0.18, 0.004, z), (0.12, 0.056, 0.046), PLASTIC)
    p.box((0.285, -0.008, z), (0.10, 0.028, 0.028), BLACK)
    p.box((-0.07, 0.07, z), (0.046, 0.10, 0.03), PLASTIC, turn=-0.26)
    p.box((0.05, 0.10, z), (0.034, 0.16, 0.024), BLACK, turn=0.14)
    p.box((0.0, 0.052, z), (0.06, 0.008, 0.016), BLACK)
    # The folded stock, and the sights.
    p.box((-0.02, -0.03, z + 0.012), (0.22, 0.008, 0.008), SILVER)
    p.box((-0.02, -0.04, z - 0.008), (0.22, 0.008, 0.008), SILVER)
    p.box((-0.09, -0.032, z), (0.012, 0.012, 0.03), BLACK)
    p.box((0.22, -0.034, z), (0.008, 0.012, 0.012), BEAD)
    p.finish()


def assault_rifle():
    """An assault rifle lying on its side, muzzle along +X, its grip and
    curved magazine towards the front, the red dot away from the eye."""
    p = Part("ITEM_AssaultRifle")
    z = 0.026
    # The stock, the receiver, the handguard, the barrel, the flash hider.
    p.box((-0.33, 0.01, z), (0.18, 0.09, 0.046), TAN)
    p.box((-0.22, -0.01, z), (0.10, 0.034, 0.03), BLACK)
    p.box((0.0, 0.0, z), (0.32, 0.06, 0.048), BLACK)
    p.box((0.29, 0.0, z), (0.27, 0.058, 0.056), TAN)
    p.box((0.52, -0.004, z), (0.20, 0.022, 0.022), BLACK)
    p.box((0.635, -0.004, z), (0.05, 0.028, 0.028), SILVER)
    # The grip, the curved magazine, the trigger guard.
    p.box((-0.07, 0.07, z), (0.046, 0.10, 0.03), BLACK, turn=-0.3)
    p.box((0.035, 0.09, z), (0.05, 0.12, 0.026), BLACK, turn=0.18)
    p.box((0.065, 0.17, z), (0.048, 0.07, 0.025), BLACK, turn=0.42)
    p.box((-0.01, 0.05, z), (0.07, 0.008, 0.016), BLACK)
    # The red dot on the rail.
    p.box((0.02, -0.05, z), (0.06, 0.04, 0.03), BLACK)
    p.box((0.05, -0.052, z), (0.004, 0.02, 0.02), RED_DOT)
    p.finish()


def rounds_556():
    """A small box, lid off, rounds standing in two rows: slim brass, dark
    tips."""
    p = Part("ITEM_Rounds556")
    w, d, h = 0.13, 0.065, 0.055
    p.box((0, 0, h / 2), (w, d, h), CARTON)
    p.box((0, 0, h * 0.5), (w + 0.004, d + 0.004, 0.02), LABEL)
    for row in (-0.015, 0.015):
        for i in range(6):
            x = -0.05 + i * 0.02
            p.box((x, row, h + 0.018), (0.009, 0.009, 0.036), BRASS)
            p.box((x, row, h + 0.041), (0.006, 0.006, 0.012), LEAD)
    p.finish()
