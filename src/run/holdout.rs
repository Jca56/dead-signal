//! A run in the holdout, from the run's side: begun with nothing but a
//! pistol and a knife; each step, the points counted up, the rounds on and
//! the arena's props kept in step; each frame, what's on the walls bought,
//! doors opened and boards nailed back, with the round and the points
//! over it all.

use lntrn_math::{Rect, Vec3};
use lntrn_ui::{AreaCx, Ui};

use super::loot::{self, NOTE_FOR};
use super::Run;
use super::seat::Seat;
use crate::bag_ui::Icons;
use crate::combat::Combat;
use crate::holdout::rounds::{Event, Wave};
use crate::holdout::{Holdout, arena::Arena};
use crate::input::Prompt;
use crate::profile::perks::Perks;
use crate::settings::keys::Action;
use crate::sound::Sfx;
use crate::world::Game;

impl Run {
    /// A fresh holdout in `arena` for `players`: each whole, with a pistol
    /// and a knife, the windows boarded, the first round on its way.
    pub fn start_holdout(&mut self, game: &mut Game, combat: &mut Combat, arena: Arena, best_round: u32, players: usize) {
        *self = Self::default();
        self.best_round = best_round;
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.subsec_nanos());
        combat.reset(players);
        self.seats = (0..players).map(|n| Seat::new(n, Holdout::loadout(), Perks::default(), seed, combat)).collect();
        for seat in &mut self.seats {
            (seat.fit, seat.unburdened) = (Holdout::fit(), true);
        }
        game.world.insert_resource(crate::zombie::Stealth(1.0));
        game.world.insert_resource(crate::zombie::Heat::default());
        let mut holdout = Holdout::new(arena, seed.rotate_left(3), self.seats.len());
        holdout.begin(&mut game.world);
        self.holdout = Some(holdout);
    }

    /// (The dev's.) Everything up in a holdout dead (but a Juggernaut,
    /// which stays as it would) and this round over,
    /// `skip` rounds passed over, and the next to be `wave` (if any's said).
    pub fn dev_round(&mut self, game: &mut Game, wave: Option<Wave>, skip: u32) {
        let Some(h) = &mut self.holdout else { return };
        for mut z in game.world.query::<&mut crate::zombie::brain::Zombie>().iter_mut(&mut game.world).filter(|z| z.kind != crate::zombie::kind::Kind::Juggernaut) {
            z.hurt(f64::INFINITY, false, false, Vec3::ZERO);
        }
        crate::zombie::rift::clear(&mut game.world);
        h.rounds.force(wave, skip);
    }

    /// (The dev's.) What the first player has in hand, put through the
    /// Amplifier for nothing.
    pub fn dev_amplify(&mut self, combat: &mut Combat) {
        let (Some(h), Some(seat)) = (&mut self.holdout, self.seats.first_mut()) else { return };
        let took = h.amplify_free(seat.n, &mut seat.bag, combat.arms[seat.n].hands.held);
        if took.is_some() {
            combat.play(crate::sound::Sfx::Amplify, 0.9);
            seat.take_up(combat, took);
        }
    }

    /// The round a holdout ended on, once.
    pub fn take_holdout_round(&mut self) -> Option<u32> {
        self.holdout_over.take()
    }

    /// The rounds a step on: the points earned, the dead brought in about
    /// the players, the boards and doors shown as they are.
    pub(super) fn holdout_step(&mut self, game: &mut Game, combat: &mut Combat, dt: f64) {
        let feet: Vec<Vec3> = self.seats.iter().filter(|s| s.standing()).filter_map(|s| game.player(s.n).map(|(body, _)| body.pos)).collect();
        let Some(h) = &mut self.holdout else { return };
        let alive = crate::zombie::alive(&mut game.world) as u32;
        for seat in &mut self.seats {
            h.score(seat.n, &seat.stats);
            seat.stats.biggest_horde = seat.stats.biggest_horde.max(alive);
        }
        let mut refill = false;
        for event in h.update(&mut game.world, &feet, dt) {
            match event {
                // A new round: the radio crackles (and the hounds are heard).
                Event::Began(wave) => {
                    combat.play(Sfx::Static, 0.9);
                    if wave == Wave::Hounds {
                        combat.play(Sfx::Howl, 0.9);
                    }
                }
                Event::Cleared(wave) => {
                    // The last hound dead: everyone's guns are full again.
                    refill |= wave == Wave::Hounds;
                    // What's next is heard, far off, before the radio says it.
                    match h.rounds.next {
                        Wave::Hounds => combat.play(Sfx::Howl, 0.5),
                        Wave::Dead { boss: 1.. } => combat.play(Sfx::Bellow, 0.4),
                        Wave::Dead { .. } => {}
                    }
                }
                // A Juggernaut down at last: the same, with its points.
                Event::Felled(_) => refill = true,
            }
        }
        if refill {
            combat.play(Sfx::Pickup, 1.0);
            for seat in self.seats.iter_mut().filter(|s| !s.out) {
                Holdout::max_ammo(&mut seat.bag);
                seat.note = Some(("MAX AMMO", NOTE_FOR));
            }
        }
        crate::holdout::props::sync(&mut game.world, h);
    }

    /// The rest of a holdout's frame, for each player: what's in front of
    /// them used, their HUD over their pane (`panes`, by seat), and their
    /// bag (playing together, over their own pane).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn holdout_frame(&mut self, ui: &mut Ui, cx: &mut AreaCx<()>, game: &mut Game, combat: &mut Combat, icons: &Icons, dt: f64, panes: &[Rect]) {
        let Some(h) = &mut self.holdout else { return };
        let alone = self.seats.len() == 1;
        let standing = self.seats.iter().find(|s| s.standing()).map(|s| s.n);
        for seat in &mut self.seats {
            let pane = panes.get(seat.n).copied().unwrap_or_else(|| ui.clip());
            ui.draw.push_clip(pane);
            if seat.out {
                // Bled out: the round and the points, watching another.
                crate::holdout::hud::draw(ui, pane, h, seat.n);
                crate::mates::out(ui, pane, standing);
            } else {
                seat.hold_out(ui, pane, cx, game, combat, icons, dt, h, alone);
            }
            ui.draw.pop_clip();
        }
    }
}

