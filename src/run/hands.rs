//! What's in hand, from the run's side: the slots' keys (or the wheel; or
//! a pad's Y, tapped for the other gun and held for the blade) to switch
//! between what's carried in the slots, bare fists
//! when there's nothing, and the hands kept in step with the bag: a gun's
//! rounds go back into it (and rounds loaded into it in the bag come into
//! the hands), a weapon thrown out of its slot is let go of, one picked up
//! into empty hands is taken up. With the radio out (`radio.rs`) what's
//! held waits, put away, till the radio is.

use lntrn_ui::Ui;

use super::seat::Seat;
use crate::combat::Combat;
use crate::input::Swap;
use crate::loot::bag::Slot;
use crate::settings::keys::Action;
use crate::weapon::Weapon;

/// How far the wheel turns for one step through the slots, pixels.
const WHEEL_STEP: f64 = 30.0;

impl Seat {
    /// The first slot with something in it, the primary first.
    pub(super) fn first_armed(&self) -> Option<Slot> {
        Slot::ALL.into_iter().find(|&s| self.bag.slot(s).is_some())
    }

    /// Take up what's in `slot` (none, or nothing there: bare fists).
    pub(super) fn take_up(&self, combat: &mut Combat, slot: Option<Slot>) {
        let found = slot.and_then(|s| self.bag.slot(s).and_then(|stack| stack.kind.weapon().map(|w| (s, w, stack))));
        match found {
            Some((s, weapon, stack)) => {
                // (Amplified as it is: it holds what that makes it hold.)
                combat.arms[self.n].hands.tier = stack.tier;
                combat.take_up(self.n, Some(s), weapon, stack.loaded);
            }
            None => {
                combat.arms[self.n].hands.tier = 0;
                combat.take_up(self.n, None, Weapon::Fists, 0);
            }
        }
    }

    /// A frame of the hands before they're used: kept in step with the
    /// bag, and switched if asked (and `free`: not patching up or in the
    /// bag).
    pub(super) fn switch_hands(&mut self, ui: &mut Ui, combat: &mut Combat, free: bool) {
        let hands = &combat.arms[self.n].hands;
        // What's held was thrown out of its slot: it's gone from the hands.
        let lost = hands.held.is_some_and(|s| self.bag.slot(s).and_then(|st| st.kind.weapon()) != Some(hands.weapon));
        if lost {
            // (The radio out, it waits its turn.)
            if self.radio_out() {
                combat.arms[self.n].hands.put_away(self.first_armed());
            } else {
                self.take_up(combat, self.first_armed());
            }
            return;
        }
        // The gun last in hand: what a pad's Y goes back to from the blade.
        if let Some(slot) = hands.held.filter(|s| *s != Slot::Melee) {
            self.gun = Some(slot);
        }
        // A pad's Y held on: the blade, whatever it was being put away for.
        let swap = self.input.swap();
        if swap == Some(Swap::Blade) && free && self.bag.slot(Slot::Melee).is_some() {
            combat.arms[self.n].hands.put_away(Some(Slot::Melee));
            self.radio_away(combat);
        }
        // Put away: up with what's next (if it's still there). (Not while
        // Y's down still: it may yet be the blade that's wanted. Nor with
        // the radio out: the hands are its, till a weapon's asked for.)
        if let Some(next) = combat.arms[self.n].hands.stowed()
            && !self.radio_out()
        {
            if self.input.swapping() {
                return;
            }
            let next = match next {
                Some(s) if self.bag.slot(s).is_none() => self.first_armed(),
                next => next,
            };
            self.take_up(combat, next);
            return;
        }
        let mut want = None;
        for slot in Slot::ALL {
            if self.input.pressed(ui, Action::slot(slot)) && self.bag.slot(slot).is_some() {
                want = Some(slot);
            }
        }
        self.wheel += self.input.wheel(ui);
        if self.wheel.abs() >= WHEEL_STEP {
            // Up goes back through the slots, down on.
            want = self.step(combat, if self.wheel > 0.0 { -1 } else { 1 }).or(want);
            self.wheel = 0.0;
        }
        // A pad's Y, as it goes down: the other gun.
        if swap == Some(Swap::Guns) {
            want = self.other_gun(combat).or(want);
        }
        // Bare fists and something picked up: it's taken up.
        if want.is_none() && combat.arms[self.n].hands.held.is_none() && combat.arms[self.n].hands.switching().is_none() && !combat.arms[self.n].hands.busy() {
            want = self.first_armed();
        }
        if free && let Some(slot) = want {
            combat.arms[self.n].hands.put_away(Some(slot));
            self.radio_away(combat);
        }
    }

