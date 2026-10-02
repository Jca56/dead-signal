//! The handheld radio, from the run's side: Q (a pad's View, held) pulls
//! it out and puts it away (out, a tap of a pad's View puts it away too,
//! and doesn't bring the bag up); out, the hands are its (what was held is put
//! away, to come back after), the feet stand still, and the keys that
//! walk (a pad's d-pad) punch its code in; a whole code's called in, if
//! there's the signal for it (what they kill charges it), and the signal
//! spent as it's sent. A drop called for, its flare comes up in the left
//! hand: the trigger held aims its throw, let go throws it. Anything else
//! the hands are
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

    /// Whether it's in hand (or wanted there): the feet stand still to
    /// dial.
    pub(super) fn radio_held(&self) -> bool {
        self.radio.is_some_and(|r| r.holds())
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
        // (With a flare to throw, the trigger's for that.)
        let cut = pulls.strike || (pulls.fire && radio.flare().is_none());
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
        let arrow = self.input.dial(ui);
        let Some(radio) = &mut self.radio else { return };
        radio.signal.charge(&self.stats);
        if let Some((arrow, dialed)) = arrow.and_then(|a| radio.press(a).map(|d| (a, d))) {
            // A whole code, and not the signal for it: refused.
            let refused = matches!(dialed, Dialed::Called(call) if !radio.signal.has(call.entry().cost));
            if refused {
                radio.refuse();
                self.note = Some(("NOT ENOUGH SIGNAL", super::loot::NOTE_FOR));
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
                Cue::Over(call) => {
                    self.sent(game, call);
                    (Sfx::RadioOver, 0.7)
                }
                Cue::Lit => (Sfx::Ignite, 0.5),
                Cue::Thrown(call) => {
                    if let Some((from, vel)) = self.flare_throw(game) {
                        crate::support::throw_flare(&mut game.world, call, from, vel, self.n);
                    }
                    (Sfx::Whoosh, 0.9)
                }
            };
            combat.play(sfx, gain);
        }
    }

    /// `call`'s gone out: its signal's spent. A drop's flare is theirs to
    /// throw; anything else is the run's to begin.
    fn sent(&mut self, game: &mut Game, call: Call) {
        let Some(radio) = &mut self.radio else { return };
        if !radio.signal.spend(call.entry().cost) {
            return;
        }
        if call.dropped() {
            radio.give_flare(call);
        } else {
            crate::support::called(&mut game.world, call, self.n);
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
        card::draw(ui, pane, window, &radio, &card::Hints { dial, away: self.input.name(Action::Radio), throw: self.input.name(Action::Fire) });
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
    use crate::world::Game;
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
        seat.radio.iter_mut().for_each(|r| r.signal.fill());
        seat.take_up(combat, Some(Slot::Sidearm));
        seat
    }

    /// A world with a floor and what a flare needs, the player on it.
    fn game() -> Game {
        let mut game = Game::new();
        let mut solids = crate::collide::Solids::new();
        solids.add(&crate::collide::box_tris(lntrn_math::Vec3::new(-60.0, -1.0, -60.0), lntrn_math::Vec3::new(60.0, 0.0, 60.0)));
        game.world.insert_resource(crate::world::Solid(solids));
        game.spawn_player(0, 0.0, 0.0, 0.0);
        game
    }

    /// `frames` of the hands and the radio, with `key` pressed on the first
    /// (the keys theirs) and the trigger pulled on it if `fire`.
    fn run(h: &mut Harness, seat: &mut Seat, combat: &mut Combat, key: Option<Key>, fire: bool, frames: u32) {
        run_in(&mut game(), h, seat, combat, key, (fire, false), frames);
    }

    /// The same in `game`, the trigger pulled on the first frame and held
    /// through them all as `trigger` says.
    fn run_in(game: &mut Game, h: &mut Harness, seat: &mut Seat, combat: &mut Combat, key: Option<Key>, trigger: (bool, bool), frames: u32) {
        if let Some(key) = key {
            h.key(key);
        }
        for k in 0..frames {
            h.frame(|ui| {
                seat.input.update(ui, Some(Default::default()), true, Default::default(), DT);
                seat.input.set_dialing(seat.radio_held());
                seat.radioing(ui, game, combat, true, Pulls { fire: trigger.0 && k == 0, hold: trigger.1, strike: false }, DT);
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
        assert!(seat.radio.is_some_and(|r| r.dialing() && r.dial().len() == 0));
        assert_eq!(seat.radio.map(|r| r.signal.bars()), Some(2.0), "three of its five bars spent");
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
        h.frame(|ui| seat.radioing(ui, &mut game(), &mut combat, false, Pulls::default(), DT));
        assert!(seat.radio_shown().is_some_and(|s| s.stowed >= 0.0) && !seat.radio_held());
        run(&mut h, &mut seat, &mut combat, None, false, 90);
        assert!(!seat.radio_out() && combat.arms[0].hands.held == Some(Slot::Melee));
    }

    #[test]
    fn kills_charge_the_signal_and_a_code_there_s_not_the_signal_for_is_refused() {
        let mut h = Harness::new(800.0, 600.0);
        let mut combat = Combat::new();
        let mut seat = seat(&mut combat);
        seat.radio = Some(Radio::default());
        // Nothing killed yet: an ammo drop's code is refused, nothing keyed.
        run(&mut h, &mut seat, &mut combat, Some(Key::Char('q')), false, 90);
        for key in ['s', 's', 'w', 'd'] {
            run(&mut h, &mut seat, &mut combat, Some(Key::Char(key)), false, 2);
        }
        assert_eq!(seat.note.map(|(n, _)| n), Some("NOT ENOUGH SIGNAL"));
        assert!(seat.radio.is_some_and(|r| r.calling().is_none() && r.dial().len() == 0 && r.wrong().is_some()));
        assert_eq!(seat.radio_shown().map(|s| s.clip), Some("Idle"));
        // Ten kills, a bar: still not the two it costs. Twenty: now it goes
        // out, and the bars with it.
        seat.stats.gun_kills = 10;
        for key in ['s', 's', 'w', 'd'] {
            run(&mut h, &mut seat, &mut combat, Some(Key::Char(key)), false, 2);
        }
        assert!(seat.radio.is_some_and(|r| r.calling().is_none() && r.signal.bars() == 1.0));
        seat.stats.gun_kills = 20;
        for key in ['s', 's', 'w', 'd'] {
            run(&mut h, &mut seat, &mut combat, Some(Key::Char(key)), false, 2);
        }
        assert_eq!(seat.radio.map(|r| (r.calling(), r.signal.bars())), Some((Some(Call::AmmoDrop), 2.0)), "not spent till it's sent");
        run(&mut h, &mut seat, &mut combat, None, false, 90);
        assert_eq!(seat.radio.map(|r| (r.flare(), r.signal.bars())), Some((Some(Call::AmmoDrop), 0.0)), "spent, and its flare's theirs");
        // Put away mid-word, before it's sent: nothing's spent.
        let mut seat = self::seat(&mut combat);
        seat.radio = Some(Radio::default());
        seat.stats.gun_kills = 20;
        run(&mut h, &mut seat, &mut combat, Some(Key::Char('q')), false, 90);
        for key in ['s', 's', 'w', 'd'] {
            run(&mut h, &mut seat, &mut combat, Some(Key::Char(key)), false, 2);
        }
        run(&mut h, &mut seat, &mut combat, Some(Key::Char('q')), false, 90);
        assert_eq!(seat.radio.map(|r| (r.out(), r.flare(), r.signal.bars())), Some((false, None, 2.0)));
    }

    #[test]
    fn a_drop_s_flare_is_aimed_with_the_trigger_held_and_thrown_as_it_s_let_go() {
        use crate::support::Flare;
        let mut h = Harness::new(800.0, 600.0);
        let mut combat = Combat::new();
        let mut seat = seat(&mut combat);
        let mut game = game();
        run_in(&mut game, &mut h, &mut seat, &mut combat, Some(Key::Char('q')), (false, false), 90);
        for key in ['s', 'w', 'd', 'a'] {
            run_in(&mut game, &mut h, &mut seat, &mut combat, Some(Key::Char(key)), (false, false), 2);
        }
        run_in(&mut game, &mut h, &mut seat, &mut combat, None, (false, false), 120);
        assert!(seat.radio.is_some_and(|r| r.marking() && r.flare() == Some(Call::MedicDrop)));
        assert!(seat.throw_arc().is_none());
        // The trigger held: the radio stays out, and the arc's shown.
        run_in(&mut game, &mut h, &mut seat, &mut combat, None, (true, true), 10);
        assert!(seat.radio_held() && seat.throw_arc().is_some_and(|(dots, lands)| !dots.is_empty() && lands.is_some()));
        assert_eq!(game.world.query::<&Flare>().iter(&game.world).count(), 0);
        // Let go: thrown with the left hand, the radio still up, the dial
        // free again.
        run_in(&mut game, &mut h, &mut seat, &mut combat, None, (false, false), 60);
        let flares: Vec<Flare> = game.world.query::<&Flare>().iter(&game.world).copied().collect();
        assert!(flares.len() == 1 && flares[0].call == Call::MedicDrop && flares[0].by == 0, "{flares:?}");
        assert!(seat.radio.is_some_and(|r| r.flare().is_none() && r.dialing()) && seat.radio_shown().is_some_and(|s| s.clip == "Idle"));
        assert!(seat.throw_arc().is_none());
    }

    #[test]
    fn on_a_pad_view_held_pulls_it_out_and_tapped_puts_it_away_and_not_the_bag_up() {
        use crate::input::pad::{frame_holding, frame_with};
        use lntrn_sys::gamepad::Button;
        let mut h = Harness::new(800.0, 600.0);
        let mut combat = Combat::new();
        let mut seat = seat(&mut combat);
        let mut world = game();
        let mut frame = |seat: &mut Seat, combat: &mut Combat, pad, dt: f64| {
            let mut bag = false;
            h.frame(|ui| {
                seat.input.update(ui, None, true, pad, dt);
                seat.input.set_dialing(seat.radio_held());
                seat.radioing(ui, &mut world, combat, true, Pulls::default(), dt);
                seat.switch_hands(ui, combat, true);
                combat.arms[0].hands.update(Trigger::default(), dt);
                bag = seat.input.pressed(ui, Action::Inventory);
            });
            bag
        };
        let (tap, held, rest) = (frame_with(&[Button::Select]), frame_holding(&[Button::Select]), Default::default());
        // Held: out it comes (and no bag as it's let go).
        frame(&mut seat, &mut combat, held.merge(tap), DT);
        assert!(!frame(&mut seat, &mut combat, held, 0.5) && seat.radio_out());
        assert!(!frame(&mut seat, &mut combat, rest, DT));
        for _ in 0..60 {
            frame(&mut seat, &mut combat, rest, DT);
        }
        assert!(seat.radio_held());
        // Tapped, the radio out: away it goes, and the bag stays shut.
        assert!(!frame(&mut seat, &mut combat, tap, DT), "the tap's the radio's");
        assert!(!seat.radio_held());
        for _ in 0..60 {
            frame(&mut seat, &mut combat, rest, DT);
        }
        assert!(!seat.radio_out() && combat.arms[0].hands.held == Some(Slot::Sidearm));
        // Tapped again, the radio away: the bag.
        assert!(frame(&mut seat, &mut combat, tap, DT));
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