impl Seat {
    /// A holdout's frame for this player: what's in front of them used (E
    /// to buy or open, held to nail boards back), the HUD, the bag (over
    /// their `pane`, unless they're playing `alone`).
    #[allow(clippy::too_many_arguments)]
    fn hold_out(&mut self, ui: &mut Ui, pane: Rect, cx: &mut AreaCx<()>, game: &mut Game, combat: &mut Combat, icons: &Icons, dt: f64, h: &mut Holdout, alone: bool) {
        // (Down, the bag's put away.)
        if !self.standing() {
            self.shut_bag(game, cx);
        }
        // Down, or picking someone up (whose prompt it is), nothing else.
        let busy = self.open.is_some() || self.vitals.healing.is_some() || !self.standing() || self.reviving.is_some();
        let aimed = if busy { None } else { loot::eye(game, self.n).and_then(|(eye, dir)| h.aimed(&game.world, eye, dir)) };
        // (Boards are nailed, and someone picked up, by holding: a tap of a
        // pad's X is still a reload there.)
        self.input.set_prompt(match aimed {
            _ if self.reviving.is_some() => Prompt::Hold,
            Some(crate::holdout::Aimed::Window(_)) => Prompt::Hold,
            Some(_) => Prompt::Press,
            None => Prompt::None,
        });
        if let Some(a) = aimed
            && self.input.pressed(ui, Action::Interact)
        {
            let held = combat.arms[self.n].hands.held;
            let (sound, note, took) = h.press(&mut game.world, self.n, a, &mut self.bag, held);
            // (Out of the Amplifier, a flash of what it's made of it.)
            if sound == Some(crate::sound::Sfx::Amplify)
                && let (Some(slot), Some((eye, dir))) = (took, loot::eye(game, self.n))
                && let Some(glow) = self.bag.slot(slot).and_then(|g| crate::weapon::amp::glow(g.tier))
            {
                combat.fx.flash(eye + dir * 0.8, 9.0, glow.map(|c| c * 3.0), 0.5);
            }
            if let Some(sfx) = sound {
                combat.play(sfx, 0.9);
            }
            if let Some(note) = note {
                self.note = Some((note, NOTE_FOR));
            }
            if took.is_some() {
                self.take_up(combat, took);
            }
        }
        if let Some(sfx) = h.hold(&mut game.world, self.n, aimed, self.input.held(ui, Action::Interact), dt) {
            combat.play(sfx, 0.8);
        }
        let prompt = match self.reviving {
            Some((j, _)) => Some(("E", format!("REVIVE P{}", j + 1))),
            None => aimed.map(|a| ("E", h.prompt(a, &self.bag, combat.arms[self.n].hands.held))),
        };
        let picking_up = self.reviving.map(|(_, p)| p).filter(|&p| p > 0.0);
        self.hud(ui, pane, combat, game, prompt, h.nail_progress(self.n).or(picking_up));
        crate::holdout::hud::draw(ui, pane, h, self.n);
        if let Some(d) = self.down {
            crate::mates::down(ui, pane, d.left, super::down::BLEED_OUT, d.revive);
        }
        if self.standing() {
            self.bag_ui.pane = (!alone).then_some(pane);
            self.looting(ui, cx, game, combat, icons, dt, loot::Aimed::Nothing);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loot::gear::Wear;
    use crate::loot::{Kind, Stack};
    use crate::throw::{Burning, Felt};
    use crate::vitals::{Affliction, Vitals};
    use crate::zombie::brain::Blow;

    #[test]
    fn a_bag_up_on_a_pad_leaves_the_pointer_with_whoever_has_the_mouse() {
        let mut h = lntrn_ui::testing::Harness::new(800.0, 600.0);
        h.frame(|ui| {
            let mut seat = Seat::default();
            seat.open = Some(super::super::seat::Open { container: None });
            assert!(seat.rummaging() && seat.wants_lock(), "no mouse of theirs to let go of");
            seat.input.update(ui, Some(Default::default()), true, Default::default(), 0.016);
            assert!(!seat.wants_lock(), "the mouse theirs, it's free for the bag");
        });
    }

    #[test]
    fn armored_a_hounds_bite_does_not_catch_and_in_a_holdout_armor_weighs_nothing() {
        let mut game = Game::new();
        game.spawn_player(0, 0.0, 0.0, 0.0);
        let mut combat = Combat::new();
        let mut seat = Seat::default();
        seat.vitals = Vitals::with(&Perks::default());
        let whole = seat.vitals.hp;
        let bite = Blow { push: Vec3::ZERO, damage: 10.0, leaves: Some(Affliction::Burn) };
        let alight = |game: &mut Game| game.world.query::<&Burning>().iter(&game.world).count();
        // In a vest: it takes the bite, and the fire doesn't catch.
        *seat.bag.worn_mut(Wear::Chest) = Some(Stack::fresh(Kind::LightVest, 1));
        assert!(seat.suffer(&mut game, &mut combat, [bite], false, Felt::default(), false).is_none());
        assert_eq!((alight(&mut game), seat.bag.armor(), seat.vitals.hp), (0, (30, 40), whole));
        // In nothing: it bites, and they burn.
        *seat.bag.worn_mut(Wear::Chest) = None;
        seat.suffer(&mut game, &mut combat, [bite], false, Felt::default(), false);
        assert_eq!((alight(&mut game), seat.vitals.hp), (1, whole - 10.0));
        // A plate carrier weighs on a run out in the wilds, not on a holdout.
        *seat.bag.worn_mut(Wear::Chest) = Some(Stack::fresh(Kind::PlateCarrier, 1));
        assert_eq!(seat.weight(), 2);
        (seat.fit, seat.unburdened) = (Holdout::fit(), true);
        assert_eq!(seat.weight(), 0);
    }
}
