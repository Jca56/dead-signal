//! The radio's boosts: 2X POINTS (everything earned is doubled) and
//! INSTAKILL (any hit kills, but a Juggernaut, which is only hit harder).
//! Each lasts a while from when it's called, for everyone, and calling it
//! again while it's up adds as long again.

use crate::radio::codes::Call;

/// How long a boost lasts, seconds; and the most that can be stacked up.
pub const LASTS: f64 = 30.0;
const MOST: f64 = 90.0;
/// How many times as hard a Juggernaut's hit while INSTAKILL's up.
pub const JUGGERNAUT: f64 = 3.0;
/// The last of a boost: the HUD blinks it.
pub const ENDING: f64 = 5.0;

/// How long each has left, seconds.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Boosts {
    pub double: f64,
    pub instakill: f64,
}

impl Boosts {
    /// `call`'s boost, begun (or as long again, if it's up). Whether it
    /// is one.
    pub fn start(&mut self, call: Call) -> bool {
        let left = match call {
            Call::DoublePoints => &mut self.double,
            Call::Instakill => &mut self.instakill,
            _ => return false,
        };
        *left = (*left + LASTS).min(MOST);
        true
    }

    /// `dt` on.
    pub fn update(&mut self, dt: f64) {
        self.double = (self.double - dt).max(0.0);
        self.instakill = (self.instakill - dt).max(0.0);
    }

    /// How many times what's earned is worth.
    pub fn points(&self) -> u32 {
        if self.double > 0.0 { 2 } else { 1 }
    }

    pub fn instakill(&self) -> bool {
        self.instakill > 0.0
    }

    /// Those that are up, as the HUD shows them: its name, how long it has
    /// left, and which it is.
    pub fn up(&self) -> impl Iterator<Item = (&'static str, f64, Call)> {
        [(Call::DoublePoints, self.double), (Call::Instakill, self.instakill)].into_iter().filter(|(_, left)| *left > 0.0).map(|(call, left)| (call.entry().name, left, call))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_boost_lasts_its_while_and_called_again_lasts_as_long_again() {
        let mut b = Boosts::default();
        assert!(b.points() == 1 && !b.instakill() && b.up().count() == 0);
        assert!(!b.start(Call::AmmoDrop), "not a boost");
        assert!(b.start(Call::DoublePoints));
        assert!(b.points() == 2 && !b.instakill());
        b.update(LASTS - 1.0);
        assert_eq!(b.up().collect::<Vec<_>>(), [("2X POINTS", 1.0, Call::DoublePoints)]);
        // Called again with a second left: that and thirty more.
        b.start(Call::DoublePoints);
        b.start(Call::Instakill);
        assert!((b.double - (LASTS + 1.0)).abs() < 1e-9 && b.instakill());
        b.update(LASTS + 0.5);
        assert!(b.points() == 2 && !b.instakill());
        b.update(1.0);
        assert_eq!((b.points(), b.up().count()), (1, 0));
        // No more than so much stacked up.
        for _ in 0..10 {
            b.start(Call::Instakill);
        }
        assert_eq!(b.instakill, MOST);
    }

    #[test]
    fn with_double_points_up_everything_earned_is_worth_twice() {
        use crate::holdout::tests::{built, world_of};
        use crate::stats::Stats;
        let (mut world, mut h) = world_of(built());
        let start = h.wallets[0].points;
        // A hit and a kill to the body: 10 and 50.
        let mut stats = Stats { hits: 1, gun_kills: 1, ..Stats::default() };
        h.score(0, &stats);
        assert_eq!(h.wallets[0].points, start + 60);
        // The same again with 2X POINTS up: 120. And once it's run out, 60.
        h.boosts.start(Call::DoublePoints);
        (stats.hits, stats.gun_kills) = (2, 2);
        h.score(0, &stats);
        assert_eq!(h.wallets[0].points, start + 180);
        h.update(&mut world, &[], LASTS + 0.1);
        (stats.hits, stats.gun_kills) = (3, 3);
        h.score(0, &stats);
        assert_eq!(h.wallets[0].points, start + 240);
    }
}
