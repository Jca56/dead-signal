//! The player between runs: what's kept in the stash, what's carried into
//! the next run (the backpack and pockets, the loadout), the XP earned
//! (`xp.rs`) and the perks it's bought (`perks.rs`); the money made
//! trading and what it's bought (`trade.rs`); kept on disk (`save.rs`).

pub mod perks;
pub mod save;
pub mod trade;
pub mod xp;

use perks::{Perk, Perks, RANKS};
use crate::loot::bag::Bag;
use crate::loot::grid::Grid;
use crate::loot::{Kind, Stack};

/// The stash's size to begin with, cells across and down (it's made
/// bigger by trading).
pub const STASH: (u8, u8) = (10, 10);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Profile {
    /// What the player called them (empty: not named).
    pub name: String,
    pub stash: Grid,
    pub loadout: Bag,
    pub xp: u32,
    pub runs: u32,
    pub extractions: u32,
    pub perks: Perks,
    /// Money, and how many times the stash has been made bigger; what's
    /// been bought of Sparks' rarer things.
    pub money: u32,
    pub stash_tier: u8,
    pub bought: trade::Bought,
}

impl Profile {
    /// Someone new: a loaded pistol in hand, a daypack on, and a little in
    /// the stash to start with.
    pub fn new_player() -> Self {
        let mut stash = Grid::new(STASH.0, STASH.1);
        for stack in [Stack::new(Kind::Rounds, 60), Stack::new(Kind::Bandage, 2), Stack::one(Kind::Medkit)] {
            stash.place(stack);
        }
        let mut loadout = Bag::sized((0, 0), (0, 0));
        *loadout.worn_mut(crate::loot::gear::Wear::Back) = Some(Stack::one(Kind::Daypack));
        loadout.refit(Perks::default().fit());
        loadout.add(starting_pistol());
        Self { name: String::new(), stash, loadout, xp: 0, runs: 0, extractions: 0, perks: Perks::default(), money: 0, stash_tier: 0, bought: trade::Bought::default() }
    }

    /// Perk points not yet spent.
    pub fn points_left(&self) -> u32 {
        perks::points(xp::level(self.xp).0).saturating_sub(self.perks.spent())
    }

    /// Take the next rank of `p`, if there are the points. Whether it was
    /// taken.
    pub fn raise(&mut self, p: Perk) -> bool {
        let rank = self.perks.rank(p);
        if rank >= RANKS || self.points_left() < perks::cost(rank) {
            return false;
        }
        self.perks.set(p, rank + 1);
        self.refit()
    }

    /// Give back the last rank of `p` (free, for now). Whether it could be
    /// given back: not if the bag would shrink and the stash has no room
    /// for what no longer fits.
    pub fn lower(&mut self, p: Perk) -> bool {
        let rank = self.perks.rank(p);
        if rank == 0 {
            return false;
        }
        let before = self.clone();
        self.perks.set(p, rank - 1);
        if !self.refit() {
            *self = before;
            return false;
        }
        true
    }

    /// The bag made the size the perks say: everything laid where it was
    /// if it still fits, else anywhere in it, else in the stash. Whether
    /// everything found a place (if not, nothing's been lost: the caller
    /// puts things back as they were).
    pub fn refit(&mut self) -> bool {
        let homeless = self.loadout.refit(self.perks.fit());
        homeless.into_iter().all(|s| self.stash.place(s).count == 0)
    }

    /// A run is over, `bag` what was carried at its end. Got out: it all
    /// comes home, and the XP with it. Dead: only the pockets survive (not
    /// the weapons in hand, nor anything worn), and no XP; what the pockets
    /// held that won't go in them now (cargo pants lost) goes to the stash.
    pub fn settle(&mut self, got_out: bool, bag: Bag, earned: u32) {
        self.runs += 1;
        if got_out {
            self.loadout = bag;
            self.xp += earned;
            self.extractions += 1;
        } else {
            self.loadout = Bag { pockets: bag.pockets, ..Bag::sized((0, 0), (0, 0)) };
            self.refit();
        }
    }

