//! A mystery drop's gun, over the crate it came in: for a moment every gun
//! it could be flicks past, slower and slower, and then it's the one it
//! is, turning there to be taken (by whoever looks at it from within
//! reach) till the crate's about gone; in its last seconds it blinks.
//! Which gun it is, and what taking it does, is a holdout's
//! (`holdout/mystery.rs`); how it's drawn is in `draw/prize.rs`.

use bevy_ecs::prelude::*;
use lntrn_math::Vec3;

use super::{Event, STANDS, Support};
use crate::loot::Stack;
use crate::player::STEP;
use crate::sound::Sfx;
use crate::world::Solid;
use crate::zombie::Horde;

/// How long the guns flick past before it's the one it is, how many go
/// by, and how much slower the last of them than the first (the power the
/// time's raised to).
pub const SHUFFLES: f64 = 2.5;
pub const FLICKS: u32 = 16;
const SLOWS: f64 = 1.6;
/// How long it's there in all (its crate sinks away just after), and its
/// last seconds, when it blinks: so often at first, and at the very end.
pub const LASTS: f64 = STANDS - 1.5;
const BLINKS: f64 = 6.0;
const BLINK_RATE: (f64, f64) = (2.0, 7.0);
/// How high over its crate's foot it turns.
pub const OVER: f64 = 1.15;
/// How far away it can be taken from, and how near the middle of the view
/// it must be (the cosine of the angle off it).
const REACH: f64 = 2.8;
const AIM: f64 = 0.93;

/// When the `k`th gun flicks past, seconds into the shuffle.
pub fn flick_at(k: u32) -> f64 {
    SHUFFLES * (f64::from(k) / f64::from(FLICKS)).powf(SLOWS)
}

/// The gun: which, whose call it was (their seat), where it turns, and
/// how long it's been there.
#[derive(Component, Clone, Copy, Debug)]
pub struct Prize {
    pub gun: Stack,
    pub by: usize,
    pub at: Vec3,
    pub age: f64,
}

impl Prize {
    /// Whether it's the gun it is yet: there to be taken.
    pub fn settled(&self) -> bool {
        self.age >= SHUFFLES
    }

    /// How many guns have flicked past so far.
    pub fn flick(&self) -> u32 {
        ((self.age / SHUFFLES).clamp(0.0, 1.0).powf(1.0 / SLOWS) * f64::from(FLICKS)) as u32
    }

    /// Whether it's to be seen just now (in its last seconds it's there,
    /// then not, quicker and quicker).
    pub fn shows(&self) -> bool {
        let left = LASTS - self.age;
        let rate = BLINK_RATE.1 + (BLINK_RATE.0 - BLINK_RATE.1) * (left / BLINKS).clamp(0.0, 1.0);
        left > BLINKS || (left * rate).fract() < 0.6
    }
}

/// A mystery drop's crate is down at `at`: `gun`, over it, the others
/// flicking past first. It was player `by`'s call.
pub fn set(world: &mut World, gun: Stack, at: Vec3, by: usize) {
    let at = at + Vec3::new(0.0, OVER, 0.0);
    world.spawn(Prize { gun, by, at, age: 0.0 });
    world.resource_mut::<Horde>().sounds.push((Sfx::Mystery, at, 1.0));
}

/// A fixed step of every one.
pub(super) fn step(world: &mut World) {
    let (mut sounds, mut events, mut gone) = (Vec::new(), Vec::new(), Vec::new());
    for (e, mut p) in world.query::<(Entity, &mut Prize)>().iter_mut(world) {
        let was = p.settled();
        p.age += STEP;
        // It's the gun it is: whoever called for it is told, and it's
        // heard (one that's been amplified, as the Amplifier is).
        if !was && p.settled() {
            events.push(Event::Won { by: p.by, gun: p.gun });
            sounds.push((Sfx::Prize, p.at, 1.0));
            if p.gun.tier > 0 {
                sounds.push((Sfx::Amplify, p.at, 0.9));
            }
        }
        if p.age >= LASTS {
            gone.push(e);
        }
    }
    for e in gone {
        world.despawn(e);
    }
    world.resource_mut::<Horde>().sounds.extend(sounds);
    world.resource_mut::<Support>().events.extend(events);
}

/// The gun the eye at `eye`, looking along `dir`, could take: the nearest
/// that's settled, within reach, near the middle of the view, nothing
/// solid in between.
pub fn in_view(world: &mut World, eye: Vec3, dir: Vec3) -> Option<(Entity, Stack)> {
    let mut best: Option<(Entity, Prize, f64)> = None;
    for (e, p) in world.query::<(Entity, &Prize)>().iter(world).filter(|(_, p)| p.settled()) {
        let to = p.at - eye;
        let d = to.length();
        if !(1e-6..=REACH).contains(&d) || to.dot(dir) / d < AIM {
            continue;
        }
        if best.is_none_or(|(_, _, b)| d < b) {
            best = Some((e, *p, d));
        }
    }
    let (e, p, d) = best?;
    let blocked = world.resource::<Solid>().0.raycast(eye, (p.at - eye) * (1.0 / d), d - 0.1).is_some();
    (!blocked).then_some((e, p.gun))
}
