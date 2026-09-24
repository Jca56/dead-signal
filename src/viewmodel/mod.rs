//! The arms in front of the eye, and how they move: they trail a fast turn
//! of the head and spring back, bob in step with the stride (more at a
//! sprint, when they also drop and tilt), lift off a jump and dip with the
//! landing, settle when crouched, and breathe with their idle animation.
//! Whatever's in them is its own viewmodel (the arms and that weapon, and
//! its clips), dropped out of view as it's put away and raised as it's
//! taken up. All in camera space: x right, y up, -z ahead.

use std::collections::HashMap;

use lntrn_math::{Mat4, Quat, Transform, Vec2, Vec3};

use crate::assets::Rigged;
use crate::render::SkinnedMeshId;
use crate::head::{EYE_CROUCH, EYE_STAND, View};
use crate::player::Body;
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

pub struct Viewmodel {
    /// Every weapon's viewmodel.
    rigs: HashMap<Weapon, Rigged<SkinnedMeshId>>,
    sway: Sway,
    /// Up or down in the air, metres.
    lift: f64,
}

impl Viewmodel {
    pub fn new(rigs: HashMap<Weapon, Rigged<SkinnedMeshId>>) -> Self {
        Self { rigs, sway: Sway::default(), lift: 0.0 }
    }

    /// Forget the last run's motion.
    pub fn reset(&mut self) {
        self.sway = Sway::default();
        self.lift = 0.0;
    }

    /// Move the lag and the lift along by `dt`.
    pub fn update(&mut self, view: &View, body: &Body, dt: f64) {
        if dt <= 0.0 {
            return;
        }
        self.sway.update(view.yaw, view.pitch, dt);

        // Rising, the arms trail down; falling, they float up.
        let lift = if body.grounded { 0.0 } else { (-body.vel.y * 0.004).clamp(-0.025, 0.025) };
        self.lift += (lift - self.lift) * (1.0 - (-12.0 * dt).exp());
    }

    /// The arms and what's in `hands` as they are drawn this frame (the
    /// clip playing, blended toward the weapon's aimed one as far as the
    /// sights are up; idles loop on the game's clock, `time`); none if that
    /// weapon's viewmodel didn't load.
    pub fn draw(&self, view: &View, hands: &Hands, time: f64, lowered: f64) -> Option<SkinnedDraw> {
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
        Some(SkinnedDraw { mesh: rig.mesh, model: self.placement(view, lowered, hands.stowed_amount(), aim), joints })
    }

    /// Where the whole rig sits in front of the eye.
    /// `lowered` (0–1) drops the gun out of the way (patching up);
    /// `stowed` (0–1) takes it right down out of view, tipping forward;
    /// `aim` (0–1) steadies it, the sights held on the middle: all the
    /// way up, nothing moves it (moved, the near sight would slide off the
    /// far one), and what's left of the sway and bob turns it about the eye
    /// (which keeps both sights on one line through it).
    fn placement(&self, view: &View, lowered: f64, stowed: f64, aim: f64) -> Mat4 {
        let steady = 1.0 - AIM_STEADY * aim;
        let still = 1.0 - aim;
        let sway = self.sway.angle * steady;
        let sprint = view.sprint_amount.max(lowered);
        let amount = view.bob_amount * view.feel.bob * (1.0 + 0.6 * sprint) * steady;
        let phase = view.bob_phase;
        let crouch = ((EYE_STAND - view.eye) / (EYE_STAND - EYE_CROUCH)).clamp(0.0, 1.0) * (1.0 - aim);
        let stow = stowed.clamp(0.0, 1.0);
        let stow = stow * stow * (3.0 - 2.0 * stow);
        let offset = Vec3::new(
            (phase.cos() * 0.012 * amount - sway.x * 0.1) * still + 0.04 * stow,
            ((phase * 2.0).sin() * 0.008 * amount + sway.y * 0.1 + self.lift + view.dip * 0.35) * still - 0.06 * sprint - 0.015 * crouch - STOW_DROP * stow,
            0.02 * sprint,
        );
        let turn = Quat::from_rotation_y(sway.x) * Quat::from_rotation_x(sway.y - 0.3 * sprint - STOW_TIP * stow) * Quat::from_rotation_z(phase.cos() * 0.015 * amount - sway.x * 0.3 + 0.1 * sprint);
        // Turned about the chest from the hip, about the eye down the sights.
        let pivot = PIVOT * still;
        Mat4::from_translation(offset + pivot) * Mat4::from_quat(turn) * Mat4::from_translation(-pivot)
    }
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
        let mut vm = Viewmodel::new(HashMap::new());
        // Mid-turn, mid-stride, just landed, in the air.
        vm.sway.angle = Vec2::new(0.05, -0.04);
        vm.lift = 0.02;
        let mut view = View::facing(0.0);
        (view.bob_amount, view.bob_phase, view.dip) = (1.0, 0.9, -0.1);
        let m = vm.placement(&view, 0.0, 0.0, 1.0);
        // The sight line (the eye's own axis) stays a line through the eye.
        let (rear, front) = (m.transform_point(Vec3::new(0.0, 0.0, -0.17)), m.transform_point(Vec3::new(0.0, 0.0, -0.34)));
        let apart = rear.normalize().cross(front.normalize()).length();
        assert!(apart < 1e-9, "the sights part by {apart}");
        // From the hip the same motion does move the gun about.
        let m = vm.placement(&view, 0.0, 0.0, 0.0);
        let (rear, front) = (m.transform_point(Vec3::new(0.0, 0.0, -0.17)), m.transform_point(Vec3::new(0.0, 0.0, -0.34)));
        assert!(rear.normalize().cross(front.normalize()).length() > 1e-3);
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
