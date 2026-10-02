//! The gunship: a helicopter that comes in over the compound, flies a
//! slow circle round it for a while, and with the gun in its door cuts
//! down the dead out in the open that it can see, those nearest the
//! players first, its searchlight on whichever it's at. Under a roof
//! they're safe from it; the players always are. What it kills is
//! whoever called it in's. Called for again while it's up, it stays as
//! long again. Then it flies off.

use bevy_ecs::prelude::*;
use lntrn_math::Vec3;

use super::{Event, Support};
use crate::player::{Body, Player, STEP};
use crate::sound::Sfx;
use crate::world::Solid;
use crate::zombie::{self, Horde, brain::Zombie};

/// How long it takes to come in, how long it stays, and to fly off.
pub const ARRIVES: f64 = 4.0;
pub const STAYS: f64 = 45.0;
const LEAVES: f64 = 5.0;
/// Its circle: how high over the compound's middle, how long a lap takes,
/// and how far out it comes in from (and goes off to).
pub const HIGH: f64 = 20.0;
const LAP: f64 = 26.0;
const FROM: f64 = 170.0;
/// Its gun: a round this often, what one does to the dead, how far it
/// reaches, and how often it looks for something nearer the players.
pub const EVERY: f64 = 0.08;
pub const ROUND: f64 = 60.0;
const REACH: f64 = 95.0;
const LOOKS: f64 = 0.3;
/// How high the sky's looked for over one of the dead, and how wide of
/// its mark a round lands.
const SKY: f64 = 80.0;
const WIDE: f64 = 0.55;
/// A beat of its rotor is heard this often; its gun, every so many
/// rounds.
const BEATS: f64 = 0.21;
const HEARD: u32 = 2;

#[derive(Component, Clone, Copy, Debug)]
pub struct Gunship {
    /// Who called it in (their seat).
    pub by: usize,
    /// The middle of its circle (on the ground), and how far out the
    /// circle reaches, east and west and north and south.
    pub over: Vec3,
    pub out: (f64, f64),
    /// How long since it was called for, and when it's to go.
    pub t: f64,
    pub until: f64,
    /// What it's shooting at, and where that is (its light's on it).
    target: Option<Entity>,
    pub lit: Option<Vec3>,
    /// Till its next round, its next look about, and its rotor's next
    /// beat; rounds fired; its luck.
    round_in: f64,
    look_in: f64,
    beat_in: f64,
    rounds: u32,
    seed: u32,
}

impl Gunship {
    /// Whether it's over the compound, its gun to hand (not still coming
    /// in, nor gone off).
    pub fn on_station(&self) -> bool {
        self.t >= ARRIVES && self.t < self.until
    }

    /// How long it has left over the compound.
    pub fn left(&self) -> f64 {
        (self.until - self.t.max(ARRIVES)).max(0.0)
    }

    /// Where on its circle it is at `t`, and the way it's flying there.
    fn circle(&self, t: f64) -> (Vec3, Vec3) {
        let a = t / LAP * std::f64::consts::TAU;
        let at = self.over + Vec3::new(a.cos() * self.out.0, HIGH, a.sin() * self.out.1);
        (at, Vec3::new(-a.sin() * self.out.0, 0.0, a.cos() * self.out.1).normalize())
    }

    /// Where it is, and the way it's flying: in along the line of its
    /// circle, round it, and off along it.
    pub fn place(&self) -> (Vec3, Vec3) {
        if self.t < ARRIVES {
            let (at, way) = self.circle(ARRIVES);
            let short = 1.0 - self.t / ARRIVES;
            (at - way * (FROM * short * short) + Vec3::new(0.0, 14.0 * short, 0.0), way)
        } else if self.t < self.until {
            self.circle(self.t)
        } else {
            let (at, way) = self.circle(self.until);
            let gone = ((self.t - self.until) / LEAVES).min(1.0);
            (at + way * (FROM * gone * gone) + Vec3::new(0.0, 14.0 * gone, 0.0), way)
        }
    }

    /// Where its gun is: in its door, on the side that's to the middle of
    /// its circle.
    pub fn gun(&self) -> Vec3 {
        let (at, way) = self.place();
        at + Vec3::new(way.z, 0.0, -way.x) * -1.1 - Vec3::new(0.0, 0.5, 0.0)
    }

    /// Whether its gun's firing just now.
    pub fn firing(&self) -> bool {
        self.on_station() && self.target.is_some()
    }

    fn rand(&mut self) -> f64 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        f64::from(self.seed) / f64::from(u32::MAX)
    }
}

