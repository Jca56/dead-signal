//! The rounds: how many of the dead each brings (more for more players),
//! how tough and how fast they are, and bringing them in by the windows of
//! the open zones (the nearer a player, the likelier: each player in
//! turn), never too many up at once. A round is over when all of it is
//! dead; a short breather, and the next begins.

use bevy_ecs::prelude::*;
use lntrn_math::Vec3;

use super::arena::Arena;
use crate::loot::Dice;
use crate::zombie::{self, brain::{State, Zombie}, kind::Kind, looks::Theme};

/// The most of the dead up at once; each player past the first, this many
/// more, and a round brings this share of its dead again.
pub const MOST_UP: usize = 20;
const MORE_UP: usize = 6;
const MORE_DEAD: f64 = 0.5;
/// Seconds before the first round, and between rounds.
const FIRST_WAIT: f64 = 4.0;
pub const BREATHER: f64 = 10.0;
/// Seconds between the dead coming in, early on, and at the quickest.
const SPAWN_EVERY: (f64, f64) = (2.0, 0.5);
/// Of the open windows, the dead come in by one of this many nearest a
/// player.
const NEAREST: usize = 4;
/// How far about a window's spot outside they start.
const SCATTER: f64 = 2.0;

/// How many of the dead a round brings.
pub fn count(round: u32) -> u32 {
    match round {
        0 => 0,
        1..=5 => [6, 8, 11, 14, 18][round as usize - 1],
        _ => 18 + 2 * (round - 5),
    }
}

/// How many of the dead a round brings for `players`.
pub fn brings(round: u32, players: usize) -> u32 {
    (f64::from(count(round)) * (1.0 + MORE_DEAD * players.saturating_sub(1) as f64)).round() as u32
}

/// The most of the dead up at once for `players`.
pub fn most_up(players: usize) -> usize {
    MOST_UP + MORE_UP * players.saturating_sub(1)
}

/// How much a Shambler of a round takes to kill: fifty more each round to
/// the tenth, then a twelfth or so more each. (Gentler than it might be:
/// the guns are the ones made for the wilds, where the dead never
/// toughen.)
pub fn toughness(round: u32) -> f64 {
    if round <= 10 { 150.0 + 50.0 * f64::from(round.saturating_sub(1)) } else { 600.0 * 1.08f64.powi(round as i32 - 10) }
}

/// How fast one of a round walks, m/s, by luck (`roll`, `pick` 0–1): all
/// shamble at first; from the third round some jog, from the sixth some
/// run, more each round.
pub fn pace(round: u32, roll: f64, pick: f64) -> f64 {
    let r = f64::from(round);
    let runs = ((r - 5.0) * 0.1).clamp(0.0, 0.8);
    let jogs = ((r - 2.0) * 0.15).clamp(0.0, 0.6);
    let (lo, hi) = if roll < runs {
        (5.0, 5.6)
    } else if roll < runs + jogs * (1.0 - runs) {
        (3.0, 3.5)
    } else {
        (1.4, 1.9)
    };
    lo + (hi - lo) * pick
}

#[derive(Clone, Debug)]
pub struct Rounds {
    /// The round on (0 before the first).
    pub round: u32,
    /// Still to come in this round.
    left: u32,
    /// Till the next round, while between them; since this one began.
    pub between: f64,
    pub begun: f64,
    spawn_in: f64,
    dice: Dice,
    /// How many are playing, and which of them the next of the dead comes
    /// in near.
    players: usize,
    turn: usize,
}

impl Rounds {
    pub fn new(seed: u32, players: usize) -> Self {
        Self { round: 0, left: 0, between: FIRST_WAIT, begun: 0.0, spawn_in: 0.0, dice: Dice(seed | 1), players, turn: 0 }
    }

    /// Whether it's a breather between rounds.
    pub fn resting(&self) -> bool {
        self.between > 0.0
    }

