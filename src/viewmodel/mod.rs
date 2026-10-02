//! The arms in front of the eye, and how they move: they trail a fast turn
//! of the head and spring back, bob in step with the stride (more at a
//! sprint, when they also drop and tilt), lift off a jump and dip with the
//! landing, settle when crouched, and breathe with their idle animation.
//! Whatever's in them is its own viewmodel (the arms and that weapon, and
//! its clips), dropped out of view as it's put away and raised as it's
//! taken up; the radio, pulled out, is one too, in the weapon's place.
//! All in camera space: x right, y up, -z ahead.

use std::collections::HashMap;

use lntrn_math::{Mat4, Quat, Transform, Vec2, Vec3};

use crate::assets::Rigged;
use crate::render::SkinnedMeshId;
use crate::head::{EYE_CROUCH, EYE_STAND, View};
use crate::player::Body;
use crate::radio::Shown;
use crate::render::SkinnedDraw;
use crate::weapon::{Clip, Hands, Weapon};
use lntrn_model::Gltf;

/// Seconds of turn the arms lag by, and the most they may lag, radians.
const LAG: f64 = 0.018;
const MAX_SWAY: f64 = 0.07;
/// The spring that pulls a lag back: stiffness, and damping just short of
/// a wobble.
const STIFF: f64 = 150.0;
const DAMP: f64 = 18.0;
/// Where the arms turn about: low in the chest, just behind the eye.
const PIVOT: Vec3 = Vec3::new(0.0, -0.25, 0.05);
/// Down the sights, how much of the sway, bob and bounce is steadied.
const AIM_STEADY: f64 = 0.8;
/// Put away, how far the arms drop, metres, and tip forward, radians.
const STOW_DROP: f64 = 0.32;
const STOW_TIP: f64 = 0.7;
/// Where the radio's held, as its model has it (out at the right, well
/// ahead), and how far it reaches to the side of that.
const RADIO_AT: Vec3 = Vec3::new(0.225, -0.125, -0.37);
const RADIO_REACH: f64 = 0.05;

/// How far the arms trail the view: a spring pulled by how fast it turns.
#[derive(Clone, Copy, Debug, Default)]
struct Sway {
    /// Where the view pointed last frame, to see how fast it turns.
    last_look: Option<(f64, f64)>,
    /// The lag, radians: x is yaw, y is pitch.
    angle: Vec2,
    vel: Vec2,
}

impl Sway {
    fn update(&mut self, yaw: f64, pitch: f64, dt: f64) {
        let (dyaw, dpitch) = self.last_look.map_or((0.0, 0.0), |(y, p)| (yaw - y, pitch - p));
        self.last_look = Some((yaw, pitch));
        let target = Vec2::new((-dyaw / dt * LAG).clamp(-MAX_SWAY, MAX_SWAY), (-dpitch / dt * LAG).clamp(-MAX_SWAY, MAX_SWAY));
        let accel = (target - self.angle) * STIFF - self.vel * DAMP;
        self.vel += accel * dt;
        self.angle += self.vel * dt;
    }
}

/// How one player's arms are moving: the lag behind their turn, and up
/// or down in the air, metres.
#[derive(Clone, Copy, Debug, Default)]
struct Motion {
    sway: Sway,
    lift: f64,
}

pub struct Viewmodel {
    /// Every weapon's viewmodel; and the radio's.
    rigs: HashMap<Weapon, Rigged<SkinnedMeshId>>,
    radio: Option<Rigged<SkinnedMeshId>>,
    /// Each player's arms' motion, by seat.
    motions: Vec<Motion>,
}

impl Viewmodel {
    pub fn new(rigs: HashMap<Weapon, Rigged<SkinnedMeshId>>, radio: Option<Rigged<SkinnedMeshId>>) -> Self {
        Self { rigs, radio, motions: vec![Motion::default()] }
    }

    /// Forget the last run's motion: fresh, for each of `players`.
    pub fn reset(&mut self, players: usize) {
        self.motions = vec![Motion::default(); players];
    }

    /// Move player `seat`'s lag and lift along by `dt`.
    pub fn update(&mut self, seat: usize, view: &View, body: &Body, dt: f64) {
        let Some(m) = self.motions.get_mut(seat).filter(|_| dt > 0.0) else { return };
        m.sway.update(view.yaw, view.pitch, dt);

        // Rising, the arms trail down; falling, they float up.
        let lift = if body.grounded { 0.0 } else { (-body.vel.y * 0.004).clamp(-0.025, 0.025) };
        m.lift += (lift - m.lift) * (1.0 - (-12.0 * dt).exp());
    }

