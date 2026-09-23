//! The dead: the Shambler. Each is an entity with a body (the same capsule
//! and movement as the player's, at its own pace), a mind (`brain.rs`), and
//! a figure to draw and to shoot (`figure.rs`); they find their way on the
//! nav grid (`nav.rs`). The world tells them of shots; they tell the world
//! what they say and whom they hit.

pub mod brain;
pub mod director;
pub mod figure;
pub mod nav;

use bevy_ecs::prelude::*;
use lntrn_math::{Mat4, Quat, Vec2, Vec3};

use crate::player::{self, Body, Player, RADIUS, STEP};
use crate::sound::Sfx;
use crate::world::{Blend, Solid};
use brain::{Senses, State, Zombie};
use figure::{Figure, Model};
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
    pub blows: Vec<Vec3>,
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

#[allow(clippy::too_many_arguments)]
fn think(mut dead: Query<(&mut Zombie, &mut Body, &mut Beat), Without<Player>>, players: Query<&Body, With<Player>>, solid: Res<Solid>, nav: Res<Nav>, stealth: Option<Res<Stealth>>, mut noises: ResMut<Noises>, mut horde: ResMut<Horde>) {
    let player = players.iter().next().map(|b| b.pos);
    horde.tick = horde.tick.wrapping_add(1);
    let tick = horde.tick;
    let new = std::mem::take(&mut *noises);
    let fresh = |at: u32| tick.wrapping_sub(at) < FAR_EVERY;
    horde.shots.retain(|(at, _)| fresh(*at));
    horde.snarls.retain(|(at, _)| fresh(*at));
    horde.shots.extend(new.shots.into_iter().map(|n| (tick, n)));
    horde.snarls.extend(new.snarls.into_iter().map(|n| (tick, n)));
    let shots: Vec<(Vec3, f64)> = horde.shots.iter().map(|(_, n)| *n).collect();
    let snarls: Vec<(Vec3, Vec3)> = horde.snarls.iter().map(|(_, n)| *n).collect();
    let searches = std::cell::Cell::new(SEARCHES);
    let senses = Senses { solids: &solid.0, nav: nav.0.as_ref(), player, noises: &shots, alerts: &snarls, searches: &searches, sight: stealth.map_or(1.0, |s| s.0) };
    for (mut z, mut body, mut beat) in &mut dead {
        // Far off, it steps less often, and further each time. (What's
        // heard is kept as long as the farthest go between steps.)
        let far = player.map_or(0.0, |p| Vec2::new(body.pos.x - p.x, body.pos.z - p.z).length());
        let n = every(far);
        if !(tick + beat.phase).is_multiple_of(n) {
            beat.since += 1;
            continue;
        }
        *beat = Beat { every: n, since: 0, ..*beat };
        let dt = STEP * f64::from(n);
        let mut intent = z.think(&body, &senses, dt);
        let voice = body.pos + Vec3::new(0.0, 1.5, 0.0);
        horde.sounds.extend(intent.sounds.drain(..).map(|(sfx, gain)| (sfx, voice, gain)));
        if let Some(push) = intent.hit {
            horde.blows.push(push);
        }
        if let Some(seen) = intent.alert {
            noises.snarls.push((body.pos, seen));
        }
        if z.dead() {
            body.prev = body.pos;
            continue;
        }
        let before = body.pos;
        let gait = z.gait;
        player::step_body(&mut body, z.yaw, &mut intent.controls, &solid.0, &gait, dt);
        // Kept out of the player's way.
        if let Some(p) = player {
            let off = Vec2::new(body.pos.x - p.x, body.pos.z - p.z);
            let d = off.length();
            if d < PERSONAL && d > 1e-6 {
                let out = off * ((PERSONAL - d) / d);
                body.pos.x += out.x;
                body.pos.z += out.y;
            }
        }
        z.walked += Vec2::new(body.pos.x - before.x, body.pos.z - before.z).length();
    }
    elbow(&mut dead);
}

/// The living dead near the player nudge each other apart (a shove, so
/// walls still stop them), and a crowd spreads round the player instead
/// of stacking.
fn elbow(dead: &mut Query<(&mut Zombie, &mut Body, &mut Beat), Without<Player>>) {
    let at: Vec<Option<Vec3>> = dead.iter().map(|(z, b, beat)| (!z.dead() && beat.every == 1).then_some(b.pos)).collect();
    let apart = RADIUS * 2.0;
    let mut nudges = vec![Vec3::ZERO; at.len()];
    for i in 0..at.len() {
        for j in i + 1..at.len() {
            let (Some(a), Some(b)) = (at[i], at[j]) else { continue };
            if (a.y - b.y).abs() > 1.5 {
                continue;
            }
            let off = Vec3::new(a.x - b.x, 0.0, a.z - b.z);
            let d = off.length();
            if d >= apart {
                continue;
            }
            // Exactly on top of each other: any way will do, as long as
            // it's opposite ways.
            let dir = if d > 1e-6 { off * (1.0 / d) } else { Vec3::new(1.0, 0.0, 0.0) };
            let nudge = dir * ((apart - d) * ELBOW);
            nudges[i] += nudge;
            nudges[j] -= nudge;
        }
    }
    for ((_, mut body, _), nudge) in dead.iter_mut().zip(nudges) {
        if nudge != Vec3::ZERO {
            body.push += nudge;
        }
    }
}

