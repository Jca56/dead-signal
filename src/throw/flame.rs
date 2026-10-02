//! A flamethrower's stream, a puff at a time: everything in its cone that
//! it can reach (not through a wall) is scorched and catches, the dead and
//! the other players alike; where it ends, on the ground or run down a
//! wall, it leaves a small fire burning. The puffs themselves fly on a
//! moment, to be seen.

use std::sync::atomic::{AtomicU32, Ordering};

use bevy_ecs::prelude::*;
use lntrn_math::{Vec2, Vec3};

use super::{BURNS_ON, Booms, Burning, Fire};
use crate::player::{Body, Player, STEP};
use crate::world::Solid;
use crate::zombie::{self, brain::Zombie};

/// How fast a puff flies, m/s; how much of that it loses a second, and
/// how fast it rises as it goes.
const SPEED: f64 = 16.0;
const DRAG: f64 = 1.1;
const RISE: f64 = 1.4;
/// How far off true a puff may fly, a share of its speed.
const WANDER: f64 = 0.06;
/// The fire it leaves: how wide, how long; no nearer another than this
/// (that one burns on instead); and never more of them than this.
const POOL: f64 = 1.1;
const POOL_FOR: f64 = 4.0;
const POOL_APART: f64 = 0.9;
const POOLS: usize = 48;
/// How far down from where the stream ends (or meets a wall) the ground is
/// looked for.
const DRIP: f64 = 3.0;
/// What of a body the stream's aimed at: how high its middle, how wide it
/// is.
const CHEST: f64 = 1.0;
const WIDE: f64 = 0.4;

/// A puff of the stream, in the air.
#[derive(Component, Clone, Copy, Debug)]
pub struct Puff {
    pub pos: Vec3,
    pub prev: Vec3,
    vel: Vec3,
    pub age: f64,
    pub life: f64,
    /// What sets it apart from the next, 0–1.
    pub seed: f64,
}

/// A number 0–1, another each time.
fn luck() -> f64 {
    static N: AtomicU32 = AtomicU32::new(0x9E37_79B9);
    let mut x = N.load(Ordering::Relaxed);
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    N.store(x, Ordering::Relaxed);
    f64::from(x) / f64::from(u32::MAX)
}

/// Whether the stream from `eye` along `dir` reaches a body standing at
/// `feet`: within `range`, within its cone (`half` its width, radians),
/// and nothing solid between.
fn reaches(world: &World, eye: Vec3, dir: Vec3, feet: Vec3, range: f64, half: f64) -> bool {
    let to = feet + Vec3::new(0.0, CHEST, 0.0) - eye;
    let d = to.length();
    if d > range || d < 1e-6 {
        return d < 1e-6;
    }
    let to = to * (1.0 / d);
    if dir.dot(to).clamp(-1.0, 1.0).acos() > half + (WIDE / d).atan() {
        return false;
    }
    world.resource::<Solid>().0.raycast(eye, to, d).is_none_or(|h| h.t > d - WIDE)
}

/// A puff of player `by`'s stream, from their `eye` along `dir` (seen to
/// leave the `muzzle`): each of the dead it reaches takes `damage` and
/// catches; so does any other player; and where it ends it leaves fire.
pub fn spray(world: &mut World, by: usize, eye: Vec3, dir: Vec3, muzzle: Vec3, damage: f64, range: f64, cone: f64) {
    let half = cone.to_radians() * 0.5;
    let dead: Vec<Entity> = world.query::<(Entity, &Zombie, &Body)>().iter(world).filter(|(_, z, b)| !z.dead() && reaches(world, eye, dir, b.pos, range, half)).map(|(e, _, _)| e).collect();
    let mut kills = Vec::new();
    for e in dead {
        world.entity_mut(e).insert(Burning { left: BURNS_ON, by });
        if let Some(mut z) = world.get_mut::<Zombie>(e) {
            z.by = Some(by);
        }
        // (Fire finds everything: `limb`, so no plate turns it.)
        let hit = zombie::Impact { damage, head: false, limb: true, blow: false, shove: 0.0, stumble: false, takedown: false };
        if zombie::hurt(world, e, dir, eye, hit) {
            kills.push((by, e));
        }
    }
    world.resource_mut::<Booms>().kills.extend(kills);
    // Whoever else is in the way of it.
    let others: Vec<Entity> = world.query::<(Entity, &Player, &Body)>().iter(world).filter(|(_, p, b)| p.0 != by && reaches(world, eye, dir, b.pos, range, half)).map(|(e, _, _)| e).collect();
    for e in others {
        world.entity_mut(e).insert(Burning { left: BURNS_ON, by });
    }
    // Where it ends: on what it meets, or the ground under it.
    let ground = {
        let solid = &world.resource::<Solid>().0;
        let down = |from: Vec3| solid.raycast(from, -Vec3::Y, DRIP).map(|g| g.point);
        match solid.raycast(eye, dir, range) {
            Some(h) if h.normal.y > 0.5 => Some(h.point),
            Some(h) => down(h.point + h.normal * 0.15),
            None => down(eye + dir * range),
        }
    };
    if let Some(at) = ground {
        pool(world, at, by);
    }
    let wander = Vec3::new(luck() - 0.5, luck() - 0.5, luck() - 0.5) * (2.0 * WANDER);
    world.spawn(Puff { pos: muzzle, prev: muzzle, vel: (dir + wander) * SPEED, age: 0.0, life: range / SPEED, seed: luck() });
}

