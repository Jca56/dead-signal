//! How many of the dead are about, and where. A run starts with the whole
//! map peopled: a crowd in the town (some of them upstairs), a group at
//! every place, and wanderers in the woods between; never close to where
//! the player starts. Then, as the run goes on, a trickle keeps some of
//! them near the player: whenever fewer are close than the kills so far
//! call for, a new one comes in from out of sight. When the dead surge
//! (the way out is being worked), they come fast and many.

use bevy_ecs::prelude::*;
use lntrn_math::{Vec2, Vec3};

use super::Nav;
use crate::loot::Dice;
use crate::map::sites::{Kind, Site};

/// How many of the dead the whole map starts with, and the most there may
/// ever be up at once.
pub const FIRST: usize = 250;
const MOST: usize = 300;
/// How many at each kind of place (the rest wander).
fn at_place(kind: Kind) -> usize {
    match kind {
        Kind::Town => 80,
        Kind::Military => 20,
        Kind::Farm | Kind::Gas => 14,
        Kind::Crash => 12,
        Kind::Cabin | Kind::Pad | Kind::Radio => 10,
    }
}
/// Nothing starts nearer the player than this, metres.
const CLEAR_OF_START: f64 = 45.0;
/// What counts as near the player, how many should be near to begin with,
/// and the most near as the kills mount (one more each two kills); while
/// the dead surge, the most there are near.
const NEAR: f64 = 80.0;
const NEAR_FIRST: usize = 8;
const NEAR_MOST: usize = 30;
const NEAR_SURGE: usize = 45;
/// Seconds between newcomers, and before trying again when there was
/// nowhere out of sight to put one; and between newcomers while the dead
/// surge.
const TRICKLE: f64 = 1.5;
const RETRY: f64 = 0.25;
const SURGE_TRICKLE: f64 = 0.3;

/// How many should be near the player, `kills` into a run.
pub fn near_budget(kills: u32) -> usize {
    (NEAR_FIRST + kills as usize / 2).min(NEAR_MOST)
}

#[derive(Default)]
pub struct Director {
    wait: f64,
    /// The dead surging: as many as there can be near, coming in fast.
    pub surge: bool,
    /// The most that were ever near the player at once.
    pub peak: usize,
}

/// How many of the dead are standing within `NEAR` of `eye`, flat.
fn near(world: &mut World, eye: Vec3) -> usize {
    world
        .query::<(&super::Zombie, &crate::player::Body)>()
        .iter(world)
        .filter(|(z, b)| !z.dead() && Vec2::new(b.pos.x - eye.x, b.pos.z - eye.z).length() < NEAR)
        .count()
}

impl Director {
    /// A run begins: the map peopled, clear of the player at `eye`. The
    /// ground's height is `ground`'s (none off the land); `seed` picks
    /// where they stand.
    pub fn begin(&mut self, world: &mut World, sites: &[Site], ground: &dyn Fn(f64, f64) -> Option<f64>, eye: Vec3, seed: u32) {
        *self = Self::default();
        let feet = eye - Vec3::new(0.0, 1.6, 0.0);
        let mut dice = Dice(seed | 1);
        let spots = {
            let Some(nav) = world.resource::<Nav>().0.as_ref() else { return };
            // Somewhere to stand at (x, z), on a floor (upstairs, now and
            // then), clear of the start, and a way from it to the player.
            let stand = |dice: &mut Dice, x: f64, z: f64, base: f64| -> Option<Vec3> {
                if Vec2::new(x - feet.x, z - feet.z).length() < CLEAR_OF_START {
                    return None;
                }
                let up = if dice.unit() < 0.25 { 3.0 } else { 0.0 };
                let h = nav.height_at(Vec3::new(x, base + 1.0 + up, z))?;
                let at = Vec3::new(x, h, z);
                nav.connects(at, feet).then_some(at)
            };
            let mut spots = Vec::new();
            for site in sites {
                let want = at_place(site.kind);
                let mut placed = 0;
                for _ in 0..want * 20 {
                    if placed == want {
                        break;
                    }
                    let local = Vec2::new((dice.unit() * 2.0 - 1.0) * site.plot.half.x, (dice.unit() * 2.0 - 1.0) * site.plot.half.y);
                    let p = site.plot.world(local);
                    if let Some(at) = stand(&mut dice, p.x, p.y, site.plot.height) {
                        spots.push(at);
                        placed += 1;
                    }
                }
            }
            // The rest wander anywhere on the land.
            let half = crate::map::HALF - 10.0;
            for _ in 0..FIRST * 20 {
                if spots.len() >= FIRST {
                    break;
                }
                let (x, z) = ((dice.unit() * 2.0 - 1.0) * half, (dice.unit() * 2.0 - 1.0) * half);
                if let Some(base) = ground(x, z)
                    && let Some(at) = stand(&mut dice, x, z, base)
                {
                    spots.push(at);
                }
            }
            spots
        };
        for at in spots {
            let yaw = dice.unit() * std::f64::consts::TAU;
            super::spawn(world, at, yaw);
        }
        self.peak = near(world, eye);
        self.wait = TRICKLE;
    }

    /// Bring one in near the player, out of sight, if fewer are near than
    /// there should be and it's time.
    pub fn update(&mut self, world: &mut World, kills: u32, eye: Vec3, forward: Vec3, dt: f64) {
        self.wait -= dt;
        let close = near(world, eye);
        self.peak = self.peak.max(close);
        let want = if self.surge { NEAR_SURGE } else { near_budget(kills) };
        if close >= want || self.wait > 0.0 || super::alive(world) >= MOST {
            return;
        }
        self.wait = if !super::spawn_unseen(world, eye, forward) { RETRY } else if self.surge { SURGE_TRICKLE } else { TRICKLE };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_near_budget_grows_with_kills_and_stops_at_a_crowd() {
        assert_eq!(near_budget(0), NEAR_FIRST);
        assert_eq!(near_budget(2), NEAR_FIRST + 1);
        assert_eq!(near_budget(1000), NEAR_MOST);
        const { assert!(NEAR_SURGE > NEAR_MOST && MOST > FIRST) };
    }

    #[test]
    fn the_places_and_the_wanderers_make_up_the_first() {
        // A town and one of every other kind: most of the dead are at the
        // places, the rest wander.
        let kinds = [Kind::Town, Kind::Military, Kind::Farm, Kind::Farm, Kind::Gas, Kind::Crash, Kind::Cabin, Kind::Pad, Kind::Radio];
        let placed: usize = kinds.iter().map(|&k| at_place(k)).sum();
        assert!(placed < FIRST && FIRST - placed >= 40, "{placed} at the places, {} wandering", FIRST - placed);
    }
}
