//! A strafing run: a strip of open ground marked, a plane comes in along
//! it a few seconds after, and its guns rake the strip from one end to
//! the other. Everything in the strip under open sky as the rounds pass
//! is hit: the dead cut down (the kills whoever called it in's), a player
//! badly hurt. Under a roof, nothing: the rounds stop on the roof.

use bevy_ecs::prelude::*;
use lntrn_math::Vec3;

use super::{Event, Support, looked_at, open_sky, under};
use crate::collide::Solids;
use crate::player::{Body, Player, STEP};
use crate::sound::Sfx;
use crate::throw::{Booms, SHAKE};
use crate::world::Solid;
use crate::zombie::{self, Horde, brain::Zombie};

/// The strip: how long and how wide.
pub const LONG: f64 = 30.0;
pub const WIDE: f64 = 5.0;
/// How long after it's marked the rounds begin, and how long they take to
/// go its length.
pub const WARNS: f64 = 3.0;
pub const RAKES: f64 = 1.0;
/// What a round does to one of the dead, and to a player.
pub const TO_THE_DEAD: f64 = 900.0;
pub const TO_A_PLAYER: f64 = 70.0;
/// Rounds seen to land each step (what's hit is everything in the strip,
/// whichever they land on); and how far off the rounds landing shake a
/// player.
const ROUNDS: usize = 3;
const SHAKES: f64 = 45.0;
/// The plane: how high it comes over, how fast, how far behind where its
/// rounds land it is, and how long after the last of them it's gone.
pub const HIGH: f64 = 24.0;
const FLIES: f64 = 95.0;
const BEHIND: f64 = 45.0;
const GONE: f64 = 5.0;

/// A strip of ground: its middle, the way it runs (flat), and whether
/// there's open sky over its middle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Strip {
    pub mid: Vec3,
    pub dir: Vec3,
    pub open: bool,
}

impl Strip {
    /// The strip marked from `eye`, looking along `look`: on the ground
    /// where the look lands (out ahead, looking at the sky), running
    /// straight away from the eye.
    pub fn marked(solid: &Solids, eye: Vec3, look: Vec3) -> Option<Strip> {
        let (mid, dir) = looked_at(solid, eye, look)?;
        Some(Strip { mid, dir, open: open_sky(solid, mid) })
    }

    /// Where `p` is in it: how far along from its start, and how far to
    /// the side of its middle line.
    pub fn place(&self, p: Vec3) -> (f64, f64) {
        let d = p - self.mid;
        (d.x * self.dir.x + d.z * self.dir.z + LONG * 0.5, d.x * self.dir.z - d.z * self.dir.x)
    }

    /// The point of it `along` from its start and `across` from its middle
    /// line, at its middle's height.
    pub fn point(&self, along: f64, across: f64) -> Vec3 {
        self.mid + self.dir * (along - LONG * 0.5) + Vec3::new(self.dir.z, 0.0, -self.dir.x) * across
    }

    /// What a round coming down on that point meets: the ground, or a
    /// roof.
    pub fn meets(&self, solid: &Solids, along: f64, across: f64) -> Option<Vec3> {
        under(solid, self.point(along, across))
    }
}

/// A run called in: its strip, whose it is, how long since it was marked,
/// and its luck (where each round lands).
#[derive(Component, Clone, Copy, Debug)]
pub struct Strafe {
    pub strip: Strip,
    pub by: usize,
    pub t: f64,
    seed: u32,
}

impl Strafe {
    /// How far along the strip the rounds have got (none yet: 0; past its
    /// end: its length).
    pub fn front(&self) -> f64 {
        ((self.t - WARNS) / RAKES).clamp(0.0, 1.0) * LONG
    }

    /// Whether its rounds are still to come, or coming: the strip's no
    /// place to stand.
    pub fn threatens(&self) -> bool {
        self.t < WARNS + RAKES
    }

    /// Where the plane is, and the way it flies.
    pub fn plane(&self) -> (Vec3, Vec3) {
        (self.strip.point(FLIES * (self.t - WARNS) - BEHIND, 0.0) + Vec3::new(0.0, HIGH, 0.0), self.strip.dir)
    }

    fn rand(&mut self) -> f64 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        f64::from(self.seed) / f64::from(u32::MAX)
    }
}

