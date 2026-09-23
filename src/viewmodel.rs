//! The arms in front of the eye, and how they move: they trail a fast turn
//! of the head and spring back, bob in step with the stride (more at a
//! sprint, when they also drop and tilt), lift off a jump and dip with the
//! landing, settle when crouched, and breathe with their idle animation.
//! All in camera space: x right, y up, -z ahead.

use lntrn_math::{Mat4, Quat, Transform, Vec2, Vec3};

use crate::assets::Rigged;
use crate::render::SkinnedMeshId;
use crate::head::{EYE_CROUCH, EYE_STAND, View};
use crate::player::Body;
use crate::render::SkinnedDraw;

/// Seconds of turn the arms lag by, and the most they may lag, radians.
const LAG: f64 = 0.018;
const MAX_SWAY: f64 = 0.07;
/// The spring that pulls a lag back: stiffness, and damping just short of
/// a wobble.
const STIFF: f64 = 150.0;
const DAMP: f64 = 18.0;
/// Where the arms turn about: low in the chest, just behind the eye.
const PIVOT: Vec3 = Vec3::new(0.0, -0.25, 0.05);

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
    rig: Rigged<SkinnedMeshId>,
    sway: Sway,
    /// Up or down in the air, metres.
    lift: f64,
}

impl Viewmodel {
    pub fn new(rig: Rigged<SkinnedMeshId>) -> Self {
        Self { rig, sway: Sway::default(), lift: 0.0 }
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

    /// The arms as they are drawn this frame, playing `clip` at `t`
    /// seconds (a looping clip wraps round).
    pub fn draw(&self, view: &View, clip: &str, t: f64, looping: bool, lowered: f64) -> SkinnedDraw {
        let gltf = &self.rig.gltf;
        let mut pose: Vec<Transform> = gltf.rest_pose();
        if let Some(anim) = gltf.animations.iter().find(|a| a.name.as_deref() == Some(clip)) {
            let length = anim.duration();
            if length > 0.0 {
                anim.sample(if looping { t.rem_euclid(length) } else { t.min(length) }, &mut pose);
            }
        }
        let joints = gltf.skins[self.rig.skin].joint_matrices(&gltf.world_matrices(&pose));
        SkinnedDraw { mesh: self.rig.mesh, model: self.placement(view, lowered), joints }
    }

    /// Where the whole rig sits in front of the eye.
    /// `lowered` (0–1) drops the gun out of the way (patching up).
    fn placement(&self, view: &View, lowered: f64) -> Mat4 {
        let sway = self.sway.angle;
        let sprint = view.sprint_amount.max(lowered);
        let amount = view.bob_amount * (1.0 + 0.6 * sprint);
        let phase = view.bob_phase;
        let crouch = ((EYE_STAND - view.eye) / (EYE_STAND - EYE_CROUCH)).clamp(0.0, 1.0);
        let offset = Vec3::new(
            phase.cos() * 0.012 * amount - sway.x * 0.1,
            (phase * 2.0).sin() * 0.008 * amount + sway.y * 0.1 - 0.06 * sprint - 0.015 * crouch + self.lift + view.dip * 0.35,
            0.02 * sprint,
        );
        let turn = Quat::from_rotation_y(sway.x) * Quat::from_rotation_x(sway.y - 0.3 * sprint) * Quat::from_rotation_z(phase.cos() * 0.015 * amount - sway.x * 0.3 + 0.1 * sprint);
        Mat4::from_translation(offset + PIVOT) * Mat4::from_quat(turn) * Mat4::from_translation(-PIVOT)
    }
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
mod model_tests {
    use lntrn_math::{Mat4, Vec3, Vec4};
    use lntrn_model::Gltf;

    fn arms() -> Gltf {
        Gltf::load(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/models/arms.glb")).expect("arms.glb")
    }

    #[test]
    fn the_rest_pose_is_the_modelled_pose() {
        let g = arms();
        let skin = &g.skins[0];
        assert_eq!(skin.joints.len(), 23, "19 for the arms, 4 for the pistol");
        let joints = skin.joint_matrices(&g.world_matrices(&g.rest_pose()));
        for (i, m) in joints.iter().enumerate() {
            assert!(m.approx_eq(&Mat4::IDENTITY, 1e-4), "joint {i} moves the mesh at rest");
        }
        let idle = g.animations.iter().find(|a| a.name.as_deref() == Some("Idle")).expect("an Idle loop");
        assert!((idle.duration() - 3.0).abs() < 0.05, "idle lasts {}", idle.duration());
    }

    #[test]
    fn both_hands_are_in_the_lower_frame() {
        let g = arms();
        let skin = &g.skins[0];
        let proj = Mat4::perspective_infinite_reverse_z(55f64.to_radians(), 16.0 / 9.0, 0.01);
        let prim = &g.meshes[g.nodes.iter().find_map(|n| n.skin.and(n.mesh)).unwrap()].primitives[0];
        for side in [".R", ".L"] {
            let hand = skin.joints.iter().position(|&j| g.nodes[j].name.as_deref() == Some(&format!("hand{side}"))).unwrap() as u16;
            let mut seen = 0;
            for (p, (j, w)) in prim.positions.iter().zip(prim.joints.iter().zip(&prim.weights)) {
                if j[0] != hand || w[0] < 0.99 {
                    continue;
                }
                let c = proj * Vec4::from_vec3(Vec3::new(f64::from(p[0]), f64::from(p[1]), f64::from(p[2])), 1.0);
                let (x, y) = (c.x / c.w, c.y / c.w);
                assert!(c.w > 0.0, "{side} hand behind the eye");
                assert!(x.abs() < 1.0 && y > -1.0 && y < 0.0, "{side} hand at {x:.2}, {y:.2}: off the lower frame");
                assert!(if side == ".R" { x > 0.0 } else { x < 0.0 }, "{side} hand on the wrong side");
                seen += 1;
            }
            assert!(seen > 10, "{side}: {seen} hand vertices");
        }
    }

    /// A bone's place in the world at `t` seconds into `anim`.
    fn bone_at(g: &Gltf, anim: &str, t: f64, bone: &str) -> Mat4 {
        let mut pose = g.rest_pose();
        g.animations.iter().find(|a| a.name.as_deref() == Some(anim)).unwrap_or_else(|| panic!("no {anim}")).sample(t, &mut pose);
        let i = g.nodes.iter().position(|n| n.name.as_deref() == Some(bone)).unwrap_or_else(|| panic!("no bone {bone}"));
        g.world_matrices(&pose)[i]
    }

    #[test]
    fn the_pistol_animations_do_what_they_say() {
        let g = arms();
        let length = |name: &str| g.animations.iter().find(|a| a.name.as_deref() == Some(name)).map(|a| a.duration()).unwrap_or(-1.0);
        for (name, want) in [("PistolIdle", 3.0), ("PistolFire", 0.2), ("PistolReload", 1.4), ("PistolMelee", 0.5)] {
            assert!((length(name) - want).abs() < 0.05, "{name} lasts {}", length(name));
        }
        // The gun in the hand where poses.py put it: Blender (0.085, 0.30,
        // -0.165) is the game's (0.085, -0.165, -0.30).
        let at = bone_at(&g, "PistolIdle", 0.0, "gun").translation();
        assert!((at - Vec3::new(0.085, -0.165, -0.30)).length() < 0.01, "the gun is held at {at:?}");
        // The flash on the first frame of a shot only.
        let size = |anim: &str, t: f64| bone_at(&g, anim, t, "flash").col(0).length();
        assert!(size("PistolFire", 0.0) > 0.9 && size("PistolFire", 0.1) < 0.05 && size("PistolIdle", 0.5) < 0.05);
        // The slide back on the shot, home after.
        let slide = |t: f64| bone_at(&g, "PistolFire", t, "slide").translation() - bone_at(&g, "PistolFire", t, "gun").translation();
        assert!((slide(1.0 / 30.0) - slide(0.0)).length() > 0.03, "the slide racks back");
        assert!((slide(0.2) - slide(0.0)).length() < 0.005, "and comes home");
        // The magazine well out of the grip halfway through a reload.
        let mag = |t: f64| (bone_at(&g, "PistolReload", t, "mag").translation() - bone_at(&g, "PistolReload", t, "gun").translation()).length();
        assert!(mag(0.5) > mag(0.0) + 0.3, "mag out: {} vs {}", mag(0.5), mag(0.0));
        assert!((mag(1.4) - mag(0.0)).abs() < 0.005, "and back in");
    }

    #[test]
    fn both_hands_hold_the_pistol_in_view() {
        let g = arms();
        let gun = bone_at(&g, "PistolIdle", 1.0, "gun").translation();
        let left = bone_at(&g, "PistolIdle", 1.0, "hand.L").translation();
        let right = bone_at(&g, "PistolIdle", 1.0, "hand.R").translation();
        assert!((left - gun).length() < 0.12, "left hand {:.3} m off the gun", (left - gun).length());
        assert!((right - gun).length() < 0.12, "right hand {:.3} m off the gun", (right - gun).length());
        // The muzzle on screen: right of middle and low, at 16:9 and 55°.
        let proj = Mat4::perspective_infinite_reverse_z(55f64.to_radians(), 16.0 / 9.0, 0.01);
        let muzzle = bone_at(&g, "PistolIdle", 1.0, "flash").translation();
        let c = proj * Vec4::from_vec3(muzzle, 1.0);
        let (x, y) = (c.x / c.w, c.y / c.w);
        assert!(c.w > 0.0 && x > 0.0 && x < 0.7 && y < 0.0 && y > -0.8, "muzzle at {x:.2}, {y:.2}");
    }
}
