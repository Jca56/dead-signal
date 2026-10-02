//! A survivor with the radio out, as the others see it: the handset up in
//! the right hand before the chest, brought to the mouth when it's keyed,
//! down at the hip as it's pulled out and put away; and, a drop called
//! for, a lit flare up in the left hand, wound back and flung. Where each
//! is, the arms reaching for them.

use lntrn_math::{Mat4, Quat, Vec3};

use super::rig::{Bone, Pose, Rig, Side};
use crate::radio::{Shown, THROW};

/// Where the handset's held, from the chest (the figure's space: +X
/// right, +Y up, -Z ahead): up to be read, at the mouth (from the eyes),
/// and down at the hip (from the hips).
const UP: Vec3 = Vec3::new(0.16, 0.02, -0.30);
const MOUTH: Vec3 = Vec3::new(0.07, -0.11, -0.13);
const HIP: Vec3 = Vec3::new(0.24, -0.02, -0.08);
/// Where the wrists are on the handset and on the flare (each its own
/// space), and which way each elbow bends.
const WRIST: Vec3 = Vec3::new(0.03, -0.07, 0.03);
const FLARE_WRIST: Vec3 = Vec3::new(-0.03, 0.0, 0.02);
const RIGHT_POLE: Vec3 = Vec3::new(0.8, -1.0, 0.25);
const LEFT_POLE: Vec3 = Vec3::new(-0.6, -1.0, 0.1);
/// Where the flare's held, from the chest: up at the left; wound back;
/// flung; and the hand followed through. And how far into the throw it's
/// let go (as the handset's own clip has it).
const LIT: Vec3 = Vec3::new(-0.26, 0.10, -0.28);
const WOUND: Vec3 = Vec3::new(-0.30, 0.34, 0.10);
const FLUNG: Vec3 = Vec3::new(-0.16, 0.30, -0.52);
const SPENT: Vec3 = Vec3::new(-0.22, -0.25, -0.20);
const LET_GO: f64 = 0.2;

/// Smoothly from 0 to 1.
fn ease(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Where the left hand is `t` seconds into a throw, from the chest.
fn thrown(t: f64) -> Vec3 {
    let s = t / THROW;
    if s < 0.25 {
        LIT.lerp(WOUND, ease(s / 0.25))
    } else if s < 0.47 {
        WOUND.lerp(FLUNG, ease((s - 0.25) / 0.22))
    } else {
        FLUNG.lerp(SPENT, ease((s - 0.47) / 0.53))
    }
}

/// Pose the arms about the radio as `shown` has it. Where the handset is,
/// and where the lit flare is if there's one in hand (the figure's space).
pub fn hold(rig: &Rig, pose: &mut Pose, shown: Shown) -> (Mat4, Option<Mat4>) {
    let world = rig.world(pose);
    let chest = world[rig.node(Bone::Chest)].translation();
    let hips = world[rig.node(Bone::Hips)].translation();
    let eyes = rig.eyes(&world);
    // The handset: up, or at the mouth through the middle of its keying;
    // out of view, down at the hip.
    let keyed = match (shown.clip, shown.t) {
        ("Key", Some(t)) => ease(t / 0.2) * ease((1.0 - t) / 0.17),
        _ => 0.0,
    };
    let at = (chest + UP).lerp(eyes + MOUTH, keyed).lerp(hips + HIP, ease(shown.stowed));
    // Its face back at them, tipped a little; at the mouth, tipped more.
    let turn = Quat::from_rotation_y(-0.25) * Quat::from_rotation_x(0.25 + 0.5 * keyed - 1.1 * ease(shown.stowed));
    let handset = Mat4::from_translation(at) * Mat4::from_quat(turn);
    rig.reach(pose, Side::Right, handset.transform_point(WRIST), RIGHT_POLE);
    // The flare: coming up, held, or in the throw till it's let go.
    let left = match (shown.clip, shown.t) {
        ("FlareUp", Some(t)) => Some((SPENT.lerp(LIT, ease(t / 0.27)), true)),
        ("Flare", _) => Some((LIT, true)),
        ("Throw", Some(t)) => Some((thrown(t), t < LET_GO)),
        _ => None,
    };
    let flare = left.and_then(|(from_chest, in_hand)| {
        // Upright in the fist, its tip up (its stick's along its own -Z).
        let held = Mat4::from_translation(chest + from_chest) * Mat4::from_quat(Quat::from_rotation_x(std::f64::consts::FRAC_PI_2));
        rig.reach(pose, Side::Left, held.transform_point(FLARE_WRIST), LEFT_POLE);
        in_hand.then_some(held)
    });
    (handset, flare)
}
