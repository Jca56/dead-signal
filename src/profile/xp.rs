//! XP: what a run earned (kills, headshots, searching, and above all
//! getting out, the longer out there the more), and the levels it fills.

use crate::ending::Outcome;
use crate::exits::Way;
use crate::stats::Stats;

/// Per kill, and more for one through the head.
const KILL: u32 = 10;
const HEADSHOT_KILL: u32 = 5;
/// Per container searched; a supply cage is worth more.
const SEARCH: u32 = 5;
const CAGE: u32 = 25;
/// For getting out, and more for the harder ways; and for every so many
/// seconds out there.
const OUT: u32 = 100;
const OUT_BY_RADIO: u32 = 100;
const OUT_BY_TRUCK: u32 = 50;
const SECONDS_A_POINT: f64 = 10.0;

/// What a run earned, line by line, and whether it's kept (it's lost with
/// the player).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Earned {
    pub lines: Vec<(&'static str, u32)>,
    pub kept: bool,
}

impl Earned {
    pub fn total(&self) -> u32 {
        self.lines.iter().map(|(_, n)| n).sum()
    }

    /// What's actually added: all of it, or none.
    pub fn banked(&self) -> u32 {
        if self.kept { self.total() } else { 0 }
    }
}

/// What the run with `stats`, ended by `outcome`, earned.
pub fn earned(stats: &Stats, outcome: Outcome) -> Earned {
    let mut lines = Vec::new();
    let mut add = |label, n: u32| {
        if n > 0 {
            lines.push((label, n));
        }
    };
    add("Kills", stats.kills() * KILL);
    add("Headshot kills", stats.headshot_kills * HEADSHOT_KILL);
    let cages = stats.cages_opened.min(stats.containers_searched);
    add("Searching", (stats.containers_searched - cages) * SEARCH + cages * CAGE);
    let kept = matches!(outcome, Outcome::Extracted(_));
    if let Outcome::Extracted(way) = outcome {
        add(
            "Getting out",
            OUT + match way {
                Way::Radio => OUT_BY_RADIO,
                Way::Truck => OUT_BY_TRUCK,
                Way::Road => 0,
            },
        );
        add("Time out there", (stats.seconds / SECONDS_A_POINT) as u32);
    }
    Earned { lines, kept }
}

/// XP from one level to the next, starting at level 1.
pub fn to_next(level: u32) -> u32 {
    500 + 150 * (level - 1)
}

/// Where `xp` stands: the level, XP into it, and what the next takes.
pub fn level(xp: u32) -> (u32, u32, u32) {
    let (mut level, mut left) = (1, xp);
    while left >= to_next(level) {
        left -= to_next(level);
        level += 1;
    }
    (level, left, to_next(level))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_run_earns_by_its_deeds_and_keeps_it_only_out() {
        let stats = Stats { gun_kills: 10, melee_kills: 2, headshot_kills: 4, containers_searched: 5, cages_opened: 1, seconds: 305.0, ..Stats::default() };
        let out = earned(&stats, Outcome::Extracted(Way::Radio));
        let line = |e: &Earned, l: &str| e.lines.iter().find(|(k, _)| *k == l).map_or(0, |(_, n)| *n);
        assert_eq!(line(&out, "Kills"), 120);
        assert_eq!(line(&out, "Headshot kills"), 20);
        assert_eq!(line(&out, "Searching"), 4 * 5 + 25);
        assert_eq!(line(&out, "Getting out"), 200);
        assert_eq!(line(&out, "Time out there"), 30);
        assert_eq!(out.banked(), 120 + 20 + 45 + 200 + 30);
        let dead = earned(&stats, Outcome::Died(1.0));
        assert_eq!(line(&dead, "Getting out"), 0);
        assert_eq!((dead.banked(), dead.total()), (0, 185), "shown, and lost");
    }

    #[test]
    fn each_level_takes_a_little_more() {
        assert_eq!(level(0), (1, 0, 500));
        assert_eq!(level(499), (1, 499, 500));
        assert_eq!(level(500), (2, 0, 650));
        assert_eq!(level(500 + 650 + 10), (3, 10, 800));
    }
}
