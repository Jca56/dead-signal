//! What a survivor's doing with their hands, over what their legs are:
//! a blade or an axe held ready and swung, fists up and punching (a right,
//! then a left), something wound up and thrown overarm (the body's clips);
//! a gun held where they aim (`gun.rs`). Their body leans with the aim,
//! bladed to a long gun at the shoulder.

use lntrn_math::{Mat4, Quat, Vec3};

use super::body::{self, toward};
use super::gun::{self, Aim};
use super::rig::{ARMS, Bone, Clip, Pose, Rig};
use super::{Doing, Motion};
use crate::loot::Kind;
use crate::throw::Throwable;
use crate::weapon::{self, Weapon};

/// How far the body turns to a long gun at the shoulder (radians), and how
/// much of the aim's up and down each of spine, chest, neck and head take
/// (sat on the ground, the spine none of it).
const BLADED: f64 = 0.38;
const LEAN: [f64; 4] = [0.1, 0.2, 0.2, 0.3];
const LEAN_DOWN: [f64; 4] = [0.0, 0.25, 0.25, 0.3];
/// How long into a throw it's let go of (s).
const LET_GO: f64 = 0.13;

/// What's in hand this frame, in the figure's space: what, and where; a
/// shot's flash, where.
#[derive(Clone, Copy, Debug, Default)]
pub struct Holding {
    pub item: Option<(Kind, Mat4)>,
    pub flash: Option<Mat4>,
}

/// A blade's or an axe's grip (its own space: the blade along +X, its edge
/// -Z).
fn blade(weapon: Weapon) -> Option<(Kind, Vec3)> {
    match weapon {
        Weapon::Knife => Some((Kind::Knife, Vec3::new(-0.08, 0.012, 0.0))),
        Weapon::Machete => Some((Kind::Machete, Vec3::new(-0.24, 0.012, 0.0))),
        Weapon::Axe => Some((Kind::FireAxe, Vec3::new(-0.30, 0.02, 0.0))),
        _ => None,
    }
}

/// Where something thrown is held (its own space).
fn thrown_grip(what: Throwable) -> Vec3 {
    match what {
        Throwable::Molotov => Vec3::new(0.0, 0.04, 0.0),
        Throwable::PipeBomb => Vec3::new(0.0, 0.028, 0.0),
    }
}

/// The body's clip over the legs' for what's in hand: which, how far in
/// (s), how much of it.
fn over(rig: &Rig, m: &mut Motion, d: &Doing, dt: f64) -> Option<(Clip, f64, f64)> {
    if d.winding.is_some() {
        m.wind += dt;
        return Some((Clip::Windup, m.wind, 1.0));
    }
    m.wind = 0.0;
    if let Some((_, since)) = d.threw
        && since < rig.duration(Clip::Throw)
    {
        return Some((Clip::Throw, since, 1.0));
    }
    if d.down || d.out || d.reviving {
        return None;
    }
    let hands = d.hands;
    let (clip, t) = hands.clip();
    let weapon = hands.weapon;
    let hold = match weapon {
        Weapon::Knife | Weapon::Machete => Clip::HoldBlade,
        Weapon::Axe => Clip::HoldAxe,
        Weapon::Fists => Clip::Guard,
        _ => return None,
    };
    if !matches!(clip, weapon::Clip::Bash | weapon::Clip::Bash2) {
        m.blow = None;
        return Some((hold, m.time, (1.0 - hands.stowed_amount()) * (1.0 - 0.7 * m.sprint)));
    }
    // A fresh blow: fists go a right, then a left.
    if m.blow.is_none_or(|was| t < was) {
        m.jab = !m.jab;
    }
    m.blow = Some(t);
    let swing = match (weapon, clip) {
        (Weapon::Knife, _) => Clip::Stab,
        (Weapon::Machete, weapon::Clip::Bash2) => Clip::Swing2,
        (Weapon::Machete, _) => Clip::Swing,
        (Weapon::Axe, _) => Clip::Chop,
        _ if m.jab => Clip::Jab,
        _ => Clip::Cross,
    };
    let time = hands.spec().bash.time.max(1e-6);
    Some((swing, t / time * rig.duration(swing), 1.0))
}

/// Turn the spine, chest, neck and head (`shares` of `pitch`) to aim up or
/// down; bladed, the chest turned `blade` to the right, the head kept
/// ahead.
fn lean(rig: &Rig, pose: &mut Pose, pitch: f64, shares: [f64; 4], blade: f64) {
    for (bone, share) in [Bone::Spine, Bone::Chest, Bone::Neck, Bone::Head].into_iter().zip(shares) {
        if share > 0.0 {
            let world = rig.world(pose);
            rig.turn(pose, &world, bone, Quat::from_rotation_x(pitch * share));
        }
    }
    if blade > 0.0 {
        let world = rig.world(pose);
        rig.turn(pose, &world, Bone::Chest, Quat::from_rotation_y(-blade));
        let world = rig.world(pose);
        rig.turn(pose, &world, Bone::Neck, Quat::from_rotation_y(blade));
    }
}

/// Pose what the hands do over the legs' `pose`, aiming `yaw` off the way
/// the body faces and `pitch` up, sprinting or not, `dt` on. What's held.
pub fn hands(rig: &Rig, pose: &mut Pose, m: &mut Motion, d: &Doing, (yaw, pitch): (f64, f64), sprinting: bool, dt: f64) -> Holding {
    let hands = d.hands;
    let (clip, _) = hands.clip();
    let carry = sprinting && !d.down && clip == weapon::Clip::Idle && hands.aim() < 0.05;
    m.sprint = toward(m.sprint, if carry { 1.0 } else { 0.0 }, 8.0, dt);
    if let Some((clip, t, w)) = over(rig, m, d, dt) {
        let mut by = pose.clone();
        rig.sample(clip, t, &mut by);
        rig.mix(pose, &by, w, &ARMS);
        rig.mix(pose, &by, w * 0.5, &[Bone::Spine]);
    }
    body::turn_legs(rig, pose, m);
    if d.out || d.reviving {
        return Holding::default();
    }
    let gun = gun::of(hands.weapon);
    let throwing = d.winding.is_some() || d.threw.is_some_and(|(_, since)| since < rig.duration(Clip::Throw));
    let bladed = match gun {
        Some(g) if g.long() && !d.down && !throwing => BLADED * (0.85 + 0.15 * hands.aim()) * (1.0 - m.sprint),
        _ => 0.0,
    };
    let pitch = pitch.clamp(-1.2, 1.2);
    lean(rig, pose, pitch, if d.down { LEAN_DOWN } else { LEAN }, bladed);
    // Winding up, or just let go: what's thrown, in hand.
    if throwing {
        let what = d.winding.or(d.threw.filter(|&(_, since)| since < LET_GO).map(|(what, _)| what));
        let item = what.map(|what| (what.kind(), rig.in_hand(&rig.world(pose), thrown_grip(what))));
        return Holding { item, flash: None };
    }
    if let Some(g) = gun {
        let (item, flash) = gun::hold(rig, pose, m, g, d, &Aim::new(yaw, pitch));
        return Holding { item: item.map(|at| (gun::kind(g), at)), flash };
    }
    let item = blade(hands.weapon).filter(|_| hands.stowed_amount() < 0.9 && d.lowered < 0.5).map(|(kind, grip)| (kind, rig.in_hand(&rig.world(pose), grip)));
    Holding { item, flash: None }
}
