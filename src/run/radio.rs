//! The handheld radio, from the run's side: Q (a pad's View, held) pulls
//! it out and puts it away; out, the hands are its (what was held is put
//! away, to come back after) and E keys it. Anything else the hands are
//! wanted for puts it away: a shot or a blow, another weapon, a kit, a
//! throw, the bag, someone to pick up, going down.

use lntrn_ui::Ui;

use super::seat::Seat;
use crate::combat::Combat;
use crate::radio::{Cue, Shown};
use crate::settings::keys::Action;
use crate::sound::Sfx;

impl Seat {
    /// Whether the radio's out (or on its way out, or away).
    pub fn radio_out(&self) -> bool {
        self.radio.is_some_and(|r| r.out())
    }

    /// Whether it's up in the hand, to be keyed.
    pub(super) fn radio_ready(&self) -> bool {
        self.radio.is_some_and(|r| r.ready()) && self.reviving.is_none()
    }

    /// The handset as it's drawn, while it's in view.
    pub fn radio_shown(&self) -> Option<Shown> {
        self.radio.and_then(|r| r.shown())
    }

    /// Put the radio away, if it's out.
    pub(super) fn radio_away(&mut self, combat: &Combat) {
        if let Some(radio) = &mut self.radio
            && radio.put_away()
        {
            combat.play(Sfx::Click, 0.45);
        }
    }