/// Pose each one for this frame: its animation, where it stands between
/// its steps, and sinking once it has lain long enough. Lost in the fog,
/// it isn't; far off, only now and then.
fn pose(model: Option<Res<Model>>, blend: Res<Blend>, players: Query<&Body, With<Player>>, mut frames: Local<u32>, mut dead: Query<(&Zombie, &Body, &Beat, &mut Figure), Without<Player>>) {
    let Some(model) = model else { return };
    let player = players.iter().next().map(|b| b.pos);
    *frames = frames.wrapping_add(1);
    for (z, body, beat, mut figure) in &mut dead {
        let far = player.map_or(0.0, |p| Vec2::new(body.pos.x - p.x, body.pos.z - p.z).length());
        if far > UNSEEN {
            figure.hide();
            continue;
        }
        if far > MID && !figure.joints.is_empty() && !(*frames + beat.phase).is_multiple_of(POSE_FAR_EVERY) {
            continue;
        }
        let (clip, t) = z.clip();
        let (joints, points) = model.pose(clip, t);
        // Between the step it took and the next it will, over as many of
        // the world's steps as it lets go by.
        let along = ((f64::from(beat.since) + blend.0) / f64::from(beat.every)).min(1.0);
        let at = body.prev + (body.pos - body.prev) * along;
        let sink = match z.state {
            State::Dead { t } => ((t - LIE_FOR) / SINK_FOR).clamp(0.0, 1.0) * 1.8,
            _ => 0.0,
        };
        let place = Mat4::from_translation(at - Vec3::new(0.0, sink, 0.0)) * Mat4::from_quat(Quat::from_rotation_y(z.yaw));
        figure.set(place, joints, points, !z.dead());
    }
}

/// The long dead, gone.
fn bury(mut commands: Commands, dead: Query<(Entity, &Zombie)>, mut horde: ResMut<Horde>) {
    for (e, z) in &dead {
        if matches!(z.state, State::Dead { t } if t > LIE_FOR + SINK_FOR) {
            commands.entity(e).despawn();
            horde.gone += 1;
        }
    }
}

pub fn install(fixed: &mut Schedule, frame: &mut Schedule) {
    fixed.add_systems(think);
    frame.add_systems((pose, bury));
}

/// Put a Shambler at `at`, facing `yaw`.
pub fn spawn(world: &mut World, at: Vec3, yaw: f64) {
    let seed = {
        let mut h = world.resource_mut::<Horde>();
        h.seed = h.seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        h.seed
    };
    world.spawn((Zombie::new(yaw, seed), Body::at(at), Figure::default(), Beat::new(seed % 64)));
}

/// Put a Shambler somewhere the player at `eye`, looking along `forward`,
/// can't see: on walkable ground, 35–60 m off, behind something or behind
/// them. Whether there was such a place.
pub fn spawn_unseen(world: &mut World, eye: Vec3, forward: Vec3) -> bool {
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
    spawn(world, feet, yaw);
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
    raycast_past(world, from, dir, max, &[])
}

/// The nearest Shambler along a ray within `max`, but for those `past`
/// (the ones a round has already gone through).
pub fn raycast_past(world: &mut World, from: Vec3, dir: Vec3, max: f64, past: &[Entity]) -> Option<(Entity, f64, bool)> {
    let mut best = None;
    for (e, f) in world.query::<(Entity, &Figure)>().iter(world) {
        if past.contains(&e) {
            continue;
        }
        if let Some((t, head)) = f.ray(from, dir, max)
            && best.is_none_or(|(_, b, _)| t < b)
        {
            best = Some((e, t, head));
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
    pub head: bool,
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
    let damage = if hit.takedown && z.unaware() { brain::HP * 10.0 } else { hit.damage };
    let killed = z.hurt(damage, hit.head, hit.blow, from);
    if hit.stumble && !killed {
        z.stumble();
    }
    let mut sounds = Vec::new();
    if killed {
        sounds.push(Sfx::Gurgle);
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

/// A noise at `at`, heard `range` metres off.
pub fn noise(world: &mut World, at: Vec3, range: f64) {
    world.resource_mut::<Noises>().shots.push((at, range));
}

#[cfg(test)]
mod bench;
#[cfg(test)]
mod model_tests;
#[cfg(test)]
mod tests;
