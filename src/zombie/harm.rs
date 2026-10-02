//! What a hit leaves to be seen: how much it took (told to the rest of the
//! game, for whoever dealt it to see thrown up where it struck), and on
//! the one hurt, how whole it was and when it was last hurt (the bar over
//! its head, `hurts.rs`).

use bevy_ecs::prelude::*;
use lntrn_math::Vec3;

use super::brain::Zombie;
use crate::world::Clock;

/// How long after a hit the bar still shows what it had, and how fast
/// that then drains to what it has (its whole health a second, times this).
const HOLD: f64 = 0.35;
const DRAIN: f64 = 1.6;

/// A hit as it's shown: on which of the dead, where it struck, what it
/// took; to the head, turned by plate, fire (which burns on, a tick at a
/// time), fire on one it's nothing to; whether it killed; and whose it
/// was (their seat).
#[derive(Clone, Copy, Debug)]
pub struct Harm {
    pub on: Entity,
    pub at: Vec3,
    pub amount: f64,
    pub head: bool,
    pub turned: bool,
    pub fire: bool,
    pub immune: bool,
    pub killed: bool,
    pub by: Option<usize>,
}

/// One of the dead that's been hurt: what it had before anything hurt it,
/// when it was last hurt (the game clock's seconds), and what its bar
/// still shows of what the last hits took (draining down to what it has).
#[derive(Component, Clone, Copy, Debug)]
pub struct Harmed {
    pub full: f64,
    pub when: f64,
    pub trail: f64,
}

/// `e`, which had `before`, was hurt at `now`.
pub(super) fn mark(world: &mut World, e: Entity, before: f64, now: f64) {
    match world.get_mut::<Harmed>(e) {
        Some(mut h) => h.when = now,
        None => {
            world.entity_mut(e).insert(Harmed { full: before, when: now, trail: before });
        }
    }
}

/// What the last hits took drains from each bar, a moment after.
pub(super) fn drain(clock: Res<Clock>, mut dead: Query<(&Zombie, &mut Harmed)>) {
    for (z, mut h) in &mut dead {
        let has = z.hp.max(0.0);
        if h.trail > has && clock.time - h.when > HOLD {
            h.trail = (h.trail - h.full * DRAIN * clock.dt).max(has);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::Body;
    use crate::zombie::brain::dealt;
    use crate::zombie::{self, Horde, Impact};

    fn shot(damage: f64, head: bool) -> Impact {
        Impact { damage, head, limb: false, blow: false, shove: 0.0, stumble: false, takedown: false, fire: false, at: Some(Vec3::new(0.0, 1.5, 0.1)) }
    }

    #[test]
    fn a_hit_is_told_of_and_leaves_its_mark_on_the_one_hurt() {
        let mut world = World::new();
        world.insert_resource(Horde::default());
        world.insert_resource(Clock { time: 10.0, dt: 0.0 });
        let mut z = Zombie::new(0.0, 5);
        z.by = Some(1);
        let full = z.hp;
        let e = world.spawn((z, Body::at(Vec3::ZERO))).id();
        let (dir, from) = (Vec3::new(0.0, 0.0, -1.0), Vec3::new(0.0, 1.6, 2.0));
        let told = |world: &mut World| std::mem::take(&mut world.resource_mut::<Horde>().harms);
        assert!(!zombie::hurt(&mut world, e, dir, from, shot(25.0, false)));
        let harms = told(&mut world);
        assert_eq!(harms.len(), 1);
        let h = harms[0];
        assert_eq!((h.on, h.amount, h.by, h.at), (e, 25.0, Some(1), Vec3::new(0.0, 1.5, 0.1)));
        assert!(!h.head && !h.turned && !h.fire && !h.killed);
        let mark = *world.get::<Harmed>(e).unwrap();
        assert_eq!((mark.full, mark.when, mark.trail), (full, 10.0, full));
        // To the head: what it really took. Nowhere in particular: its chest.
        world.resource_mut::<Clock>().time = 11.0;
        zombie::hurt(&mut world, e, dir, from, Impact { at: None, ..shot(10.0, true) });
        let h = told(&mut world)[0];
        assert!(h.head && h.amount == dealt(10.0, true, false) && h.at == Vec3::new(0.0, 1.2, 0.0), "{h:?}");
        let mark = *world.get::<Harmed>(e).unwrap();
        assert_eq!((mark.full, mark.when), (full, 11.0), "as whole as it ever was; hurt again just now");
        // The one that kills says so; the dead are hurt no more.
        assert!(zombie::hurt(&mut world, e, dir, from, shot(1000.0, false)));
        assert!(told(&mut world)[0].killed);
        assert!(!zombie::hurt(&mut world, e, dir, from, shot(1000.0, false)));
        assert!(told(&mut world).is_empty());
    }

    #[test]
    fn what_a_hit_took_drains_from_the_bar_a_moment_after() {
        let mut world = World::new();
        world.insert_resource(Horde::default());
        world.insert_resource(Clock { time: 10.0, dt: 0.0 });
        let e = world.spawn((Zombie::new(0.0, 5), Body::at(Vec3::ZERO))).id();
        let full = world.get::<Zombie>(e).unwrap().hp;
        zombie::hurt(&mut world, e, Vec3::new(0.0, 0.0, -1.0), Vec3::new(0.0, 1.6, 2.0), shot(60.0, false));
        let mut frame = Schedule::default();
        frame.add_systems(drain);
        let mut at = |world: &mut World, time: f64| {
            *world.resource_mut::<Clock>() = Clock { time, dt: 0.1 };
            frame.run(world);
            world.get::<Harmed>(e).unwrap().trail
        };
        assert_eq!(at(&mut world, 10.2), full, "held a moment");
        let draining = at(&mut world, 10.5);
        assert!(draining < full && draining > full - 60.0, "{draining}");
        for k in 0..10 {
            at(&mut world, 10.6 + 0.1 * f64::from(k));
        }
        assert_eq!(at(&mut world, 12.0), full - 60.0, "down to what it has, and no further");
    }
}
