//! What the player throws, and what comes of it, a fixed step at a time:
//! a Molotov flies till it meets something and bursts into a pool of fire
//! (the dead that walk through it catch, and burn on a while; the player
//! it scorches); a pipe bomb bounces to a stop, beeping ever quicker, every
//! one of the dead near enough drawn to it, then blows. A flamethrower's
//! stream (`flame.rs`) burns the same way, and a player it catches burns
//! on too. The world's side of it; what the player sees is in `draw.rs`,
//! the throwing in the run.

pub mod draw;
pub mod flame;

use bevy_ecs::prelude::*;
use lntrn_math::{Vec2, Vec3};

use crate::loot::Kind;
use crate::player::{Body, Player, STEP};
use crate::sound::Sfx;
use crate::world::Solid;
use crate::zombie::{self, Horde, brain::Zombie};

/// How fast a throw leaves the hand, m/s, and how far up it's aimed from
/// where the eye looks, degrees.
pub const THROW_SPEED: f64 = 17.0;
pub const THROW_UP: f64 = 8.0;
const GRAVITY: f64 = 9.8;
/// A Molotov's fire: how wide, how long, the last of that dying down;
/// what it does a second to the dead and to the player; how long the dead
/// burn on after.
const FIRE_RADIUS: f64 = 3.0;
const FIRE_FOR: f64 = 8.0;
pub const FIRE_DYING: f64 = 1.5;
const BURN_DEAD: f64 = 30.0;
const BURN_PLAYER: f64 = 15.0;
const BURNS_ON: f64 = 3.0;
/// How long a player a hound's bitten burns.
const BITE_BURNS: f64 = 1.2;
/// The fire a hound leaves where it falls: how wide, how long.
const PYRE: (f64, f64) = (1.1, 3.0);
/// A pipe bomb: its fuse from the throw, how far off the dead come to it,
/// how far its blast reaches and how hard (the dead, the player), and how
/// far off it's heard.
const FUSE: f64 = 10.0;
pub const LURE: f64 = 40.0;
const BLAST: f64 = 6.0;
const BLAST_DEAD: f64 = 500.0;
const BLAST_PLAYER: f64 = 80.0;
const BLAST_HEARD: f64 = 150.0;
/// How far off a blast still shakes the player.
const SHAKES: f64 = 45.0;
/// The hardest a blast shakes a player, metres (right beside it).
pub const SHAKE: f64 = 0.08;
/// Bouncing: how much of its speed it keeps, and slower than this, on the
/// ground, it's come to rest.
const BOUNCE: f64 = 0.35;
const RESTS: f64 = 0.8;

/// What can be thrown.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Throwable {
    Molotov,
    PipeBomb,
}

impl Throwable {
    pub const ALL: [Throwable; 2] = [Throwable::Molotov, Throwable::PipeBomb];

    /// The thing carried.
    pub fn kind(self) -> Kind {
        match self {
            Throwable::Molotov => Kind::Molotov,
            Throwable::PipeBomb => Kind::PipeBomb,
        }
    }
}

/// Something in the air (or, a pipe bomb, come to rest and beeping).
#[derive(Component, Clone, Copy, Debug)]
pub struct Thrown {
    pub what: Throwable,
    pub pos: Vec3,
    pub prev: Vec3,
    vel: Vec3,
    /// How far it's turned end over end, and how long since it was thrown.
    pub spin: f64,
    pub age: f64,
    resting: bool,
    beep_in: f64,
    /// Its light's on (a beep just now).
    pub lit: f64,
    /// Who threw it (their seat): what it kills is theirs.
    pub by: usize,
}

/// A pool of fire on the ground.
#[derive(Component, Clone, Copy, Debug)]
pub struct Fire {
    pub at: Vec3,
    pub radius: f64,
    pub left: f64,
    crackle_in: f64,
    /// Whose Molotov it was.
    pub by: usize,
}

/// One of the dead on fire (or a player a stream of it caught): how much
/// longer, and whose fire it was.
#[derive(Component, Clone, Copy, Debug)]
pub struct Burning {
    pub left: f64,
    pub by: usize,
}

/// What came of it for the run: the dead it killed (each a player's kill:
/// their seat), blasts to throw chips from, and what it did to each
/// player.
#[derive(Resource, Default)]
pub struct Booms {
    pub kills: Vec<(usize, Entity)>,
    pub blasts: Vec<Vec3>,
    /// Each player's, by seat.
    felt: Vec<Felt>,
}