/// Player `by` calls a run in along `strip`.
pub fn call(world: &mut World, strip: Strip, by: usize) {
    let seed = (strip.mid.x.to_bits() as u32 ^ strip.mid.z.to_bits() as u32).wrapping_mul(0x9E37_79B9) | 1;
    world.spawn(Strafe { strip, by, t: 0.0, seed });
    world.resource_mut::<Horde>().sounds.push((Sfx::Jet, strip.mid + Vec3::new(0.0, 2.0, 0.0), 1.0));
}

/// A fixed step of every run.
pub(super) fn step(world: &mut World) {
    let mut runs: Vec<(Entity, Strafe)> = world.query::<(Entity, &Strafe)>().iter(world).map(|(e, s)| (e, *s)).collect();
    if runs.is_empty() {
        return;
    }
    let dead: Vec<(Entity, Vec3)> = world.query::<(Entity, &Zombie, &Body)>().iter(world).filter(|(_, z, _)| !z.dead()).map(|(e, _, b)| (e, b.pos)).collect();
    let players: Vec<(usize, Vec3)> = world.query::<(&Player, &Body)>().iter(world).map(|(p, b)| (p.0, b.pos)).collect();
    let mut sounds = Vec::new();
    let mut events = Vec::new();
    let mut struck: Vec<(usize, Entity, Vec3, Vec3)> = Vec::new();
    let mut hurt: Vec<usize> = Vec::new();
    let mut shook: Vec<(usize, f64)> = Vec::new();
    let mut gone = Vec::new();
    {
        let solid = &world.resource::<Solid>().0;
        for (e, s) in &mut runs {
            let before = s.front();
            let began = s.t < WARNS;
            s.t += STEP;
            let front = s.front();
            if s.t > WARNS + RAKES + GONE {
                gone.push(*e);
            }
            if front <= before {
                continue;
            }
            if began {
                sounds.push((Sfx::Brrt, s.strip.mid + Vec3::new(0.0, 2.0, 0.0), 1.0));
            }
            // The rounds seen to land this step, each from the plane's guns.
            let (plane, _) = s.plane();
            for _ in 0..ROUNDS {
                let (along, across) = (before + (front - before) * s.rand(), (s.rand() - 0.5) * WIDE);
                if let Some(at) = s.strip.meets(solid, along, across) {
                    events.push(Event::Round { from: at + (plane - at).normalize() * 22.0, at });
                }
            }
            // Everything in the stretch they've passed over, under open
            // sky: hit.
            let within = |p: Vec3| {
                let (along, across) = s.strip.place(p);
                along > before && along <= front && across.abs() <= WIDE * 0.5 && open_sky(solid, p + Vec3::new(0.0, 0.8, 0.0))
            };
            struck.extend(dead.iter().filter(|(_, p)| within(*p)).map(|&(z, p)| (s.by, z, p, s.strip.dir)));
            hurt.extend(players.iter().filter(|(_, p)| within(*p)).map(|(seat, _)| *seat));
            let at = s.strip.point(front, 0.0);
            shook.extend(players.iter().map(|&(seat, p)| (seat, (1.0 - (p - at).length() / SHAKES).max(0.0) * SHAKE * 0.6)));
        }
    }
    for (e, s) in runs {
        if let Some(mut now) = world.get_mut::<Strafe>(e) {
            *now = s;
        }
    }
    let mut kills = Vec::new();
    for (by, e, pos, dir) in struck {
        // (From above: `limb`, so no plate turns it.)
        let hit = zombie::Impact { damage: TO_THE_DEAD, head: false, limb: true, blow: false, shove: 6.0, stumble: true, takedown: false, fire: false, at: None };
        if let Some(mut z) = world.get_mut::<Zombie>(e) {
            z.by = Some(by);
        }
        if zombie::hurt(world, e, dir, pos - dir * 20.0 + Vec3::new(0.0, HIGH, 0.0), hit) {
            kills.push((by, e));
        }
    }
    let mut booms = world.resource_mut::<Booms>();
    for seat in hurt {
        booms.felt(seat).blasted += TO_A_PLAYER;
    }
    for (seat, shake) in shook {
        let felt = booms.felt(seat);
        felt.shake = felt.shake.max(shake);
    }
    for e in gone {
        world.despawn(e);
    }
    world.resource_mut::<Horde>().sounds.extend(sounds);
    let mut support = world.resource_mut::<Support>();
    support.events.extend(events);
    support.kills.extend(kills);
}
