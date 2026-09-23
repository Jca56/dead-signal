//! The Shambler as it's seen and hit: which animation plays and how far
//! in, the bones that pose it, and the head, body and limbs a shot can
//! find, all in the world where it stands.

use bevy_ecs::prelude::*;
use lntrn_math::{Mat4, Transform, Vec3};
use lntrn_model::Gltf;

use crate::assets::Rigged;
use crate::render::FigureMeshId;

/// Its animations, by name in `shambler.glb`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Clip {
    Walk,
    Idle,
    Attack,
    Flinch,
    Death,
}

impl Clip {
    fn name(self) -> &'static str {
        match self {
            Clip::Walk => "Walk",
            Clip::Idle => "Idle",
            Clip::Attack => "Attack",
            Clip::Flinch => "Flinch",
            Clip::Death => "Death",
        }
    }

    fn loops(self) -> bool {
        matches!(self, Clip::Walk | Clip::Idle)
    }
}

/// The bones a hit is tested against, by name.
const BONES: [&str; 15] = ["hips", "neck", "head", "upper_arm.L", "forearm.L", "hand.L", "upper_arm.R", "forearm.R", "hand.R", "thigh.L", "shin.L", "foot.L", "thigh.R", "shin.R", "foot.R"];
const HIPS: usize = 0;
const NECK: usize = 1;
const HEAD: usize = 2;
/// The limbs as capsules between bones (and how thick), each the next bone
/// down the chain from the last.
const LIMBS: [(usize, usize, f64); 8] = [(3, 4, 0.07), (4, 5, 0.06), (6, 7, 0.07), (7, 8, 0.06), (9, 10, 0.09), (10, 11, 0.08), (12, 13, 0.09), (13, 14, 0.08)];
const HEAD_RADIUS: f64 = 0.14;
const BODY_RADIUS: f64 = 0.21;

/// The Shambler's model, shared by every one of them.
#[derive(Resource)]
pub struct Model {
    pub mesh: FigureMeshId,
    gltf: Gltf,
    skin: usize,
    /// Where each of [`BONES`] is among the file's nodes.
    bones: [usize; 15],
}

impl Model {
    pub fn new(rig: Rigged<FigureMeshId>) -> Result<Self, String> {
        let mut bones = [0; 15];
        for (slot, name) in bones.iter_mut().zip(BONES) {
            *slot = rig.gltf.nodes.iter().position(|n| n.name.as_deref() == Some(name)).ok_or_else(|| format!("shambler.glb: no bone {name}"))?;
        }
        Ok(Self { mesh: rig.mesh, gltf: rig.gltf, skin: rig.skin, bones })
    }

    /// Pose it: `clip` at `t` seconds. The skinning matrices, and where the
    /// hit-tested bones are in its own space.
    pub fn pose(&self, clip: Clip, t: f64) -> (Vec<Mat4>, [Vec3; 15]) {
        let mut pose: Vec<Transform> = self.gltf.rest_pose();
        if let Some(anim) = self.gltf.animations.iter().find(|a| a.name.as_deref() == Some(clip.name())) {
            let len = anim.duration().max(1e-6);
            anim.sample(if clip.loops() { t.rem_euclid(len) } else { t.min(len) }, &mut pose);
        }
        let world = self.gltf.world_matrices(&pose);
        let joints = self.gltf.skins[self.skin].joint_matrices(&world);
        let points = self.bones.map(|i| world[i].translation());
        (joints, points)
    }
}

/// A Shambler as drawn and hit this frame.
#[derive(Component, Clone, Debug, Default)]
pub struct Figure {
    /// Where it stands and faces (and how far it has sunk).
    pub model: Mat4,
    pub joints: Vec<Mat4>,
    /// [`BONES`], in the world.
    points: [Vec3; 15],
    /// Whether a shot can find it (not when dead).
    pub solid: bool,
}

impl Figure {
    pub fn set(&mut self, model: Mat4, joints: Vec<Mat4>, local: [Vec3; 15], solid: bool) {
        self.points = local.map(|p| model.transform_point(p));
        self.model = model;
        self.joints = joints;
        self.solid = solid;
    }

