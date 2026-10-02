//! The radio's signal: what's paid to call anything in. Each player's
//! own, charged by what they kill (more for one to the head or by hand,
//! a bar at once for a Juggernaut), held in a meter of a few bars, and
//! spent a whole bar or more at a time. (Counted in sixtieths of a bar,
//! so nothing's lost in the adding.)

use crate::stats::Stats;

/// How many bars the meter holds.
pub const BARS: u32 = 5;
/// A bar; what a kill charges, and one to the head or by hand; and a
/// Juggernaut's, over its kill.
pub const BAR: u32 = 60;
pub const KILL: u32 = 6;
pub const KEEN: u32 = 9;
const JUGGERNAUT: u32 = BAR;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Signal {
    units: u32,
    /// The count as it last stood: kills, those to the head, those by hand.
    seen: [u32; 3],
}

impl Signal {
    /// How much there is, in bars.
    pub fn bars(&self) -> f64 {
        f64::from(self.units) / f64::from(BAR)
    }

    fn add(&mut self, units: u32) {
        self.units = (self.units + units).min(BARS * BAR);
    }

    /// Charge it with what's been killed since last asked, by their count
    /// `stats`.
    pub fn charge(&mut self, stats: &Stats) {
        let now = [stats.kills(), stats.headshot_kills, stats.melee_kills];
        let new = [0, 1, 2].map(|i| now[i].saturating_sub(self.seen[i]));
        self.seen = now;
        let keen = (new[1] + new[2]).min(new[0]);
        self.add(KILL * (new[0] - keen) + KEEN * keen);
    }

    /// What's been killed since last asked charges nothing (any hit was a
    /// kill: INSTAKILL's up).
    pub fn skip(&mut self, stats: &Stats) {
        self.seen = [stats.kills(), stats.headshot_kills, stats.melee_kills];
    }

    /// `kills` of theirs weren't their own doing (what they called in
    /// did it): they charge nothing.
    pub fn forgo(&mut self, kills: u32) {
        self.seen[0] += kills;
    }

    /// A Juggernaut killed: a bar at once.
    pub fn juggernaut(&mut self) {
        self.add(JUGGERNAUT);
    }

    /// `bars` of it back (what was called for couldn't come).
    pub fn refund(&mut self, bars: u32) {
        self.add(bars * BAR);
    }

    /// (The dev's.) All of it.
    pub fn fill(&mut self) {
        self.units = BARS * BAR;
    }

    /// Whether there's `bars` of it to spend.
    pub fn has(&self, bars: u32) -> bool {
        self.units >= bars * BAR
    }

    /// Spend `bars` of it, if there's that much. Whether it was spent.
    pub fn spend(&mut self, bars: u32) -> bool {
        let has = self.has(bars);
        if has {
            self.units -= bars * BAR;
        }
        has
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ten_kills_charge_a_bar_fewer_to_the_head_or_by_hand_and_it_holds_only_so_much() {
        let mut s = Signal::default();
        let mut stats = Stats::default();
        assert!(!s.has(1) && s.bars() == 0.0);
        // Nine to the body: not yet. The tenth: a bar.
        stats.gun_kills = 9;
        s.charge(&stats);
        assert!(!s.has(1) && (s.bars() - 0.9).abs() < 1e-9);
        stats.gun_kills = 10;
        s.charge(&stats);
        s.charge(&stats);
        assert!(s.has(1) && !s.has(2) && s.bars() == 1.0, "counted once");
        // To the head, by hand, and a fire's: half again for the first two.
        stats.gun_kills += 2;
        stats.headshot_kills += 2;
        stats.melee_kills += 2;
        stats.blast_kills += 1;
        s.charge(&stats);
        assert_eq!(s.units, BAR + 4 * KEEN + KILL);
        // A Juggernaut: a bar. And no more than the meter holds.
        s.juggernaut();
        assert!(s.has(2));
        stats.gun_kills += 500;
        s.charge(&stats);
        assert!(s.bars() == f64::from(BARS) && s.has(BARS) && !s.has(BARS + 1));
    }

    #[test]
    fn kills_skipped_charge_nothing_then_or_after() {
        let mut s = Signal::default();
        let mut stats = Stats { gun_kills: 40, headshot_kills: 10, ..Stats::default() };
        s.skip(&stats);
        s.charge(&stats);
        assert_eq!(s.bars(), 0.0);
        stats.gun_kills = 50;
        s.charge(&stats);
        assert_eq!(s.bars(), 1.0, "and what's killed after counts as it did");
        // Twelve more, ten of them a strafing run's: only the two are theirs.
        stats.gun_kills = 52;
        stats.blast_kills = 10;
        s.forgo(10);
        s.charge(&stats);
        assert_eq!(s.units, BAR + 2 * KILL);
    }

    #[test]
    fn it_s_spent_whole_bars_at_a_time_or_not_at_all() {
        let mut s = Signal::default();
        s.fill();
        assert!(s.spend(2) && s.bars() == 3.0);
        assert!(!s.spend(4) && s.bars() == 3.0, "not enough: none's spent");
        assert!(s.spend(3) && s.bars() == 0.0 && !s.spend(1));
    }
}