/// Player `by` calls the gunship in over the compound between `lo` and
/// `hi`. One up already stays as long again instead.
pub fn call(world: &mut World, by: usize, (lo, hi): (Vec3, Vec3)) {
    if let Some(mut up) = world.query::<&mut Gunship>().iter_mut(world).next() {
        up.until = up.until.max(up.t) + STAYS;
        return;
    }
    let over = Vec3::new((lo.x + hi.x) * 0.5, lo.y.max(hi.y), (lo.z + hi.z) * 0.5);
    // (Its circle well inside the compound's long side, and out over its
    // short one: it's seen from anywhere in it.)
    let out = (((hi.x - lo.x) * 0.27).max(16.0), ((hi.z - lo.z) * 0.45 + 6.0).max(16.0));
    world.spawn(Gunship { by, over, out, t: 0.0, until: ARRIVES + STAYS, target: None, lit: None, round_in: 0.0, look_in: 0.0, beat_in: 0.0, rounds: 0, seed: 0x51E7_A3C9 });
}

/// How long the gunship has left over the compound, if it's up (or on
/// its way).
pub fn left(world: &mut World) -> f64 {
    world.query::<&Gunship>().iter(world).map(Gunship::left).fold(0.0, f64::max)
}

/// A fixed step of it.
pub(super) fn step(world: &mut World) {
    let Some((e, mut g)) = world.query::<(Entity, &Gunship)>().iter(world).next().map(|(e, g)| (e, *g)) else { return };
    g.t += STEP;
    let mut sounds = Vec::new();
    let (at, _) = g.place();
    g.beat_in -= STEP;
    if g.beat_in <= 0.0 {
        g.beat_in = BEATS;
        sounds.push((Sfx::Rotor, at, 1.0));
    }
    let mut round = None;
    if g.on_station() {
        let gun = g.gun();
        let dead: Vec<(Entity, Vec3)> = world.query::<(Entity, &Zombie, &Body)>().iter(world).filter(|(_, z, _)| !z.dead()).map(|(e, _, b)| (e, b.pos)).collect();
        let players: Vec<Vec3> = world.query::<(&Player, &Body)>().iter(world).map(|(_, b)| b.pos).collect();
        let solid = &world.resource::<Solid>().0;
        // What it can shoot: out in the open, in reach, nothing between.
        let seen = |p: Vec3| {
            let chest = p + Vec3::new(0.0, 1.0, 0.0);
            let to = chest - gun;
            to.length() < REACH && solid.raycast(chest, Vec3::Y, SKY).is_none() && solid.raycast(gun, to.normalize(), to.length() - 0.3).is_none()
        };
        // It looks about now and then for the one nearest a player; and
        // at once, when what it was at is down or out of sight.
        g.look_in -= STEP;
        let still = g.target.and_then(|t| dead.iter().find(|(e, _)| *e == t)).filter(|(_, p)| seen(*p)).copied();
        let aim = if still.is_none() || g.look_in <= 0.0 {
            g.look_in = LOOKS;
            let near = |p: Vec3| players.iter().map(|q| (*q - p).length()).fold(f64::INFINITY, f64::min);
            dead.iter().filter(|(_, p)| seen(*p)).min_by(|a, b| near(a.1).total_cmp(&near(b.1))).copied()
        } else {
            still
        };
        g.target = aim.map(|(e, _)| e);
        g.lit = aim.map(|(_, p)| p);
        g.round_in -= STEP;
        if let Some((target, p)) = aim.filter(|_| g.round_in <= 0.0) {
            g.round_in = EVERY;
            g.rounds += 1;
            // Where it's seen to land: about its mark's feet.
            let a = g.rand() * std::f64::consts::TAU;
            let wide = g.rand() * WIDE;
            let lands = solid.raycast(p + Vec3::new(a.cos() * wide, 1.5, a.sin() * wide), -Vec3::Y, 6.0).map_or(p, |h| h.point);
            round = Some((target, p, gun, lands));
            if g.rounds % HEARD == 0 {
                sounds.push((Sfx::LmgShot, gun, 1.0));
            }
        }
    } else {
        (g.target, g.lit) = (None, None);
    }
    let gone = g.t >= g.until + LEAVES;
    if let Some(mut now) = world.get_mut::<Gunship>(e) {
        *now = g;
    }
    if gone {
        world.despawn(e);
    }
    world.resource_mut::<Horde>().sounds.extend(sounds);
    let Some((target, p, gun, lands)) = round else { return };
    let hit = zombie::Impact { damage: ROUND, head: false, limb: false, blow: false, shove: 1.5, stumble: false, takedown: false, fire: false, at: None };
    if let Some(mut z) = world.get_mut::<Zombie>(target) {
        z.by = Some(g.by);
    }
    let killed = zombie::hurt(world, target, (p - gun).normalize(), gun, hit);
    let mut support = world.resource_mut::<Support>();
    support.events.push(Event::Round { from: gun, at: lands });
    if killed {
        support.kills.push((g.by, target));
    }
}
