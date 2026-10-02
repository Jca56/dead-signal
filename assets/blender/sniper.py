"""The sniper rifle, built into the arms' mesh: a heavy fluted barrel
free of a dark synthetic stock (a raised comb, a thick pad), a bipod
folded back under the fore end, a brake on the muzzle, and a long 6×
scope on two rings over the action (its bell, its turrets, the lens).
Its bolt, its round and its clips are every bolt-action's (`rifle.py`).
"""

import math

from mathutils import Vector

import rifle
from gun_kit import box, flash, frame_matrix, gun_frame

STOCK = (0.16, 0.19, 0.15)
STOCK_DARK = (0.10, 0.12, 0.10)
BLUED = (0.11, 0.12, 0.13)
BLUED_DARK = (0.07, 0.07, 0.08)
STEEL = (0.34, 0.34, 0.35)
PAD = (0.05, 0.05, 0.05)
SCOPE = (0.08, 0.08, 0.09)
LENS = (0.22, 0.42, 0.52)

MUZZLE = Vector((0.0, 1.06, 0.045))
SCOPE_LINE = 0.112

D = rifle.Design(forend=0.36, sight_line=SCOPE_LINE, hip=Vector((0.115, 0.03, -0.16)), aim_distance=0.17, kick=11.0, aim_kick=6.0)


def build(b, wrist, fwd, back, across, hand, left=None):
    """Add the rifle to builder `b` in the right hand, and a round in the
    left (at `left`); the bones, and the gun's frame."""
    origin, right, barrel, up = gun_frame(wrist, fwd, back, across)
    to_rig = frame_matrix(origin, right, barrel, up)
    grip = math.radians(-24)
    # The stock: its grip in the hand, the butt back past the eye, its
    # comb raised for the scope, the pad.
    box(b, to_rig, Vector((0.0, -0.02, -0.012)), (0.036, 0.10, 0.056), STOCK, "gun", grip)
    box(b, to_rig, Vector((0.0, -0.22, -0.03)), (0.044, 0.30, 0.10), STOCK, "gun")
    box(b, to_rig, Vector((0.0, -0.20, 0.03)), (0.034, 0.16, 0.022), STOCK_DARK, "gun")
    box(b, to_rig, Vector((0.0, -0.38, -0.03)), (0.046, 0.026, 0.11), PAD, "gun")
    # The receiver, the trigger and its guard, the floorplate.
    box(b, to_rig, Vector((0.0, 0.10, 0.035)), (0.038, 0.22, 0.045), BLUED, "gun")
    box(b, to_rig, Vector((0.0, 0.035, -0.012)), (0.012, 0.07, 0.01), BLUED, "gun")
    box(b, to_rig, Vector((0.0, 0.03, 0.0)), (0.005, 0.008, 0.02), BLUED_DARK, "gun")
    box(b, to_rig, Vector((0.0, 0.10, 0.004)), (0.032, 0.09, 0.016), BLUED_DARK, "gun")
    # The fore end, the heavy barrel free of it and its flutes, the brake.
    box(b, to_rig, Vector((0.0, D.forend - 0.01, 0.016)), (0.05, 0.36, 0.044), STOCK, "gun")
    box(b, to_rig, Vector((0.0, 0.63, 0.045)), (0.026, 0.82, 0.026), BLUED, "gun")
    for y in (0.62, 0.76, 0.90):
        box(b, to_rig, Vector((0.0, y, 0.0585)), (0.008, 0.10, 0.003), BLUED_DARK, "gun")
    box(b, to_rig, Vector((0.0, 1.03, 0.045)), (0.036, 0.06, 0.03), STEEL, "gun")
    # The bipod, folded back under the fore end.
    for side in (-1.0, 1.0):
        box(b, to_rig, Vector((side * 0.02, 0.43, -0.012)), (0.008, 0.24, 0.008), STEEL, "gun")
    box(b, to_rig, Vector((0.0, 0.545, -0.006)), (0.052, 0.016, 0.02), STEEL, "gun")
    # The scope on its rings: the long tube, the bell at its front, the
    # eyepiece at its back, a turret on top and one at the side, the lens.
    for y in (0.03, 0.19):
        box(b, to_rig, Vector((0.0, y, 0.086)), (0.032, 0.02, 0.05), BLUED_DARK, "gun")
    box(b, to_rig, Vector((0.0, 0.12, SCOPE_LINE)), (0.032, 0.38, 0.032), SCOPE, "gun")
    box(b, to_rig, Vector((0.0, 0.34, SCOPE_LINE)), (0.054, 0.09, 0.054), SCOPE, "gun")
    box(b, to_rig, Vector((0.0, -0.085, SCOPE_LINE)), (0.042, 0.06, 0.042), SCOPE, "gun")
    box(b, to_rig, Vector((0.0, 0.11, SCOPE_LINE + 0.024)), (0.02, 0.02, 0.018), BLUED_DARK, "gun")
    box(b, to_rig, Vector((0.024, 0.11, SCOPE_LINE)), (0.018, 0.02, 0.02), BLUED_DARK, "gun")
    box(b, to_rig, Vector((0.0, 0.386, SCOPE_LINE)), (0.046, 0.004, 0.046), LENS, "gun")
    flash(b, to_rig, "flash", MUZZLE, 1.7)
    bones, barrel = rifle.action(b, to_rig, left)
    bones["gun"] = (origin, origin + barrel * 0.08, hand)
    bones["flash"] = (to_rig @ MUZZLE, to_rig @ MUZZLE + barrel * 0.04, "gun")
    return bones, (origin, right, barrel, up)


def animate(rig, gun_rest, left_rest):
    return rifle.animate(rig, gun_rest, left_rest, D)
