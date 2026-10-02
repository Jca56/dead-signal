//! Stims, from the run's side: bought at a med station (E, the points
//! paid) and jabbed in there and then, the hands its while it goes in
//! (what was held put away, to come back after); what's in the blood kept
//! in step with the body (Bulwark's health); all of it gone, going down;
//! and Lazarus, alone, bringing them back the once when they'd have died.

use bevy_ecs::prelude::*;
use lntrn_math::Vec3;

use super::loot::NOTE_FOR;
use super::seat::Seat;
use crate::combat::Combat;
use crate::holdout::Holdout;
use crate::holdout::stims::{self, BACK_WITH, Cue, HEALTH, Stim};
use crate::player::Player;
use crate::sound::Sfx;
use crate::viewmodel::Instead;
use crate::world::Game;

impl Seat {
    /// Whether the hands are taken up with what's no weapon: the radio, a
    /// stim going in.
    pub(super) fn hands_taken(&self) -> bool {
        self.radio_out() || self.stims.jabbing()
    }

    /// What's in hand in a weapon's place, as it's drawn, if anything is:
    /// a stim's injector, or the radio (in a pane that sees `sees`).
    pub fn in_hand_instead(&self, sees: [f64; 2]) -> Option<Instead> {
        match self.stims.shown() {
            Some((shown, stim)) => Some(Instead::Stim(shown, stim.colour())),
            None => self.radio_shown().map(|shown| Instead::Radio(shown, sees)),
        }
    }

    /// What the med station selling `stim` says to them (`alone`: playing
    /// alone).
    pub(super) fn stim_prompt(&self, stim: Stim, alone: bool) -> String {
        if self.stims.has(stim) {
            format!("{}  ·  IN YOUR BLOOD", stim.name())
        } else if stim == Stim::Lazarus && alone && self.stims.spent() {
            "LAZARUS  ·  SPENT".to_string()
        } else {
            format!("{} [{}]  {}", stim.name(), stim.price(), stim.does(alone))
        }
    }

    /// E at the med station selling `stim`: paid for from their points,
    /// and on its way in. (Not one they have; nor Lazarus, alone, once
    /// it's been their second life.)
    pub(super) fn buy_stim(&mut self, h: &mut Holdout, game: &Game, combat: &Combat, stim: Stim, alone: bool) {
        if self.stims.jabbing() {
            return;
        }
        let refused = if self.stims.has(stim) {
            Some("IN YOUR BLOOD ALREADY")
        } else if stim == Stim::Lazarus && alone && self.stims.spent() {
            Some("ONLY THE ONCE")
        } else if !h.pay(&game.world, self.n, stim.price()) {
            Some("NOT ENOUGH POINTS")
        } else {
            None
        };
        match refused {
            Some(why) => {
                self.note = Some((why, NOTE_FOR));
                combat.play(Sfx::DryFire, 0.9);
            }
            None => {
                self.stims.begin(stim);
                combat.play(Sfx::Pickup, 0.9);
            }
        }
    }

    /// A frame of what's in their blood, and of one going in: what's held
    /// put away for it, the needle in (it's theirs from then), and their
    /// health's most kept in step. Whether the hands are its.
    pub fn stimming(&mut self, combat: &mut Combat, dt: f64) -> bool {
        let hands = &mut combat.arms[self.n].hands;
        if self.stims.jabbing() {
            hands.stow();
        }
        if let Some(Cue::In(stim)) = self.stims.update(hands.stowed().is_some(), dt) {
            self.note = Some((stim.name(), NOTE_FOR));
            combat.play(Sfx::Jab, 0.9);
        }
        self.stim_health();
        self.stims.jabbing()
    }

    /// Their health's most, as what's in their blood has it: Bulwark in,
    /// it's that much more, and they've that much more; gone, what's over
    /// the less goes with it.
    fn stim_health(&mut self) {
        let most = self.perks.max_hp() + if self.stims.has(Stim::Bulwark) { HEALTH } else { 0.0 };
        let more = most - self.vitals.max_hp;
        if more != 0.0 {
            self.vitals.max_hp = most;
            self.vitals.hp = if more > 0.0 { self.vitals.hp + more } else { self.vitals.hp.min(most) };
        }
    }

    /// Down: every stim's gone from their blood.
    pub(super) fn lose_stims(&mut self) {
        self.stims.lose();
        self.stim_health();
    }