    /// Player `seat`'s arms and what's in `hands` as they are drawn this
    /// frame (the clip playing, blended toward the weapon's aimed one as far
    /// as the sights are up; idles loop on the game's clock, `time`); none
    /// if that weapon's viewmodel didn't load. With the `radio` in view
    /// (and how far their pane sees across and up, as tangents), it's the
    /// arms and the radio instead.
    pub fn draw(&self, seat: usize, view: &View, hands: &Hands, radio: Option<(Shown, [f64; 2])>, time: f64, lowered: f64) -> Option<SkinnedDraw> {
        if let Some((shown, sees)) = radio {
            let rig = self.radio.as_ref()?;
            let pose = sample(&rig.gltf, shown.clip, shown.t.unwrap_or(time), shown.t.is_none());
            let joints = rig.gltf.skins[rig.skin].joint_matrices(&rig.gltf.world_matrices(&pose));
            let motion = self.motions.get(seat).copied().unwrap_or_default();
            let model = Mat4::from_translation(Vec3::new(-radio_over(sees), 0.0, 0.0)) * placement(&motion, view, 0.0, shown.stowed, 0.0);
            return Some(SkinnedDraw { mesh: rig.mesh, model, joints, glow: [0.0; 4] });
        }
        let rig = self.rigs.get(&hands.weapon)?;
        let gltf = &rig.gltf;
        let (clip, t) = hands.clip();
        let idle = clip.name() == Clip::Idle.name();
        let mut pose = sample(gltf, clip.name(), if idle { time } else { t }, idle);
        let aim = hands.aim();
        if aim > 0.0 {
            let aimed = if clip == Clip::Fire { sample(gltf, "AimFire", t, false) } else { sample(gltf, "Aim", time, true) };
            for (hip, up) in pose.iter_mut().zip(&aimed) {
                *hip = Transform { translation: hip.translation.lerp(up.translation, aim), rotation: hip.rotation.slerp(up.rotation, aim), scale: hip.scale.lerp(up.scale, aim) };
            }
        }
        let joints = gltf.skins[rig.skin].joint_matrices(&gltf.world_matrices(&pose));
        let motion = self.motions.get(seat).copied().unwrap_or_default();
        // An amplified weapon glows, breathing slowly.
        let glow = crate::weapon::amp::glow(hands.tier).map_or([0.0; 4], |g| [g[0], g[1], g[2], (0.55 + 0.45 * (time * 2.2).sin()) as f32]);
        Some(SkinnedDraw { mesh: rig.mesh, model: placement(&motion, view, lowered, hands.stowed_amount(), aim), joints, glow })
    }
}

/// Where the whole rig sits in front of the eye, moving as `motion` is.
/// `lowered` (0–1) drops the gun out of the way (patching up);
/// `stowed` (0–1) takes it right down out of view, tipping forward;
/// `aim` (0–1) steadies it, the sights held on the middle: all the
/// way up, nothing moves it (moved, the near sight would slide off the
/// far one), and what's left of the sway and bob turns it about the eye
/// (which keeps both sights on one line through it).
fn placement(motion: &Motion, view: &View, lowered: f64, stowed: f64, aim: f64) -> Mat4 {
    let steady = 1.0 - AIM_STEADY * aim;
    let still = 1.0 - aim;
    let sway = motion.sway.angle * steady;
    let sprint = view.sprint_amount.max(lowered);
    let amount = view.bob_amount * view.feel.bob * (1.0 + 0.6 * sprint) * steady;
    let phase = view.bob_phase;
    let crouch = ((EYE_STAND - view.eye) / (EYE_STAND - EYE_CROUCH)).clamp(0.0, 1.0) * (1.0 - aim);
    let stow = stowed.clamp(0.0, 1.0);
    let stow = stow * stow * (3.0 - 2.0 * stow);
    let offset = Vec3::new(
        (phase.cos() * 0.012 * amount - sway.x * 0.1) * still + 0.04 * stow,
        ((phase * 2.0).sin() * 0.008 * amount + sway.y * 0.1 + motion.lift + view.dip * 0.35) * still - 0.06 * sprint - 0.015 * crouch - STOW_DROP * stow,
        0.02 * sprint,
    );
    let turn = Quat::from_rotation_y(sway.x) * Quat::from_rotation_x(sway.y - 0.3 * sprint - STOW_TIP * stow) * Quat::from_rotation_z(phase.cos() * 0.015 * amount - sway.x * 0.3 + 0.1 * sprint);
    // Turned about the chest from the hip, about the eye down the sights.
    let pivot = PIVOT * still;
    Mat4::from_translation(offset + pivot) * Mat4::from_quat(turn) * Mat4::from_translation(-pivot)
}

