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
use crate::holdout::{Holdout, arena::Arena};
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
        game.world.insert_resource(crate::zombie::Stealth(1.0));
        game.world.insert_resource(crate::zombie::Heat::default());
        let mut holdout = Holdout::new(arena, seed.rotate_left(3), self.seats.len());
        holdout.begin(&mut game.world);
        self.holdout = Some(holdout);
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
        if h.update(&mut game.world, &feet, dt) {
            // A new round: the radio crackles.
            combat.play(Sfx::Static, 0.9);
        }
        crate::holdout::props::sync(&mut game.world, h);
    }

    /// The rest of a holdout's frame, for each player: what's in front of
    /// them used, their HUD over their pane (`panes`, by seat), their bag
    /// (playing alone: its screen would cover the others' panes).
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
    /// to buy or open, held to nail boards back), the HUD, the bag (if
    /// they're playing `alone`).
    #[allow(clippy::too_many_arguments)]
    fn hold_out(&mut self, ui: &mut Ui, pane: Rect, cx: &mut AreaCx<()>, game: &mut Game, combat: &mut Combat, icons: &Icons, dt: f64, h: &mut Holdout, alone: bool) {
        // Down, or picking someone up (whose prompt it is), nothing else.
        let busy = self.open.is_some() || self.vitals.healing.is_some() || !self.standing() || self.reviving.is_some();
        let aimed = if busy { None } else { loot::eye(game, self.n).and_then(|(eye, dir)| h.aimed(&game.world, eye, dir)) };
        self.input.set_prompting(aimed.is_some() || self.reviving.is_some());
        if let Some(a) = aimed
            && self.input.pressed(ui, Action::Interact)
        {
            let (sound, note, took) = h.press(&mut game.world, self.n, a, &mut self.bag);
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
            None => aimed.map(|a| ("E", h.prompt(a, &self.bag))),
        };
        let picking_up = self.reviving.map(|(_, p)| p).filter(|&p| p > 0.0);
        self.hud(ui, pane, combat, game, prompt, h.nail_progress(self.n).or(picking_up));
        crate::holdout::hud::draw(ui, pane, h, self.n);
        if let Some(d) = self.down {
            crate::mates::down(ui, pane, d.left, super::down::BLEED_OUT, d.revive);
        }
        if alone {
            self.looting(ui, cx, game, combat, icons, dt, loot::Aimed::Nothing);
        }
    }
}
