//! The handheld radio, from the run's side: Q (a pad's View, held) pulls
//! it out and puts it away (out, a tap of a pad's View puts it away too,
//! and doesn't bring the bag up); out, the hands are its (what was held is put
//! away, to come back after), and while its dial's being worked the feet
//! stand still and the keys that walk (a pad's d-pad) punch its code in;
//! a whole code's called in there and then, if there's the signal for it
//! (what they kill charges it), and the feet are free again. What was
//! called for seen to (a strike placed, a flare thrown, a boost begun),
//! the radio goes away by itself. A drop called for, its flare comes up in the left
//! hand: the trigger held aims its throw, let go throws it. A strike
//! called for, the ground they look at is marked (a strip, a spot), and
//! the trigger sends it there. Anything else the hands are
//! wanted for puts it away: a shot or a blow, another weapon, a kit, a
//! throw, the bag, someone to pick up, going down.

use lntrn_math::Rect;
use lntrn_ui::Ui;

use super::seat::Seat;
use crate::combat::Combat;
use crate::radio::codes::{Arrow, Call, Dialed};
use crate::radio::{Cue, Shown, card};
use crate::settings::keys::Action;
use crate::sound::Sfx;
use crate::world::Game;

/// What the trigger and a blow ask this frame: the trigger pulled, and
/// held; a blow struck.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Pulls {
    pub fire: bool,
    pub hold: bool,
    pub strike: bool,
}

impl Seat {
    /// Whether the radio's out (or on its way out, or away).
    pub fn radio_out(&self) -> bool {
        self.radio.is_some_and(|r| r.out())
    }

    /// Whether it's in hand (or wanted there).
    #[cfg(test)]
    pub(super) fn radio_held(&self) -> bool {
        self.radio.is_some_and(|r| r.holds())
    }

    /// Whether its dial's being worked: the keys that walk are its, and
    /// the feet stand still. (A code in, they're free again: a strike's
    /// placed, a flare thrown, on the move.)
    pub(super) fn dialling(&self) -> bool {
        self.radio.is_some_and(|r| r.dialing())
    }

    /// Their signal, and what pulls the radio out (a pad's is held), for
    /// the HUD.
    pub fn signal_shown(&self) -> Option<(crate::radio::signal::Signal, String)> {
        let on_pad = self.input.on_pad || !self.input.has_keys();
        self.radio.map(|r| (r.signal, format!("{}{}", if on_pad { "HOLD " } else { "" }, self.input.name(Action::Radio))))
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
    /// hands `free` for it; cut short by a shot or a blow: `pulls`), its
    /// code punched in, a flare aimed and thrown, and what's heard of it.
    pub(super) fn radioing(&mut self, ui: &mut Ui, game: &mut Game, combat: &mut Combat, free: bool, pulls: Pulls, dt: f64) {
        let Some(radio) = self.radio else { return };
        // (With a flare to throw or a strike to place, the trigger's for
        // that.)
        let cut = pulls.strike || (pulls.fire && radio.flare().is_none() && radio.strike().is_none());
        if radio.out() {
            // (A pad's View tapped is the bag's: with the radio out, it
            // puts the radio away instead.)
            if !free || cut || self.input.pressed(ui, Action::Radio) || self.input.pad_pressed(Action::Inventory) {
                self.radio_away(combat);
            }
        } else if free && self.input.pressed(ui, Action::Radio) {
            self.radio.iter_mut().for_each(|r| r.pull());
            // (At once: the first arrow may come with the very next frame.)
            self.input.set_dialing(true);
        }
        // The flare in hand: the trigger held aims it, let go throws it.
        if self.aim_flare(ui, game, free && radio.marking(), pulls.hold) {
            self.radio.iter_mut().for_each(|r| {
                r.throw();
            });
        }
        // A strike to place: the ground they're looking at's marked (a
        // strip, a spot: as the strike is), and the trigger sends it there
        // (not under a roof).
        self.zone = self.radio.and_then(|r| r.placing()).filter(|_| free).and_then(|call| self.marked(game, call));
        if let Some(mark) = self.zone.filter(|_| pulls.fire) {
            if mark.open() {
                // Sent: and the radio's away at once, the hands free.
                self.radio.iter_mut().for_each(|r| r.placed());
                mark.call(&mut game.world, self.n);
                combat.play(Sfx::RadioOver, 0.7);
                self.radio_away(combat);
                self.zone = None;
            } else {
                self.note = Some(("NO OPEN SKY", super::loot::NOTE_FOR));
                combat.play(Sfx::DialWrong, 0.6);
            }
        }
        let arrow = self.input.dial(ui);
        let Some(radio) = &mut self.radio else { return };
        radio.signal.charge(&self.stats);
        if let Some((arrow, dialed)) = arrow.and_then(|a| radio.press(a).map(|d| (a, d))) {
            // A whole code: called in there and then, if there's the
            // signal for it; refused, if not.
            let refused = matches!(dialed, Dialed::Called(call) if !radio.signal.spend(call.entry().cost));
            if refused {
                radio.refuse();
                self.note = Some(("NOT ENOUGH SIGNAL", super::loot::NOTE_FOR));
            } else if let Dialed::Called(call) = dialed {
                if call.dropped() {
                    // A drop: its flare in hand at once.
                    radio.give_flare(call);
                    combat.play(Sfx::RadioOver, 0.6);
                } else if call.struck() {
                    // A strike: theirs to place, from this moment.
                    radio.give_strike(call);
                } else {
                    // Anything else is the run's to begin, now; and the
                    // radio goes away of itself once it's keyed.
                    crate::support::called(&mut game.world, call, self.n);
                    radio.leave();
                }
            }
            let tone = match arrow {
                Arrow::Up => Sfx::DialUp,
                Arrow::Right => Sfx::DialRight,
                Arrow::Down => Sfx::DialDown,
                Arrow::Left => Sfx::DialLeft,
            };
            combat.play(if refused || dialed == Dialed::Wrong { Sfx::DialWrong } else { tone }, 0.7);
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
                Cue::Over => (Sfx::RadioOver, 0.7),
                Cue::Off => (Sfx::Click, 0.45),
                Cue::Lit => (Sfx::Ignite, 0.5),
                // Thrown: and with that the radio's done with.
                Cue::Thrown(call) => {
                    if let Some((from, vel)) = self.flare_throw(game) {
                        crate::support::throw_flare(&mut game.world, call, from, vel, self.n);
                    }
                    self.radio.iter_mut().for_each(|r| r.leave());
                    (Sfx::Whoosh, 0.9)
                }
            };
            combat.play(sfx, gain);
        }
    }

    /// The ground `call`'s strike would be marked on, looking where they
    /// look now.
    fn marked(&self, game: &mut Game, call: Call) -> Option<crate::support::Mark> {
        let (body, view) = game.player(self.n)?;
        let eye = crate::head::eye_position(&view, &body, game.alpha());
        let (yaw, pitch) = view.aim();
        let look = lntrn_math::Vec3::new(-yaw.sin() * pitch.cos(), pitch.sin(), -yaw.cos() * pitch.cos());
        crate::support::Mark::of(call, &game.world.resource::<crate::world::Solid>().0, eye, look)
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
        card::draw(ui, pane, window, &radio, &card::Hints { dial, away: self.input.name(Action::Radio), throw: self.input.name(Action::Fire) });
    }
}

#[cfg(test)]
mod tests;