/// What the fires and blasts did to a player: burnt, blasted, and how
/// hard they were shaken.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Felt {
    pub scorched: f64,
    pub blasted: f64,
    pub shake: f64,
}

impl Booms {
    /// What player `seat` felt.
    pub fn of(&self, seat: usize) -> Felt {
        self.felt.get(seat).copied().unwrap_or_default()
    }

    pub(crate) fn felt(&mut self, seat: usize) -> &mut Felt {
        if self.felt.len() <= seat {
            self.felt.resize(seat + 1, Felt::default());
        }
        &mut self.felt[seat]
    }
}

/// How a throw leaves the hand: from `eye`, looking along `forward`.
pub fn launch(forward: Vec3) -> Vec3 {
    let flat = Vec2::new(forward.x, forward.z);
    let pitch = forward.y.atan2(flat.length()) + THROW_UP.to_radians();
    let flat = if flat.length() > 1e-6 { flat.normalize() } else { Vec2::new(0.0, -1.0) };
    Vec3::new(flat.x * pitch.cos(), pitch.sin(), flat.y * pitch.cos()) * THROW_SPEED
}

/// Player `by` throws `what` from `from` along the arc `vel`.
pub fn throw(world: &mut World, what: Throwable, from: Vec3, vel: Vec3, by: usize) {
    world.spawn(Thrown { what, pos: from, prev: from, vel, spin: 0.0, age: 0.0, resting: false, beep_in: 0.0, lit: 0.0, by });
}

/// Where a throw from `from` along `vel` comes down (following it as it
/// flies, the first thing it meets), and the points it passes on the way,
/// every `every` seconds: the arc to show while aiming.
pub fn arc(solid: &crate::collide::Solids, from: Vec3, vel: Vec3, every: f64) -> (Vec<Vec3>, Option<Vec3>) {
    let (mut p, mut v, mut t, mut next) = (from, vel, 0.0, every);
    let mut dots = Vec::new();
    while t < 4.0 {
        v.y -= GRAVITY * STEP;
        let step = v * STEP;
        let len = step.length();
        if let Some(h) = solid.raycast(p, step * (1.0 / len.max(1e-9)), len) {
            return (dots, Some(h.point));
        }
        p += step;
        t += STEP;
        if t >= next {
            dots.push(p);
            next += every;
        }
    }
    (dots, None)
}

/// A fixed step of it all.
pub fn step(world: &mut World) {
    fly(world);
    flame::step(world);
    burn(world);
}

/// The thrown things fly, bounce, burst, beep and blow.
fn fly(world: &mut World) {
    let mut sounds: Vec<(Sfx, Vec3, f32)> = Vec::new();
    let mut fires = Vec::new();
    let mut blasts = Vec::new();
    let mut lures = Vec::new();
    let mut gone = Vec::new();
    // (Each one's flight this step: the glass it goes through breaks.)
    let mut flights = Vec::new();
    let mut thrown: Vec<(Entity, Thrown)> = world.query::<(Entity, &Thrown)>().iter(world).map(|(e, t)| (e, *t)).collect();
    {
        let solid = &world.resource::<Solid>().0;
        for (e, t) in &mut thrown {
            t.prev = t.pos;
            t.age += STEP;
            t.lit = (t.lit - STEP).max(0.0);
            if !t.resting {
                t.vel.y -= GRAVITY * STEP;
                t.spin += STEP * 12.0;
                let step = t.vel * STEP;
                let len = step.length();
                flights.push((t.pos, step * (1.0 / len.max(1e-9)), len));
                match solid.raycast(t.pos, step * (1.0 / len.max(1e-9)), len) {
                    Some(h) if t.what == Throwable::Molotov => {
                        // It bursts, and what's in it catches: on the ground,
                        // there; against a wall, where it runs down to.
                        let at = if h.normal.y > 0.5 { Some(h.point) } else { solid.raycast(h.point + h.normal * 0.1, -Vec3::Y, 4.0).map(|g| g.point) };
                        sounds.push((Sfx::Shatter, h.point, 1.0));
                        if let Some(at) = at {
                            fires.push((at, t.by));
                            sounds.push((Sfx::Ignite, at, 1.0));
                        }
                        gone.push(*e);
                        continue;
                    }
                    Some(h) => {
                        let n = h.normal;
                        t.vel = (t.vel - n * (2.0 * t.vel.dot(n))) * BOUNCE;
                        t.pos = h.point + n * 0.05;
                        sounds.push((Sfx::Clank, h.point, 0.5));
                        if n.y > 0.5 && t.vel.length() < RESTS {
                            t.resting = true;
                            t.vel = Vec3::ZERO;
                        }
                    }
                    None => t.pos += step,
                }
            }
            if t.what == Throwable::PipeBomb {
                lures.push(t.pos);
                // Beeping ever quicker, then it's gone.
                t.beep_in -= STEP;
                if t.beep_in <= 0.0 {
                    // (Once a second at first; frantic at the end.)
                    t.beep_in = 0.08 + 0.92 * ((FUSE - t.age) / FUSE).max(0.0).powf(1.5);
                    t.lit = 0.06;
                    sounds.push((Sfx::Beep, t.pos, 1.0));
                }
                if t.age >= FUSE {
                    blasts.push((t.pos, t.by));
                    gone.push(*e);
                }
            } else if t.age > 8.0 {
                gone.push(*e);
            }
        }
    }
    for (e, t) in thrown {
        if let Some(mut now) = world.get_mut::<Thrown>(e) {
            *now = t;
        }
    }
    for (from, dir, len) in flights {
        crate::glass::through(world, from, dir, len);
    }
    for e in gone {
        world.despawn(e);
    }
    for (at, by) in fires {
        world.spawn(Fire { at, radius: FIRE_RADIUS, left: FIRE_FOR, crackle_in: 0.0, by });
    }
    world.resource_mut::<Horde>().lures = lures;
    for (at, by) in blasts {
        blast(world, at, by);
    }
    world.resource_mut::<Horde>().sounds.extend(sounds);
}

