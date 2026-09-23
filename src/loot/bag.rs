//! What the player carries: a backpack grid, two-by-two pockets, and a
//! weapon to hand in each of three slots; one store of things as far as
//! the rest of the game cares (rounds to load, kits to use, what it's all
//! worth).

use super::grid::Grid;
use super::{Kind, Stack};

pub const PACK: (u8, u8) = (6, 4);
pub const POCKETS: (u8, u8) = (2, 2);
/// A couple of magazines' worth, for trying things with (tests).
#[cfg(test)]
pub const START_ROUNDS: u32 = 24;

/// Where a weapon is carried to hand.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Slot {
    /// A long gun.
    Primary,
    Sidearm,
    Melee,
}

impl Slot {
    pub const ALL: [Slot; 3] = [Slot::Primary, Slot::Sidearm, Slot::Melee];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn name(self) -> &'static str {
        match self {
            Slot::Primary => "PRIMARY",
            Slot::Sidearm => "SIDEARM",
            Slot::Melee => "MELEE",
        }
    }

    /// The key that takes it in hand.
    pub fn key(self) -> char {
        match self {
            Slot::Primary => '1',
            Slot::Sidearm => '2',
            Slot::Melee => '3',
        }
    }

    /// Its name in a save file.
    pub fn save_key(self) -> &'static str {
        match self {
            Slot::Primary => "primary",
            Slot::Sidearm => "sidearm",
            Slot::Melee => "melee",
        }
    }

    /// The slot `kind` goes in, if it's a weapon.
    pub fn of(kind: Kind) -> Option<Slot> {
        kind.weapon().and_then(|w| w.spec().slot)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bag {
    pub pack: Grid,
    pub pockets: Grid,
    /// What's in each slot, by [`Slot::index`].
    pub slots: [Option<Stack>; 3],
}

impl Default for Bag {
    fn default() -> Self {
        Self::empty()
    }
}

impl Bag {
    /// Nothing in it at all.
    pub fn empty() -> Self {
        Self::sized(PACK, POCKETS)
    }

    /// Nothing in it, with a pack and pockets of these sizes.
    pub fn sized(pack: (u8, u8), pockets: (u8, u8)) -> Self {
        Self { pack: Grid::new(pack.0, pack.1), pockets: Grid::new(pockets.0, pockets.1), slots: [None; 3] }
    }

    pub fn slot(&self, slot: Slot) -> Option<Stack> {
        self.slots[slot.index()]
    }

    pub fn slot_mut(&mut self, slot: Slot) -> &mut Option<Stack> {
        &mut self.slots[slot.index()]
    }

    /// A couple of magazines' worth in a pocket (for tests).
    #[cfg(test)]
    pub fn with_rounds() -> Self {
        let mut bag = Self::empty();
        bag.pockets.place(Stack::new(Kind::Rounds, START_ROUNDS));
        bag
    }

    /// Put `stack` away: a weapon into its slot if that's free; else
    /// topping up what's already carried, then in the pack, then the
    /// pockets. What didn't fit.
    pub fn add(&mut self, stack: Stack) -> Stack {
        if let Some(slot) = Slot::of(stack.kind)
            && stack.count > 0
            && self.slot(slot).is_none()
        {
            *self.slot_mut(slot) = Some(stack.with_count(1));
            if stack.count == 1 {
                return stack.with_count(0);
            }
            return self.add(stack.with_count(stack.count - 1));
        }
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
        self.pack.value() + self.pockets.value() + self.slots.iter().flatten().map(|s| s.value()).sum::<u32>()
    }

    /// Everything carried: the slots, the pack, the pockets.
    pub fn everything(&self) -> impl Iterator<Item = Stack> + '_ {
        self.slots.iter().flatten().copied().chain(self.pack.items.iter().chain(&self.pockets.items).map(|i| i.stack))
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

    #[test]
    fn a_weapon_goes_to_its_slot_then_the_pack() {
        let mut bag = Bag::empty();
        assert_eq!(bag.add(Stack::gun(Kind::Pistol, 7)).count, 0);
        assert_eq!(bag.slot(Slot::Sidearm), Some(Stack::gun(Kind::Pistol, 7)), "in hand, its rounds in it");
        assert_eq!(bag.add(Stack::gun(Kind::Pistol, 3)).count, 0);
        assert_eq!(bag.pack.items[0].stack, Stack::gun(Kind::Pistol, 3), "the second in the pack, rounds and all");
        assert_eq!(bag.value(), 2 * Kind::Pistol.def().value);
        assert_eq!(bag.everything().count(), 2);
    }
}
