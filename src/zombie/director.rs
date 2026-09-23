//! How many of the dead are about: a run starts with a crowd already out
//! there, and every kill brings two more into the budget, up to a horde. Whenever fewer are up than the budget allows, a new one comes in
//! from out of sight, one at a time.

use bevy_ecs::prelude::*;
use lntrn_math::Vec3;

/// How many a run starts with, the most there will ever be, and how many
/// more each kill brings.
pub const FIRST: usize = 32;
pub const MOST: usize = 80;
const PER_KILL: usize = 2;
/// Seconds between newcomers, and before trying again when there was
/// nowhere out of sight to put one.
const TRICKLE: f64 = 0.75;
const RETRY: f64 = 0.25;
/// Between newcomers while the dead surge.
const SURGE_TRICKLE: f64 = 0.3;

/// How many may be up at once, `kills` into a run.
pub fn budget(kills: u32) -> usize {
    (FIRST + kills as usize * PER_KILL).min(MOST)
}

#[derive(Default)]
pub struct Director {
    wait: f64,
    /// The dead surging: as many as there can be, coming in fast.
    pub surge: bool,
    /// The most that were ever up at once.
    pub peak: usize,
}

impl Director {
    /// A run begins: the first of them, already out there somewhere.
    pub fn begin(&mut self, world: &mut World, eye: Vec3, forward: Vec3) {
        *self = Self::default();
        for _ in 0..FIRST * 4 {
            if super::alive(world) >= FIRST {
                break;
            }
            super::spawn_unseen(world, eye, forward);
        }
        self.peak = super::alive(world);
        self.wait = TRICKLE;
    }

    /// Bring one in if there's room in the budget and it's time.
    pub fn update(&mut self, world: &mut World, kills: u32, eye: Vec3, forward: Vec3, dt: f64) {
        self.wait -= dt;
        let up = super::alive(world);
        self.peak = self.peak.max(up);
        let most = if self.surge { MOST } else { budget(kills) };
        if up >= most || self.wait > 0.0 {
            return;
        }
        self.wait = if !super::spawn_unseen(world, eye, forward) { RETRY } else if self.surge { SURGE_TRICKLE } else { TRICKLE };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_budget_grows_with_kills_and_stops_at_a_crowd() {
        assert_eq!(budget(0), FIRST);
        assert_eq!(budget(1), FIRST + 2);
        assert_eq!(budget(24), MOST);
        assert_eq!(budget(500), MOST);
    }
}
