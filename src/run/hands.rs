//! What's in hand, from the run's side: 1, 2 and 3 (or the wheel) to
//! switch between what's carried in the slots, bare fists when there's
//! nothing, and the hands kept in step with the bag: a gun's rounds go
//! back into it, a weapon thrown out of its slot is let go of, one picked
//! up into empty hands is taken up.

use lntrn_ui::{Key, Ui};

use super::Run;
use crate::combat::Combat;
use crate::loot::bag::Slot;
use crate::weapon::Weapon;

/// How far the wheel turns for one step through the slots, pixels.
const WHEEL_STEP: f64 = 30.0;

impl Run {
    /// The first slot with something in it, the primary first.
    pub(super) fn first_armed(&self) -> Option<Slot> {
        Slot::ALL.into_iter().find(|&s| self.bag.slot(s).is_some())
    }

    /// Take up what's in `slot` (none, or nothing there: bare fists).
    pub(super) fn take_up(&self, combat: &mut Combat, slot: Option<Slot>) {
        let found = slot.and_then(|s| self.bag.slot(s).and_then(|stack| stack.kind.weapon().map(|w| (s, w, stack.loaded))));
        match found {
            Some((s, weapon, loaded)) => combat.take_up(Some(s), weapon, loaded),
            None => combat.take_up(None, Weapon::Fists, 0),
        }
    }

    /// A frame of the hands before they're used: kept in step with the
    /// bag, and switched if asked (and `free`: not patching up or in the
    /// bag).
    pub(super) fn switch_hands(&mut self, ui: &mut Ui, combat: &mut Combat, free: bool) {
        let hands = &combat.hands;
        // What's held was thrown out of its slot: it's gone from the hands.
        let lost = hands.held.is_some_and(|s| self.bag.slot(s).and_then(|st| st.kind.weapon()) != Some(hands.weapon));
        if lost {
            self.take_up(combat, self.first_armed());
            return;
        }
        // Put away: up with what's next (if it's still there).
        if let Some(next) = combat.hands.stowed() {
            let next = match next {
                Some(s) if self.bag.slot(s).is_none() => self.first_armed(),
                next => next,
            };
            self.take_up(combat, next);
            return;
        }
        let mut want = None;
        for slot in Slot::ALL {
            if pressed(ui, slot.key()) && self.bag.slot(slot).is_some() {
                want = Some(slot);
            }
        }
        self.wheel += ui.state.wheel.y;
        if self.wheel.abs() >= WHEEL_STEP {
            // Up goes back through the slots, down on.
            want = self.step(combat, if self.wheel > 0.0 { -1 } else { 1 }).or(want);
            self.wheel = 0.0;
        }
        // Bare fists and something picked up: it's taken up.
        if want.is_none() && combat.hands.held.is_none() && combat.hands.switching().is_none() && !combat.hands.busy() {
            want = self.first_armed();
        }
        if free && let Some(slot) = want {
            combat.hands.put_away(Some(slot));
        }
    }

    /// The armed slot `by` on from what's in hand (or on its way up).
    fn step(&self, combat: &Combat, by: i32) -> Option<Slot> {
        let armed: Vec<Slot> = Slot::ALL.into_iter().filter(|&s| self.bag.slot(s).is_some()).collect();
        if armed.is_empty() {
            return None;
        }
        let now = combat.hands.switching().unwrap_or(combat.hands.held);
        // From bare fists, on is the first and back the last.
        let at = now.and_then(|s| armed.iter().position(|&a| a == s)).map_or(if by > 0 { -1 } else { 0 }, |i| i as i32);
        let n = armed.len() as i32;
        Some(armed[(at + by).rem_euclid(n) as usize])
    }

    /// The gun's rounds, kept in it in the bag (so they go where it goes).
    pub(super) fn keep_rounds(&mut self, combat: &Combat) {
        if let Some(slot) = combat.hands.held
            && let Some(stack) = self.bag.slot_mut(slot)
        {
            stack.loaded = combat.hands.mag;
        }
    }
}

fn pressed(ui: &mut Ui, key: char) -> bool {
    ui.state.take_key(|k| !k.repeat && matches!(k.key, Key::Char(c) if c == key)).is_some()
}
