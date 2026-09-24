//! A run, from the player's side: the keys turned into movement, shots and
//! healing; what's in hand (`hands.rs`); what's carried and found
//! (`loot.rs`); health and stamina; the count of what happened; and dying,
//! when it comes to that.

mod hands;
mod loot;
mod out;
mod throwing;

use bevy_ecs::entity::Entity;
use lntrn_math::{Vec2, Vec3};
use lntrn_ui::{AreaCx, ShellRequest, Ui};

use crate::bag_ui::{BagUi, Icons};
use crate::combat::Combat;
use crate::settings::Settings;
use crate::settings::keys::{Action, Keys};
use crate::ending::{After, Ending, Outcome};
use crate::exits::{self, Way};
use crate::hud::{self, Hud};
use crate::loot::{Dice, Kind};
use crate::loot::bag::Bag;
use crate::map::Map;
use crate::profile::perks::Perks;
use crate::sound::Sfx;
use crate::stats::Stats;
use crate::vitals::{Kit, LOW_HP, Vitals};
use crate::weapon::Trigger;
use crate::world::Game;
use crate::zombie::director::Director;

/// How much a blow takes.
/// Seconds between heartbeats when badly hurt.
const HEARTBEAT: f64 = 1.1;
/// How fast the player walks with the bag open, and down the sights, as
/// a share.
const RUMMAGING_PACE: f64 = 0.5;
const AIMING_PACE: f64 = 0.6;
/// How fast a swing goes winded, as a share.
const WINDED_SWING: f64 = 0.75;

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
    /// The wheel's turn not yet stepped through the slots, pixels.
    wheel: f64,
    /// The player's keys, this frame (`settings::keys`), and whether a
    /// sprint's been toggled on (sprint set to toggle).
    keys: Keys,
    sprinting: bool,
    /// Till a heavy sprint's next footfall is heard.
    footfall_in: f64,
    /// The throwable picked, and a throw being aimed.
    throwable: Option<crate::throw::Throwable>,
    aiming: Option<throwing::Aiming>,
}

#[derive(Clone, Copy, Debug)]
struct Open {
    container: Option<Entity>,
}

impl Run {
    /// A fresh run: whole, nothing counted, things lying in their spots and
    /// in their containers, the first of the dead already out there.
    pub fn start(&mut self, game: &mut Game, combat: &mut Combat, loadout: Bag, xp_before: u32, perks: Perks, map: &Map) {
        *self = Self::default();
        self.bag = loadout;
        self.xp_before = xp_before;
        // What the perks make of the player.
        self.perks = perks;
        self.vitals = Vitals::with(&perks);
        combat.hands.reload_speed = perks.reload_speed();
        combat.melee = perks.melee();
        self.take_up(combat, self.first_armed());
        game.world.insert_resource(crate::zombie::Stealth(perks.seen_from()));
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.subsec_nanos());
        crate::items::scatter(&mut game.world, seed, &map.pickups);
        crate::containers::fill(&mut game.world, seed.rotate_left(13));
        self.dice = Dice(seed.rotate_left(7) | 1);
        self.begin_out(game, seed.rotate_left(21), map.truck_near());
        if let Some((eye, _)) = Self::watching(game) {
            self.director.begin(&mut game.world, &map.sites, &|x, z| map.field.height_at(x, z), eye, seed.rotate_left(3));
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
        if self.vitals.healing.is_some() || self.open.is_some() || self.throw_arc().is_some() { 1.0 } else { 0.0 }
    }

    /// Whether the pointer should be locked for looking about (not with the
    /// inventory up).
    pub fn wants_lock(&self) -> bool {
        self.open.is_none()
    }

    /// The dev's: the dead surge now.
    pub fn dev_surge(&mut self) {
        self.out.surging = true;
    }

    /// The dev's: whole again, the bleeding and poison gone.
    pub fn dev_heal(&mut self) {
        self.vitals.hp = self.vitals.max_hp;
        self.vitals.bleeding = 0;
        self.vitals.poison = 0.0;
    }

