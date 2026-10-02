//! A gun in a survivor's hands, as the others see it: held where they
//! aim, from the hip or up at the eye (a long gun's stock at the shoulder),
//! both hands on it. It kicks as it fires; its pump or bolt is worked; it's
//! tipped to reload, the other hand to the gun, to the belt and back; it's
//! swung as a club, lowered to be put away, carried across the chest at a
//! sprint. Down, the sidearm's held out in the one hand. The gun's
//! placed first, and the arms reach for it.

use lntrn_math::{Mat3, Mat4, Quat, Vec3};

use super::{Doing, Motion};
use super::rig::{Bone, Pose, Rig, Side};
use crate::loot::Kind;
use crate::weapon::{self, Act, Hands, Reload, Weapon};

/// A gun as it's held. Its own space (as `items.glb` has it): the muzzle
/// along +X, its top +Z.
pub(super) struct Gun {
    kind: Kind,
    /// Where the right hand takes it, and the left; the top the eye looks
    /// along; the muzzle.
    grip: Vec3,
    fore: Vec3,
    sight: Vec3,
    pub muzzle: Vec3,
    /// Where a magazine or a round goes in, and what's racked after it (a
    /// slide, a bolt, a charging handle; a pump is worked at the fore).
    well: Vec3,
    rack: Vec3,
    /// How far ahead of the eye its sight is, up at the eye (metres).
    ads: f64,
    /// A long gun, at the shoulder (else a sidearm, out in both hands).
    long: bool,
}

const fn at(x: f64, y: f64, z: f64) -> Vec3 {
    Vec3::new(x, y, z)
}

const PISTOL: Gun = Gun { kind: Kind::Pistol, grip: at(-0.075, 0.017, -0.075), fore: at(-0.07, 0.055, -0.09), sight: at(-0.07, 0.017, 0.032), muzzle: at(0.13, 0.017, 0.0), well: at(-0.085, 0.017, -0.15), rack: at(-0.06, 0.017, 0.02), ads: 0.36, long: false };
const SHOTGUN: Gun = Gun { kind: Kind::Shotgun, grip: at(-0.10, 0.024, -0.012), fore: at(0.30, 0.024, -0.045), sight: at(0.06, 0.024, 0.04), muzzle: at(0.82, 0.024, 0.012), well: at(0.10, 0.024, -0.04), rack: at(0.30, 0.024, -0.045), ads: 0.20, long: true };
const RIFLE: Gun = Gun { kind: Kind::Rifle, grip: at(-0.16, 0.024, -0.02), fore: at(0.20, 0.024, -0.035), sight: at(0.12, 0.024, 0.04), muzzle: at(0.93, 0.024, 0.012), well: at(0.0, 0.024, 0.035), rack: at(-0.03, 0.034, -0.03), ads: 0.3, long: true };
const SMG: Gun = Gun { kind: Kind::Smg, grip: at(-0.07, 0.022, -0.07), fore: at(0.12, 0.022, -0.035), sight: at(-0.09, 0.022, 0.038), muzzle: at(0.335, 0.022, 0.008), well: at(0.05, 0.022, -0.1), rack: at(-0.08, 0.022, 0.03), ads: 0.24, long: true };
const ASSAULT_RIFLE: Gun = Gun { kind: Kind::AssaultRifle, grip: at(-0.07, 0.026, -0.07), fore: at(0.25, 0.026, -0.04), sight: at(0.0, 0.026, 0.065), muzzle: at(0.66, 0.026, 0.004), well: at(0.05, 0.026, -0.1), rack: at(-0.12, 0.026, 0.04), ads: 0.18, long: true };

const LMG: Gun = Gun { kind: Kind::Lmg, grip: at(-0.07, 0.03, -0.07), fore: at(0.24, 0.03, -0.045), sight: at(0.0, 0.03, 0.07), muzzle: at(0.77, 0.03, 0.0), well: at(0.06, 0.042, -0.11), rack: at(-0.02, 0.03, 0.045), ads: 0.16, long: true };
const FLAMETHROWER: Gun = Gun { kind: Kind::Flamethrower, grip: at(-0.07, 0.045, -0.07), fore: at(0.26, 0.045, -0.075), sight: at(0.0, 0.045, 0.03), muzzle: at(0.755, 0.045, 0.0), well: at(0.09, 0.045, -0.11), rack: at(0.01, 0.045, 0.035), ads: 0.2, long: true };