    /// The gun that isn't in hand (or on its way up): the sidearm for the
    /// primary and the primary for the sidearm; from the blade (or bare
    /// fists), the gun last held. None, if there's no other.
    fn other_gun(&self, combat: &Combat) -> Option<Slot> {
        let has = |s: &Slot| self.bag.slot(*s).is_some();
        let hands = &combat.arms[self.n].hands;
        match hands.switching().unwrap_or(hands.held) {
            Some(Slot::Primary) => Some(Slot::Sidearm).filter(has),
            Some(Slot::Sidearm) => Some(Slot::Primary).filter(has),
            _ => self.gun.filter(has).or_else(|| [Slot::Primary, Slot::Sidearm].into_iter().find(has)),
        }
    }

    /// The armed slot `by` on from what's in hand (or on its way up).
    fn step(&self, combat: &Combat, by: i32) -> Option<Slot> {
        let armed: Vec<Slot> = Slot::ALL.into_iter().filter(|&s| self.bag.slot(s).is_some()).collect();
        if armed.is_empty() {
            return None;
        }
        let now = combat.arms[self.n].hands.switching().unwrap_or(combat.arms[self.n].hands.held);
        // From bare fists, on is the first and back the last.
        let at = now.and_then(|s| armed.iter().position(|&a| a == s)).map_or(if by > 0 { -1 } else { 0 }, |i| i as i32);
        let n = armed.len() as i32;
        Some(armed[(at + by).rem_euclid(n) as usize])
    }

    /// The gun in hand has the rounds its slot says (rounds dropped onto
    /// it in the bag load it).
    pub(super) fn pull_rounds(&self, combat: &mut Combat) {
        if let Some(stack) = combat.arms[self.n].hands.held.and_then(|slot| self.bag.slot(slot)) {
            let hands = &mut combat.arms[self.n].hands;
            hands.tier = stack.tier;
            hands.mag = stack.loaded.min(hands.capacity());
        }
    }

    /// The gun's rounds, kept in it in the bag (so they go where it goes).
    pub(super) fn keep_rounds(&mut self, combat: &Combat) {
        if let Some(slot) = combat.arms[self.n].hands.held
            && let Some(stack) = self.bag.slot_mut(slot)
        {
            stack.loaded = combat.arms[self.n].hands.mag;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loot::bag::Bag;
    use crate::loot::{Kind, Stack};

    #[test]
    fn a_pad_s_y_goes_between_the_guns_and_back_from_the_blade_to_the_gun_last_held() {
        let mut combat = Combat::new();
        let mut seat = Seat::default();
        seat.bag = Bag::empty();
        *seat.bag.slot_mut(Slot::Sidearm) = Some(Stack::gun(Kind::Pistol, 12));
        *seat.bag.slot_mut(Slot::Melee) = Some(Stack::one(Kind::Knife));
        // One gun: there's no other.
        combat.take_up(0, Some(Slot::Sidearm), Weapon::Pistol, 12);
        assert_eq!(seat.other_gun(&combat), None);
        // Two: each is the other's.
        *seat.bag.slot_mut(Slot::Primary) = Some(Stack::gun(Kind::Shotgun, 5));
        assert_eq!(seat.other_gun(&combat), Some(Slot::Primary));
        combat.take_up(0, Some(Slot::Primary), Weapon::Shotgun, 5);
        assert_eq!(seat.other_gun(&combat), Some(Slot::Sidearm));
        // From the blade, back to the gun last in hand (the primary, if
        // none's been).
        combat.take_up(0, Some(Slot::Melee), Weapon::Knife, 0);
        assert_eq!(seat.other_gun(&combat), Some(Slot::Primary));
        seat.gun = Some(Slot::Sidearm);
        assert_eq!(seat.other_gun(&combat), Some(Slot::Sidearm));
        // (Gone from the bag since: the other, then.)
        *seat.bag.slot_mut(Slot::Sidearm) = None;
        assert_eq!(seat.other_gun(&combat), Some(Slot::Primary));
    }
}
