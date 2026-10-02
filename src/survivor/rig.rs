//! The survivors' rig as the game poses it: its bones by name, its clips
//! (`assets/blender/survivor.py`), a pose (every node's own transform)
//! sampled from them and mixed, bones turned in the figure's own space,
//! and an arm reaching for a point (two bones, the elbow bent towards a
//! pole).

use bevy_ecs::prelude::*;
use lntrn_math::{Mat4, Quat, Transform, Vec3};
use lntrn_model::Gltf;

use crate::assets::Rigged;
use crate::render::FigureMeshId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bone {
    Hips,
    Spine,
    Chest,
    Neck,
    Head,
    UpperArmL,
    ForearmL,
    HandL,
    UpperArmR,
    ForearmR,
    HandR,
    ThighL,
    ShinL,
    FootL,
    ThighR,
    ShinR,
    FootR,
}

impl Bone {
    pub const ALL: [Bone; 17] = [
        Bone::Hips,
        Bone::Spine,
        Bone::Chest,
        Bone::Neck,
        Bone::Head,
        Bone::UpperArmL,
        Bone::ForearmL,
        Bone::HandL,
        Bone::UpperArmR,
        Bone::ForearmR,
        Bone::HandR,
        Bone::ThighL,
        Bone::ShinL,
        Bone::FootL,
        Bone::ThighR,
        Bone::ShinR,
        Bone::FootR,
    ];

    fn name(self) -> &'static str {
        match self {
            Bone::Hips => "hips",
            Bone::Spine => "spine",
            Bone::Chest => "chest",
            Bone::Neck => "neck",
            Bone::Head => "head",
            Bone::UpperArmL => "upper_arm.L",
            Bone::ForearmL => "forearm.L",
            Bone::HandL => "hand.L",
            Bone::UpperArmR => "upper_arm.R",
            Bone::ForearmR => "forearm.R",
            Bone::HandR => "hand.R",
            Bone::ThighL => "thigh.L",
            Bone::ShinL => "shin.L",
            Bone::FootL => "foot.L",
            Bone::ThighR => "thigh.R",
            Bone::ShinR => "shin.R",
            Bone::FootR => "foot.R",
        }
    }
}

/// What the body's clips (what's held, swung, thrown) take over from the
/// legs' (the spine's only half).
pub const ARMS: [Bone; 8] = [Bone::Chest, Bone::Neck, Bone::UpperArmL, Bone::ForearmL, Bone::HandL, Bone::UpperArmR, Bone::ForearmR, Bone::HandR];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

impl Side {
    /// Its arm, shoulder to hand.
    pub fn arm(self) -> [Bone; 3] {
        match self {
            Side::Left => [Bone::UpperArmL, Bone::ForearmL, Bone::HandL],
            Side::Right => [Bone::UpperArmR, Bone::ForearmR, Bone::HandR],
        }
    }
}

/// Its animations, by name in `survivor.glb`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Clip {
    Idle,
    Walk,
    Jog,
    Sprint,
    Crouch,
    CrouchWalk,
    Jump,
    Land,
    Down,
    Downed,
    Scoot,
    BleedOut,
    Kneel,
    HoldBlade,
    HoldAxe,
    Guard,
    Stab,
    Swing,
    Swing2,
    Chop,
    Jab,
    Cross,
    Windup,
    Throw,
}

impl Clip {
    pub const ALL: [Clip; 24] = [
        Clip::Idle,
        Clip::Walk,
        Clip::Jog,
        Clip::Sprint,
        Clip::Crouch,
        Clip::CrouchWalk,
        Clip::Jump,
        Clip::Land,
        Clip::Down,
        Clip::Downed,
        Clip::Scoot,
        Clip::BleedOut,
        Clip::Kneel,
        Clip::HoldBlade,
        Clip::HoldAxe,
        Clip::Guard,
        Clip::Stab,
        Clip::Swing,
        Clip::Swing2,
        Clip::Chop,
        Clip::Jab,
        Clip::Cross,
        Clip::Windup,
        Clip::Throw,
    ];

