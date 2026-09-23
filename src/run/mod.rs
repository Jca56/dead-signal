//! A run, from the player's side: the keys turned into movement, shots and
//! healing; what's carried and found (`loot.rs`); health and stamina; the
//! count of what happened; and dying, when it comes to that.

mod loot;
mod out;

use bevy_ecs::entity::Entity;
use lntrn_math::{Vec2, Vec3};
use lntrn_ui::{AreaCx, Key, ShellRequest, Ui};

use crate::bag_ui::{BagUi, Icons};
use crate::combat::Combat;
use crate::ending::{After, Ending, Outcome};
use crate::exits::{self, Way};
use crate::hud::{self, Hud};
use crate::loot::{Dice, Kind};
use crate::loot::bag::Bag;
use crate::profile::perks::Perks;
use crate::sound::Sfx;
use crate::stats::Stats;
use crate::vitals::{Kit, LOW_HP, Vitals};
use crate::weapon::Trigger;
use crate::world::Game;
use crate::zombie::director::Director;

/// How much a blow takes.
const BLOW_DAMAGE: f64 = 20.0;
/// Seconds between heartbeats when badly hurt.
const HEARTBEAT: f64 = 1.1;
/// How fast the player walks with the bag open, as a share.
const RUMMAGING_PACE: f64 = 0.5;

#[derive(Default)]
pub struct Run {
    pub vitals: Vitals,
    pub stats: Stats,
    pub ending: Option<Ending>,
    pub bag: Bag,
    heartbeat: f64,
    /// Where the player was last frame, for distance walked.
    last: Option<Vec3>,
    director: Director,
    bag_ui: BagUi,
    /// The inventory screen, when it's up: with what's being searched, if
    /// anything.
    open: Option<Open>,
    /// A container being searched, and for how long.
    search: Option<loot::Search>,
    /// A word flashed under the middle ("NO ROOM"), and how long it stays.
    note: Option<(&'static str, f64)>,
    /// Luck, for where things thrown down land.
    dice: Dice,
    /// Finding and working the ways out.
    out: out::Out,
    /// The XP there was before this run, and how it ended once it has
    /// (for the profile to settle, once).
    xp_before: u32,
    result: Option<(bool, u32)>,
    /// The perks the player came in with.
    perks: Perks,
}

#[derive(Clone, Copy, Debug)]
struct Open {
    container: Option<Entity>,
}

impl Run {
    /// A fresh run: whole, nothing counted, things lying in their spots and
    /// in their containers, the first of the dead already out there.
    pub fn start(&mut self, game: &mut Game, combat: &mut Combat, loadout: Bag, xp_before: u32, perks: Perks) {
        *self = Self::default();
        self.bag = loadout;
        self.xp_before = xp_before;
        // What the perks make of the player.
        self.perks = perks;
        self.vitals = Vitals::with(&perks);
        combat.pistol.reload_speed = perks.reload_speed();
        combat.melee = perks.melee();
        game.world.insert_resource(crate::zombie::Stealth(perks.seen_from()));
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.subsec_nanos());
        crate::items::scatter(&mut game.world, seed);
        crate::containers::fill(&mut game.world, seed.rotate_left(13));
        self.dice = Dice(seed.rotate_left(7) | 1);
        self.begin_out(game, seed.rotate_left(21));
        if let Some((eye, forward)) = Self::watching(game) {
            self.director.begin(&mut game.world, eye, forward);
        }
    }

    /// Where the player's eye is and which way it looks, flat.
    fn watching(game: &mut Game) -> Option<(Vec3, Vec3)> {
        let (body, view) = game.player()?;
        Some((body.pos + Vec3::new(0.0, 1.6, 0.0), Vec3::new(-view.yaw.sin(), 0.0, -view.yaw.cos())))
    }

    /// How the run ended, once (got out or not, the XP banked), and what
    /// was carried then.
    pub fn take_result(&mut self) -> Option<(bool, Bag, u32)> {
        let (got_out, xp) = self.result.take()?;
        Some((got_out, self.bag.clone(), xp))
    }

    /// Walked out on (back to the title): as good as dead, but for the
    /// pockets. What was carried.
    pub fn abandon(&mut self) -> Bag {
        std::mem::take(&mut self.bag)
    }

