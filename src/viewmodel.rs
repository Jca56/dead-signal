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
        let amount = view.bob_amount * (1.0 + 0.6 * sprint) * steady;
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
mod model_tests {
    use lntrn_math::{Mat4, Vec3, Vec4};
    use lntrn_model::Gltf;

    use crate::weapon::Weapon;

    fn viewmodel(weapon: Weapon) -> Gltf {
        let name = weapon.spec().model;
        Gltf::load(format!("{}/assets/models/viewmodel_{name}.glb", env!("CARGO_MANIFEST_DIR"))).unwrap_or_else(|e| panic!("{name}: {e}"))
    }

    fn length(g: &Gltf, name: &str) -> f64 {
        g.animations.iter().find(|a| a.name.as_deref() == Some(name)).map(|a| a.duration()).unwrap_or(-1.0)
    }

    /// Where a point is on screen (x right, y up, -1 to 1), at 16:9 and
    /// 55°; none if it's behind the eye.
    fn on_screen(p: Vec3) -> Option<(f64, f64)> {
        let proj = Mat4::perspective_infinite_reverse_z(55f64.to_radians(), 16.0 / 9.0, 0.01);
        let c = proj * Vec4::from_vec3(p, 1.0);
        (c.w > 0.0).then(|| (c.x / c.w, c.y / c.w))
    }

    #[test]
    fn every_viewmodel_rests_as_modelled_with_its_clips() {
        for weapon in Weapon::ALL {
            let spec = weapon.spec();
            let g = viewmodel(weapon);
            let skin = &g.skins[0];
            assert!(skin.joints.len() <= crate::render::MAX_JOINTS, "{}: {} bones", spec.name, skin.joints.len());
            let joints = skin.joint_matrices(&g.world_matrices(&g.rest_pose()));
            for (i, m) in joints.iter().enumerate() {
                assert!(m.approx_eq(&Mat4::IDENTITY, 1e-4), "{}: joint {i} moves the mesh at rest", spec.name);
            }
            // Every clip the hands play is there, as long as they think.
            assert!((length(&g, "Idle") - 3.0).abs() < 0.05, "{} idle lasts {}", spec.name, length(&g, "Idle"));
            assert!((length(&g, "Bash") - spec.bash.time).abs() < 0.05, "{} bash lasts {}", spec.name, length(&g, "Bash"));
            if let Some(shot) = spec.shot {
                assert!((length(&g, "Fire") - shot.time).abs() < 0.05, "{} fire lasts {}", spec.name, length(&g, "Fire"));
            }
            let clips = match spec.reload {
                Some(crate::weapon::Reload::Magazine { time, .. }) => vec![("Reload", time)],
                Some(crate::weapon::Reload::Rounds { start, each, end, .. }) => vec![("ReloadStart", start), ("ReloadShell", each), ("ReloadEnd", end)],
                None => vec![],
            };
            for (clip, time) in clips {
                assert!((length(&g, clip) - time).abs() < 0.05, "{} {clip} lasts {}", spec.name, length(&g, clip));
            }
            if spec.shot.is_some() {
                assert!(length(&g, "Aim") > 0.0 && length(&g, "AimFire") > 0.0, "{} has sights", spec.name);
            }
        }
        assert_eq!(viewmodel(Weapon::Fists).skins[0].joints.len(), 19, "the arms alone");
        assert_eq!(viewmodel(Weapon::Pistol).skins[0].joints.len(), 23, "19 for the arms, 4 for the pistol");
        assert_eq!(viewmodel(Weapon::Shotgun).skins[0].joints.len(), 23, "19 for the arms, 4 for the shotgun");
    }