    /// How far along a ray (unit `dir`) it is hit, and whether in the head.
    pub fn ray(&self, from: Vec3, dir: Vec3, max: f64) -> Option<(f64, bool)> {
        if !self.solid {
            return None;
        }
        let p = &self.points;
        // The head: a ball over the head bone, up along it.
        let crown = p[HEAD] + (p[HEAD] - p[NECK]).normalize() * 0.08;
        let mut best = ray_capsule(from, dir, crown, crown, HEAD_RADIUS).filter(|&t| t <= max).map(|t| (t, true));
        let mut take = |hit: Option<f64>| {
            if let Some(t) = hit.filter(|&t| t <= max)
                && best.is_none_or(|(b, _)| t < b)
            {
                best = Some((t, false));
            }
        };
        take(ray_capsule(from, dir, p[HIPS], p[NECK], BODY_RADIUS));
        for (a, b, r) in LIMBS {
            take(ray_capsule(from, dir, p[a], p[b], r));
        }
        best
    }
}

/// How far along a ray it first meets the capsule `a`–`b` of radius `r`.
fn ray_capsule(from: Vec3, dir: Vec3, a: Vec3, b: Vec3, r: f64) -> Option<f64> {
    // March the ray's closest approach: sample the segment finely enough
    // for bodies this size (a ray into a limb a few centimetres across).
    let mut best: Option<f64> = None;
    let steps = ((b - a).length() / (r * 0.5)).ceil().max(1.0) as usize;
    for i in 0..=steps {
        let c = a + (b - a) * (i as f64 / steps as f64);
        // Ray against the sphere at `c`.
        let oc = from - c;
        let half_b = oc.dot(dir);
        let q = oc.dot(oc) - r * r;
        let disc = half_b * half_b - q;
        if disc < 0.0 {
            continue;
        }
        let t = -half_b - disc.sqrt();
        let t = if t < 0.0 { -half_b + disc.sqrt() } else { t };
        if t >= 0.0 && best.is_none_or(|b| t < b) {
            best = Some(t);
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_capsule_is_found_where_it_is() {
        let (a, b) = (Vec3::new(0.0, 1.0, -5.0), Vec3::new(0.0, 2.0, -5.0));
        let t = ray_capsule(Vec3::new(0.0, 1.5, 0.0), Vec3::new(0.0, 0.0, -1.0), a, b, 0.2).expect("hit");
        assert!((t - 4.8).abs() < 0.01, "{t}");
        assert!(ray_capsule(Vec3::new(0.5, 1.5, 0.0), Vec3::new(0.0, 0.0, -1.0), a, b, 0.2).is_none());
    }

    #[test]
    fn the_head_and_body_of_the_real_model_are_where_a_shot_finds_them() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/models/shambler.glb");
        let gltf = Gltf::load(path).expect("shambler.glb");
        let skin = 0;
        let bones = BONES.map(|name| gltf.nodes.iter().position(|n| n.name.as_deref() == Some(name)).unwrap());
        let model = Model { mesh: FigureMeshId::placeholder(), gltf, skin, bones };
        let (joints, local) = model.pose(Clip::Idle, 0.0);
        let mut f = Figure::default();
        // Standing 10 m ahead of the eye, facing it (a yaw of pi).
        let at = Mat4::from_translation(Vec3::new(0.0, 0.0, -10.0)) * Mat4::from_quat(lntrn_math::Quat::from_rotation_y(std::f64::consts::PI));
        f.set(at, joints, local, true);
        let head_y = f.points[HEAD].y + 0.08;
        let (_, head) = f.ray(Vec3::new(0.0, head_y, 0.0), Vec3::new(0.0, 0.0, -1.0), 50.0).expect("the head");
        assert!(head, "a head-high shot is a headshot");
        let (t, head) = f.ray(Vec3::new(0.0, 1.2, 0.0), Vec3::new(0.0, 0.0, -1.0), 50.0).expect("the body");
        assert!(!head && t > 9.0 && t < 10.0, "a chest-high shot hits the body at {t}");
        assert!(f.ray(Vec3::new(1.2, 1.2, 0.0), Vec3::new(0.0, 0.0, -1.0), 50.0).is_none(), "a shot past it misses");
        f.solid = false;
        assert!(f.ray(Vec3::new(0.0, 1.2, 0.0), Vec3::new(0.0, 0.0, -1.0), 50.0).is_none(), "the dead are not hit");
    }
}
