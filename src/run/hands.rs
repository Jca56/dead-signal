//! What's in hand, from the run's side: the slots' keys (or the wheel, or
//! a pad's Y) to switch between what's carried in the slots, bare fists
//! when there's nothing, and the hands kept in step with the bag: a gun's
//! rounds go back into it (and rounds loaded into it in the bag come into
//! the hands), a weapon thrown out of its slot is let go of, one picked up
//! into empty hands is taken up.

use lntrn_ui::Ui;

use super::seat::Seat;
use crate::combat::Combat;
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
        let found = slot.and_then(|s| self.bag.slot(s).and_then(|stack| stack.kind.weapon().map(|w| (s, w, stack.loaded))));
        match found {
            Some((s, weapon, loaded)) => combat.take_up(self.n, Some(s), weapon, loaded),
            None => combat.take_up(self.n, None, Weapon::Fists, 0),
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
            self.take_up(combat, self.first_armed());
            return;
        }
        // Put away: up with what's next (if it's still there).
        if let Some(next) = combat.arms[self.n].hands.stowed() {
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
        // A pad's Y: on to the next.
        if self.input.next_weapon() {
            want = self.step(combat, 1).or(want);
        }
        // Bare fists and something picked up: it's taken up.
        if want.is_none() && combat.arms[self.n].hands.held.is_none() && combat.arms[self.n].hands.switching().is_none() && !combat.arms[self.n].hands.busy() {
            want = self.first_armed();
        }
        if free && let Some(slot) = want {
            combat.arms[self.n].hands.put_away(Some(slot));
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
            combat.arms[self.n].hands.mag = stack.loaded.min(combat.arms[self.n].hands.spec().mag);
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