/// How far the radio's moved over to be in a pane that sees `sees`
/// (tangents, across and up): a narrow one (side by side) doesn't reach
/// out to where it's held.
fn radio_over(sees: [f64; 2]) -> f64 {
    (RADIO_AT.x + RADIO_REACH - sees[0] * -RADIO_AT.z).max(0.0)
}

/// Where the radio's near side is across a pane that sees `sees`, from -1
/// (the pane's left) to 1 (its right): what's drawn beside it keeps to
/// the left of that.
pub fn radio_side(sees: [f64; 2]) -> f64 {
    (RADIO_AT.x - radio_over(sees) - RADIO_REACH * 1.6) / -RADIO_AT.z / sees[0]
}

/// `clip` of `gltf` at `t` seconds (a looping clip wraps round), over its
/// rest pose (just the rest pose if there's no such clip).
fn sample(gltf: &Gltf, clip: &str, t: f64, looping: bool) -> Vec<Transform> {
    let mut pose = gltf.rest_pose();
    if let Some(anim) = gltf.animations.iter().find(|a| a.name.as_deref() == Some(clip)) {
        let length = anim.duration();
        if length > 0.0 {
            anim.sample(if looping { t.rem_euclid(length) } else { t.min(length) }, &mut pose);
        }
    }
    pose
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f64 = 1.0 / 60.0;

    #[test]
    fn a_turn_left_trails_right_and_settles() {
        let mut sway = Sway::default();
        // 3 rad/s to the left for half a second.
        let mut yaw = 0.0;
        for _ in 0..30 {
            yaw += 3.0 * DT;
            sway.update(yaw, 0.0, DT);
        }
        assert!(sway.angle.x < -0.03 && sway.angle.x >= -MAX_SWAY - 0.01, "lagged {}", sway.angle.x);
        assert_eq!(sway.angle.y, 0.0, "no pitch, no pitch lag");
        // Then still: back to rest within two seconds.
        for _ in 0..120 {
            sway.update(yaw, 0.0, DT);
        }
        assert!(sway.angle.x.abs() < 1e-3, "settled at {}", sway.angle.x);
    }

    #[test]
    fn down_the_sights_the_sway_and_bob_never_part_the_sights() {
        use crate::head::View;
        // Mid-turn, mid-stride, just landed, in the air.
        let motion = Motion { sway: Sway { angle: Vec2::new(0.05, -0.04), ..Sway::default() }, lift: 0.02 };
        let mut view = View::facing(0.0);
        (view.bob_amount, view.bob_phase, view.dip) = (1.0, 0.9, -0.1);
        let m = placement(&motion, &view, 0.0, 0.0, 1.0);
        // The sight line (the eye's own axis) stays a line through the eye.
        let (rear, front) = (m.transform_point(Vec3::new(0.0, 0.0, -0.17)), m.transform_point(Vec3::new(0.0, 0.0, -0.34)));
        let apart = rear.normalize().cross(front.normalize()).length();
        assert!(apart < 1e-9, "the sights part by {apart}");
        // From the hip the same motion does move the gun about.
        let m = placement(&motion, &view, 0.0, 0.0, 0.0);
        let (rear, front) = (m.transform_point(Vec3::new(0.0, 0.0, -0.17)), m.transform_point(Vec3::new(0.0, 0.0, -0.34)));
        assert!(rear.normalize().cross(front.normalize()).length() > 1e-3);
    }

    #[test]
    fn the_radio_s_moved_over_into_a_narrow_pane_and_left_where_it_is_in_a_wide_one() {
        use crate::render::viewmodel_sees;
        // The whole window, and one pane above another: it's in view.
        assert_eq!(radio_over(viewmodel_sees(16.0 / 9.0, 16.0 / 9.0)), 0.0);
        assert_eq!(radio_over(viewmodel_sees(16.0 / 9.0, 32.0 / 9.0)), 0.0);
        // Side by side: over, till its far side is at the pane's edge.
        let sees = viewmodel_sees(16.0 / 9.0, 8.0 / 9.0);
        let over = radio_over(sees);
        assert!(over > 0.08 && over < 0.13, "{over}");
        assert!((RADIO_AT.x + RADIO_REACH - over - sees[0] * -RADIO_AT.z).abs() < 1e-9);
    }

    #[test]
    fn a_flick_is_capped() {
        let mut sway = Sway::default();
        sway.update(0.0, 0.0, DT);
        sway.update(2.0, -2.0, DT);
        for _ in 0..10 {
            sway.update(2.0, -2.0, DT);
            assert!(sway.angle.x.abs() <= MAX_SWAY * 1.3 && sway.angle.y.abs() <= MAX_SWAY * 1.3, "{:?}", sway.angle);
        }
    }
}

#[cfg(test)]
mod model_tests;
