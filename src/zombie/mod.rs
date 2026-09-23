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
/// How hard a shot and a blow shove one, m/s (a blow sends it about a
/// metre back).
const SHOT_SHOVE: f64 = 0.6;
const BLOW_SHOVE: f64 = 7.0;
/// Where a new one comes from: this far off, and out of sight.
const SPAWN_NEAR: f64 = 35.0;
const SPAWN_FAR: f64 = 60.0;

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
}

fn think(mut dead: Query<(&mut Zombie, &mut Body), Without<Player>>, players: Query<&Body, With<Player>>, solid: Res<Solid>, nav: Res<Nav>, mut noises: ResMut<Noises>, mut horde: ResMut<Horde>) {
    let player = players.iter().next().map(|b| b.pos);
    let heard = std::mem::take(&mut *noises);
    let searches = std::cell::Cell::new(SEARCHES);
    let senses = Senses { solids: &solid.0, nav: nav.0.as_ref(), player, noises: &heard.shots, alerts: &heard.snarls, searches: &searches };
    for (mut z, mut body) in &mut dead {
        let mut intent = z.think(&body, &senses, STEP);
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
        player::step_body(&mut body, z.yaw, &mut intent.controls, &solid.0, &gait, STEP);
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

/// The living dead nudge each other apart (a shove, so walls still stop
/// them), and a crowd spreads round the player instead of stacking.
fn elbow(dead: &mut Query<(&mut Zombie, &mut Body), Without<Player>>) {
    let at: Vec<Option<Vec3>> = dead.iter().map(|(z, b)| (!z.dead()).then_some(b.pos)).collect();
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
    for ((_, mut body), nudge) in dead.iter_mut().zip(nudges) {
        if nudge != Vec3::ZERO {
            body.push += nudge;
        }
    }
}

/// Pose each one for this frame: its animation, where it stands between
/// steps, and sinking once it has lain long enough.
fn pose(model: Option<Res<Model>>, blend: Res<Blend>, mut dead: Query<(&Zombie, &Body, &mut Figure)>) {
    let Some(model) = model else { return };
    for (z, body, mut figure) in &mut dead {
        let (clip, t) = z.clip();
        let (joints, points) = model.pose(clip, t);
        let at = body.prev + (body.pos - body.prev) * blend.0;
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
    world.spawn((Zombie::new(yaw, seed), Body::at(at), Figure::default()));
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
            let p = Vec3::new(eye.x + a.cos() * r, 0.0, eye.z + a.sin() * r);
            let Some(h) = nav.height_at(p) else { continue };
            let feet = Vec3::new(p.x, h, p.z);
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
pub fn raycast(world: &mut World, from: Vec3, dir: Vec3, max: f64) -> Option<(Entity, f64, bool)> {
    let mut best = None;
    for (e, f) in world.query::<(Entity, &Figure)>().iter(world) {
        if let Some((t, head)) = f.ray(from, dir, max)
            && best.is_none_or(|(_, b, _)| t < b)
        {
            best = Some((e, t, head));
        }
    }
    best
}

/// Hurt the Shambler `e` with a hit along `dir` from `from`. Whether it died.
pub fn hurt(world: &mut World, e: Entity, dir: Vec3, from: Vec3, damage: f64, head: bool, blow: bool) -> bool {
    let Some(mut z) = world.get_mut::<Zombie>(e) else { return false };
    let killed = z.hurt(damage, head, blow, from);
    let mut sounds = Vec::new();
    if killed {
        sounds.push(Sfx::Gurgle);
    }
    // (Shot from straight above, it isn't shoved at all.)
    let flat = Vec3::new(dir.x, 0.0, dir.z);
    let push = if flat.length() > 1e-6 { flat.normalize() * if blow { BLOW_SHOVE } else { SHOT_SHOVE } } else { Vec3::ZERO };
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