const PISTOL_45: Gun = Gun { kind: Kind::Pistol45, grip: at(-0.075, 0.017, -0.075), fore: at(-0.07, 0.055, -0.09), sight: at(-0.08, 0.017, 0.034), muzzle: at(0.15, 0.017, 0.0), well: at(-0.085, 0.017, -0.15), rack: at(-0.06, 0.017, 0.02), ads: 0.36, long: false };
const MAGNUM: Gun = Gun { kind: Kind::Magnum, grip: at(-0.07, 0.022, -0.07), fore: at(-0.065, 0.06, -0.085), sight: at(-0.04, 0.022, 0.032), muzzle: at(0.25, 0.022, 0.004), well: at(0.03, 0.022, 0.0), rack: at(-0.05, 0.022, 0.03), ads: 0.36, long: false };
const MINI_UZI: Gun = Gun { kind: Kind::MiniUzi, grip: at(0.0, 0.024, -0.07), fore: at(0.005, 0.062, -0.085), sight: at(-0.07, 0.024, 0.046), muzzle: at(0.215, 0.024, 0.0), well: at(-0.01, 0.024, -0.2), rack: at(0.03, 0.024, 0.036), ads: 0.34, long: false };
const AK47: Gun = Gun { kind: Kind::Ak47, grip: at(-0.07, 0.026, -0.07), fore: at(0.24, 0.026, -0.035), sight: at(0.12, 0.026, 0.05), muzzle: at(0.64, 0.026, 0.002), well: at(0.05, 0.026, -0.1), rack: at(0.02, 0.05, 0.01), ads: 0.22, long: true };
const BULLPUP: Gun = Gun { kind: Kind::Bullpup, grip: at(-0.01, 0.028, -0.07), fore: at(0.20, 0.028, -0.065), sight: at(0.13, 0.028, 0.083), muzzle: at(0.47, 0.028, 0.008), well: at(-0.155, 0.028, -0.10), rack: at(0.02, 0.0, 0.02), ads: 0.24, long: true };
const RPK: Gun = Gun { kind: Kind::Rpk, grip: at(-0.07, 0.034, -0.07), fore: at(0.24, 0.034, -0.04), sight: at(0.12, 0.034, 0.05), muzzle: at(0.80, 0.034, 0.002), well: at(0.075, 0.034, -0.11), rack: at(0.02, 0.058, 0.01), ads: 0.22, long: true };

const SNIPER: Gun = Gun { kind: Kind::Sniper, grip: at(-0.16, 0.028, -0.02), fore: at(0.20, 0.028, -0.04), sight: at(-0.17, 0.028, 0.078), muzzle: at(1.08, 0.028, 0.012), well: at(0.0, 0.028, 0.035), rack: at(-0.03, 0.04, -0.03), ads: 0.1, long: true };

/// The gun `weapon` is, if it's one.
pub(super) fn of(weapon: Weapon) -> Option<&'static Gun> {
    Some(match weapon {
        Weapon::Pistol => &PISTOL,
        Weapon::Shotgun => &SHOTGUN,
        Weapon::Rifle => &RIFLE,
        Weapon::Smg => &SMG,
        Weapon::AssaultRifle => &ASSAULT_RIFLE,
        Weapon::Lmg => &LMG,
        Weapon::Flamethrower => &FLAMETHROWER,
        Weapon::Pistol45 => &PISTOL_45,
        Weapon::Magnum => &MAGNUM,
        Weapon::MiniUzi => &MINI_UZI,
        Weapon::Ak47 => &AK47,
        Weapon::Bullpup => &BULLPUP,
        Weapon::Rpk => &RPK,
        Weapon::Sniper => &SNIPER,
        _ => return None,
    })
}

impl Gun {
    pub(super) fn long(&self) -> bool {
        self.long
    }
}

/// Where each wrist is from what it holds (the gun's space): the right
/// behind and under the grip, the left under the fore and to its side.
const RIGHT_WRIST: Vec3 = at(-0.05, 0.0, -0.035);
const LEFT_WRIST: Vec3 = at(-0.035, 0.03, -0.045);
/// Which way each elbow bends (the figure's space): out and down.
const RIGHT_POLE: Vec3 = at(0.8, -1.0, 0.25);
const LEFT_POLE: Vec3 = at(-0.5, -1.0, 0.1);
/// Where the belt's pouch is, from the hips (the figure's space).
const POUCH: Vec3 = at(-0.14, -0.05, -0.10);
/// How long a shot's flash shows, seconds.
const FLASH: f64 = 0.05;

