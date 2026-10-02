//! A run: each player's side of it (`seat.rs`: their keys turned into
//! movement, shots and healing, their health and stamina, the count of
//! what happened); what's in hand (`hands.rs`); what's carried and found
//! (`loot.rs`); going down and being picked up, playing together
//! (`down.rs`); the radio they pull out (`radio.rs`) and what it calls
//! down (`support.rs`); and what the players
//! share: the dead brought in, the ways
//! out (`out.rs`) or a holdout's rounds (`holdout.rs`), and the end, when
//! it comes to that.

mod down;
mod hands;
mod holdout;
mod loot;
mod out;
mod radio;
mod seat;
mod stims;
mod support;
mod throwing;

use lntrn_math::{Rect, Vec3};
use lntrn_ui::{AreaCx, ShellRequest, Ui};

use crate::bag_ui::Icons;
use crate::combat::Combat;
use crate::ending::{After, Ending, Outcome};
use crate::input::Feed;
use crate::exits::{self, Way};
use crate::loot::Dice;
use crate::loot::bag::Bag;
use crate::map::Map;
use crate::profile::perks::Perks;
use crate::settings::Settings;
use crate::sound::Sfx;
use crate::world::Game;
use crate::zombie::director::Director;
use seat::Seat;
#[cfg(test)]
pub use support::MEDIC;

#[derive(Default)]
pub struct Run {
    /// Each player's side of it, by seat.
    pub seats: Vec<Seat>,
    pub ending: Option<Ending>,
    director: Director,
    /// Luck, for what the radio says.
    dice: Dice,
    /// Finding and working the ways out.
    out: out::Out,
    /// The XP there was before this run, and how it ended once it has
    /// (for the profile to settle, once).
    xp_before: u32,
    result: Option<(bool, u32)>,
    /// A holdout's, when this run is one (no loot, no way out); the best
    /// round before it, and the round it ended on (for the profile to
    /// keep, once).
    pub holdout: Option<crate::holdout::Holdout>,
    best_round: u32,
    holdout_over: Option<u32>,
}