    /// A frame of a run while alive: what the keys do, and what came of it.
    pub fn play(&mut self, ui: &mut Ui, cx: &mut AreaCx<()>, game: &mut Game, combat: &mut Combat, locked: bool, icons: &Icons) {
        let dt = game.clock().dt;
        let open = self.open.is_some();
        let (feel, keys, toggle_crouch, toggle_sprint) = game.world.get_resource::<Settings>().map_or((Default::default(), Keys::default(), true, false), |s| (s.feel(), s.keys, s.toggle_crouch, s.toggle_sprint));
        self.keys = keys;
        self.bag_ui.slot_keys = keys.slot_names();
        // What's worn weighs on the sprint, the breath and the feet; the
        // armor worn has room for a plate or it hasn't.
        let (fast, breath, _) = crate::loot::gear::burden(self.bag.weight());
        game.world.insert_resource(crate::player::Load(fast));
        self.vitals.breath = breath;
        let (armor, most) = self.bag.armor();
        self.vitals.armor_room = most - armor;
        if let Some(mut view) = game.player_view_mut() {
            view.feel = feel;
        }
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
        let axis = |neg: bool, pos: bool| f64::from(i8::from(pos) - i8::from(neg));
        let walk = Vec2::new(axis(keys.held(ui, Action::Left), keys.held(ui, Action::Right)), axis(keys.held(ui, Action::Back), keys.held(ui, Action::Forward)));
        // Sprinting, held or toggled (a toggled sprint ends when running
        // forward does).
        let sprint_key = if toggle_sprint {
            if keys.pressed(ui, Action::Sprint) {
                self.sprinting = !self.sprinting;
            }
            self.sprinting
        } else {
            keys.held(ui, Action::Sprint)
        };
        if walk.y <= 0.0 {
            self.sprinting = false;
        }
        let wants_sprint = sprint_key && walk.y > 0.0;
        let sprint = wants_sprint && combat.sprint_allowed() && self.vitals.can_sprint();
        let jump = keys.pressed(ui, Action::Jump);
        // Patching up takes both hands and standing still; rummaging, a
        // slow walk at most.
        let (walk, sprint, jump) = if self.vitals.healing.is_some() {
            (Vec2::ZERO, false, false)
        } else if open {
            (walk * RUMMAGING_PACE, false, false)
        } else {
            (walk * (1.0 - (1.0 - AIMING_PACE) * combat.hands.aim()), sprint, jump)
        };
        // Crouching, toggled or held: a hold flips it whenever the key and
        // the body disagree.
        let crouch = if toggle_crouch {
            keys.pressed(ui, Action::Crouch)
        } else {
            let crouched = game.player().is_some_and(|(b, _)| b.want_crouch);
            keys.held(ui, Action::Crouch) != crouched
        };
        {
            let mut controls = game.controls_mut();
            controls.walk = walk;
            controls.sprint = sprint;
            controls.jump |= jump;
            // (A hold says what it wants each frame; a tap flips it, once a
            // tap, however many frames before the body steps.)
            if toggle_crouch {
                controls.crouch_toggle ^= crouch;
            } else {
                controls.crouch_toggle = crouch;
            }
        }

        // Patching up: 4 a bandage, 5 a medkit, from what's carried. Firing,
        // striking or a blow stops it (the kit is kept).
        for (key, kit) in [(Action::Bandage, Kit::Bandage), (Action::Medkit, Kit::Medkit), (Action::Plate, Kit::Plate)] {
            if keys.pressed(ui, key) && self.vitals.start_heal(kit, self.bag.count(kit.kind())) {
                combat.play(Sfx::Heal, 0.8);
            }
        }
        let firing = locked && !open && keys.pressed(ui, Action::Fire);
        let holding = locked && !open && keys.held(ui, Action::Fire);
        let striking = !open && keys.pressed(ui, Action::Bash);
        if self.vitals.healing.is_some() && (firing || striking) {
            self.vitals.interrupt();
        }
        let busy = self.vitals.healing.is_some() || open;
        // A throw being aimed puts the gun down.
        let busy = self.throwing(ui, game, combat, !busy && locked) || busy;
        if !busy && keys.pressed(ui, Action::FireMode) && combat.hands.switch_fire() {
            combat.play(Sfx::Tick, 0.9);
        }
        self.switch_hands(ui, combat, !busy);
        // The sights up while the right button's held; a sprint takes them
        // down.
        let aim = locked && keys.held(ui, Action::Aim) && !wants_sprint;
        let trigger = if busy { Trigger::default() } else { Trigger { fire: firing, hold: holding, reload: keys.pressed(ui, Action::Reload), melee: striking, aim } };
        self.pull_rounds(combat);
        // Reloading draws on the rounds carried, of the kind the gun takes.
        let ammo = combat.hands.spec().ammo;
        combat.hands.spare = ammo.map_or(0, |kind| self.bag.count(kind));
        let spare = combat.hands.spare;
        // Swings spend stamina; winded, they come slower.
        combat.hands.swing_speed = if self.vitals.winded { WINDED_SWING } else { 1.0 };
        let spent = combat.frame(game, trigger, dt, &mut self.stats);
        self.vitals.spend(spent);
        if let Some(kind) = ammo {
            self.bag.remove(kind, spare - combat.hands.spare);
        }
        // (The dev's: a magazine that never runs down.)
        if game.world.get_resource::<crate::dev::Cheats>().is_some_and(|c| c.ammo) {
            combat.hands.mag = combat.hands.spec().mag;
        }
        self.keep_rounds(combat);

        // Blows from the dead.
        let mut blows = combat.answer_the_dead(game, true);
        blows.extend(combat.bursts(game, &mut self.stats));
        // Standing in a Spitter's bile.
        if std::mem::take(&mut game.world.resource_mut::<crate::zombie::Horde>().poisoned) {
            self.vitals.afflict(crate::vitals::Affliction::Poison);
        }
        let cheats = game.world.get_resource::<crate::dev::Cheats>().copied().unwrap_or_default();
        if cheats.god {
            self.dev_heal();
        }
        if self.booms(game, combat, cheats.god) {
            self.end(game, Outcome::Died(1.0), combat);
            return;
        }
        for blow in blows {
            if cheats.god {
                continue;
            }
            self.stats.times_hit += 1;
            // Armor takes it first; while there's any, claws don't cut.
            let (armored, _) = self.bag.armor();
            let damage = (blow.damage - f64::from(self.bag.soak(blow.damage.round() as u32))).max(0.0);
            self.stats.damage_taken += damage.min(self.vitals.hp);
            if let Some(a) = blow.leaves
                && !(a == crate::vitals::Affliction::Bleed && armored > 0)
            {
                self.vitals.afflict(a);
            }
            if self.vitals.hurt(damage) {
                let side = if self.stats.times_hit.is_multiple_of(2) { 1.0 } else { -1.0 };
                self.end(game, Outcome::Died(side), combat);
                return;
            }
        }

        // More of the dead, as the kills mount (and all of them, surging).
        if let Some((eye, forward)) = Self::watching(game) {
            self.director.surge = self.out.surging;
            self.director.update(&mut game.world, eye, forward, dt);
            self.stats.biggest_horde = self.director.peak as u32;
        }

        // Health, stamina, the kit being applied, the count.
        let sprinting = game.player().is_some_and(|(b, _)| b.sprinting && b.speed_flat() > 0.5);
        let change = self.vitals.update(dt, sprinting);
        // Heavy gear, sprinting: footfalls the dead near hear.
        let (_, _, heard) = crate::loot::gear::burden(self.bag.weight());
        self.footfall_in -= dt;
        if sprinting
            && heard > 0.0
            && self.footfall_in <= 0.0
            && let Some((body, _)) = game.player()
        {
            self.footfall_in = 0.35;
            crate::zombie::footfall(&mut game.world, body.pos, heard);
        }
        if let Some((kit, healed)) = change.healed {
            self.bag.remove(kit.kind(), 1);
            self.stats.healed += healed;
            match kit {
                Kit::Bandage => self.stats.bandages_used += 1,
                Kit::Medkit => self.stats.medkits_used += 1,
                Kit::Plate => {
                    self.bag.mend(crate::loot::gear::PLATE);
                }
            }
        }
        self.stats.healed += change.regenerated;
        self.stats.damage_taken += change.festered;
        if self.vitals.dead() {
            // Bled out, or the poison did it.
            self.end(game, Outcome::Died(1.0), combat);
            return;
        }
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
        let interact = self.keys.name(Action::Interact);
        // The kits, and the throwable picked, each with its key.
        let mut kits = vec![(self.keys.name(Action::Medkit), "MEDKIT", self.bag.count(Kind::Medkit)), (self.keys.name(Action::Bandage), "BANDAGE", self.bag.count(Kind::Bandage))];
        if self.bag.count(Kind::ArmorPlate) > 0 {
            kits.insert(0, (self.keys.name(Action::Plate), "ARMOR PLATE", self.bag.count(Kind::ArmorPlate)));
        }
        if let Some((what, n)) = self.picked() {
            kits.insert(0, (self.keys.name(Action::Throw), what.kind().def().name, n));
        }
        // A gun that switches says how it's set.
        let hands = &combat.hands;
        let weapon = match hands.spec().shot {
            Some(s) if s.select => format!("{}  ·  {}", hands.spec().name, if hands.full_auto() { "AUTO" } else { "SEMI" }),
            _ => hands.spec().name.to_string(),
        };
        hud::draw(
            ui,
            &Hud {
                weapon: &weapon,
                rounds: combat.hands.spec().ammo.map(|kind| (combat.hands.mag, self.bag.count(kind))),
                aim: combat.hands.aim(),
                scope: combat.hands.scoped(),
                marker: combat.fx.marker,
                hurt: combat.hurt,
                hp: v.hp,
                max_hp: v.max_hp,
                stamina: v.stamina,
                max_stamina: v.max_stamina,
                winded: v.winded,
                kits: &kits,
                heal: v.heal_progress().or(self.search.as_ref().map(loot::Search::progress)).or(self.out_progress()),
                prompt: prompt.as_ref().map(|(key, text)| (if key.is_empty() { "" } else { interact.as_str() }, text.as_str())),
                note: self.note.map(|(n, _)| n),
                time,
                crosshair: game.world.get_resource::<Settings>().is_none_or(|s| s.crosshair),
                bleeding: v.bleeding,
                poison: v.poison,
                armor: self.bag.armor(),
            },
        );
        let o = self.out_hud(game);
        exits::hud::draw(
            ui,
            &exits::hud::Compass { heading: o.heading, marks: &o.marks, clock: self.stats.seconds, surging: self.out.surging, under: o.under, chatter: o.chatter.as_ref().map(|(w, a)| (w.as_str(), *a)), shout: self.out.shout.map(|(w, t)| (w, t.min(1.0))) },
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