    /// A step of the rounds, with the players' feet at `feet` and zones
    /// `open`: the next brought in, if it's time; the round over, or the
    /// next begun. Whether a round began this step.
    pub fn update(&mut self, world: &mut World, arena: &Arena, open: &[bool], feet: &[Vec3], dt: f64) -> bool {
        if self.between > 0.0 {
            self.between -= dt;
            if self.between > 0.0 {
                return false;
            }
            self.round += 1;
            self.begun = 0.0;
            self.left = brings(self.round, self.players);
            self.spawn_in = 0.5;
            return true;
        }
        self.begun += dt;
        let up = zombie::alive(world);
        if self.left == 0 {
            if up == 0 {
                self.between = BREATHER;
            }
            return false;
        }
        self.spawn_in -= dt;
        if self.spawn_in > 0.0 || up >= most_up(self.players) || feet.is_empty() {
            return false;
        }
        let every = (SPAWN_EVERY.0 - 0.15 * f64::from(self.round - 1)).max(SPAWN_EVERY.1);
        self.spawn_in = every;
        // By one of the open windows nearest a player, each in turn.
        let feet = feet[self.turn % feet.len()];
        self.turn += 1;
        let mut windows: Vec<(usize, f64)> = arena.windows.iter().enumerate().filter(|(_, w)| open[w.zone]).map(|(i, w)| (i, (w.outside - feet).length())).collect();
        if windows.is_empty() {
            return false;
        }
        windows.sort_by(|a, b| a.1.total_cmp(&b.1));
        windows.truncate(NEAREST);
        let (i, _) = windows[self.dice.next() as usize % windows.len()];
        let w = &arena.windows[i];
        let jitter = w.across() * ((self.dice.unit() - 0.5) * 2.0 * SCATTER) - w.inward * (self.dice.unit() * SCATTER);
        let at = w.from + jitter;
        let yaw = (-w.inward.x).atan2(-w.inward.z);
        let e = zombie::spawn_kind(world, at, yaw, Kind::Shambler, Theme::Drifter);
        let (roll, pick) = (self.dice.unit(), self.dice.unit());
        if let Some(mut z) = world.get_mut::<Zombie>(e) {
            ready(&mut z, self.round, i as u8, pace(self.round, roll, pick));
        }
        self.left -= 1;
        false
    }
}

/// One just come in, made a round's: as tough as the round, as fast as
/// it's picked, making for window `at`, and always knowing where the
/// player is.
fn ready(z: &mut Zombie, round: u32, at: u8, walk: f64) {
    z.hp = toughness(round);
    z.relentless = true;
    z.barrier = Some(at);
    z.state = State::Breach { at, t: 0.0 };
    z.gait.walk = walk;
    z.gait.crouch = walk;
    z.gait.sprint = z.gait.sprint.max(walk + 1.0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounds_grow_in_number_and_toughness() {
        assert_eq!((count(1), count(5), count(6)), (6, 18, 20));
        // Two players: half as many again, and more up at once.
        assert_eq!((brings(1, 1), brings(1, 2), brings(5, 2)), (6, 9, 27));
        assert_eq!((most_up(1), most_up(2)), (20, 26));
        assert!((1..40).all(|r| count(r + 1) >= count(r)));
        assert_eq!((toughness(1), toughness(5), toughness(10)), (150.0, 350.0, 600.0));
        assert!((toughness(11) - 648.0).abs() < 1e-9);
        assert!((1..40).all(|r| toughness(r + 1) > toughness(r)));
    }

    #[test]
    fn at_first_they_all_shamble_and_later_many_run() {
        let rolls = |round| (0..100).map(|i| pace(round, f64::from(i) / 100.0, 0.5)).collect::<Vec<_>>();
        assert!(rolls(1).iter().all(|&p| p < 2.0));
        assert!(rolls(2).iter().all(|&p| p < 2.0));
        assert!(rolls(3).iter().any(|&p| p > 3.0));
        assert!(rolls(5).iter().all(|&p| p < 4.0));
        let late = rolls(15);
        assert!(late.iter().filter(|&&p| p > 5.0).count() >= 70);
        // Never so fast they can't be walked away from.
        assert!(late.iter().all(|&p| p < crate::player::WALK));
    }
}