    fn name(self) -> &'static str {
        match self {
            Clip::Idle => "Idle",
            Clip::Walk => "Walk",
            Clip::Jog => "Jog",
            Clip::Sprint => "Sprint",
            Clip::Crouch => "Crouch",
            Clip::CrouchWalk => "CrouchWalk",
            Clip::Jump => "Jump",
            Clip::Land => "Land",
            Clip::Down => "Down",
            Clip::Downed => "Downed",
            Clip::Scoot => "Scoot",
            Clip::BleedOut => "BleedOut",
            Clip::Kneel => "Kneel",
            Clip::HoldBlade => "HoldBlade",
            Clip::HoldAxe => "HoldAxe",
            Clip::Guard => "Guard",
            Clip::Stab => "Stab",
            Clip::Swing => "Swing",
            Clip::Swing2 => "Swing2",
            Clip::Chop => "Chop",
            Clip::Jab => "Jab",
            Clip::Cross => "Cross",
            Clip::Windup => "Windup",
            Clip::Throw => "Throw",
        }
    }

    fn loops(self) -> bool {
        matches!(self, Clip::Idle | Clip::Walk | Clip::Jog | Clip::Sprint | Clip::Crouch | Clip::CrouchWalk | Clip::Downed | Clip::Scoot | Clip::Kneel | Clip::HoldBlade | Clip::HoldAxe | Clip::Guard)
    }
}

/// Every node's own transform, as posed.
pub type Pose = Vec<Transform>;

/// Where the eyes are at rest, and how far along the hand its palm is
/// (metres, the figure's space).
const EYES: Vec3 = Vec3::new(0.0, 1.705, -0.115);
const PALM: f64 = 0.05;

/// The survivors' model, shared by all of them: every part either is put
/// together from, on the one rig.
#[derive(Resource)]
pub struct Rig {
    parts: Vec<(String, FigureMeshId)>,
    gltf: Gltf,
    skin: usize,
    /// Where each of [`Bone::ALL`] is among the file's nodes, and each of
    /// [`Clip::ALL`] among its animations.
    bones: [usize; 17],
    clips: [usize; 24],
    /// An arm's upper and fore lengths, and the way along a bone in its
    /// own frame.
    upper: f64,
    fore: f64,
    along: Vec3,
    /// The eyes, in the head's own frame; and the right hand at rest, in
    /// the figure's space.
    eyes: Vec3,
    hand: Mat4,
}

impl Rig {
    pub fn new(rig: Rigged<Vec<(String, FigureMeshId)>>) -> Result<Self, String> {
        let gltf = rig.gltf;
        let mut bones = [0; 17];
        for (slot, bone) in bones.iter_mut().zip(Bone::ALL) {
            *slot = gltf.nodes.iter().position(|n| n.name.as_deref() == Some(bone.name())).ok_or_else(|| format!("survivor.glb: no bone {}", bone.name()))?;
        }
        let mut clips = [0; 24];
        for (slot, clip) in clips.iter_mut().zip(Clip::ALL) {
            *slot = gltf.animations.iter().position(|a| a.name.as_deref() == Some(clip.name())).ok_or_else(|| format!("survivor.glb: no clip {}", clip.name()))?;
        }
        let at = |b: Bone| bones[b as usize];
        let rest = gltf.world_matrices(&gltf.rest_pose());
        let (forearm, hand) = (gltf.nodes[at(Bone::ForearmR)].transform.translation, gltf.nodes[at(Bone::HandR)].transform.translation);
        let head = rest[at(Bone::Head)].inverse().ok_or("survivor.glb: a flat head")?;
        Ok(Self { parts: rig.mesh, skin: rig.skin, bones, clips, upper: forearm.length(), fore: hand.length(), along: forearm.normalize_or(Vec3::Y), eyes: head.transform_point(EYES), hand: rest[at(Bone::HandR)], gltf })
    }