/// Where they aim, in the figure's space: ahead, up and right.
#[derive(Clone, Copy, Debug)]
pub struct Aim {
    pub f: Vec3,
    pub u: Vec3,
    pub r: Vec3,
}

impl Aim {
    /// Aiming `yaw` off the way the body faces (to the left) and `pitch`
    /// up.
    pub fn new(yaw: f64, pitch: f64) -> Aim {
        let (sy, cy) = yaw.sin_cos();
        let (sp, cp) = pitch.sin_cos();
        let f = Vec3::new(-sy * cp, sp, -cy * cp);
        let r = Vec3::new(cy, 0.0, -sy);
        Aim { f, u: r.cross(f), r }
    }
}

/// A turn taking a gun's muzzle along `ahead` and its top along `up`.
fn facing(ahead: Vec3, up: Vec3) -> Quat {
    let f = ahead.normalize_or(Vec3::NEG_Z);
    let u = (up - f * up.dot(f)).normalize_or(Vec3::Y);
    Quat::from_mat3(&Mat3::from_cols(f, u.cross(f), u))
}

/// Where a gun is: its grip, and its turn.
#[derive(Clone, Copy, Debug)]
struct Place {
    at: Vec3,
    turn: Quat,
}

impl Place {
    fn toward(self, o: Place, w: f64) -> Place {
        let w = w.clamp(0.0, 1.0);
        Place { at: self.at.lerp(o.at, w), turn: self.turn.slerp(o.turn, w) }
    }

    /// Turned by `q` about its grip, and moved `by`.
    fn moved(self, q: Quat, by: Vec3) -> Place {
        Place { at: self.at + by, turn: (q * self.turn).normalize() }
    }

    fn matrix(self, g: &Gun) -> Mat4 {
        Mat4::from_translation(self.at) * Mat4::from_quat(self.turn) * Mat4::from_translation(-g.grip)
    }
}

/// Up at the eye, the sight on the line of the aim.
fn aimed(g: &Gun, eyes: Vec3, a: &Aim) -> Place {
    let turn = facing(a.f, a.u);
    Place { at: eyes + a.f * g.ads + turn * (g.grip - g.sight), turn }
}

/// From the hip: lower, to the right; a sidearm held in close.
fn hip(g: &Gun, up: Place, a: &Aim) -> Place {
    let by = if g.long { a.r * 0.10 - a.u * 0.20 - a.f * 0.12 } else { a.r * 0.06 - a.u * 0.26 - a.f * 0.16 };
    Place { at: up.at + by, turn: up.turn }
}

/// At a sprint: a long gun across the chest, muzzle up to the left; a
/// sidearm down at the side.
fn carried(g: &Gun, chest: Vec3) -> Place {
    if g.long {
        Place { at: chest + at(0.14, -0.12, -0.22), turn: facing(at(-0.7, 0.6, -0.35), at(0.3, 0.2, -1.0)) }
    } else {
        Place { at: chest + at(0.24, -0.34, -0.12), turn: facing(at(0.05, -0.95, -0.3), at(0.0, 0.0, -1.0)) }
    }
}

/// A rise and fall to 1 at `peak`, `width` either side of it.
fn bump(x: f64, peak: f64, width: f64) -> f64 {
    let d = 1.0 - ((x - peak) / width).abs();
    if d <= 0.0 { 0.0 } else { d * d * (3.0 - 2.0 * d) }
}