    /// Take the loadout into a run (it's in the run's hands now: lost with
    /// it, if it comes to that).
    pub fn take_loadout(&mut self) -> Bag {
        let mut empty = Bag::sized((0, 0), (0, 0));
        empty.refit(self.perks.fit());
        std::mem::replace(&mut self.loadout, empty)
    }
}

/// What a new survivor (or one from before there were weapons to carry)
/// starts with in hand: a pistol, loaded.
pub fn starting_pistol() -> Stack {
    Stack::gun(Kind::Pistol, crate::weapon::Weapon::Pistol.spec().mag)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loot::bag::Slot;

    #[test]
    fn points_buy_ranks_and_a_refund_never_loses_a_thing() {
        let mut p = Profile::new_player();
        assert_eq!(p.points_left(), 1, "a point at level 1");
        // A rucksack on (6×4), for Pack Mule to grow.
        *p.loadout.worn_mut(crate::loot::gear::Wear::Back) = Some(Stack::one(Kind::Rucksack));
        assert!(p.refit());
        assert!(p.raise(Perk::PackMule));
        assert_eq!(p.loadout.pack.w, 7, "the pack grew");
        assert!(!p.raise(Perk::PackMule), "rank 2 costs 2");
        p.xp = 500 + 650 + 800; // level 4: four points, one spent.
        assert!(p.raise(Perk::PackMule));
        assert_eq!(p.points_left(), 1);
        // Fill the 7×5 pack's last column; give the rank back: the
        // column's things go to the stash.
        for y in 0..5 {
            p.loadout.pack.put(Stack::one(Kind::Watch), 6, y, false);
        }
        p.stash = Grid::new(super::STASH.0, super::STASH.1);
        assert!(p.lower(Perk::PackMule));
        assert_eq!((p.loadout.pack.w, p.loadout.pack.h), (7, 4));
        assert_eq!(p.loadout.count(Kind::Watch) + p.stash.count(Kind::Watch), 5, "none lost");
        // A full pack (and pockets, where what no longer fits would go
        // next) and a full stash: the refund is refused, and nothing moves.
        let mut p2 = p.clone();
        p2.stash = Grid::new(super::STASH.0, super::STASH.1);
        while p2.stash.place(Stack::one(Kind::Ring)).count == 0 {}
        while p2.loadout.pack.place(Stack::one(Kind::Ring)).count == 0 {}
        while p2.loadout.pockets.place(Stack::one(Kind::Ring)).count == 0 {}
        let before = p2.clone();
        assert!(!p2.lower(Perk::PackMule));
        assert_eq!(p2, before);
    }

    #[test]
    fn someone_new_has_a_little_to_start_with_and_a_pistol_in_hand() {
        let p = Profile::new_player();
        assert_eq!(p.stash.count(Kind::Rounds), 60);
        assert_eq!(p.stash.count(Kind::Bandage), 2);
        assert_eq!(p.stash.count(Kind::Medkit), 1);
        assert_eq!(p.loadout.slot(Slot::Sidearm), Some(starting_pistol()));
        assert!(p.loadout.pack.items.is_empty() && p.loadout.pockets.items.is_empty());
        assert_eq!((p.xp, p.runs), (0, 0));
    }

    #[test]
    fn getting_out_keeps_it_all_and_dying_keeps_the_pockets() {
        let mut bag = Bag::empty();
        bag.pack.place(Stack::one(Kind::GoldBar));
        bag.pockets.place(Stack::one(Kind::Ring));
        bag.add(Stack::gun(Kind::Pistol, 5));
        let mut p = Profile::new_player();
        p.settle(true, bag.clone(), 120);
        assert_eq!((p.loadout.clone(), p.xp, p.extractions), (bag.clone(), 120, 1));
        let mut p = Profile::new_player();
        p.settle(false, bag, 500);
        assert_eq!(p.loadout.pack.count(Kind::GoldBar), 0, "the pack is lost");
        assert_eq!(p.loadout.pockets.count(Kind::Ring), 1, "the pockets are safe");
        assert_eq!(p.loadout.slots, [None; 3], "the weapons in hand are lost");
        assert_eq!((p.xp, p.runs, p.extractions), (0, 1, 0), "no XP for dying");
    }
}