    /// About to die, alone: Lazarus brings them back, if it's in their
    /// blood and hasn't before. On their feet with half their health,
    /// what was killing them gone, the dead about them thrown off.
    /// Whether it did.
    pub(super) fn second_life(&mut self, game: &mut Game, combat: &mut Combat) -> bool {
        if !self.stims.second_life() {
            return false;
        }
        let v = &mut self.vitals;
        (v.hp, v.bleeding, v.poison, v.healing) = (v.max_hp * BACK_WITH, 0, 0.0, None);
        let alight: Vec<Entity> = game.world.query::<(Entity, &Player)>().iter(&game.world).filter(|(_, p)| p.0 == self.n).map(|(e, _)| e).collect();
        for e in alight {
            game.world.entity_mut(e).remove::<crate::throw::Burning>();
        }
        if let Some((body, _)) = game.player(self.n) {
            stims::throw_off(&mut game.world, body.pos);
            combat.fx.flash(body.pos + Vec3::new(0.0, 1.2, 0.0), 14.0, Stim::Lazarus.colour().map(|c| c * 3.0), 0.5);
        }
        combat.play(Sfx::Jolt, 1.0);
        self.note = Some(("LAZARUS", NOTE_FOR * 1.5));
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::holdout::arena::Arena;
    use crate::holdout::stims::{IN_AT, JAB, SAFE_FOR};
    use crate::loot::bag::Slot;
    use crate::profile::perks::Perks;
    use crate::run::Run;
    use crate::vitals::Vitals;
    use crate::weapon::Trigger;
    use crate::zombie::brain::Blow;

    const DT: f64 = 1.0 / 60.0;

    fn holdout(players: usize) -> Holdout {
        let arena = Arena { reach: 20.0, bounds: (Vec3::splat(-20.0), Vec3::splat(20.0)), zones: vec!["YARD"], start: 0, spawn: Vec3::ZERO, windows: Vec::new(), doors: Vec::new(), buys: Vec::new(), lamps: Vec::new(), signs: Vec::new() };
        Holdout::new(arena, 1, players)
    }

    /// A seat in a holdout's kit, its pistol up.
    fn seat(n: usize, combat: &mut Combat) -> Seat {
        let mut seat = Seat::default();
        (seat.n, seat.bag, seat.vitals) = (n, Holdout::loadout(), Vitals::with(&Perks::default()));
        seat.take_up(combat, Some(Slot::Sidearm));
        seat
    }

    /// `seconds` of a seat's stims and hands.
    fn run(seat: &mut Seat, combat: &mut Combat, seconds: f64) {
        for _ in 0..(seconds / DT).round() as usize {
            seat.stimming(combat, DT);
            combat.arms[seat.n].hands.update(Trigger::default(), DT);
        }
    }

    #[test]
    fn bought_it_s_paid_for_and_jabbed_in_and_the_gun_comes_back_after() {
        let game = Game::new();
        let mut combat = Combat::new();
        let mut h = holdout(1);
        let mut seat = seat(0, &mut combat);
        let whole = seat.vitals.max_hp;
        assert_eq!(seat.stim_prompt(Stim::Bulwark, true), "BULWARK [2500]  +75 HEALTH");
        // Too few points: refused, nothing begun.
        seat.buy_stim(&mut h, &game, &combat, Stim::Bulwark, true);
        assert_eq!((seat.note.map(|(n, _)| n), seat.stims.jabbing(), h.wallets[0].points), (Some("NOT ENOUGH POINTS"), false, 500));
        // With them: paid, the pistol put away, the injector up, and half
        // a second into the jab it's theirs: that much more health.
        h.wallets[0].points = 6000;
        seat.vitals.hp = 40.0;
        seat.buy_stim(&mut h, &game, &combat, Stim::Bulwark, true);
        assert_eq!((h.wallets[0].points, seat.stims.jabbing(), seat.hands_taken()), (3500, true, true));
        seat.buy_stim(&mut h, &game, &combat, Stim::Twitch, true);
        assert_eq!(h.wallets[0].points, 3500, "one at a time");
        run(&mut seat, &mut combat, 1.0 + IN_AT);
        assert!(seat.stims.has(Stim::Bulwark) && combat.arms[0].hands.stowed().is_some());
        assert_eq!((seat.vitals.max_hp, seat.vitals.hp), (whole + HEALTH, 40.0 + HEALTH));
        assert!(seat.in_hand_instead([1.0, 1.0]).is_some_and(|i| matches!(i, Instead::Stim(shown, _) if shown.clip == "Jab")));
        assert_eq!(seat.stim_prompt(Stim::Bulwark, true), "BULWARK  ·  IN YOUR BLOOD");
        // It can't be bought twice.
        run(&mut seat, &mut combat, JAB);
        assert!(!seat.stims.jabbing() && !seat.hands_taken());
        seat.buy_stim(&mut h, &game, &combat, Stim::Bulwark, true);
        assert_eq!((seat.note.map(|(n, _)| n), h.wallets[0].points), (Some("IN YOUR BLOOD ALREADY"), 3500));
        // Down, it's gone, and what health was over the less with it.
        seat.vitals.hp = whole + 50.0;
        seat.lose_stims();
        assert_eq!((seat.vitals.max_hp, seat.vitals.hp, seat.stims.has(Stim::Bulwark)), (whole, whole, false));
    }

    #[test]
    fn alone_lazarus_brings_them_back_the_once_and_together_it_s_a_faster_hand_up() {
        let mut game = Game::new();
        game.spawn_player(0, 0.0, 0.0, 0.0);
        let mut combat = Combat::new();
        combat.reset(1);
        let mut run = Run { seats: vec![seat(0, &mut combat)], holdout: Some(holdout(1)), ..Run::default() };
        let whole = run.seats[0].vitals.max_hp;
        assert_eq!(run.seats[0].stim_prompt(Stim::Lazarus, true), "LAZARUS [1500]  SECOND LIFE");
        assert_eq!(run.seats[0].stim_prompt(Stim::Lazarus, false), "LAZARUS [1500]  FAST REVIVE");
        run.seats[0].stims.take(Stim::Lazarus);
        // A blow that would kill: back on their feet with half, bleeding
        // no more, and for a moment nothing can touch them.
        let kills = Blow { push: Vec3::ZERO, damage: 500.0, leaves: None };
        run.seats[0].vitals.bleeding = 2;
        let died = run.seats[0].suffer(&mut game, &mut combat, [kills], false, Default::default(), false);
        assert!(died.is_some_and(|how| !run.fall(&mut game, &mut combat, 0, how)), "it's not over");
        let s = &run.seats[0];
        assert!(s.standing() && run.ending.is_none());
        assert_eq!((s.vitals.hp, s.vitals.bleeding, s.stims.has(Stim::Lazarus), s.stims.spent()), (whole * BACK_WITH, 0, false, true));
        assert!(run.seats[0].suffer(&mut game, &mut combat, [kills], false, Default::default(), false).is_none(), "safe, for a moment");
        // It's not for sale again; and the next time, it's over.
        assert_eq!(run.seats[0].stim_prompt(Stim::Lazarus, true), "LAZARUS  ·  SPENT");
        for _ in 0..((SAFE_FOR + 0.1) / DT) as usize {
            run.seats[0].stimming(&mut combat, DT);
        }
        let died = run.seats[0].suffer(&mut game, &mut combat, [kills], false, Default::default(), false);
        assert!(died.is_some_and(|how| run.fall(&mut game, &mut combat, 0, how)) && run.ending.is_some());
        // Together: no second life (they go down, and it's gone with the
        // rest), but whoever has it picks the other up in half the time.
        let mut combat = Combat::new();
        combat.reset(2);
        game.spawn_player(1, 1.0, 0.0, 0.0);
        let mut run = Run { seats: vec![seat(0, &mut combat), seat(1, &mut combat)], holdout: Some(holdout(2)), ..Run::default() };
        run.seats[0].stims.take(Stim::Lazarus);
        run.seats[0].stims.take(Stim::Bulwark);
        assert!(!run.fall(&mut game, &mut combat, 0, crate::ending::Outcome::Died(1.0)));
        let s = &run.seats[0];
        assert!(s.down.is_some() && s.stims.all().count() == 0 && !s.stims.spent() && s.vitals.max_hp == whole);
        assert_eq!((run.seats[1].stims.revives(), Stim::Lazarus.price()), (1.0, 1500));
        run.seats[1].stims.take(Stim::Lazarus);
        assert_eq!(run.seats[1].stims.revives(), 2.0);
    }
}
