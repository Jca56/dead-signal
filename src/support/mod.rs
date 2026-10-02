//! What the radio calls down, the world's side of it, a fixed step at a
//! time. A drop: a flare thrown to mark where (it flies, bounces to a
//! stop and burns there, red smoke going up from it), and, if there's
//! open sky over it, a crate that comes down on it under a parachute and
//! opens; under a roof the flare gutters out and nothing comes. A strafing
//! run (`strafe.rs`): a strip of ground raked by a plane's guns. The
//! gunship (`gunship.rs`): a helicopter circling the compound, its gun on
//! the dead in the open. What it
//! means to the players (what the crate held, the signal back for a flare
//! that guttered) is the run's: it's told through [`Support`], as is
//! what's called for that needs no marking (a boost). What's
//! seen of it is in `draw.rs`.

pub mod draw;
pub mod gunship;
pub mod strafe;
#[cfg(test)]
mod tests;

use bevy_ecs::prelude::*;
use lntrn_math::Vec3;

use crate::player::STEP;
use crate::radio::codes::Call;
use crate::sound::Sfx;
use crate::world::Solid;
use crate::zombie::Horde;

const GRAVITY: f64 = 9.8;
/// A flare bouncing: how much of its speed it keeps, and slower than
/// this, on the ground, it's come to rest.
const BOUNCE: f64 = 0.3;
const RESTS: f64 = 1.2;
/// How high the sky's looked for over a flare; how long one under a roof
/// burns before it gutters out; and one that never comes to rest, before
/// it's given up on.
const SKY: f64 = 80.0;
const GUTTERS: f64 = 1.5;
const LOST: f64 = 10.0;
/// A drop: how long after its flare's come to rest it's let go overhead,
/// how high, and how fast it comes down.
pub const COMES_IN: f64 = 2.5;
pub const FROM: f64 = 36.0;
pub const FALLS: f64 = 6.0;
/// How long the flare burns on after the crate's down, and the empty
/// crate stands there; and how long its parachute takes to fall in a
/// heap.
const BURNS_ON: f64 = 3.0;
pub const STANDS: f64 = 30.0;
pub const SETTLES: f64 = 1.4;

/// A flare, thrown for a drop: in the air, or come to rest and burning.
#[derive(Component, Clone, Copy, Debug)]
pub struct Flare {
    pub call: Call,
    /// Who threw it (their seat).
    pub by: usize,
    pub pos: Vec3,
    pub prev: Vec3,
    vel: Vec3,
    /// How far it's turned end over end, and how long since it was thrown.
    pub spin: f64,
    pub age: f64,
    /// How long it's been at rest, once it is; whether there's sky over
    /// it there; and whether its drop's been let go.
    pub rest: Option<f64>,
    pub sky: bool,
    sent: bool,
    crackle_in: f64,
}

/// A crate coming down under its parachute, or down and open.
#[derive(Component, Clone, Copy, Debug)]
pub struct Drop {
    pub call: Call,
    pub by: usize,
    /// Where it lands; how high over that it is, and was a step ago.
    pub at: Vec3,
    pub height: f64,
    pub prev: f64,
    pub age: f64,
    /// How long it's been down, once it is.
    pub down: Option<f64>,
}

/// What came of it, for the run.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// Something that needs no marking was called for, by player `by`:
    /// it's the run's to begin (a boost).
    Called { call: Call, by: usize },
    /// A flare guttered out with nothing sent (a roof over it, or it was
    /// lost): what was called for can't come.
    Guttered { call: Call, by: usize },
    /// A crate's down, at `at`.
    Landed { call: Call, by: usize, at: Vec3 },
    /// One of a strafing run's rounds (or the gunship's) landed at `at`,
    /// come from `from`.
    Round { from: Vec3, at: Vec3 },
}

/// What's come of what was called down since the run last asked.
#[derive(Resource, Default)]
pub struct Support {
    pub events: Vec<Event>,
    /// The dead it killed, each whoever called it in's (their seat).
    pub kills: Vec<(usize, Entity)>,
}

/// Player `by` called for `call`, which needs no marking.
pub fn called(world: &mut World, call: Call, by: usize) {
    world.resource_mut::<Support>().events.push(Event::Called { call, by });
}

/// Player `by` throws a flare for `call` from `from` along the arc `vel`.
pub fn throw_flare(world: &mut World, call: Call, from: Vec3, vel: Vec3, by: usize) {
    world.spawn(Flare { call, by, pos: from, prev: from, vel, spin: 0.0, age: 0.0, rest: None, sky: false, sent: false, crackle_in: 0.0 });
}