/// Fire left at `at` by player `by`'s stream: a fire already burning there
/// burns on; else a new one, if there aren't too many.
fn pool(world: &mut World, at: Vec3, by: usize) {
    let mut fires = world.query::<&mut Fire>();
    let mut count = 0;
    for mut f in fires.iter_mut(world) {
        count += 1;
        if Vec2::new(f.at.x - at.x, f.at.z - at.z).length() < POOL_APART && (f.at.y - at.y).abs() < 1.0 {
            f.left = f.left.max(POOL_FOR);
            return;
        }
    }
    if count < POOLS {
        world.spawn(Fire { at, radius: POOL, left: POOL_FOR, crackle_in: 0.0, by });
    }
}

/// A fixed step of the puffs in the air: on they fly, slowing and rising,
/// till they're spent or meet something.
pub(super) fn step(world: &mut World) {
    let mut gone = Vec::new();
    let mut puffs: Vec<(Entity, Puff)> = world.query::<(Entity, &Puff)>().iter(world).map(|(e, p)| (e, *p)).collect();
    {
        let solid = &world.resource::<Solid>().0;
        for (e, p) in &mut puffs {
            p.prev = p.pos;
            p.age += STEP;
            p.vel = p.vel * (1.0 - DRAG * STEP) + Vec3::new(0.0, RISE * STEP, 0.0);
            let step = p.vel * STEP;
            let len = step.length();
            if p.age >= p.life || solid.raycast(p.pos, step * (1.0 / len.max(1e-9)), len).is_some() {
                gone.push(*e);
            } else {
                p.pos += step;
            }
        }
    }
    for (e, p) in puffs {
        if let Some(mut now) = world.get_mut::<Puff>(e) {
            *now = p;
        }
    }
    for e in gone {
        world.despawn(e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collide::{Solids, box_tris};
    use crate::throw::{self, BURN_PLAYER};
    use crate::zombie::Horde;
    use crate::zombie::kind::Kind as Dead;
    use crate::zombie::looks::Theme;

    /// A floor, a wall across it at z -6 (from x 2 on), two players (the
    /// one with the flamethrower at the middle, looking down -z), and what
    /// the dead and the fires need.
    fn world(buddy: Vec3) -> World {
        let mut s = Solids::new();
        s.add(&box_tris(Vec3::new(-60.0, -1.0, -60.0), Vec3::new(60.0, 0.0, 60.0)));
        s.add(&box_tris(Vec3::new(2.0, 0.0, -6.2), Vec3::new(20.0, 3.0, -6.0)));
        let mut w = World::new();
        w.insert_resource(Solid(s));
        w.insert_resource(Horde::default());
        w.insert_resource(Booms::default());
        w.insert_resource(zombie::Noises::default());
        w.insert_resource(zombie::Heat::default());
        w.spawn((Player(0), Body::at(Vec3::ZERO)));
        w.spawn((Player(1), Body::at(buddy)));
        w
    }

    const EYE: Vec3 = Vec3::new(0.0, 1.6, 0.0);
    const AHEAD: Vec3 = Vec3::new(0.0, 0.0, -1.0);

    fn puff(w: &mut World) {
        spray(w, 0, EYE, AHEAD, EYE + AHEAD * 0.9, 8.0, 9.0, 22.0);
    }

    fn hp(w: &mut World, e: Entity) -> f64 {
        w.get::<Zombie>(e).map_or(0.0, |z| z.hp)
    }

    #[test]
    fn the_stream_burns_what_is_in_its_cone_and_reach_and_not_behind_a_wall() {
        let mut w = world(Vec3::new(30.0, 0.0, 30.0));
        let at = |w: &mut World, x: f64, z: f64| zombie::spawn_kind(w, Vec3::new(x, 0.0, z), 0.0, Dead::Shambler, Theme::Drifter);
        let (near, far, aside, behind) = (at(&mut w, 0.3, -5.0), at(&mut w, 0.0, -14.0), at(&mut w, 5.0, -4.0), at(&mut w, 0.0, 4.0));
        let before = hp(&mut w, near);
        puff(&mut w);
        assert!((before - hp(&mut w, near) - 8.0).abs() < 1e-9, "the one in it took {}", before - hp(&mut w, near));
        assert!(w.get::<Burning>(near).is_some_and(|b| b.by == 0), "and it's alight, the thrower's");
        for (what, e) in [("out of its reach", far), ("off to the side", aside), ("behind them", behind)] {
            assert!(w.get::<Burning>(e).is_none() && hp(&mut w, e) == before, "the one {what} burnt");
        }
        // Round the wall's end it's reached; through the wall, not.
        let mut w = world(Vec3::new(30.0, 0.0, 30.0));
        let hidden = at(&mut w, 4.0, -8.0);
        let dir = (Vec3::new(4.0, 1.0, -8.0) - EYE).normalize();
        spray(&mut w, 0, EYE, dir, EYE, 8.0, 12.0, 22.0);
        assert!(w.get::<Burning>(hidden).is_none(), "burnt through a wall");
        // A second of it, and then the burning on: dead, and theirs.
        let mut w = world(Vec3::new(30.0, 0.0, 30.0));
        let e = at(&mut w, 0.0, -4.0);
        for _ in 0..40 {
            puff(&mut w);
        }
        assert!(w.get::<Zombie>(e).is_none_or(|z| z.dead()), "forty puffs left it standing with {}", hp(&mut w, e));
        assert_eq!(w.resource::<Booms>().kills, vec![(0, e)]);
    }

    #[test]
    fn it_sets_another_player_alight_and_never_its_own() {
        let mut w = world(Vec3::new(0.0, 0.0, -5.0));
        puff(&mut w);
        let alight: Vec<usize> = w.query::<(&Player, &Burning)>().iter(&w).map(|(p, _)| p.0).collect();
        assert_eq!(alight, vec![1], "who's alight");
        // They burn on a while after, and then it's out.
        for _ in 0..(1.0 / STEP) as usize {
            throw::step(&mut w);
        }
        let felt = w.resource::<Booms>().of(1).scorched;
        assert!(felt > BURN_PLAYER * 0.8 && w.resource::<Booms>().of(0).scorched == 0.0, "a second alight scorched them {felt}");
        for _ in 0..(BURNS_ON / STEP) as usize {
            throw::step(&mut w);
        }
        assert!(w.query::<(&Player, &Burning)>().iter(&w).next().is_none(), "still alight");
    }

    #[test]
    fn where_it_ends_it_leaves_a_fire_and_one_fire_a_place() {
        let mut w = world(Vec3::new(30.0, 0.0, 30.0));
        // Out over open ground: under where it ends. Again and again: the
        // one fire, burning on.
        for _ in 0..30 {
            puff(&mut w);
        }
        let fires: Vec<Fire> = w.query::<&Fire>().iter(&w).copied().collect();
        assert_eq!(fires.len(), 1, "{fires:?}");
        assert!((fires[0].at - Vec3::new(0.0, 0.0, -9.0)).length() < 0.1 && fires[0].by == 0, "{:?}", fires[0]);
        // Against a wall: down at its foot. Swept about: never too many.
        let dir = (Vec3::new(6.0, 1.2, -6.0) - EYE).normalize();
        spray(&mut w, 0, EYE, dir, EYE, 8.0, 12.0, 22.0);
        assert!(w.query::<&Fire>().iter(&w).any(|f| f.at.y.abs() < 0.05 && (f.at.z + 5.85).abs() < 0.2), "no fire at the wall's foot");
        for k in 0..400 {
            let a = f64::from(k) * 0.37;
            spray(&mut w, 0, EYE, Vec3::new(a.cos(), -0.3, a.sin()).normalize(), EYE, 8.0, 9.0, 22.0);
        }
        assert!(w.query::<&Fire>().iter(&w).count() <= POOLS);
        // The puffs fly a moment, and are gone.
        assert!(w.query::<&Puff>().iter(&w).count() > 100);
        for _ in 0..(1.0 / STEP) as usize {
            throw::step(&mut w);
        }
        assert_eq!(w.query::<&Puff>().iter(&w).count(), 0);
    }
}