    /// How far the gun is lowered (patching up, or rummaging), 0–1.
    pub fn lowered(&self) -> f64 {
        if self.vitals.healing.is_some() || self.open.is_some() { 1.0 } else { 0.0 }
    }

    /// Whether the pointer should be locked for looking about (not with the
    /// inventory up).
    pub fn wants_lock(&self) -> bool {
        self.open.is_none()
    }

    /// A frame of a run while alive: what the keys do, and what came of it.
    pub fn play(&mut self, ui: &mut Ui, cx: &mut AreaCx<()>, game: &mut Game, combat: &mut Combat, locked: bool, icons: &Icons) {
        let dt = game.clock().dt;
        let open = self.open.is_some();
        if open {
            // The pointer is the inventory's.
        } else if locked {
            let motion = ui.state.locked_motion;
            if let Some(mut view) = game.player_view_mut() {
                view.look(motion);
            }
        } else if ui.state.pressed {
            // The lock was refused or lost: a click takes it back.
            cx.request(ShellRequest::LockPointer(true));
        }
        let held = |ui: &Ui, keys: &[char], other: Key| ui.state.keys_down.iter().any(|k| *k == other || matches!(k, Key::Char(c) if keys.iter().any(|w| c.eq_ignore_ascii_case(w))));
        let pressed = |ui: &mut Ui, keys: &[char]| ui.state.take_key(|k| !k.repeat && matches!(k.key, Key::Char(c) if keys.iter().any(|w| c.eq_ignore_ascii_case(w)))).is_some();
        let axis = |neg: bool, pos: bool| f64::from(i8::from(pos) - i8::from(neg));
        let walk = Vec2::new(axis(held(ui, &['a'], Key::ArrowLeft), held(ui, &['d'], Key::ArrowRight)), axis(held(ui, &['s'], Key::ArrowDown), held(ui, &['w'], Key::ArrowUp)));
        let wants_sprint = ui.state.keys_down.contains(&Key::Shift) && walk.y > 0.0;
        let sprint = wants_sprint && combat.sprint_allowed() && self.vitals.can_sprint();
        let jump = ui.state.take_key(|k| !k.repeat && matches!(k.key, Key::Space | Key::Char(' '))).is_some();
        // Patching up takes both hands and standing still; rummaging, a
        // slow walk at most.
        let (walk, sprint, jump) = if self.vitals.healing.is_some() {
            (Vec2::ZERO, false, false)
        } else if open {
            (walk * RUMMAGING_PACE, false, false)
        } else {
            (walk, sprint, jump)
        };
        let crouch = pressed(ui, &['c']);
        {
            let mut controls = game.controls_mut();
            controls.walk = walk;
            controls.sprint = sprint;
            controls.jump |= jump;
            controls.crouch_toggle ^= crouch;
        }

        // Patching up: 3 a bandage, 4 a medkit, from what's carried. Firing,
        // striking or a blow stops it (the kit is kept).
        for (key, kit) in [('3', Kit::Bandage), ('4', Kit::Medkit)] {
            if pressed(ui, &[key]) && self.vitals.start_heal(kit, self.bag.count(kit.kind())) {
                combat.play(Sfx::Heal, 0.8);
            }
        }
        let firing = locked && !open && ui.state.pressed;
        let striking = !open && (ui.state.middle_pressed || pressed(ui, &['v']));
        if self.vitals.healing.is_some() && (firing || striking) {
            self.vitals.interrupt();
        }
        let trigger = if self.vitals.healing.is_some() || open {
            Trigger::default()
        } else {
            Trigger { fire: firing, reload: pressed(ui, &['r']), melee: striking }
        };
        // Reloading draws on the rounds carried.
        combat.pistol.spare = self.bag.count(Kind::Rounds);
        let spare = combat.pistol.spare;
        combat.frame(game, trigger, dt, &mut self.stats);
        self.bag.remove(Kind::Rounds, spare - combat.pistol.spare);

        // Blows from the dead.
        let blows = combat.answer_the_dead(game, true);
        for _ in 0..blows {
            self.stats.times_hit += 1;
            self.stats.damage_taken += BLOW_DAMAGE.min(self.vitals.hp);
            if self.vitals.hurt(BLOW_DAMAGE) {
                let side = if self.stats.times_hit.is_multiple_of(2) { 1.0 } else { -1.0 };
                self.end(game, Outcome::Died(side), combat);
                return;
            }
        }

        // More of the dead, as the kills mount (and all of them, surging).
        if let Some((eye, forward)) = Self::watching(game) {
            self.director.surge = self.out.surging;
            self.director.update(&mut game.world, self.stats.kills(), eye, forward, dt);
            self.stats.biggest_horde = self.director.peak as u32;
        }

        // Health, stamina, the kit being applied, the count.
        let sprinting = game.player().is_some_and(|(b, _)| b.sprinting && b.speed_flat() > 0.5);
        let change = self.vitals.update(dt, sprinting);
        if let Some((kit, healed)) = change.healed {
            self.bag.remove(kit.kind(), 1);
            self.stats.healed += healed;
            match kit {
                Kit::Bandage => self.stats.bandages_used += 1,
                Kit::Medkit => self.stats.medkits_used += 1,
            }
        }
        self.stats.healed += change.regenerated;
        self.stats.seconds += dt;
        if let Some((body, _)) = game.player() {
            if let Some(last) = self.last {
                let d = Vec2::new(body.pos.x - last.x, body.pos.z - last.z).length();
                if body.sprinting { self.stats.sprinted += d } else { self.stats.walked += d }
            }
            self.last = Some(body.pos);
            self.stats.jumps = body.jumps;
        }
        if self.vitals.hp < LOW_HP {
            self.heartbeat -= dt;
            if self.heartbeat <= 0.0 {
                combat.play(Sfx::Heartbeat, 0.9);
                self.heartbeat = HEARTBEAT;
            }
        }
        if let Some((_, t)) = &mut self.note {
            *t -= dt;
            if *t <= 0.0 {
                self.note = None;
            }
        }
        // Looking about for things, searching, the bag; the ways out.
        let aimed = self.aim(game);
        let at_exit = if let loot::Aimed::Exit(i) = aimed { Some(i) } else { None };
        if let Some(way) = self.getting_out(ui, game, combat, at_exit.filter(|_| self.open.is_none()), dt) {
            self.end(game, Outcome::Extracted(way), combat);
            return;
        }
        let prompt = if self.open.is_some() { None } else { self.prompt(game, &aimed) };
        self.hud(ui, combat, game, prompt);
        self.looting(ui, cx, game, combat, icons, dt, aimed);
    }

