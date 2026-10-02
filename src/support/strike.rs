//! A precision strike: a spot of open ground marked, and a moment after
//! one heavy shell comes straight down on it. No fire, no blast: a blow.
//! Everything within its circle under open sky is struck (hardest at the
//! middle) and flung outward: the dead killed, whoever called it's; a
//! player badly hurt. Under a roof, nothing.

use bevy_ecs::prelude::*;
use lntrn_math::Vec3;

use super::{Event, Support, looked_at, open_sky, under};
use crate::collide::Solids;
use crate::player::{Body, Player, STEP};
use crate::sound::Sfx;
use crate::throw::{Booms, SHAKE};
use crate::world::Solid;
use crate::zombie::{self, Horde, brain::Zombie};

/// How far from its middle it reaches, and how far up or down from it.
pub const RADIUS: f64 = 5.0;
const LEVEL: f64 = 3.0;
/// How long after it's marked the shell lands.
pub const WARNS: f64 = 2.0;
/// What it does to one of the dead, and to a player: at its middle, and
/// at its edge.
pub const TO_THE_DEAD: (f64, f64) = (1200.0, 500.0);
pub const TO_A_PLAYER: (f64, f64) = (80.0, 30.0);
/// How hard it flings the dead (m/s, at its middle and its edge), and how
/// far off it shakes a player.
const FLINGS: (f64, f64) = (16.0, 6.0);
const SHAKES: f64 = 60.0;
/// The shell: for how long it's seen coming down, and from how high; and
/// how long the dust hangs after.
pub const FALLS: f64 = 0.3;
pub const FROM: f64 = 75.0;
pub const SETTLES: f64 = 1.6;

/// A spot of ground: its middle, and whether there's open sky over it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spot {
    pub at: Vec3,
    pub open: bool,
}

impl Spot {
    /// The spot marked from `eye`, looking along `look`: the ground where
    /// the look lands.
    pub fn marked(solid: &Solids, eye: Vec3, look: Vec3) -> Option<Spot> {
        let (at, _) = looked_at(solid, eye, look)?;
        Some(Spot { at, open: open_sky(solid, at) })
    }

    /// The ground under the point `off` from its middle (flat), or what's
    /// over it.
    pub fn ground(&self, solid: &Solids, off: (f64, f64)) -> Option<Vec3> {
        under(solid, self.at + Vec3::new(off.0, 0.0, off.1))
    }

    /// How near its middle `p` is, from 1 (at it) to 0 (at its edge); none
    /// outside it.
    pub fn share(&self, p: Vec3) -> Option<f64> {
        let d = p - self.at;
        let flat = d.x.hypot(d.z);
        (flat <= RADIUS && d.y.abs() <= LEVEL).then(|| 1.0 - flat / RADIUS)
    }
}

/// A strike called in: its spot, whose it is, and how long since it was
/// marked.
#[derive(Component, Clone, Copy, Debug)]
pub struct Strike {
    pub spot: Spot,
    pub by: usize,
    pub t: f64,
}

impl Strike {
    /// Whether its shell's still to land: the circle's no place to stand.
    pub fn threatens(&self) -> bool {
        self.t < WARNS
    }

    /// How high over its spot the shell is, while it's seen coming down.
    pub fn shell(&self) -> Option<f64> {
        let left = WARNS - self.t;
        (left > 0.0 && left <= FALLS).then(|| FROM * left / FALLS)
    }

    /// How long since it landed, once it has.
    pub fn landed(&self) -> Option<f64> {
        (self.t >= WARNS).then_some(self.t - WARNS)
    }
}

/// Player `by` calls a strike in on `spot`.
pub fn call(world: &mut World, spot: Spot, by: usize) {
    world.spawn(Strike { spot, by, t: 0.0 });
    world.resource_mut::<Horde>().sounds.push((Sfx::Whistle, spot.at + Vec3::new(0.0, 2.0, 0.0), 1.0));
}

/// A fixed step of every strike.
pub(super) fn step(world: &mut World) {
    let mut strikes: Vec<(Entity, Strike)> = world.query::<(Entity, &Strike)>().iter(world).map(|(e, s)| (e, *s)).collect();
    if strikes.is_empty() {
        return;
    }
    let mut landed = Vec::new();
    let mut gone = Vec::new();
    for (e, s) in &mut strikes {
        let before = s.t;
        s.t += STEP;
        if before < WARNS && s.t >= WARNS {
            landed.push(*s);
        }
        if s.t > WARNS + SETTLES {
            gone.push(*e);
        }
    }
    for (e, s) in strikes {
        if let Some(mut now) = world.get_mut::<Strike>(e) {
            *now = s;
        }
    }
    for e in gone {
        world.despawn(e);
    }
    for s in landed {
        let dead: Vec<(Entity, Vec3)> = world.query::<(Entity, &Zombie, &Body)>().iter(world).filter(|(_, z, _)| !z.dead()).map(|(e, _, b)| (e, b.pos)).collect();
        let players: Vec<(usize, Vec3)> = world.query::<(&Player, &Body)>().iter(world).map(|(p, b)| (p.0, b.pos)).collect();
        // What's in its circle, in the open: how near its middle each is.
        let solid = &world.resource::<Solid>().0;
        let within = |p: Vec3| s.spot.share(p).filter(|_| open_sky(solid, p + Vec3::new(0.0, 0.8, 0.0)));
        let struck: Vec<(Entity, Vec3, f64)> = dead.iter().filter_map(|&(e, p)| within(p).map(|k| (e, p, k))).collect();
        let hurt: Vec<(usize, f64)> = players.iter().filter_map(|&(seat, p)| within(p).map(|k| (seat, k))).collect();
        let mut kills = Vec::new();
        for (e, pos, k) in struck {
            // (From above: `limb`, so no plate turns it.) Flung outward.
            let out = Vec3::new(pos.x - s.spot.at.x, 0.0, pos.z - s.spot.at.z).try_normalize().unwrap_or(Vec3::X);
            let hit = zombie::Impact { damage: TO_THE_DEAD.1 + (TO_THE_DEAD.0 - TO_THE_DEAD.1) * k, head: false, limb: true, blow: true, shove: FLINGS.1 + (FLINGS.0 - FLINGS.1) * k, stumble: true, takedown: false, fire: false, at: None };
            if let Some(mut z) = world.get_mut::<Zombie>(e) {
                z.by = Some(s.by);
            }
            if zombie::hurt(world, e, out, s.spot.at + Vec3::new(0.0, 1.0, 0.0), hit) {
                kills.push((s.by, e));
            }
        }
        let mut booms = world.resource_mut::<Booms>();
        for (seat, k) in hurt {
            booms.felt(seat).blasted += TO_A_PLAYER.1 + (TO_A_PLAYER.0 - TO_A_PLAYER.1) * k;
        }
        for (seat, p) in players {
            let felt = booms.felt(seat);
            felt.shake = felt.shake.max((1.0 - (p - s.spot.at).length() / SHAKES).max(0.0) * SHAKE * 1.2);
        }
        world.resource_mut::<Horde>().sounds.push((Sfx::Impact, s.spot.at + Vec3::new(0.0, 1.0, 0.0), 1.0));
        zombie::noise(world, s.spot.at, 120.0);
        let mut support = world.resource_mut::<Support>();
        support.events.push(Event::Impact { at: s.spot.at });
        support.kills.extend(kills);
    }
}