/// Player `by`'s pipe bomb goes off at `at`: the dead near it torn apart
/// (plate is no help; the kills theirs), the players hurt and shaken if
/// near, and it's heard far off.
fn blast(world: &mut World, at: Vec3, by: usize) {
    let dead: Vec<(Entity, Vec3)> = world.query::<(Entity, &Zombie, &Body)>().iter(world).filter(|(_, z, b)| !z.dead() && (b.pos - at).length() < BLAST).map(|(e, _, b)| (e, b.pos)).collect();
    let mut kills = Vec::new();
    for (e, pos) in dead {
        let share = 1.0 - (pos - at).length() / BLAST;
        // (A blast finds everything: `limb` so no plate turns it.)
        let hit = zombie::Impact { damage: BLAST_DEAD * share + 120.0, head: false, limb: true, blow: false, shove: 4.0 + 14.0 * share, stumble: true, takedown: false, fire: false, at: None };
        blame(world, e, by);
        if zombie::hurt(world, e, pos - at, at, hit) {
            kills.push((by, e));
        }
    }
    let players: Vec<(usize, Vec3)> = world.query::<(&Player, &Body)>().iter(world).map(|(p, b)| (p.0, b.pos)).collect();
    let mut booms = world.resource_mut::<Booms>();
    booms.kills.extend(kills);
    booms.blasts.push(at);
    for (seat, p) in players {
        let d = (p - at).length();
        let felt = booms.felt(seat);
        if d < BLAST {
            felt.blasted += BLAST_PLAYER * (1.0 - d / BLAST);
        }
        felt.shake = felt.shake.max((1.0 - d / SHAKES).max(0.0) * SHAKE);
    }
    world.resource_mut::<Horde>().sounds.push((Sfx::Explosion, at + Vec3::new(0.0, 0.5, 0.0), 1.0));
    zombie::noise(world, at, BLAST_HEARD);
}