    /// The meshes of the parts named.
    pub fn meshes(&self, names: &[&str]) -> Vec<FigureMeshId> {
        names.iter().filter_map(|want| self.parts.iter().find(|(name, _)| name == want).map(|&(_, mesh)| mesh)).collect()
    }

    pub fn node(&self, bone: Bone) -> usize {
        self.bones[bone as usize]
    }

    /// The pose at rest.
    pub fn rest(&self) -> Pose {
        self.gltf.rest_pose()
    }

    /// How long `clip` runs, seconds.
    pub fn duration(&self, clip: Clip) -> f64 {
        self.gltf.animations[self.clips[clip as usize]].duration().max(1e-6)
    }

    /// `clip` at `t` seconds (round and round, if it loops; else held at
    /// its end), over the whole of `pose`.
    pub fn sample(&self, clip: Clip, t: f64, pose: &mut Pose) {
        let len = self.duration(clip);
        let t = if clip.loops() { t.rem_euclid(len) } else { t.clamp(0.0, len) };
        self.gltf.animations[self.clips[clip as usize]].sample(t, pose);
    }

    /// `clip` at the share `phase` (0–1) through it.
    pub fn sample_at(&self, clip: Clip, phase: f64, pose: &mut Pose) {
        self.sample(clip, phase * self.duration(clip), pose);
    }

    /// `into`'s `bones` taken `w` of the way to `from`'s.
    pub fn mix(&self, into: &mut Pose, from: &Pose, w: f64, bones: &[Bone]) {
        if w <= 0.0 {
            return;
        }
        for &bone in bones {
            let i = self.node(bone);
            let (a, b) = (into[i], from[i]);
            into[i].rotation = a.rotation.slerp(b.rotation, w.min(1.0));
            into[i].translation = a.translation.lerp(b.translation, w.min(1.0));
        }
    }

    /// Every node's matrix in the figure's space.
    pub fn world(&self, pose: &Pose) -> Vec<Mat4> {
        self.gltf.world_matrices(pose)
    }

    /// Each bone's skinning matrix.
    pub fn joints(&self, world: &[Mat4]) -> Vec<Mat4> {
        self.gltf.skins[self.skin].joint_matrices(world)
    }

    /// Turn `bone` (and all hanging from it) by `q`, about its head, in the
    /// figure's space (`world` as the pose stands).
    pub fn turn(&self, pose: &mut Pose, world: &[Mat4], bone: Bone, q: Quat) {
        let i = self.node(bone);
        let parent = self.gltf.nodes[i].parent.map_or(Quat::IDENTITY, |p| world[p].to_trs().1);
        pose[i].rotation = (parent.inverse() * q * parent * pose[i].rotation).normalize();
    }

    /// Point `bone` along `dir` (the least turn that does), in the figure's
    /// space.
    fn point(&self, pose: &mut Pose, bone: Bone, dir: Vec3) {
        let world = self.world(pose);
        let now = world[self.node(bone)].to_trs().1 * self.along;
        self.turn(pose, &world, bone, Quat::from_rotation_arc(now.normalize_or(Vec3::Y), dir.normalize_or(Vec3::NEG_Y)));
    }

    /// The arm on `side` reaching its wrist to `wrist` (as far as it
    /// can), the elbow bent towards `pole`, the hand straight on from the
    /// forearm; all in the figure's space.
    pub fn reach(&self, pose: &mut Pose, side: Side, wrist: Vec3, pole: Vec3) {
        let [upper, fore, hand] = side.arm();
        let shoulder = self.world(pose)[self.node(upper)].translation();
        let (a, b) = (self.upper, self.fore);
        let to = wrist - shoulder;
        let d = to.length().clamp(0.05, a + b - 1e-4);
        let along = to.normalize_or(Vec3::NEG_Y);
        let cos = ((a * a + d * d - b * b) / (2.0 * a * d)).clamp(-1.0, 1.0);
        let bend = (pole - along * pole.dot(along)).normalize_or(Vec3::NEG_Y);
        let elbow = shoulder + along * (a * cos) + bend * (a * (1.0 - cos * cos).sqrt());
        self.point(pose, upper, elbow - shoulder);
        self.point(pose, fore, shoulder + along * d - elbow);
        let rest = self.gltf.nodes[self.node(hand)].transform.rotation;
        pose[self.node(hand)].rotation = rest;
    }

