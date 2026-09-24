//! The dead: the Shambler. Each is an entity with a body (the same capsule
//! and movement as the player's, at its own pace), a mind (`brain.rs`), and
//! a figure to draw and to shoot (`figure.rs`); they find their way on the
//! nav grid (`nav.rs`). The world tells them of shots; they tell the world
//! what they say and whom they hit.

pub mod brain;
pub mod director;
pub mod figure;
pub mod kind;
pub mod looks;
pub mod nav;
pub mod spit;
mod special;
mod steer;
mod step;

use bevy_ecs::prelude::*;
use lntrn_math::Vec3;

use crate::player::Body;
use crate::sound::Sfx;
use crate::world::Solid;
use brain::{Senses, State, Zombie};
use kind::Kind;
use figure::{Figure, Model, Zone};
use looks::{Looks, Theme};
use nav::NavGrid;

/// How long the dead lie before they sink, and how long sinking takes.
const LIE_FOR: f64 = 20.0;
const SINK_FOR: f64 = 3.0;
/// It keeps this far from the player's middle (their bodies don't pass).
const PERSONAL: f64 = 0.65;
/// How many of the dead may find their way (a search of the grid) in one
/// step; the rest follow the way they have a step or two longer.
const SEARCHES: u32 = 8;
/// How hard the dead keep out of each other's way, per metre too close.
const ELBOW: f64 = 3.0;
/// How hard a blow shoves one, m/s (about a metre back); a shot's is its
/// gun's (`weapon/spec.rs`).
const BLOW_SHOVE: f64 = 7.0;
/// Where a new one comes from: this far off, and out of sight.
const SPAWN_NEAR: f64 = 35.0;
const SPAWN_FAR: f64 = 60.0;
/// The far dead take their steps less often (and longer, so they go as
/// far): every step within `CLOSE` of the player, every `MID_EVERY` out to
/// `MID`, every `FAR_EVERY` past that. Only the close ones keep out of
/// each other's way.
const CLOSE: f64 = 60.0;
const MID: f64 = 130.0;
const MID_EVERY: u32 = 3;
const FAR_EVERY: u32 = 8;
/// Past this, nothing of them shows (the fog has them): not posed. Past
/// `MID`, posed every third frame.
const UNSEEN: f64 = 240.0;
const POSE_FAR_EVERY: u32 = 3;

/// How often one of the dead takes its step (every how many of the
/// world's), and how many it has let go by since it last did.
#[derive(Component, Clone, Copy, Debug)]
pub struct Beat {
    every: u32,
    since: u32,
    /// Its place in the beat, so the far ones don't all step at once.
    phase: u32,
}

impl Beat {
    /// Stepping every step to begin with, at `phase` in the beat.
    pub fn new(phase: u32) -> Self {
        Self { every: 1, since: 0, phase }
    }
}

/// How often one this far (flat) from the player steps.
fn every(distance: f64) -> u32 {
    if distance < CLOSE {
        1
    } else if distance < MID {
        MID_EVERY
    } else {
        FAR_EVERY
    }
}

/// How far the dead see the player: a share of how far they see (the
/// player's light feet).
#[derive(Resource)]
pub struct Stealth(pub f64);

impl Default for Stealth {
    fn default() -> Self {
        Self(1.0)
    }
}

/// Where the dead can walk; built once the world is loaded.
#[derive(Resource, Default)]
pub struct Nav(pub Option<NavGrid>);

/// What the dead hear, since they last listened: noises made (where, and
/// how far they carry), and each other's snarls (where from, and where the player was seen).
#[derive(Resource, Default)]
pub struct Noises {
    pub shots: Vec<(Vec3, f64)>,
    pub snarls: Vec<(Vec3, Vec3)>,
}