/// The fires burn down; the dead in them catch, and burn on; a player in
/// one is scorched.
fn burn(world: &mut World) {
    let mut sounds = Vec::new();
    let mut out = Vec::new();
    let fires: Vec<(Entity, Fire)> = world.query::<(Entity, &Fire)>().iter(world).map(|(e, f)| (e, *f)).collect();
    for (e, mut f) in fires {
        f.left -= STEP;
        f.crackle_in -= STEP;
        if f.crackle_in <= 0.0 {
            f.crackle_in = 1.0;
            sounds.push((Sfx::Crackle, f.at, 0.9));
        }
        if f.left <= 0.0 {
            out.push(e);
        } else if let Some(mut now) = world.get_mut::<Fire>(e) {
            *now = f;
        }
    }
    for e in out {
        world.despawn(e);
    }
    let fires: Vec<Fire> = world.query::<&Fire>().iter(world).copied().collect();
    // The fire `p` stands in, if any: whose it is.
    let within = |p: Vec3| fires.iter().find(|f| Vec2::new(p.x - f.at.x, p.z - f.at.z).length() < f.size() && (p.y - f.at.y).abs() < 1.2).map(|f| f.by);
    // The dead: those in a fire catch; those on fire burn. (Not a hound:
    // fire's nothing to it.)
    let dead: Vec<(Entity, Vec3, Option<Burning>)> = world.query::<(Entity, &Zombie, &Body, Option<&Burning>)>().iter(world).filter(|(_, z, _, _)| !z.dead() && !z.kind.fireproof()).map(|(e, _, b, burning)| (e, b.pos, burning.copied())).collect();
    let mut kills = Vec::new();
    for (e, pos, burning) in dead {
        let now = match within(pos) {
            Some(by) => Some(Burning { left: BURNS_ON, by }),
            None => burning.map(|b| Burning { left: b.left - STEP, ..b }).filter(|b| b.left > 0.0),
        };
        match now {
            Some(b) => {
                world.entity_mut(e).insert(b);
                blame(world, e, b.by);
                let hit = zombie::Impact { damage: BURN_DEAD * STEP, head: false, limb: true, blow: false, shove: 0.0, stumble: false, takedown: false, fire: true, at: None };
                if zombie::hurt(world, e, Vec3::ZERO, pos, hit) {
                    kills.push((b.by, e));
                }
            }
            None if burning.is_some() => {
                world.entity_mut(e).remove::<Burning>();
            }
            None => {}
        }
    }
    // The players: scorched standing in a fire, and while one a stream
    // caught burns on.
    let players: Vec<(Entity, usize, Vec3, Option<Burning>)> = world.query::<(Entity, &Player, &Body, Option<&Burning>)>().iter(world).map(|(e, p, b, burning)| (e, p.0, b.pos, burning.copied())).collect();
    let mut scorched = Vec::new();
    for (e, seat, p, burning) in players {
        let alight = burning.map(|b| Burning { left: b.left - STEP, ..b }).filter(|b| b.left > 0.0);
        match alight {
            Some(b) => {
                world.entity_mut(e).insert(b);
            }
            None if burning.is_some() => {
                world.entity_mut(e).remove::<Burning>();
            }
            None => {}
        }
        if within(p).is_some() || alight.is_some() {
            scorched.push(seat);
        }
    }
    let mut booms = world.resource_mut::<Booms>();
    booms.kills.extend(kills);
    for seat in scorched {
        booms.felt(seat).scorched += BURN_PLAYER * STEP;
    }
    world.resource_mut::<Horde>().sounds.extend(sounds);
}

/// Player `seat` is set alight (a hound's bite): they burn a moment.
pub fn alight(world: &mut World, seat: usize) {
    let who = world.query::<(Entity, &Player)>().iter(world).find(|(_, p)| p.0 == seat).map(|(e, _)| e);
    if let Some(e) = who {
        world.entity_mut(e).insert(Burning { left: BITE_BURNS, by: seat });
    }
}

/// A hound's gone up in flames at `at` (player `by`'s kill, what it burns
/// theirs).
pub fn pyre(world: &mut World, at: Vec3, by: usize) {
    world.spawn(Fire { at, radius: PYRE.0, left: PYRE.1, crackle_in: 0.0, by });
    world.resource_mut::<Horde>().sounds.push((Sfx::Ignite, at + Vec3::new(0.0, 0.4, 0.0), 0.9));
}

/// One of the dead was hurt by player `by`: whatever it takes with it is
/// theirs.
fn blame(world: &mut World, e: Entity, by: usize) {
    if let Some(mut z) = world.get_mut::<Zombie>(e) {
        z.by = Some(by);
    }
}

impl Fire {
    /// How wide it is now (dying down at the end).
    pub fn size(&self) -> f64 {
        self.radius * (self.left / FIRE_DYING).clamp(0.0, 1.0).sqrt()
    }
}

/// Everything thrown and burning, gone (back to the title).
pub fn clear(world: &mut World) {
    let all: Vec<Entity> = world.query_filtered::<Entity, Or<(With<Thrown>, With<Fire>, With<flame::Puff>)>>().iter(world).collect();
    for e in all {
        world.despawn(e);
    }
    let burning: Vec<Entity> = world.query_filtered::<Entity, With<Burning>>().iter(world).collect();
    for e in burning {
        world.entity_mut(e).remove::<Burning>();
    }
}

#[cfg(test)]
mod tests;