    /// Where the eyes are, as posed (`world`).
    pub fn eyes(&self, world: &[Mat4]) -> Vec3 {
        world[self.node(Bone::Head)].transform_point(self.eyes)
    }

    /// How a thing held in the right hand sits in it: the hand's frame
    /// (`world`, as posed) times the returned, which puts a thing's `grip`
    /// (in its own space) in the palm, the thing turned as the hand is from
    /// rest, where at rest it points ahead (its +X) and its edge down the
    /// fingers (its -Z).
    pub fn in_hand(&self, world: &[Mat4], grip: Vec3) -> Mat4 {
        let fingers = self.hand.transform_vector(self.along).normalize_or(Vec3::NEG_Y);
        let ahead = (Vec3::NEG_Z - fingers * Vec3::NEG_Z.dot(fingers)).normalize_or(Vec3::NEG_Z);
        let palm = self.hand.translation() + fingers * PALM;
        // Its +X ahead, its +Z up the fingers, its +Y the third way.
        let held = Mat4::from_cols(ahead.extend(0.0), (-fingers).cross(ahead).extend(0.0), (-fingers).extend(0.0), palm.extend(1.0));
        let rest = self.hand.inverse().unwrap_or(Mat4::IDENTITY) * held * Mat4::from_translation(-grip);
        world[self.node(Bone::HandR)] * rest
    }

    /// Where the palm of the hand on `side` is, as posed.
    #[cfg(test)]
    pub fn palm(&self, world: &[Mat4], side: Side) -> Vec3 {
        let m = world[self.node(side.arm()[2])];
        m.transform_point(self.along * PALM)
    }
}

#[cfg(test)]
impl Rig {
    /// The parts named, skinned by `joints`: each triangle's corners and
    /// their colours (a region's shade in `palette`'s colour).
    pub fn triangles(&self, names: &[&str], joints: &[Mat4], palette: &[[f32; 3]; crate::render::PALETTE]) -> Vec<(Vec3, [f32; 3])> {
        let mut out = Vec::new();
        for node in self.gltf.nodes.iter().filter(|n| n.name.as_deref().is_some_and(|name| names.contains(&name))) {
            let Some(mesh) = node.mesh else { continue };
            for p in &self.gltf.meshes[mesh].primitives {
                for &i in &p.indices {
                    let i = i as usize;
                    let at = Vec3::new(f64::from(p.positions[i][0]), f64::from(p.positions[i][1]), f64::from(p.positions[i][2]));
                    let (j, w) = (p.joints.get(i).copied().unwrap_or([0; 4]), p.weights.get(i).copied().unwrap_or([1.0, 0.0, 0.0, 0.0]));
                    let skinned = (0..4).fold(Vec3::ZERO, |sum, k| sum + joints[usize::from(j[k])].transform_point(at) * f64::from(w[k]));
                    let c = p.colors.get(i).copied().unwrap_or([1.0; 4]);
                    let colour = if c[3] < 0.99 {
                        let region = ((c[3] / 0.2).round() as usize).min(crate::render::PALETTE - 1);
                        [c[0] * palette[region][0], c[1] * palette[region][1], c[2] * palette[region][2]]
                    } else {
                        [c[0], c[1], c[2]]
                    };
                    out.push((skinned, colour));
                }
            }
        }
        out
    }
}