/// What the dead did that the rest of the game hears of: sounds where they
/// are, blows that landed on the player (the way they push), and how many
/// have gone for good since last asked.
#[derive(Resource, Default)]
pub struct Horde {
    pub sounds: Vec<(Sfx, Vec3, f32)>,
    pub blows: Vec<brain::Blow>,
    /// Globs thrown this step (from, at), Spitters bursting (where), and
    /// the bursts the combat side has yet to deal out.
    pub spits: Vec<(Vec3, Vec3)>,
    pub bursting: Vec<Vec3>,
    pub bursts: Vec<Vec3>,
    /// The player's standing in a puddle of bile.
    pub poisoned: bool,
    pub gone: usize,
    seed: u32,
    /// The world's steps so far.
    tick: u32,
    /// What was heard over the last few steps (shots, snarls), and on
    /// which: kept long enough for the far dead, who step less often, to
    /// hear them on their next.
    shots: Vec<(u32, (Vec3, f64))>,
    snarls: Vec<(u32, (Vec3, Vec3))>,
}

impl Horde {
    /// 0 to 1, from the horde's own luck.
    pub fn roll(&mut self) -> f64 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        f64::from(self.seed) / f64::from(u32::MAX)
    }
}

pub fn install(fixed: &mut Schedule, frame: &mut Schedule) {
    fixed.add_systems((step::think, spit::fly, spit::fester).chain());
    frame.add_systems((step::pose, step::bury));
}

/// One of the dead in soldier's fatigues and gear (they carry more, and
/// better).
#[derive(Component, Clone, Copy, Debug)]
pub struct Soldier;

/// Put one of `kind` at `at`, facing `yaw` (a Shambler, one of `theme`'s).
pub fn spawn_kind(world: &mut World, at: Vec3, yaw: f64, kind: Kind, theme: Theme) {
    let seed = {
        let mut h = world.resource_mut::<Horde>();
        h.seed = h.seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        h.seed
    };
    let dice = &mut crate::loot::Dice(seed.rotate_left(16) ^ 0x5BD1_E995 | 1);
    let looks = match kind {
        Kind::Shambler => Looks::roll(theme, dice),
        Kind::Ripper => Looks::ripper(dice),
        Kind::Spitter => Looks::spitter(dice),
        Kind::Juggernaut => Looks::juggernaut(dice),
    };
    let mut zombie = Zombie::of(kind, yaw, seed);
    if looks.headless() {
        zombie.hp *= looks::HEADLESS_TOUGHNESS;
    }
    let mut e = world.spawn((zombie, Body::at(at), Figure::of(&looks), looks, Beat::new(seed % 64)));
    if theme == Theme::Soldier {
        e.insert(Soldier);
    }
}

/// Put a Shambler somewhere the player at `eye`, looking along `forward`,
/// can't see: on walkable ground, 35–60 m off, behind something or behind
/// them. Whether there was such a place.
pub fn spawn_unseen(world: &mut World, eye: Vec3, forward: Vec3, kind: Kind) -> bool {
    let spot = {
        let (Some(nav), solid) = (world.resource::<Nav>().0.as_ref(), &world.resource::<Solid>().0) else { return false };
        let mut seed = world.resource::<Horde>().seed ^ 0x9E37_79B9;
        let mut rand = || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            f64::from(seed) / f64::from(u32::MAX)
        };
        let mut found = None;
        for _ in 0..200 {
            let a = rand() * std::f64::consts::TAU;
            let r = SPAWN_NEAR + (SPAWN_FAR - SPAWN_NEAR) * rand();
            // Near the player's own level: on the ground, or a floor of a
            // house.
            let p = Vec3::new(eye.x + a.cos() * r, eye.y - 1.6, eye.z + a.sin() * r);
            let Some(h) = nav.height_at(p) else { continue };
            let feet = Vec3::new(p.x, h, p.z);
            // Somewhere the player can be got to from (not a store's roof).
            if !nav.connects(feet, eye - Vec3::new(0.0, 1.6, 0.0)) {
                continue;
            }
            let chest = feet + Vec3::new(0.0, 1.2, 0.0) - eye;
            let dist = chest.length();
            let dir = chest * (1.0 / dist);
            let behind = dir.dot(forward) < 0.3;
            let hidden = solid.raycast(eye, dir, dist).is_some();
            if behind || hidden {
                found = Some(feet);
                break;
            }
        }
        found
    };
    let Some(feet) = spot else { return false };
    let yaw = (-(eye.x - feet.x)).atan2(-(eye.z - feet.z));
    spawn_kind(world, feet, yaw, kind, Theme::Drifter);
    true
}