/// (For a look at one.) A crate for `call` over `at`, `height` up (down
/// already, at none), and a flare burning there this past while.
#[cfg(test)]
pub fn drop_at(world: &mut World, call: Call, at: Vec3, height: f64) {
    world.spawn(Drop { call, by: 0, at, height, prev: height, age: 3.0, down: (height <= 0.0).then_some(0.0) });
    world.spawn(Flare { call, by: 0, pos: at + Vec3::new(0.0, 0.04, 0.0), prev: at, vel: Vec3::ZERO, spin: 0.0, age: 9.0, rest: Some(7.0), sky: true, sent: true, crackle_in: 0.0 });
}

/// Everything called down, gone (a new run).
pub fn clear(world: &mut World) {
    let all: Vec<Entity> = world.query_filtered::<Entity, Or<(With<Flare>, With<Drop>, With<strafe::Strafe>, With<gunship::Gunship>)>>().iter(world).collect();
    for e in all {
        world.despawn(e);
    }
    if let Some(mut s) = world.get_resource_mut::<Support>() {
        s.events.clear();
        s.kills.clear();
    }
}

/// A fixed step of it all.
pub fn step(world: &mut World) {
    strafe::step(world);
    gunship::step(world);
    let mut sounds: Vec<(Sfx, Vec3, f32)> = Vec::new();
    let mut events = Vec::new();
    let mut gone = Vec::new();
    let mut sent = Vec::new();
    let mut flares: Vec<(Entity, Flare)> = world.query::<(Entity, &Flare)>().iter(world).map(|(e, f)| (e, *f)).collect();
    {
        let solid = &world.resource::<Solid>().0;
        for (e, f) in &mut flares {
            f.prev = f.pos;
            f.age += STEP;
            f.crackle_in -= STEP;
            if f.crackle_in <= 0.0 {
                f.crackle_in = 0.55;
                sounds.push((Sfx::Crackle, f.pos, 0.35));
            }
            let Some(rest) = &mut f.rest else {
                f.vel.y -= GRAVITY * STEP;
                f.spin += STEP * 10.0;
                let step = f.vel * STEP;
                let len = step.length();
                match solid.raycast(f.pos, step * (1.0 / len.max(1e-9)), len) {
                    Some(h) => {
                        let n = h.normal;
                        f.vel = (f.vel - n * (2.0 * f.vel.dot(n))) * BOUNCE;
                        f.pos = h.point + n * 0.04;
                        sounds.push((Sfx::Clank, h.point, 0.3));
                        if n.y > 0.5 && f.vel.length() < RESTS {
                            f.vel = Vec3::ZERO;
                            f.rest = Some(0.0);
                            f.sky = solid.raycast(f.pos + Vec3::new(0.0, 0.3, 0.0), Vec3::Y, SKY).is_none();
                        }
                    }
                    None => f.pos += step,
                }
                // (Thrown off the world, or into somewhere it never
                // settles: lost.)
                if f.age > LOST {
                    events.push(Event::Guttered { call: f.call, by: f.by });
                    gone.push(*e);
                }
                continue;
            };
            *rest += STEP;
            if !f.sky {
                if *rest >= GUTTERS {
                    events.push(Event::Guttered { call: f.call, by: f.by });
                    gone.push(*e);
                }
            } else if !f.sent && *rest >= COMES_IN {
                f.sent = true;
                sent.push(Drop { call: f.call, by: f.by, at: f.pos - Vec3::new(0.0, 0.04, 0.0), height: FROM, prev: FROM, age: 0.0, down: None });
                sounds.push((Sfx::Flyover, f.pos, 1.0));
            } else if *rest >= COMES_IN + FROM / FALLS + BURNS_ON {
                gone.push(*e);
            }
        }
    }
    for (e, f) in flares {
        if let Some(mut now) = world.get_mut::<Flare>(e) {
            *now = f;
        }
    }
    for mut d in world.query::<&mut Drop>().iter_mut(world) {
        d.age += STEP;
        d.prev = d.height;
        match &mut d.down {
            Some(t) => *t += STEP,
            None => {
                d.height = (d.height - FALLS * STEP).max(0.0);
                if d.height <= 0.0 {
                    d.down = Some(0.0);
                    events.push(Event::Landed { call: d.call, by: d.by, at: d.at });
                    sounds.push((Sfx::Thud, d.at, 1.0));
                }
            }
        }
    }
    let stood: Vec<Entity> = world.query::<(Entity, &Drop)>().iter(world).filter(|(_, d)| d.down.is_some_and(|t| t >= STANDS)).map(|(e, _)| e).collect();
    for e in gone.into_iter().chain(stood) {
        world.despawn(e);
    }
    for d in sent {
        world.spawn(d);
    }
    world.resource_mut::<Horde>().sounds.extend(sounds);
    world.resource_mut::<Support>().events.extend(events);
}
