//! What the player carries: a weapon to hand in each of three slots, what's
//! worn (`gear.rs`), and the grids that come of it: the backpack worn's,
//! the pockets (bigger in cargo pants), a chest rig's, a bandolier's (rounds
//! only). One store of things as far as the rest of the game cares (rounds
//! to load, kits to use, what it's all worth).

use super::gear::{Gear, Wear};
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
    /// What's worn, by [`Wear::index`].
    pub worn: [Option<Stack>; 5],
    /// A chest rig's grid, and a bandolier's (none worn: nothing across).
    pub rig: Grid,
    pub belt: Grid,
}

/// What the player's perks make of what's worn: more backpack (on any pack
/// worn), and pockets this big before cargo pants.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fit {
    pub pack_bonus: (u8, u8),
    pub pockets: (u8, u8),
}

impl Default for Fit {
    fn default() -> Self {
        Self { pack_bonus: (0, 0), pockets: POCKETS }
    }
}

/// Why gear won't go on (or come off).
pub const NO_ROOM: &str = "NO ROOM FOR WHAT'S IN IT";

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
        Self { pack: Grid::new(pack.0, pack.1), pockets: Grid::new(pockets.0, pockets.1), slots: [None; 3], worn: [None; 5], rig: Grid::new(0, 0), belt: Grid::new(0, 0) }
    }

    /// What's worn on `wear`.
    pub fn worn(&self, wear: Wear) -> Option<Stack> {
        self.worn[wear.index()]
    }

    pub fn worn_mut(&mut self, wear: Wear) -> &mut Option<Stack> {
        &mut self.worn[wear.index()]
    }

    /// The gear worn, what it is.
    fn gear(&self) -> impl Iterator<Item = (Stack, Gear)> + '_ {
        self.worn.iter().flatten().filter_map(|s| s.kind.gear().map(|g| (*s, g)))
    }

    /// How big each grid is, with this worn and `fit`: the backpack worn's
    /// (and the bonus; none worn, no backpack), the pockets (and cargo
    /// pants'), a rig's, a bandolier's.
    fn sizes(&self, fit: Fit) -> [(u8, u8); 4] {
        let grid = |wear: Wear| self.worn(wear).and_then(|s| s.kind.gear()).and_then(|g| g.grid);
        let pack = grid(Wear::Back).map_or((0, 0), |(w, h)| (w + fit.pack_bonus.0, h + fit.pack_bonus.1));
        let more = self.worn(Wear::Legs).and_then(|s| s.kind.gear()).map_or((0, 0), |g| g.pockets);
        [pack, (fit.pockets.0 + more.0, fit.pockets.1 + more.1), grid(Wear::Chest).unwrap_or((0, 0)), grid(Wear::Belt).unwrap_or((0, 0))]
    }

    /// Every grid made the size what's worn (and `fit`) says: everything
    /// laid where it was if it still fits, else anywhere it goes. What
    /// found no place at all.
    pub fn refit(&mut self, fit: Fit) -> Vec<Stack> {
        let sizes = self.sizes(fit);
        let mut homeless = Vec::new();
        for (grid, (w, h)) in [&mut self.pack, &mut self.pockets, &mut self.rig, &mut self.belt].into_iter().zip(sizes) {
            if (grid.w, grid.h) == (w, h) {
                continue;
            }
            let old = std::mem::replace(grid, Grid::new(w, h));
            for i in old.items {
                if !grid.put(i.stack, i32::from(i.x), i32::from(i.y), i.turned) {
                    homeless.push(i.stack);
                }
            }
        }
        // What had to move goes wherever it'll go: the same kind topped up,
        // then any room, rounds in the belt first.
        homeless.into_iter().filter_map(|s| Some(self.stow(s)).filter(|rest| rest.count > 0)).collect()
    }

    /// Put `stack` on (it's worn where it goes): everything carried laid
    /// out again to suit. What was worn there before, taken off; or why it
    /// won't go on (and then nothing's changed).
    pub fn wear(&mut self, stack: Stack, fit: Fit) -> Result<Option<Stack>, &'static str> {
        let Some(g) = stack.kind.gear() else { return Err("NOT SOMETHING TO WEAR") };
        let before = self.clone();
        let old = self.worn_mut(g.wear).replace(stack.with_count(1));
        if !self.refit(fit).is_empty() {
            *self = before;
            return Err(NO_ROOM);
        }
        Ok(old)
    }

    /// Take off what's worn on `wear`: it, or why it won't come off (what's
    /// in it has nowhere else to go).
    pub fn take_off(&mut self, wear: Wear, fit: Fit) -> Result<Stack, &'static str> {
        let before = self.clone();
        let Some(old) = self.worn_mut(wear).take() else { return Err("NOTHING WORN THERE") };
        if !self.refit(fit).is_empty() {
            *self = before;
            return Err(NO_ROOM);
        }
        Ok(old)
    }

    /// Armor points left, and the most there could be.
    pub fn armor(&self) -> (u32, u32) {
        self.gear().filter(|(_, g)| g.armor > 0).fold((0, 0), |(left, most), (s, g)| (left + s.loaded.min(g.armor), most + g.armor))
    }

    /// Take up to `points` off the armor worn (the helmet last). How many
    /// it soaked.
    pub fn soak(&mut self, points: u32) -> u32 {
        let mut left = points;
        for wear in [Wear::Chest, Wear::Head] {
            if let Some(s) = self.worn_mut(wear).as_mut() {
                let took = s.loaded.min(left);
                s.loaded -= took;
                left -= took;
            }
        }
        points - left
    }

    /// Put `points` back on the armor worn (the chest first). How many
    /// went on.
    pub fn mend(&mut self, points: u32) -> u32 {
        let mut left = points;
        for wear in [Wear::Chest, Wear::Head] {
            if let Some(s) = self.worn_mut(wear).as_mut()
                && let Some(most) = s.armor()
            {
                let put = (most - s.loaded.min(most)).min(left);
                s.loaded += put;
                left -= put;
            }
        }
        points - left
    }

    /// How heavy all that's worn is, together.
    pub fn weight(&self) -> u32 {
        self.gear().map(|(_, g)| u32::from(g.weight)).sum()
    }

    /// Room for `stack` anywhere carried: rounds into the belt first, the
    /// same kind topped up, then a place of its own in the pack, the rig,
    /// the pockets. What didn't fit.
    fn stow(&mut self, stack: Stack) -> Stack {
        let stack = if stack.kind.is_ammo() {
            let rest = self.belt.top_up(stack);
            self.belt.place(rest)
        } else {
            stack
        };
        let stack = self.pockets.top_up(stack);
        let stack = self.rig.top_up(stack);
        let stack = self.pack.top_up(stack);
        let stack = self.pack.place(stack);
        let stack = self.rig.place(stack);
        self.pockets.place(stack)
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
        self.stow(stack)
    }

    pub fn count(&self, kind: Kind) -> u32 {
        self.pack.count(kind) + self.pockets.count(kind) + self.rig.count(kind) + self.belt.count(kind)
    }

    /// Take up to `n` of `kind`: the pockets, the belt and the rig first
    /// (to hand), the pack last. How many were taken.
    pub fn remove(&mut self, kind: Kind, n: u32) -> u32 {
        let mut taken = 0;
        for grid in [&mut self.pockets, &mut self.belt, &mut self.rig, &mut self.pack] {
            taken += grid.remove(kind, n - taken);
        }
        taken
    }

    pub fn value(&self) -> u32 {
        self.everything().map(|s| s.value()).sum()
    }

    /// Everything carried: the slots, what's worn, and every grid.
    pub fn everything(&self) -> impl Iterator<Item = Stack> + '_ {
        let grids = self.pack.items.iter().chain(&self.pockets.items).chain(&self.rig.items).chain(&self.belt.items);
        self.slots.iter().flatten().copied().chain(self.worn.iter().flatten().copied()).chain(grids.map(|i| i.stack))
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

    #[test]
    fn a_backpack_worn_is_the_backpack_and_what_is_in_it_moves_with_the_swap() {
        let fit = Fit::default();
        let mut bag = Bag::sized((0, 0), (0, 0));
        assert!(bag.refit(fit).is_empty());
        assert_eq!((bag.pack.w, bag.pockets.w), (0, 2), "no pack worn, no pack");
        assert_eq!(bag.wear(Stack::one(Kind::Rucksack), fit), Ok(None));
        assert_eq!((bag.pack.w, bag.pack.h), (6, 4));
        for _ in 0..5 {
            bag.pack.place(Stack::one(Kind::Battery));
        }
        // Onto a daypack: five batteries won't go in a 4×3.
        let before = bag.clone();
        assert_eq!(bag.wear(Stack::one(Kind::Daypack), fit), Err(NO_ROOM));
        assert_eq!(bag, before, "nothing changed");
        // Onto a hiking pack: it all comes with it, the rucksack handed back.
        assert_eq!(bag.wear(Stack::one(Kind::HikingPack), fit), Ok(Some(Stack::one(Kind::Rucksack))));
        assert_eq!((bag.pack.w, bag.pack.h, bag.pack.count(Kind::Battery)), (7, 5, 5));
        // It won't come off with things in it; empty, it will.
        assert_eq!(bag.take_off(Wear::Back, fit), Err(NO_ROOM));
        bag.pack.items.clear();
        assert_eq!(bag.take_off(Wear::Back, fit), Ok(Stack::one(Kind::HikingPack)));
        // Pack Mule on any pack.
        let mule = Fit { pack_bonus: (2, 1), ..fit };
        bag.wear(Stack::one(Kind::Daypack), mule).unwrap();
        assert_eq!((bag.pack.w, bag.pack.h), (6, 4));
    }

    #[test]
    fn rounds_go_on_the_belt_and_pants_make_the_pockets_bigger() {
        let fit = Fit::default();
        let mut bag = Bag::sized((0, 0), (0, 0));
        bag.wear(Stack::one(Kind::Bandolier), fit).unwrap();
        bag.wear(Stack::one(Kind::CargoPants), fit).unwrap();
        bag.wear(Stack::one(Kind::ChestRig), fit).unwrap();
        assert_eq!((bag.belt.w, bag.belt.h, bag.pockets.w, bag.pockets.h, bag.rig.w, bag.rig.h), (4, 1, 3, 3, 4, 2));
        assert_eq!(bag.add(Stack::new(Kind::Shells, 20)).count, 0);
        assert_eq!(bag.belt.count(Kind::Shells), 20, "rounds to the belt");
        assert_eq!(bag.add(Stack::one(Kind::Watch)).count, 0);
        assert_eq!(bag.belt.count(Kind::Watch), 0, "nothing else on it");
        assert_eq!(bag.count(Kind::Shells), 20);
        assert_eq!(bag.remove(Kind::Shells, 5), 5);
        assert_eq!(bag.everything().filter(|s| s.kind.gear().is_some()).count(), 3, "what's worn is carried too");
    }

    #[test]
    fn armor_soaks_the_chest_first_and_mends_it_first() {
        let fit = Fit::default();
        let mut bag = Bag::sized((0, 0), (0, 0));
        bag.wear(Stack::fresh(Kind::LightVest, 1), fit).unwrap();
        bag.wear(Stack::fresh(Kind::BikeHelmet, 1), fit).unwrap();
        assert_eq!(bag.armor(), (55, 55));
        assert_eq!(bag.soak(50), 50);
        assert_eq!(bag.worn(Wear::Chest).unwrap().loaded, 0, "the vest first");
        assert_eq!(bag.armor(), (5, 55));
        assert_eq!(bag.soak(20), 5, "no more than it has");
        assert_eq!(bag.mend(super::super::gear::PLATE), 40);
        assert_eq!(bag.worn(Wear::Chest).unwrap().loaded, 40, "the vest first");
        assert_eq!(bag.weight(), 1);
    }
}