impl Run {
    /// A fresh run, the player's own: whole, nothing counted, things lying
    /// in their spots and in their containers, the first of the dead
    /// already out there.
    pub fn start(&mut self, game: &mut Game, combat: &mut Combat, loadout: Bag, xp_before: u32, perks: Perks, map: &Map) {
        *self = Self::default();
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.subsec_nanos());
        combat.reset(1);
        self.seats = vec![Seat::new(0, loadout, perks, seed, combat)];
        self.xp_before = xp_before;
        game.world.insert_resource(crate::zombie::Stealth(perks.seen_from()));
        game.world.insert_resource(crate::items::Ground::default());
        crate::items::scatter(&mut game.world, seed, &map.pickups);
        crate::containers::fill(&mut game.world, seed.rotate_left(13));
        self.dice = Dice(seed.rotate_left(7) | 1);
        self.begin_out(game, seed.rotate_left(21), map);
        if let Some((eye, _)) = Self::watching(game, 0) {
            self.director.begin(&mut game.world, &map.sites, &map.landmarks, &|x, z| map.field.height_at(x, z), eye, seed.rotate_left(3));
        }
    }

    /// Where player `seat`'s eye is and which way it looks, flat.
    fn watching(game: &mut Game, seat: usize) -> Option<(Vec3, Vec3)> {
        let (body, view) = game.player(seat)?;
        Some((body.pos + Vec3::new(0.0, 1.6, 0.0), Vec3::new(-view.yaw.sin(), 0.0, -view.yaw.cos())))
    }

    /// How the run ended, once (got out or not, the XP banked), and what
    /// was carried then.
    pub fn take_result(&mut self) -> Option<(bool, Bag, u32)> {
        let (got_out, xp) = self.result.take()?;
        Some((got_out, self.seats.first().map(|s| s.bag.clone()).unwrap_or_default(), xp))
    }

    /// Walked out on (back to the title): as good as dead, but for the
    /// pockets. What was carried.
    pub fn abandon(&mut self) -> Bag {
        self.seats.first_mut().map(|s| std::mem::take(&mut s.bag)).unwrap_or_default()
    }

    /// Whether the pointer should be locked for looking about (whoever
    /// has the mouse hasn't their inventory up).
    pub fn wants_lock(&self) -> bool {
        self.seats.iter().all(Seat::wants_lock)
    }

    /// What each player's doing, for their figure (`now`: the game's
    /// clock).
    pub fn doing<'a>(&'a self, combat: &'a Combat, now: f64) -> Vec<crate::survivor::Doing<'a>> {
        self.seats
            .iter()
            .map(|s| crate::survivor::Doing {
                seat: s.n,
                hands: &combat.arms[s.n].hands,
                winding: s.winding(),
                threw: s.threw.map(|(what, at)| (what, now - at)),
                lowered: s.lowered(),
                reviving: s.reviving.is_some_and(|(_, p)| p > 0.0),
                down: s.down.is_some(),
                out: s.out,
                radio: s.radio_shown(),
            })
            .collect()
    }

    /// The dev's: the dead surge now.
    pub fn dev_surge(&mut self) {
        self.out.surging = true;
    }

    /// The dev's: everyone whole again, the bleeding and poison gone.
    pub fn dev_heal(&mut self) {
        for seat in &mut self.seats {
            seat.dev_heal();
        }
    }

    /// Put any inventory screen away (whatever is held goes back). Whether
    /// one was up.
    pub fn shut_bags(&mut self, game: &mut Game, cx: &mut AreaCx<()>) -> bool {
        let mut any = false;
        for seat in &mut self.seats {
            any |= seat.shut_bag(game, cx);
        }
        any
    }

    /// A frame of a run while anyone's standing: what each one's controls
    /// do (what their hands are on: `feeds`, by seat), what the dead do to
    /// them, and what came of it, over each one's part of the window
    /// (`panes`).
    #[allow(clippy::too_many_arguments)]
    pub fn play(&mut self, ui: &mut Ui, cx: &mut AreaCx<()>, game: &mut Game, combat: &mut Combat, locked: bool, icons: &Icons, feeds: &[Feed], panes: &[Rect]) {
        let dt = game.clock().dt;
        let keys = game.world.get_resource::<Settings>().map_or_else(Default::default, |s| s.keys);
        for seat in &mut self.seats {
            let feed = feeds.get(seat.n).copied().unwrap_or_default();
            seat.input.update(ui, feed.keys.then_some(keys), locked, feed.pad, dt);
        }
        combat.update(dt);
        for seat in &mut self.seats {
            // (Their points lent them for it: the radio may spend some.)
            let mut wallet = self.holdout.as_mut().and_then(|h| h.wallets.get_mut(seat.n));
            seat.points = wallet.as_ref().map(|w| w.points);
            seat.act(ui, cx, game, combat, locked, dt);
            if let (Some(w), Some(left)) = (&mut wallet, seat.points) {
                w.points = left;
            }
        }

        // Blows from the dead, the bile stood in, and what the fires and
        // blasts did (a Spitter's burst's kills are whoever killed it's).
        let mut blows = combat.answer_the_dead(game, true);
        let (burst, burst_kills) = combat.bursts(game);
        blows.extend(burst);
        for by in burst_kills {
            if let Some(seat) = self.seats.iter_mut().find(|s| s.n == by) {
                seat.stats.burst_kills += 1;
            }
        }
        let poisoned = std::mem::take(&mut game.world.resource_mut::<crate::zombie::Horde>().poisoned);
        let god = game.world.get_resource::<crate::dev::Cheats>().is_some_and(|c| c.god);
        let booms = self.booms(game, combat);
        for i in 0..self.seats.len() {
            let n = self.seats[i].n;
            let theirs = blows.iter().filter(|(s, _)| *s == n).map(|(_, b)| *b);
            let died = self.seats[i].suffer(game, combat, theirs, poisoned.contains(&n), booms.of(n), god);
            if let Some(how) = died
                && self.fall(game, combat, i, how)
            {
                return;
            }
        }

        // More of the dead, as the kills mount (and all of them, surging:
        // about the first player, a run with the ways out being theirs
        // alone); in a holdout, round after round of them.
        if self.holdout.is_some() {
            self.holdout_step(game, combat, dt);
        } else if let Some((eye, forward)) = Self::watching(game, 0) {
            self.director.surge = self.out.surging;
            self.director.update(&mut game.world, eye, forward, dt);
            self.seats[0].stats.biggest_horde = self.director.peak as u32;
        }

        for i in 0..self.seats.len() {
            if self.seats[i].live(game, combat, dt) && self.fall(game, combat, i, Outcome::Died(1.0)) {
                return;
            }
        }
        if !self.revive_and_bleed(ui, game, combat, dt) {
            self.end(game, Outcome::Died(1.0), combat);
            return;
        }
        if self.holdout.is_some() {
            self.holdout_frame(ui, cx, game, combat, icons, dt, panes);
            return;
        }
        // Looking about for things, searching, the bag; the ways out. (A
        // run with the ways out is the first player's alone.)
        let seat = &mut self.seats[0];
        let aimed = seat.aim(game);
        let at_exit = if let loot::Aimed::Exit(i) = aimed { Some(i) } else { None };
        let at_exit = at_exit.filter(|_| seat.open.is_none());
        if let Some(way) = self.out.getting_out(seat, ui, game, combat, at_exit, dt) {
            self.end(game, Outcome::Extracted(way), combat);
            return;
        }
        let prompt = if seat.open.is_some() { None } else { seat.prompt(game, &aimed) };
        seat.input.set_prompt(match &prompt {
            Some((key, _)) if !key.is_empty() => seat.asks(game, &aimed),
            _ => crate::input::Prompt::None,
        });
        let whole = ui.clip();
        seat.hud(ui, whole, combat, game, prompt, self.out.progress());
        let o = self.out.hud(game, 0);
        exits::hud::draw(
            ui,
            &exits::hud::Compass { heading: o.heading, marks: &o.marks, clock: seat.stats.seconds, surging: self.out.surging, under: o.under, chatter: o.chatter.as_ref().map(|(w, a)| (w.as_str(), *a)), shout: self.out.shout.map(|(w, t)| (w, t.min(1.0))) },
        );
        seat.looting(ui, cx, game, combat, icons, dt, aimed);
    }

    /// The run is over: dead, or out. (A holdout played together shows
    /// everyone's numbers; else the end is the first player's.)
    fn end(&mut self, game: &mut Game, outcome: Outcome, combat: &mut Combat) {
        for seat in &mut self.seats {
            if let Some(open) = seat.open.take() {
                seat.close_bag(game, open);
            }
        }
        game.release_controls();
        if let Some(h) = &self.holdout {
            let round = h.rounds.round;
            self.holdout_over = Some(round);
            let players = self.seats.iter().map(|s| (s.stats.clone(), h.wallet(s.n).map_or(0, |w| w.earned))).collect();
            self.ending = Some(Ending::holdout(players, round, self.best_round));
            return;
        }
        let first = &mut self.seats[0];
        first.stats.loot_value = first.bag.value();
        match outcome {
            Outcome::Extracted(Way::Truck) => combat.play(Sfx::Engine, 1.0),
            Outcome::Extracted(Way::Radio) => combat.play(Sfx::Rotor, 1.0),
            _ => {}
        }
        let earned = crate::profile::xp::earned(&first.stats, outcome);
        self.result = Some((matches!(outcome, Outcome::Extracted(_)), earned.banked()));
        self.ending = Some(Ending::new(outcome, first.stats.clone(), &first.bag, earned, self.xp_before));
    }

    /// A frame of the end: the fall (if it was death), the words, the
    /// numbers. What the player chose, once they have.
    pub fn ending(&mut self, ui: &mut Ui, cx: &mut AreaCx<()>, game: &mut Game, combat: &mut Combat, active: bool, icons: &Icons) -> Option<After> {
        let dt = game.clock().dt;
        let ending = self.ending.as_mut()?;
        ending.update(dt);
        if ending.words_begin(dt) {
            combat.play(if matches!(ending.outcome, Outcome::Died(_)) { Sfx::Died } else { Sfx::Safe }, 1.0);
        }
        if ending.showing_stats() && ui.state.pointer_locked {
            cx.request(ShellRequest::LockPointer(false));
        }
        combat.answer_the_dead(game, false);
        ending.draw(ui, active, icons)
    }
}