fn ramp(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// How far `act` in `marks` is being done at `t` (a rise and fall
/// `width` either side of it).
fn working(marks: &[(f64, Act)], act: Act, t: f64, width: f64) -> f64 {
    marks.iter().filter(|(_, a)| *a == act).map(|&(at, _)| bump(t, at, width)).fold(0.0, f64::max)
}

/// Where the left hand's going: a place on the gun, or the belt's pouch.
#[derive(Clone, Copy, Debug)]
enum Spot {
    On(Vec3),
    Pouch,
}

/// A reload's left hand: how far through it is, and its path, share by
/// share (to the gun's well, the pouch and back, the rack). None: on the
/// fore.
fn reloading(g: &Gun, hands: &Hands) -> Option<(f64, Vec<(f64, Spot)>)> {
    let (clip, t) = hands.clip();
    let (fore, well, rack) = (Spot::On(g.fore), Spot::On(g.well), Spot::On(g.rack));
    let keys: Vec<(f64, Spot)> = match (clip, hands.spec().reload?) {
        (weapon::Clip::Reload, Reload::Magazine { time, marks }) => {
            let when = |act: Act, or: f64| marks.iter().find(|(_, a)| *a == act).map_or(or, |&(at, _)| at / time);
            let (out, into, racked) = (when(Act::MagOut, 0.2), when(Act::MagIn, 0.7), when(Act::SlideRack, 0.85));
            let mut keys = vec![(0.0, fore), (0.08, well), (out, well), (out + 0.16, Spot::Pouch), (into - 0.18, Spot::Pouch), (into, well), (into + 0.04, well), (racked - 0.02, rack), (racked + 0.06, rack), (1.0, fore)];
            for k in 1..keys.len() {
                keys[k].0 = keys[k].0.max(keys[k - 1].0);
            }
            keys.into_iter().map(|(s, spot)| (s * time, spot)).collect()
        }
        (weapon::Clip::ReloadStart, Reload::Rounds { start, .. }) => vec![(0.0, fore), (start, well)],
        (weapon::Clip::ReloadShell, Reload::Rounds { each, insert_at, .. }) => vec![(0.0, well), (each * 0.35, Spot::Pouch), (each * 0.45, Spot::Pouch), (insert_at, well), (each, well)],
        (weapon::Clip::ReloadEnd, Reload::Rounds { end, .. }) => vec![(0.0, well), (end * 0.6, fore), (end, fore)],
        _ => return None,
    };
    let len = keys.last().map_or(1.0, |k| k.0).max(1e-6);
    Some(((t / len).clamp(0.0, 1.0), keys.into_iter().map(|(at, spot)| (at / len, spot)).collect()))
}

/// How far a reload's tipped the gun (0–1).
fn tipped(hands: &Hands) -> f64 {
    let (clip, t) = hands.clip();
    match (clip, hands.spec().reload) {
        (weapon::Clip::Reload, Some(Reload::Magazine { time, .. })) => ramp(t / time / 0.12).min(ramp((1.0 - t / time) / 0.12)),
        (weapon::Clip::ReloadStart, Some(Reload::Rounds { start, .. })) => ramp(t / start),
        (weapon::Clip::ReloadShell, _) => 1.0,
        (weapon::Clip::ReloadEnd, Some(Reload::Rounds { end, .. })) => 1.0 - ramp(t / end / 0.7),
        _ => 0.0,
    }
}

/// The marks the clip under way has (its pump or bolt worked).
fn marks(hands: &Hands) -> &'static [(f64, Act)] {
    let spec = hands.spec();
    match (hands.clip().0, spec.reload) {
        (weapon::Clip::Fire, _) => spec.shot.map_or(&[], |s| s.marks),
        (weapon::Clip::ReloadStart, Some(Reload::Rounds { start_marks, .. })) => start_marks,
        (weapon::Clip::ReloadEnd, Some(Reload::Rounds { end_marks, .. })) => end_marks,
        _ => &[],
    }
}

