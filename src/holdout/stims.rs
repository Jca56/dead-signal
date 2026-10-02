//! Stims: what a holdout's med stations sell (`station.rs`), each its own,
//! in its own part of the compound. Bought, it's jabbed into the arm there
//! and then, and it's in the blood from the moment the needle's in, till
//! they next go down: then they're all gone. What each does is here, and
//! nowhere else: the rest of the game asks.
//!
//! Lazarus is two things. Playing alone, it's a second life: when they'd
//! die it brings them back on their feet, the dead about them thrown off,
//! the once in a run (it can't be had again). Playing together, whoever
//! has it picks the others up in half the time.

use bevy_ecs::prelude::*;
use lntrn_math::Vec3;

use crate::player::{Body, SPRINT, WALK};
use crate::radio::Shown;
use crate::zombie::brain::Zombie;

/// One of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Stim {
    Bulwark,
    Twitch,
    Rush,
    Lazarus,
}

pub const ALL: [Stim; 4] = [Stim::Bulwark, Stim::Twitch, Stim::Rush, Stim::Lazarus];

/// Bulwark: how much more health. Twitch: how many times as fast a reload
/// goes. Rush: how many times as fast a sprint is. Lazarus: how many
/// times as fast someone's picked up; and, alone, the share of their
/// health they're brought back with, how far about them the dead are
/// thrown off and how hard (m/s), and how long nothing can hurt them
/// after.
pub const HEALTH: f64 = 75.0;
pub const RELOADS: f64 = 2.0;
pub const SPRINTS: f64 = 1.2;
pub const REVIVES: f64 = 2.0;
pub const BACK_WITH: f64 = 0.5;
pub const THROWS: (f64, f64) = (6.0, 9.0);
pub const SAFE_FOR: f64 = 2.0;
/// How long the injector takes to come up and to go down, how long the
/// jab's clip is, and how far into it the needle's in: it's theirs from
/// then.
const RAISE: f64 = 0.22;
const LOWER: f64 = 0.18;
pub const JAB: f64 = 1.5;
pub const IN_AT: f64 = 0.5;

impl Stim {
    pub fn index(self) -> usize {
        self as usize
    }

    pub fn name(self) -> &'static str {
        match self {
            Stim::Bulwark => "BULWARK",
            Stim::Twitch => "TWITCH",
            Stim::Rush => "RUSH",
            Stim::Lazarus => "LAZARUS",
        }
    }

    /// What its station asks, in points.
    pub fn price(self) -> u32 {
        match self {
            Stim::Bulwark => 2500,
            Stim::Twitch => 3000,
            Stim::Rush => 2000,
            Stim::Lazarus => 1500,
        }
    }

    /// Its colour, as it looks: its station's lights, what's in its
    /// injector, its mark by the health.
    pub fn colour(self) -> [f32; 3] {
        match self {
            Stim::Bulwark => [1.0, 0.20, 0.16],
            Stim::Twitch => [0.30, 1.0, 0.36],
            Stim::Rush => [1.0, 0.80, 0.16],
            Stim::Lazarus => [0.30, 0.72, 1.0],
        }
    }

    /// What it does, in a few words (`alone`: playing alone).
    pub fn does(self, alone: bool) -> &'static str {
        match self {
            Stim::Bulwark => "+75 HEALTH",
            Stim::Twitch => "FAST RELOAD",
            Stim::Rush => "FAST SPRINT",
            Stim::Lazarus if alone => "SECOND LIFE",
            Stim::Lazarus => "FAST REVIVE",
        }
    }
}

/// How far a jab's got.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Step {
    /// Wanted, once what's in hand is put away.
    Wanted,
    Raise,
    In,
    Lower,
}

/// A stim going in: which, and how far along.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Jab {
    stim: Stim,
    step: Step,
    t: f64,
}

/// What's heard (and comes) of a jab.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cue {
    /// The injector's up, in hand.
    Out,
    /// The needle's in: it's theirs.
    In(Stim),
}