    /// The run is over: dead, or out.
    fn end(&mut self, game: &mut Game, outcome: Outcome, combat: &mut Combat) {
        if let Some(open) = self.open.take() {
            self.close_bag(game, open);
        }
        self.stats.loot_value = self.bag.value();
        match outcome {
            Outcome::Extracted(Way::Truck) => combat.play(Sfx::Engine, 1.0),
            Outcome::Extracted(Way::Radio) => combat.play(Sfx::Rotor, 1.0),
            _ => {}
        }
        let earned = crate::profile::xp::earned(&self.stats, outcome);
        self.result = Some((matches!(outcome, Outcome::Extracted(_)), earned.banked()));
        self.ending = Some(Ending::new(outcome, self.stats.clone(), &self.bag, earned, self.xp_before));
        *game.controls_mut() = Default::default();
    }

    fn hud(&self, ui: &mut Ui, combat: &Combat, game: &mut Game, prompt: Option<(&'static str, String)>) {
        let time = game.clock().time;
        let v = &self.vitals;
        hud::draw(
            ui,
            &Hud {
                mag: combat.pistol.mag,
                spare: self.bag.count(Kind::Rounds),
                marker: combat.fx.marker,
                hurt: combat.hurt,
                hp: v.hp,
                max_hp: v.max_hp,
                stamina: v.stamina,
                max_stamina: v.max_stamina,
                winded: v.winded,
                bandages: self.bag.count(Kind::Bandage),
                medkits: self.bag.count(Kind::Medkit),
                heal: v.heal_progress().or(self.search.as_ref().map(loot::Search::progress)).or(self.out_progress()),
                prompt: prompt.as_ref().map(|(key, text)| (*key, text.as_str())),
                note: self.note.map(|(n, _)| n),
                time,
            },
        );
        let o = self.out_hud(game);
        exits::hud::draw(
            ui,
            &exits::hud::Compass { heading: o.heading, marks: &o.marks, clock: self.stats.seconds, surging: self.out.surging, under: o.under, chatter: o.chatter, shout: self.out.shout.map(|(w, t)| (w, t.min(1.0))) },
        );
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