/// Hold `g` as `d`'s hands have it, aiming along `a` (down: in the one
/// hand; lowered, if they're busy), the arms reaching for it. Where it is
/// and, if it's just fired, its flash (the figure's space); nothing if
/// it's put away.
pub(super) fn hold(rig: &Rig, pose: &mut Pose, m: &Motion, g: &Gun, d: &Doing, a: &Aim) -> (Option<Mat4>, Option<Mat4>) {
    let (hands, down, lowered) = (d.hands, d.down, d.lowered);
    let spec = hands.spec();
    let (clip, t) = hands.clip();
    let stowed = hands.stowed_amount();
    if stowed > 0.95 {
        return (None, None);
    }
    let world = rig.world(pose);
    let eyes = rig.eyes(&world);
    let chest = world[rig.node(Bone::Chest)].translation();
    let shoulder = world[rig.node(Bone::UpperArmR)].translation();
    let up = aimed(g, eyes, a);
    let mut place = if down {
        Place { at: shoulder + a.f * 0.47 - a.u * 0.03, turn: up.turn }
    } else {
        hip(g, up, a).toward(up, hands.aim()).toward(carried(g, chest), m.sprint)
    };
    // A shot kicks it up and back.
    if clip == weapon::Clip::Fire
        && let Some(shot) = spec.shot
    {
        let k = (-t / 0.06).exp();
        let kick = (shot.kick * if g.long { 1.6 } else { 3.0 }).to_radians() * k;
        place = place.moved(Quat::from_axis_angle(a.r, kick), -a.f * (if g.long { 0.05 } else { 0.04 }) * k);
    }
    // Swung as a club: thrust out, the muzzle dipped.
    if matches!(clip, weapon::Clip::Bash | weapon::Clip::Bash2) {
        let b = bump(t / spec.bash.time, spec.bash.strike_at / spec.bash.time, 0.5);
        place = place.moved(Quat::from_axis_angle(a.r, -0.35 * b), a.f * 0.2 * b + a.u * 0.03 * b);
    }
    // Tipped in to reload.
    let tip = tipped(hands);
    if tip > 0.0 {
        let q = Quat::from_axis_angle(a.f, 0.5 * tip) * Quat::from_axis_angle(a.r, 0.2 * tip);
        place = place.moved(q, (-a.f * 0.08 - a.u * 0.07 - a.r * 0.02) * tip);
    }
    // Brought up, or put away (or the hands busy with something else):
    // lowered.
    let low = stowed.max(0.6 * lowered);
    if low > 0.0 {
        place = place.moved(Quat::from_axis_angle(a.r, -1.2 * low), -a.u * 0.15 * low);
    }
    let item = place.matrix(g);
    // The right hand on the grip (or working the bolt); the left on the
    // fore (working the pump), or reloading.
    let bolt = working(marks(hands), Act::Bolt, t, 0.15);
    let right = item.transform_point((g.grip + RIGHT_WRIST).lerp(g.rack + at(-0.03, 0.02, -0.02), bolt));
    rig.reach(pose, Side::Right, right, RIGHT_POLE);
    // The left hand on it too, but for a sidearm at a sprint (that arm
    // pumps) and down (it's leant on).
    if !down && (g.long || m.sprint < 0.5) {
        let pump = working(marks(hands), Act::Pump, t, 0.12);
        let pouch = rig.world(pose)[rig.node(Bone::Hips)].translation() + POUCH;
        let spot = |s: Spot| match s {
            Spot::On(p) => item.transform_point(p + LEFT_WRIST - Vec3::X * 0.09 * pump),
            Spot::Pouch => pouch,
        };
        let left = match reloading(g, hands) {
            Some((s, path)) => along(s, &path, spot),
            None => spot(Spot::On(g.fore)),
        };
        rig.reach(pose, Side::Left, left, LEFT_POLE);
    }
    // (A stream of fire is its own light: the puffs of it are in the
    // world to be seen.)
    let flash = (clip == weapon::Clip::Fire && t < FLASH && spec.shot.is_none_or(|s| s.stream.is_none())).then(|| item * Mat4::from_translation(g.muzzle));
    (Some(item), flash)
}

/// Where the hand is `s` of the way along `path` (spots at their shares).
fn along(s: f64, path: &[(f64, Spot)], spot: impl Fn(Spot) -> Vec3) -> Vec3 {
    for w in path.windows(2) {
        let ((a, sa), (b, sb)) = (w[0], w[1]);
        if s <= b {
            let t = if b > a { ramp((s - a) / (b - a)) } else { 1.0 };
            return spot(sa).lerp(spot(sb), t);
        }
    }
    spot(path.last().map_or(Spot::Pouch, |k| k.1))
}

/// What `g` is, for drawing.
pub(super) fn kind(g: &Gun) -> Kind {
    g.kind
}

/// Where each wrist goes on `g` placed at `item` (steady: not reloading,
/// nor working a pump or bolt).
#[cfg(test)]
pub(super) fn wrists(g: &Gun, item: Mat4) -> (Vec3, Vec3) {
    (item.transform_point(g.grip + RIGHT_WRIST), item.transform_point(g.fore + LEFT_WRIST))
}