/// What's in a player's blood, and what's on its way in.
#[derive(Clone, Copy, Debug, Default)]
pub struct Stims {
    held: [bool; ALL.len()],
    /// Lazarus has brought them back, alone: it's not to be had again.
    spent: bool,
    jab: Option<Jab>,
    /// Seconds left that nothing can hurt them (just brought back).
    safe: f64,
}

impl Stims {
    pub fn has(&self, stim: Stim) -> bool {
        self.held[stim.index()]
    }

    /// Every one in their blood, in their order.
    pub fn all(&self) -> impl Iterator<Item = Stim> + '_ {
        ALL.into_iter().filter(|s| self.has(*s))
    }

    /// Whether Lazarus has been their second life already.
    pub fn spent(&self) -> bool {
        self.spent
    }

    /// Whether one's going in: the hands are its.
    pub fn jabbing(&self) -> bool {
        self.jab.is_some()
    }

    /// Whether nothing can hurt them just now.
    pub fn safe(&self) -> bool {
        self.safe > 0.0
    }

    /// Begin jabbing `stim` in (once the hands are free of what they hold).
    pub fn begin(&mut self, stim: Stim) {
        if self.jab.is_none() {
            self.jab = Some(Jab { stim, step: Step::Wanted, t: 0.0 });
        }
    }

    /// `stim`'s in their blood.
    pub fn take(&mut self, stim: Stim) {
        self.held[stim.index()] = true;
    }

    /// Down: all of it's gone, and a jab not yet in. (What Lazarus has
    /// done already stays done.)
    pub fn lose(&mut self) {
        *self = Self { spent: self.spent, ..Self::default() };
    }

    /// Alone, about to die: Lazarus brings them back, if it's in their
    /// blood. It's used up, for good. Whether it did.
    pub fn second_life(&mut self) -> bool {
        let can = self.has(Stim::Lazarus) && !self.spent;
        if can {
            self.held[Stim::Lazarus.index()] = false;
            (self.spent, self.safe) = (true, SAFE_FOR);
        }
        can
    }

    /// How many times as fast they reload; and pick someone up.
    pub fn reloads(&self) -> f64 {
        if self.has(Stim::Twitch) { RELOADS } else { 1.0 }
    }

    pub fn revives(&self) -> f64 {
        if self.has(Stim::Lazarus) { REVIVES } else { 1.0 }
    }

    /// The share of a sprint's lead over a walk they have, their gear
    /// leaving them `share` of it: what makes the sprint itself that many
    /// times as fast.
    pub fn sprint(&self, share: f64) -> f64 {
        if !self.has(Stim::Rush) {
            return share;
        }
        let sprint = WALK + (SPRINT - WALK) * share;
        (sprint * SPRINTS - WALK) / (SPRINT - WALK)
    }

    /// Move on by `dt`, the hands `empty` (what they held put away) or
    /// not; what's heard, and what's gone in.
    pub fn update(&mut self, empty: bool, dt: f64) -> Option<Cue> {
        self.safe = (self.safe - dt).max(0.0);
        let jab = self.jab.as_mut()?;
        let before = jab.t;
        jab.t += dt;
        match jab.step {
            Step::Wanted if empty => {
                (jab.step, jab.t) = (Step::Raise, 0.0);
                return Some(Cue::Out);
            }
            Step::Wanted => {}
            Step::Raise if jab.t >= RAISE => (jab.step, jab.t) = (Step::In, 0.0),
            Step::In => {
                let stim = jab.stim;
                if jab.t >= JAB {
                    (jab.step, jab.t) = (Step::Lower, 0.0);
                }
                if before < IN_AT && before + dt >= IN_AT {
                    self.take(stim);
                    return Some(Cue::In(stim));
                }
            }
            Step::Lower if jab.t >= LOWER => self.jab = None,
            Step::Raise | Step::Lower => {}
        }
        None
    }

    /// The injector as it's drawn, while it's in view, and what's in it.
    pub fn shown(&self) -> Option<(Shown, Stim)> {
        let jab = self.jab?;
        let shown = match jab.step {
            Step::Wanted => return None,
            Step::Raise => Shown { clip: "Jab", t: Some(0.0), stowed: 1.0 - (jab.t / RAISE).min(1.0) },
            Step::In => Shown { clip: "Jab", t: Some(jab.t), stowed: 0.0 },
            Step::Lower => Shown { clip: "Jab", t: Some(JAB), stowed: (jab.t / LOWER).min(1.0) },
        };
        Some((shown, jab.stim))
    }
}

