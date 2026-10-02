//! The handheld radio, from the run's side: Q (a pad's View, held) pulls
//! it out and puts it away; out, the hands are its (what was held is put
//! away, to come back after), the feet stand still, and the keys that
//! walk (a pad's d-pad) punch its code in. Anything else the hands are
//! wanted for puts it away: a shot or a blow, another weapon, a kit, a
//! throw, the bag, someone to pick up, going down.

use lntrn_math::Rect;
use lntrn_ui::Ui;

use super::seat::Seat;
use crate::combat::Combat;
use crate::radio::codes::{Arrow, Dialed};
use crate::radio::{Cue, Shown, card};
use crate::settings::keys::Action;
use crate::sound::Sfx;

impl Seat {
    /// Whether the radio's out (or on its way out, or away).
    pub fn radio_out(&self) -> bool {
        self.radio.is_some_and(|r| r.out())
    }

    /// Whether it's in hand (or wanted there): the feet stand still to
    /// dial.
    pub(super) fn radio_held(&self) -> bool {
        self.radio.is_some_and(|r| r.holds())
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
    /// hands `free` for it; `cut` short by a shot or a blow), its code
    /// punched in, and what's heard of it.
    pub(super) fn radioing(&mut self, ui: &mut Ui, combat: &mut Combat, free: bool, cut: bool, dt: f64) {
        let Some(out) = self.radio.map(|r| r.out()) else { return };
        if out {
            if !free || cut || self.input.pressed(ui, Action::Radio) {
                self.radio_away(combat);
            }
        } else if free && self.input.pressed(ui, Action::Radio) {
            self.radio.iter_mut().for_each(|r| r.pull());
            // (At once: the first arrow may come with the very next frame.)
            self.input.set_dialing(true);
        }
        let arrow = self.input.dial(ui);
        let Some(radio) = &mut self.radio else { return };
        if let Some((arrow, dialed)) = arrow.and_then(|a| radio.press(a).map(|d| (a, d))) {
            let tone = match arrow {
                Arrow::Up => Sfx::DialUp,
                Arrow::Right => Sfx::DialRight,
                Arrow::Down => Sfx::DialDown,
                Arrow::Left => Sfx::DialLeft,
            };
            combat.play(if dialed == Dialed::Wrong { Sfx::DialWrong } else { tone }, 0.7);
        }
        // Wanted or in hand, the hands are its: what's held goes away.
        let hands = &mut combat.arms[self.n].hands;
        if radio.holds() {
            hands.stow();
        }
        for cue in radio.update(hands.stowed().is_some(), dt) {
            let (sfx, gain) = match cue {
                Cue::On => (Sfx::RadioOn, 0.8),
                Cue::Talk => (Sfx::RadioTalk, 0.8),
                // (What it's called for comes of it from here, in time:
                // for now, its name's flashed.)
                Cue::Over(call) => {
                    self.note = Some((call.entry().name, super::loot::NOTE_FOR));
                    (Sfx::RadioOver, 0.7)
                }
            };
            combat.play(sfx, gain);
        }
    }

    /// The radio's card, beside it over their `pane` of the `window`,
    /// while it's out.
    pub fn radio_card(&self, ui: &mut Ui, pane: Rect, window: Rect) {
        let Some(radio) = self.radio.filter(|r| r.card() > 0.0) else { return };
        let on_pad = self.input.on_pad || !self.input.has_keys();
        // What dials: the keys that walk, written together if each is a
        // letter ("WASD").
        let keys = [Action::Forward, Action::Left, Action::Back, Action::Right].map(|a| self.input.name(a));
        let dial = match on_pad {
            true => "D-PAD".to_string(),
            false if keys.iter().all(|k| k.chars().count() == 1) => keys.concat(),
            false => keys.join(" "),
        };
        card::draw(ui, pane, window, &radio, &card::Hints { dial, away: self.input.name(Action::Radio), held: on_pad });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loot::bag::{Bag, Slot};
    use crate::loot::{Kind, Stack};
    use crate::radio::Radio;
    use crate::radio::codes::Call;
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
                seat.input.set_dialing(seat.radio_held());
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
        assert!(seat.radio_held() && combat.arms[0].hands.stowed() == Some(Some(Slot::Sidearm)));
        assert_eq!(seat.radio_shown().map(|s| (s.clip, s.stowed)), Some(("Idle", 0.0)));
        // W, D, D: a strafing run's code. It's keyed, its name's flashed
        // as it goes out, and the dial's clear for the next.
        for key in ['w', 'd', 'd'] {
            run(&mut h, &mut seat, &mut combat, Some(Key::Char(key)), false, 2);
        }
        assert_eq!(seat.radio.and_then(|r| r.calling()), Some(Call::StrafingRun));
        assert_eq!(seat.radio_shown().map(|s| s.clip), Some("Key"));
        run(&mut h, &mut seat, &mut combat, None, false, 90);
        assert_eq!(seat.note.map(|(n, _)| n), Some("STRAFING RUN"));
        assert!(seat.radio.is_some_and(|r| r.dialing() && r.dial().len() == 0));
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
        assert!(seat.radio_held());
        run(&mut h, &mut seat, &mut combat, None, true, 90);
        assert!(!seat.radio_out() && combat.arms[0].hands.held == Some(Slot::Sidearm) && !combat.arms[0].hands.busy());
        // The blade's key: the radio down, and it's the blade that comes up.
        run(&mut h, &mut seat, &mut combat, Some(Key::Char('q')), false, 90);
        run(&mut h, &mut seat, &mut combat, Some(Key::Char('3')), false, 90);
        assert!(!seat.radio_out() && combat.arms[0].hands.held == Some(Slot::Melee) && combat.arms[0].hands.weapon == Weapon::Knife);
        // The hands wanted for something else (not `free`): away at once.
        run(&mut h, &mut seat, &mut combat, Some(Key::Char('q')), false, 90);
        h.frame(|ui| seat.radioing(ui, &mut combat, false, false, DT));
        assert!(seat.radio_shown().is_some_and(|s| s.stowed >= 0.0) && !seat.radio_held());
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