    #[test]
    fn both_hands_are_in_the_lower_frame() {
        for weapon in Weapon::ALL {
            let g = viewmodel(weapon);
            let skin = &g.skins[0];
            let prim = &g.meshes[g.nodes.iter().find_map(|n| n.skin.and(n.mesh)).unwrap()].primitives[0];
            for side in [".R", ".L"] {
                let hand = skin.joints.iter().position(|&j| g.nodes[j].name.as_deref() == Some(&format!("hand{side}"))).unwrap() as u16;
                let mut seen = 0;
                for (p, (j, w)) in prim.positions.iter().zip(prim.joints.iter().zip(&prim.weights)) {
                    if j[0] != hand || w[0] < 0.99 {
                        continue;
                    }
                    let (x, y) = on_screen(Vec3::new(f64::from(p[0]), f64::from(p[1]), f64::from(p[2]))).unwrap_or_else(|| panic!("{side} hand behind the eye"));
                    assert!(x.abs() < 1.0 && y > -1.0 && y < 0.0, "{weapon:?} {side} hand at {x:.2}, {y:.2}: off the lower frame");
                    assert!(if side == ".R" { x > 0.0 } else { x < 0.0 }, "{weapon:?} {side} hand on the wrong side");
                    seen += 1;
                }
                assert!(seen > 10, "{weapon:?} {side}: {seen} hand vertices");
            }
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
    fn the_jab_lands_ahead_in_view_and_comes_back() {
        let g = viewmodel(Weapon::Fists);
        let strike = Weapon::Fists.spec().bash.strike_at;
        let fist = |t: f64| bone_at(&g, "Bash", t, "hand.R").translation();
        let (rest, out) = (fist(0.0), fist(strike));
        // Ahead is -z: the fist goes a good way out.
        assert!(rest.z - out.z > 0.12, "the fist only reaches {:.3} m", rest.z - out.z);
        let (x, y) = on_screen(out).expect("in front");
        assert!(x.abs() < 0.6 && y.abs() < 0.8, "the fist lands at {x:.2}, {y:.2}");
        assert!((fist(0.5) - rest).length() < 0.005, "and comes home");
    }

    #[test]
    fn the_pistol_animations_do_what_they_say() {
        let g = viewmodel(Weapon::Pistol);
        // The gun in the hand where poses.py put it: Blender (0.085, 0.30,
        // -0.165) is the game's (0.085, -0.165, -0.30).
        let at = bone_at(&g, "Idle", 0.0, "gun").translation();
        assert!((at - Vec3::new(0.085, -0.165, -0.30)).length() < 0.001, "the gun is held at {at:?}");
        // The flash on the first frame of a shot only.
        let size = |anim: &str, t: f64| bone_at(&g, anim, t, "flash").col(0).length();
        assert!(size("Fire", 0.0) > 0.9 && size("Fire", 0.1) < 0.05 && size("Idle", 0.5) < 0.05);
        // The slide back on the shot, home after.
        let slide = |t: f64| bone_at(&g, "Fire", t, "slide").translation() - bone_at(&g, "Fire", t, "gun").translation();
        assert!((slide(1.0 / 30.0) - slide(0.0)).length() > 0.03, "the slide racks back");
        assert!((slide(0.2) - slide(0.0)).length() < 0.005, "and comes home");
        // The magazine well out of the grip halfway through a reload.
        let mag = |t: f64| (bone_at(&g, "Reload", t, "mag").translation() - bone_at(&g, "Reload", t, "gun").translation()).length();
        assert!(mag(0.5) > mag(0.0) + 0.3, "mag out: {} vs {}", mag(0.5), mag(0.0));
        assert!((mag(1.4) - mag(0.0)).abs() < 0.005, "and back in");
    }

    #[test]
    fn aimed_the_pistols_sights_are_on_the_middle_of_the_view() {
        let g = viewmodel(Weapon::Pistol);
        let skin = &g.skins[0];
        let prim = &g.meshes[g.nodes.iter().find_map(|n| n.skin.and(n.mesh)).unwrap()].primitives[0];
        let slide = skin.joints.iter().position(|&j| g.nodes[j].name.as_deref() == Some("slide")).unwrap() as u16;
        for (anim, t) in [("Aim", 0.0), ("Aim", 1.5), ("AimFire", 0.2)] {
            let mut pose = g.rest_pose();
            g.animations.iter().find(|a| a.name.as_deref() == Some(anim)).unwrap().sample(t, &mut pose);
            let joints = skin.joint_matrices(&g.world_matrices(&pose));
            // The slide's corners as posed; the sights are its highest.
            let posed: Vec<Vec3> = prim
                .positions
                .iter()
                .zip(prim.joints.iter().zip(&prim.weights))
                .filter(|(_, (j, w))| j[0] == slide && w[0] > 0.99)
                .map(|(p, _)| joints[usize::from(slide)].transform_point(Vec3::new(f64::from(p[0]), f64::from(p[1]), f64::from(p[2]))))
                .collect();
            let top = posed.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
            let sights: Vec<&Vec3> = posed.iter().filter(|p| p.y > top - 0.002).collect();
            let (near, far) = (sights.iter().map(|p| -p.z).fold(f64::INFINITY, f64::min), sights.iter().map(|p| -p.z).fold(0.0, f64::max));
            assert!(far - near > 0.12, "{anim}: rear and front sights both on top ({near:.3} to {far:.3} m ahead)");
            // Each sight's middle (the rear's notch, the front's post) on
            // the middle of the view.
            let mid = (near + far) * 0.5;
            for rear in [true, false] {
                let those: Vec<&&Vec3> = sights.iter().filter(|p| (-p.z < mid) == rear).collect();
                let centre = those.iter().fold(Vec3::ZERO, |a, p| a + ***p) * (1.0 / those.len() as f64);
                let (x, y) = on_screen(centre).expect("in front");
                assert!(x.abs() < 0.01 && y.abs() < 0.01, "{anim} at {t}: the {} sight at {x:.3}, {y:.3}", if rear { "rear" } else { "front" });
            }
            // And the post shows through the notch, light either side of it:
            // on screen, the post's widest is inside the ears' nearest.
            let across = |rear: bool| sights.iter().filter(|p| (-p.z < mid) == rear).map(|p| on_screen(**p).expect("in front").0).collect::<Vec<f64>>();
            let notch = across(true).iter().map(|x| x.abs()).fold(f64::INFINITY, f64::min);
            let post = across(false).iter().map(|x| x.abs()).fold(0.0, f64::max);
            assert!(post < notch * 0.8, "{anim} at {t}: the post ({post:.4} wide each side) fills the notch ({notch:.4})");
        }
    }

    #[test]
    fn both_hands_hold_the_pistol_in_view() {
        let g = viewmodel(Weapon::Pistol);
        let gun = bone_at(&g, "Idle", 1.0, "gun").translation();
        let left = bone_at(&g, "Idle", 1.0, "hand.L").translation();
        let right = bone_at(&g, "Idle", 1.0, "hand.R").translation();
        assert!((left - gun).length() < 0.12, "left hand {:.3} m off the gun", (left - gun).length());
        assert!((right - gun).length() < 0.12, "right hand {:.3} m off the gun", (right - gun).length());
        // The muzzle on screen: right of middle and low.
        let muzzle = bone_at(&g, "Idle", 1.0, "flash").translation();
        let (x, y) = on_screen(muzzle).expect("in front");
        assert!(x > 0.0 && x < 0.7 && y < 0.0 && y > -0.8, "muzzle at {x:.2}, {y:.2}");
    }

    /// Where `bone`'s vertices whose colour passes `pick` are, posed by
    /// `anim` at `t`: their middle.
    fn painted(g: &Gltf, anim: &str, t: f64, bone: &str, pick: impl Fn([f32; 4]) -> bool) -> Vec3 {
        let skin = &g.skins[0];
        let prim = &g.meshes[g.nodes.iter().find_map(|n| n.skin.and(n.mesh)).unwrap()].primitives[0];
        let joint = skin.joints.iter().position(|&j| g.nodes[j].name.as_deref() == Some(bone)).unwrap() as u16;
        let mut pose = g.rest_pose();
        g.animations.iter().find(|a| a.name.as_deref() == Some(anim)).unwrap().sample(t, &mut pose);
        let m = skin.joint_matrices(&g.world_matrices(&pose))[usize::from(joint)];
        let found: Vec<Vec3> = prim
            .positions
            .iter()
            .zip(prim.joints.iter().zip(&prim.weights))
            .zip(&prim.colors)
            .filter(|((_, (j, w)), c)| j[0] == joint && w[0] > 0.99 && pick(**c))
            .map(|((p, _), _)| m.transform_point(Vec3::new(f64::from(p[0]), f64::from(p[1]), f64::from(p[2]))))
            .collect();
        assert!(!found.is_empty(), "nothing so painted on {bone}");
        found.iter().fold(Vec3::ZERO, |a, p| a + *p) * (1.0 / found.len() as f64)
    }

    /// The orange of a bead or a post.
    fn orange(c: [f32; 4]) -> bool {
        c[0] > 0.5 && c[1] < c[0] * 0.6 && c[2] < 0.1
    }

    #[test]
    fn aimed_the_shotguns_bead_is_on_the_middle_of_the_view() {
        let g = viewmodel(Weapon::Shotgun);
        for (anim, t) in [("Aim", 0.0), ("Aim", 1.5), ("AimFire", 0.79)] {
            let (x, y) = on_screen(painted(&g, anim, t, "gun", orange)).expect("in front");
            assert!(x.abs() < 0.01 && y.abs() < 0.01, "{anim} at {t}: the bead at {x:.3}, {y:.3}");
        }
    }

    #[test]
    fn the_shotgun_is_held_in_both_hands_and_pumped() {
        let g = viewmodel(Weapon::Shotgun);
        let pump = |anim: &str, t: f64| bone_at(&g, anim, t, "pump").translation();
        let gun = |anim: &str, t: f64| bone_at(&g, anim, t, "gun").translation();
        for (anim, t) in [("Idle", 0.0), ("Idle", 1.5), ("Aim", 0.0), ("Fire", 0.4), ("AimFire", 0.4), ("Bash", 0.27)] {
            let left = bone_at(&g, anim, t, "hand.L").translation();
            let right = bone_at(&g, anim, t, "hand.R").translation();
            assert!((left - pump(anim, t)).length() < 0.12, "{anim} at {t}: the left hand {:.3} m off the forend", (left - pump(anim, t)).length());
            assert!((right - gun(anim, t)).length() < 0.12, "{anim} at {t}: the right hand {:.3} m off the stock", (right - gun(anim, t)).length());
        }
        // The pump back along the gun mid-Fire, and home after.
        let along = |anim: &str, t: f64| (pump(anim, t) - gun(anim, t)).length();
        assert!(along("Fire", 0.0) - along("Fire", 0.4) > 0.05, "racked back: {:.3} vs {:.3}", along("Fire", 0.4), along("Fire", 0.0));
        assert!((along("Fire", 0.8) - along("Fire", 0.0)).abs() < 0.005, "and home");
        // The muzzle and the forend on screen at the hip.
        for bone in ["flash", "pump"] {
            let (x, y) = on_screen(bone_at(&g, "Idle", 1.0, bone).translation()).expect("in front");
            assert!(x > -0.2 && x < 0.9 && y < 0.1 && y > -0.95, "{bone} at {x:.2}, {y:.2}");
        }
    }

    #[test]
    fn a_shell_is_only_in_hand_to_load_it_and_goes_in_the_port() {
        let g = viewmodel(Weapon::Shotgun);
        let size = |anim: &str, t: f64| bone_at(&g, anim, t, "shell").col(0).length();
        for (anim, t) in [("Idle", 1.0), ("Fire", 0.3), ("Aim", 0.5), ("ReloadEnd", 0.4), ("ReloadShell", 0.36)] {
            assert!(size(anim, t) < 0.05, "a shell in hand in {anim} at {t}");
        }
        assert!(size("ReloadShell", 0.0) > 0.9 && size("ReloadShell", 0.29) > 0.9, "a shell in hand to load");
        // Just before it's thumbed in, it's at the loading port: under the
        // receiver, a little ahead of the grip.
        let port = bone_at(&g, "ReloadShell", 0.3, "gun").transform_point(Vec3::ZERO);
        let shell = bone_at(&g, "ReloadShell", 0.3, "shell").translation();
        assert!((shell - port).length() < 0.25, "the shell {:.3} m off the gun", (shell - port).length());
    }
}