/// The dead about `at` thrown off it and sent stumbling (Lazarus, bringing
/// someone back): none of them hurt.
pub fn throw_off(world: &mut World, at: Vec3) {
    for (mut z, mut body) in world.query::<(&mut Zombie, &mut Body)>().iter_mut(world) {
        let away = Vec3::new(body.pos.x - at.x, 0.0, body.pos.z - at.z);
        let far = away.length();
        if z.dead() || far > THROWS.0 || (body.pos.y - at.y).abs() > 2.5 {
            continue;
        }
        body.push += away.try_normalize().unwrap_or(Vec3::X) * (THROWS.1 * (1.0 - 0.5 * far / THROWS.0));
        z.stumble();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f64 = 1.0 / 60.0;

    /// `seconds` of `stims`, the hands empty; what was heard.
    fn run(stims: &mut Stims, seconds: f64) -> Vec<Cue> {
        (0..(seconds / DT).round() as usize).filter_map(|_| stims.update(true, DT)).collect()
    }

    #[test]
    fn a_jab_waits_for_the_hands_comes_up_goes_in_and_goes_away() {
        let mut s = Stims::default();
        assert!(!s.jabbing() && s.shown().is_none() && s.all().count() == 0);
        s.begin(Stim::Twitch);
        // What's in hand isn't away yet: it waits, out of sight.
        for _ in 0..30 {
            assert_eq!(s.update(false, DT), None);
        }
        assert!(s.jabbing() && s.shown().is_none() && !s.has(Stim::Twitch));
        // Away: up it comes.
        assert_eq!(s.update(true, DT), Some(Cue::Out));
        let (shown, stim) = s.shown().expect("coming up");
        assert!(stim == Stim::Twitch && shown.clip == "Jab" && shown.stowed > 0.9 && shown.t == Some(0.0));
        // Another can't be begun on top of it.
        s.begin(Stim::Rush);
        // The needle's in half a second into the clip: theirs from then.
        assert_eq!(run(&mut s, RAISE + IN_AT - 0.05), []);
        assert!(!s.has(Stim::Twitch) && s.shown().is_some_and(|(sh, _)| sh.stowed == 0.0 && sh.t.is_some_and(|t| t > 0.3)));
        assert_eq!(run(&mut s, 0.1), [Cue::In(Stim::Twitch)]);
        assert!(s.has(Stim::Twitch) && s.jabbing());
        // The clip's played out, and it goes down.
        assert_eq!(run(&mut s, JAB - IN_AT), []);
        assert!(s.shown().is_some_and(|(sh, _)| sh.stowed > 0.0 && sh.t == Some(JAB)));
        assert_eq!(run(&mut s, LOWER + 0.05), []);
        assert!(!s.jabbing() && s.shown().is_none());
        assert_eq!(s.all().collect::<Vec<_>>(), [Stim::Twitch], "the second was never begun");
    }

    #[test]
    fn each_does_what_it_does_and_down_they_re_all_gone() {
        let mut s = Stims::default();
        assert_eq!((s.reloads(), s.revives(), s.sprint(1.0), s.sprint(0.5)), (1.0, 1.0, 1.0, 0.5));
        for stim in ALL {
            s.take(stim);
        }
        assert_eq!(s.all().collect::<Vec<_>>(), ALL);
        assert_eq!((s.reloads(), s.revives()), (RELOADS, REVIVES));
        // A sprint a fifth faster, whatever their gear leaves them of it.
        for share in [1.0, 0.6] {
            let (plain, rushed) = (WALK + (SPRINT - WALK) * share, WALK + (SPRINT - WALK) * s.sprint(share));
            assert!((rushed - plain * SPRINTS).abs() < 1e-9, "{rushed} for {plain}");
        }
        // Mid-jab, down: all gone, the jab too.
        s.begin(Stim::Bulwark);
        s.lose();
        assert!(s.all().count() == 0 && !s.jabbing() && !s.spent());
        assert_eq!((s.reloads(), s.revives(), s.sprint(1.0)), (1.0, 1.0, 1.0));
    }

    #[test]
    fn lazarus_is_a_second_life_the_once() {
        let mut s = Stims::default();
        assert!(!s.second_life(), "not in their blood");
        s.take(Stim::Lazarus);
        s.take(Stim::Bulwark);
        assert!(s.second_life());
        assert!(!s.has(Stim::Lazarus) && s.has(Stim::Bulwark) && s.spent() && s.safe());
        // Nothing can hurt them for a moment; then it can.
        run(&mut s, SAFE_FOR - 0.1);
        assert!(s.safe());
        run(&mut s, 0.2);
        assert!(!s.safe());
        // Had again (it can't be, alone: its station won't sell it), it'd
        // not do it twice; and going down doesn't undo that.
        s.take(Stim::Lazarus);
        assert!(!s.second_life());
        s.lose();
        assert!(s.spent());
    }

    #[test]
    fn the_dead_about_them_are_thrown_off_and_those_further_are_left() {
        let mut world = World::new();
        let mut at = |x: f64, z: f64| world.spawn((Zombie::new(0.0, 3), Body::at(Vec3::new(x, 0.0, z)))).id();
        let (near, far) = (at(2.0, 0.0), at(0.0, THROWS.0 + 1.0));
        throw_off(&mut world, Vec3::ZERO);
        let push = |world: &World, e| world.get::<Body>(e).map(|b| b.push).unwrap();
        assert!(push(&world, near).x > THROWS.1 * 0.5 && push(&world, near).z == 0.0, "{:?}", push(&world, near));
        assert_eq!(push(&world, far), Vec3::ZERO);
        assert!(world.get::<Zombie>(near).is_some_and(|z| !z.dead()), "thrown, not hurt");
    }

    #[test]
    fn a_med_station_for_each_stands_in_its_own_part_of_the_compound_out_from_a_wall() {
        use crate::holdout::arena::Wares;
        use crate::holdout::tests::{built, world_of};
        use crate::holdout::{Aimed, station};
        let (world, h) = world_of(built());
        let at = |stim: Stim| {
            let found: Vec<usize> = (0..h.arena.buys.len()).filter(|&i| h.arena.buys[i].wares == Wares::Stim(stim)).collect();
            assert_eq!(found.len(), 1, "one station for {}", stim.name());
            found[0]
        };
        assert_eq!(ALL.map(|stim| h.arena.zones[h.arena.buys[at(stim)].zone]), ["BARRACKS", "BROADCAST FLOOR", "MOTOR POOL", "CONTROL ROOM"]);
        let solid = &world.resource::<crate::world::Solid>().0;
        for stim in ALL {
            // Its cabinet's there, solid, its front where it's used from;
            // and there's room to stand before it.
            let b = h.arena.buys[at(stim)];
            let chest = b.at - Vec3::new(0.0, 0.4, 0.0);
            let front = solid.raycast(chest + b.facing * 1.2, -b.facing, 2.0).map(|hit| (hit.point - chest).dot(b.facing));
            assert!(front.is_some_and(|d| d.abs() < 0.03), "{}: its front's {front:?} out from where it's used", stim.name());
            assert!(solid.raycast(chest - b.facing * (station::DEEP + 0.3), b.facing, station::DEEP).is_some(), "{}: no wall behind it", stim.name());
        }
        // Lazarus is where a holdout starts: stood before it, looking at
        // it, it's what E would buy.
        let b = h.arena.buys[at(Stim::Lazarus)];
        let aimed = h.aimed(&world, b.at + b.facing * 1.2, -b.facing);
        assert_eq!(aimed, Some(Aimed::Buy(at(Stim::Lazarus))));
        assert_eq!(aimed.and_then(|a| h.stim_at(a)), Some(Stim::Lazarus));
        assert_eq!(h.prompt(aimed.unwrap(), &crate::holdout::Holdout::loadout(), None), "LAZARUS [1500]");
    }
}