    /// A frame of the radio: pulled out or put away if asked (and the
    /// hands `free` for it; `cut` short by a shot or a blow), keyed, and
    /// what's heard of it.
    pub(super) fn radioing(&mut self, ui: &mut Ui, combat: &mut Combat, free: bool, cut: bool, dt: f64) {
        let Some(mut radio) = self.radio else { return };
        if radio.out() {
            if !free || cut || self.input.pressed(ui, Action::Radio) {
                self.radio_away(combat);
            } else if self.radio_ready() && self.input.pressed(ui, Action::Interact) {
                radio.key();
                self.radio = Some(radio);
            }
        } else if free && self.input.pressed(ui, Action::Radio) {
            radio.pull();
            self.radio = Some(radio);
        }
        let Some(radio) = &mut self.radio else { return };
        // Wanted or in hand, the hands are its: what's held goes away.
        let hands = &mut combat.arms[self.n].hands;
        if radio.holds() {
            hands.stow();
        }
        for cue in radio.update(hands.stowed().is_some(), dt) {
            let (sfx, gain) = match cue {
                Cue::On => (Sfx::RadioOn, 0.8),
                Cue::Talk => (Sfx::RadioTalk, 0.8),
                Cue::Over => (Sfx::RadioOver, 0.7),
            };
            combat.play(sfx, gain);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loot::bag::{Bag, Slot};
    use crate::loot::{Kind, Stack};
    use crate::radio::Radio;
    use crate::weapon::{Trigger, Weapon};
    use lntrn_ui::Key;
    use lntrn_ui::testing::Harness;

    const DT: f64 = 1.0 / 60.0;

    /// A seat with a pistol and a knife and a radio, the pistol up in hand.
    fn seat(combat: &mut Combat) -> Seat {
        let mut seat = Seat::default();
        seat.bag = Bag::empty();
        *seat.bag.slot_mut(Slot::Sidearm) = Some(Stack::gun(Kind::Pistol, 12));
        *seat.bag.slot_mut(Slot::Melee) = Some(Stack::one(Kind::Knife));
        seat.radio = Some(Radio::default());
        seat.take_up(combat, Some(Slot::Sidearm));
        seat
    }

    /// `frames` of the hands and the radio, with `key` pressed on the first
    /// (the keys theirs) and the trigger pulled on it if `fire`.
    fn run(h: &mut Harness, seat: &mut Seat, combat: &mut Combat, key: Option<Key>, fire: bool, frames: u32) {
        if let Some(key) = key {
            h.key(key);
        }
        for k in 0..frames {
            h.frame(|ui| {
                seat.input.update(ui, Some(Default::default()), true, Default::default(), DT);
                seat.radioing(ui, combat, true, fire && k == 0, DT);
                seat.switch_hands(ui, combat, true);
                combat.arms[0].hands.update(Trigger::default(), DT);
            });
        }
    }

    #[test]
    fn pulled_out_the_gun_goes_away_and_comes_back_when_it_s_put_away() {
        let mut h = Harness::new(800.0, 600.0);
        let mut combat = Combat::new();
        let mut seat = seat(&mut combat);
        run(&mut h, &mut seat, &mut combat, None, false, 60);
        assert!(!seat.radio_out() && seat.radio_shown().is_none());
        // Q: the pistol's put away, then the radio comes up; the pistol
        // stays away while it's out.
        run(&mut h, &mut seat, &mut combat, Some(Key::Char('q')), false, 2);
        assert!(seat.radio_out() && seat.radio_shown().is_none(), "the gun's not away yet");
        run(&mut h, &mut seat, &mut combat, None, false, 90);
        assert!(seat.radio_ready() && combat.arms[0].hands.stowed() == Some(Some(Slot::Sidearm)));
        assert_eq!(seat.radio_shown().map(|s| (s.clip, s.stowed)), Some(("Idle", 0.0)));
        // E keys it; and it's ready again after.
        run(&mut h, &mut seat, &mut combat, Some(Key::Char('e')), false, 2);
        assert_eq!(seat.radio_shown().map(|s| s.clip), Some("Key"));
        run(&mut h, &mut seat, &mut combat, None, false, 90);
        assert!(seat.radio_ready());
        // Q again: down it goes, and the pistol's back in hand.
        run(&mut h, &mut seat, &mut combat, Some(Key::Char('q')), false, 90);
        let hands = &combat.arms[0].hands;
        assert!(!seat.radio_out() && hands.held == Some(Slot::Sidearm) && hands.weapon == Weapon::Pistol && !hands.busy());
        assert_eq!(hands.mag, 12);
    }

    #[test]
    fn a_shot_or_another_weapon_puts_it_away() {
        let mut h = Harness::new(800.0, 600.0);
        let mut combat = Combat::new();
        let mut seat = seat(&mut combat);
        // The trigger: the radio down, the pistol back up.
        run(&mut h, &mut seat, &mut combat, Some(Key::Char('q')), false, 90);
        assert!(seat.radio_ready());
        run(&mut h, &mut seat, &mut combat, None, true, 90);
        assert!(!seat.radio_out() && combat.arms[0].hands.held == Some(Slot::Sidearm) && !combat.arms[0].hands.busy());
        // The blade's key: the radio down, and it's the blade that comes up.
        run(&mut h, &mut seat, &mut combat, Some(Key::Char('q')), false, 90);
        run(&mut h, &mut seat, &mut combat, Some(Key::Char('3')), false, 90);
        assert!(!seat.radio_out() && combat.arms[0].hands.held == Some(Slot::Melee) && combat.arms[0].hands.weapon == Weapon::Knife);
        // The hands wanted for something else (not `free`): away at once.
        run(&mut h, &mut seat, &mut combat, Some(Key::Char('q')), false, 90);
        h.frame(|ui| seat.radioing(ui, &mut combat, false, false, DT));
        assert!(seat.radio_shown().is_some_and(|s| s.stowed >= 0.0) && !seat.radio_ready());
        run(&mut h, &mut seat, &mut combat, None, false, 90);
        assert!(!seat.radio_out() && combat.arms[0].hands.held == Some(Slot::Melee));
    }

    #[test]
    fn no_radio_no_radio() {
        let mut h = Harness::new(800.0, 600.0);
        let mut combat = Combat::new();
        let mut seat = seat(&mut combat);
        seat.radio = None;
        run(&mut h, &mut seat, &mut combat, Some(Key::Char('q')), false, 60);
        assert!(!seat.radio_out() && combat.arms[0].hands.held == Some(Slot::Sidearm) && !combat.arms[0].hands.busy());
    }
}
