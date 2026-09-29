//! A run in the holdout, from the run's side: begun with nothing but a
//! pistol and a knife; each step, the points counted up, the rounds on and
//! the arena's props kept in step; each frame, what's on the walls bought,
//! doors opened and boards nailed back, with the round and the points
//! over it all.

use lntrn_math::Vec3;
use lntrn_ui::{AreaCx, Ui};

use super::loot::{self, NOTE_FOR};
use super::Run;
use crate::bag_ui::Icons;
use crate::combat::Combat;
use crate::holdout::{Holdout, arena::Arena};
use crate::profile::perks::Perks;
use crate::settings::keys::Action;
use crate::sound::Sfx;
use crate::vitals::Vitals;
use crate::world::Game;

impl Run {
    /// A fresh holdout in `arena`: whole, a pistol and a knife, the
    /// windows boarded, the first round on its way.
    pub fn start_holdout(&mut self, game: &mut Game, combat: &mut Combat, arena: Arena, best_round: u32) {
        *self = Self::default();
        self.best_round = best_round;
        self.bag = Holdout::loadout();
        let perks = Perks::default();
        self.perks = perks;
        self.vitals = Vitals::with(&perks);
        combat.hands.reload_speed = perks.reload_speed();
        combat.melee = perks.melee();
        self.take_up(combat, self.first_armed());
        game.world.insert_resource(crate::zombie::Stealth(1.0));
        game.world.insert_resource(crate::zombie::Heat::default());
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.subsec_nanos());
        self.dice = crate::loot::Dice(seed.rotate_left(7) | 1);
        let mut holdout = Holdout::new(arena, seed.rotate_left(3));
        holdout.begin(&mut game.world);
        self.holdout = Some(holdout);
    }

    /// The round a holdout ended on, once.
    pub fn take_holdout_round(&mut self) -> Option<u32> {
        self.holdout_over.take()
    }

    /// The rounds a step on, the player at `eye`: the points earned, the
    /// dead brought in, the boards and doors shown as they are.
    pub(super) fn holdout_step(&mut self, game: &mut Game, combat: &mut Combat, eye: Vec3, dt: f64) {
        let Some(h) = &mut self.holdout else { return };
        h.score(&self.stats);
        if h.update(&mut game.world, eye - Vec3::new(0.0, 1.6, 0.0), dt) {
            // A new round: the radio crackles.
            combat.play(Sfx::Static, 0.9);
        }
        crate::holdout::props::sync(&mut game.world, h);
    }

    /// The rest of a holdout's frame: what's in front of the player used
    /// (E to buy or open, held to nail boards back), the HUD, the bag.
    pub(super) fn holdout_frame(&mut self, ui: &mut Ui, cx: &mut AreaCx<()>, game: &mut Game, combat: &mut Combat, icons: &Icons, dt: f64) {
        // (Held apart while the run's own are used alongside.)
        let Some(mut h) = self.holdout.take() else { return };
        let busy = self.open.is_some() || self.vitals.healing.is_some();
        let aimed = if busy { None } else { loot::eye(game).and_then(|(eye, dir)| h.aimed(&game.world, eye, dir)) };
        if let Some(a) = aimed
            && self.keys.pressed(ui, Action::Interact)
        {
            let (sound, note, took) = h.press(&mut game.world, a, &mut self.bag);
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
        if let Some(sfx) = h.hold(&mut game.world, aimed, self.keys.held(ui, Action::Interact), dt) {
            combat.play(sfx, 0.8);
        }
        let prompt = aimed.map(|a| ("E", h.prompt(a, &self.bag)));
        self.holdout = Some(h);
        self.hud(ui, combat, game, prompt);
        self.looting(ui, cx, game, combat, icons, dt, loot::Aimed::Nothing);
    }
}