/// Every Shambler, gone (back to the title).
pub fn clear(world: &mut World) {
    let all: Vec<Entity> = world.query_filtered::<Entity, With<Zombie>>().iter(world).collect();
    for e in all {
        world.despawn(e);
    }
}

/// How many are up and about (not lying dead).
pub fn alive(world: &mut World) -> usize {
    world.query::<&Zombie>().iter(world).filter(|z| !z.dead()).count()
}

/// The nearest Shambler along a ray within `max`: which, how far, the head.
#[cfg(test)]
pub fn raycast(world: &mut World, from: Vec3, dir: Vec3, max: f64) -> Option<(Entity, f64, bool)> {
    raycast_past(world, from, dir, max, &[]).map(|(e, t, zone)| (e, t, zone == Zone::Head))
}

/// The nearest Shambler along a ray within `max`, but for those `past`
/// (the ones a round has already gone through): which, how far, where.
pub fn raycast_past(world: &mut World, from: Vec3, dir: Vec3, max: f64, past: &[Entity]) -> Option<(Entity, f64, Zone)> {
    let mut best = None;
    for (e, f) in world.query::<(Entity, &Figure)>().iter(world) {
        if past.contains(&e) {
            continue;
        }
        if let Some((t, zone)) = f.ray_zone(from, dir, max)
            && best.is_none_or(|(_, b, _)| t < b)
        {
            best = Some((e, t, zone));
        }
    }
    best
}

/// A hit on one of the dead: how much, whether to the head, whether a
/// blow (not a shot), how hard it shoves, m/s, whether it sends it
/// stumbling (a blow always does), and whether it kills outright one that
/// hasn't noticed the player.
#[derive(Clone, Copy, Debug)]
pub struct Impact {
    pub damage: f64,
    /// Where it struck: the head, an arm or leg, or (neither) the body.
    pub head: bool,
    pub limb: bool,
    pub blow: bool,
    pub shove: f64,
    pub stumble: bool,
    /// A killing blow to one that never saw it coming.
    pub takedown: bool,
}

/// A blow's shove, m/s, `times` the usual.
pub fn blow_shove(times: f64) -> f64 {
    BLOW_SHOVE * times
}

/// Hurt the Shambler `e` with `hit` along `dir` from `from`. Whether it
/// died.
pub fn hurt(world: &mut World, e: Entity, dir: Vec3, from: Vec3, hit: Impact) -> bool {
    let Some(mut z) = world.get_mut::<Zombie>(e) else { return false };
    // (No knife in the back drops a Juggernaut.)
    let damage = if hit.takedown && z.unaware() && z.kind != Kind::Juggernaut { z.hp * 10.0 } else { hit.damage * z.plating(dir, hit.limb) };
    let killed = z.hurt(damage, hit.head, hit.blow, from);
    if hit.stumble && !killed {
        z.stumble();
    }
    let mut sounds = Vec::new();
    if killed {
        sounds.push(if z.kind == Kind::Spitter { Sfx::Swell } else { Sfx::Gurgle });
    }
    // (Shot from straight above, it isn't shoved at all.)
    let flat = Vec3::new(dir.x, 0.0, dir.z);
    let push = if flat.length() > 1e-6 { flat.normalize() * hit.shove } else { Vec3::ZERO };
    let at = world.get_mut::<Body>(e).map(|mut body| {
        body.push += push;
        body.pos
    });
    if let Some(at) = at {
        let mut horde = world.resource_mut::<Horde>();
        horde.sounds.extend(sounds.into_iter().map(|s| (s, at + Vec3::new(0.0, 1.2, 0.0), 1.0)));
    }
    killed
}

/// Whether a hit along `dir` (on a limb, or not) on `e` meets plate.
pub fn plated(world: &World, e: Entity, dir: Vec3, limb: bool) -> bool {
    world.get::<Zombie>(e).is_some_and(|z| z.plating(dir, limb) < 1.0)
}

/// A noise at `at`, heard `range` metres off.
pub fn noise(world: &mut World, at: Vec3, range: f64) {
    world.resource_mut::<Noises>().shots.push((at, range));
}

#[cfg(test)]
mod bench;
#[cfg(test)]
mod model_tests;
#[cfg(test)]
mod special_tests;
#[cfg(test)]
mod tests;
