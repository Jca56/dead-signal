//! The player between runs: what's kept in the stash, what's carried into
//! the next run (the backpack and pockets, the loadout), and the XP earned
//! (`xp.rs`); kept on disk (`save.rs`).

pub mod save;
pub mod xp;

use crate::loot::bag::Bag;
use crate::loot::grid::Grid;
use crate::loot::{Kind, Stack};

/// The stash's size, cells across and down.
pub const STASH: (u8, u8) = (10, 10);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Profile {
    pub stash: Grid,
    pub loadout: Bag,
    pub xp: u32,
    pub runs: u32,
    pub extractions: u32,
}

impl Profile {
    /// Someone new: an empty bag, and a little in the stash to start with.
    pub fn new_player() -> Self {
        let mut stash = Grid::new(STASH.0, STASH.1);
        for stack in [Stack::new(Kind::Rounds, 60), Stack::new(Kind::Bandage, 2), Stack::one(Kind::Medkit)] {
            stash.place(stack);
        }
        Self { stash, loadout: Bag::empty(), xp: 0, runs: 0, extractions: 0 }
    }

    /// A run is over, `bag` what was carried at its end. Got out: it all
    /// comes home, and the XP with it. Dead: only the pockets survive, and
    /// no XP.
    pub fn settle(&mut self, got_out: bool, bag: Bag, earned: u32) {
        self.runs += 1;
        if got_out {
            self.loadout = bag;
            self.xp += earned;
            self.extractions += 1;
        } else {
            self.loadout = Bag { pack: Grid::new(bag.pack.w, bag.pack.h), pockets: bag.pockets };
        }
    }

    /// Take the loadout into a run (it's in the run's hands now: lost with
    /// it, if it comes to that).
    pub fn take_loadout(&mut self) -> Bag {
        std::mem::take(&mut self.loadout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn someone_new_has_a_little_to_start_with_and_nothing_on_them() {
        let p = Profile::new_player();
        assert_eq!(p.stash.count(Kind::Rounds), 60);
        assert_eq!(p.stash.count(Kind::Bandage), 2);
        assert_eq!(p.stash.count(Kind::Medkit), 1);
        assert_eq!(p.loadout, Bag::empty());
        assert_eq!((p.xp, p.runs), (0, 0));
    }

    #[test]
    fn getting_out_keeps_it_all_and_dying_keeps_the_pockets() {
        let mut bag = Bag::empty();
        bag.pack.place(Stack::one(Kind::GoldBar));
        bag.pockets.place(Stack::one(Kind::Ring));
        let mut p = Profile::new_player();
        p.settle(true, bag.clone(), 120);
        assert_eq!((p.loadout.clone(), p.xp, p.extractions), (bag.clone(), 120, 1));
        let mut p = Profile::new_player();
        p.settle(false, bag, 500);
        assert_eq!(p.loadout.pack.count(Kind::GoldBar), 0, "the pack is lost");
        assert_eq!(p.loadout.pockets.count(Kind::Ring), 1, "the pockets are safe");
        assert_eq!((p.xp, p.runs, p.extractions), (0, 1, 0), "no XP for dying");
    }
}
