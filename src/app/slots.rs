//! The save slots as the app has them: who's playing, each slot's card,
//! and doing what the slots screen asks (`slots_ui.rs`).

use lntrn_ui::Ui;

use super::DeadSignal;
use crate::profile::Profile;
use crate::profile::save::SLOTS;
use crate::slots_ui::{SlotEvent, Summary};

impl DeadSignal {
    /// Who's playing, for the title: "SLOT 2 · ALVA · LEVEL 7".
    pub(super) fn playing(&self) -> String {
        let p = &self.profile;
        let level = crate::profile::xp::level(p.xp).0;
        if p.name.is_empty() {
            format!("SLOT {} · LEVEL {level}", self.saves.slot)
        } else {
            format!("SLOT {} · {} · LEVEL {level}", self.saves.slot, p.name.to_uppercase())
        }
    }

    /// Every slot's card: its player (the one playing as they are now), or
    /// none.
    pub(super) fn slot_cards(&self) -> Vec<Option<Summary>> {
        (1..=SLOTS)
            .map(|n| {
                if !self.saves.used(n) {
                    None
                } else if n == self.saves.slot {
                    Some(Summary::of(&self.profile))
                } else {
                    self.saves.load(n).map(|p| Summary::of(&p))
                }
            })
            .collect()
    }

    /// A frame of the slots screen, and what it asked done. Whether it was
    /// closed.
    pub(super) fn slots_frame(&mut self, ui: &mut Ui, active: bool) -> bool {
        let current = self.saves.slot;
        let Some(screen) = &mut self.slots else { return true };
        let Some(event) = screen.frame(ui, current, active) else { return false };
        match event {
            SlotEvent::Closed => {
                self.slots = None;
                return true;
            }
            SlotEvent::Chosen(n) => {
                // The one playing is kept as they are, then the other's
                // taken up (someone new, in an empty slot: written at once).
                self.saves.store(&self.profile);
                self.saves.choose(n);
                self.profile = self.saves.profile();
                self.saves.store(&self.profile);
            }
            SlotEvent::Renamed(n, name) => {
                if n == self.saves.slot {
                    self.profile.name = name;
                    self.saves.store(&self.profile);
                } else if let Some(mut p) = self.saves.load(n) {
                    p.name = name;
                    self.saves.store_in(n, &p);
                }
            }
            SlotEvent::Deleted(n) => {
                self.saves.delete(n);
                if n == self.saves.slot {
                    // Nobody left in it: someone new, once they're played.
                    self.profile = Profile::new_player();
                }
            }
        }
        let cards = self.slot_cards();
        if let Some(screen) = &mut self.slots {
            screen.refresh(cards);
        }
        false
    }
}
