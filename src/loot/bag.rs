//! What the player carries: a backpack grid and two-by-two pockets, one
//! store of things as far as the rest of the game cares (rounds to load,
//! kits to use, what it's all worth).

use super::grid::Grid;
use super::{Kind, Stack};

pub const PACK: (u8, u8) = (6, 4);
pub const POCKETS: (u8, u8) = (2, 2);
/// A couple of magazines' worth, for trying things with (tests).
#[cfg(test)]
pub const START_ROUNDS: u32 = 24;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bag {
    pub pack: Grid,
    pub pockets: Grid,
}

impl Default for Bag {
    fn default() -> Self {
        Self::empty()
    }
}

impl Bag {
    /// Nothing in it at all.
    pub fn empty() -> Self {
        Self { pack: Grid::new(PACK.0, PACK.1), pockets: Grid::new(POCKETS.0, POCKETS.1) }
    }

    /// A couple of magazines' worth in a pocket (for tests).
    #[cfg(test)]
    pub fn with_rounds() -> Self {
        let mut bag = Self::empty();
        bag.pockets.place(Stack::new(Kind::Rounds, START_ROUNDS));
        bag
    }

    /// Put `stack` away: topping up what's already carried, then in the
    /// pack, then the pockets. What didn't fit.
    pub fn add(&mut self, stack: Stack) -> Stack {
        let stack = self.pockets.top_up(stack);
        let stack = self.pack.top_up(stack);
        let stack = self.pack.place(stack);
        self.pockets.place(stack)
    }

    pub fn count(&self, kind: Kind) -> u32 {
        self.pack.count(kind) + self.pockets.count(kind)
    }

    /// Take up to `n` of `kind`, pockets first. How many were taken.
    pub fn remove(&mut self, kind: Kind, n: u32) -> u32 {
        let from_pockets = self.pockets.remove(kind, n);
        from_pockets + self.pack.remove(kind, n - from_pockets)
    }

    pub fn value(&self) -> u32 {
        self.pack.value() + self.pockets.value()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Whether all of `stack` would go in.
    fn room_for(bag: &Bag, stack: Stack) -> bool {
        bag.clone().add(stack).count == 0
    }

    #[test]
    fn rounds_in_a_pocket_leave_room_in_the_pack() {
        let bag = Bag::with_rounds();
        assert_eq!(bag.count(Kind::Rounds), START_ROUNDS);
        assert_eq!(bag.pockets.items.len(), 1);
        assert!(bag.pack.items.is_empty());
    }

    #[test]
    fn it_fills_the_pack_then_the_pockets_then_says_no() {
        let mut bag = Bag::with_rounds();
        // Six batteries fill the 6×4 pack; then a 2×2 pocket takes one more.
        for _ in 0..6 {
            assert_eq!(bag.add(Stack::one(Kind::Battery)).count, 0);
        }
        assert!(!room_for(&bag, Stack::one(Kind::Battery)), "the pocket has rounds in it");
        bag.remove(Kind::Rounds, START_ROUNDS);
        assert!(room_for(&bag, Stack::one(Kind::Battery)));
        // Rounds top up the pocket's stack before taking a new place.
        let mut bag = Bag::with_rounds();
        bag.add(Stack::new(Kind::Rounds, 6));
        assert_eq!(bag.pockets.items[0].stack.count, 30);
        assert!(bag.pack.items.is_empty());
    }
}
