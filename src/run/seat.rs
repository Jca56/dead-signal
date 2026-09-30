//! One player's side of a run: their keys turned into looking, moving,
//! shots and healing; what the dead and the fires did to them; their
//! health, stamina and count; and their HUD. What's in their hands is in
//! `hands.rs`, what they carry and find in `loot.rs`, their throws in
//! `throwing.rs`.

use bevy_ecs::entity::Entity;
use lntrn_math::{Vec2, Vec3};
use lntrn_ui::{AreaCx, ShellRequest, Ui};

use super::loot;
use super::throwing::Aiming;
use crate::bag_ui::BagUi;
use crate::combat::Combat;
use crate::ending::Outcome;
use crate::hud::{self, Hud};
use crate::loot::bag::Bag;
use crate::loot::{Dice, Kind};
use crate::profile::perks::Perks;
use crate::settings::Settings;
use crate::settings::keys::{Action, Keys};
use crate::sound::Sfx;
use crate::stats::Stats;
use crate::throw::{Felt, Throwable};
use crate::vitals::{Affliction, Kit, LOW_HP, Vitals};
use crate::weapon::Trigger;
use crate::world::Game;
use crate::zombie::brain::Blow;

/// Seconds between heartbeats when badly hurt.
const HEARTBEAT: f64 = 1.1;
/// How fast a player walks with the bag open, and down the sights, as a
/// share.
const RUMMAGING_PACE: f64 = 0.5;
const AIMING_PACE: f64 = 0.6;
/// How fast a swing goes winded, as a share.
const WINDED_SWING: f64 = 0.75;

#[derive(Default)]
pub struct Seat {
    /// Which player: 0 is the first.
    pub n: usize,
    pub vitals: Vitals,
    pub stats: Stats,
    pub bag: Bag,
    heartbeat: f64,
    /// Where they were last frame, for distance walked.
    last: Option<Vec3>,
    pub(super) bag_ui: BagUi,
    /// The inventory screen, when it's up: with what's being searched, if
    /// anything.
    pub(super) open: Option<Open>,
    /// A container being searched, and for how long.
    pub(super) search: Option<loot::Search>,
    /// A word flashed under the middle ("NO ROOM"), and how long it stays.
    pub(super) note: Option<(&'static str, f64)>,
    /// Luck, for where things thrown down land.
    pub(super) dice: Dice,
    /// The perks they came in with.
    pub(super) perks: Perks,
    /// The wheel's turn not yet stepped through the slots, pixels.
    pub(super) wheel: f64,
    /// Their keys, this frame (`settings::keys`), and whether a sprint's
    /// been toggled on (sprint set to toggle).
    pub(super) keys: Keys,
    sprinting: bool,
    /// Till a heavy sprint's next footfall is heard.
    footfall_in: f64,
    /// The throwable picked, and a throw being aimed.
    pub(super) throwable: Option<Throwable>,
    pub(super) aiming: Option<Aiming>,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Open {
    pub container: Option<Entity>,
}

impl Seat {
    /// Player `n`, fresh: whole, carrying `bag`, with `perks`, and what's
    /// in the first slot taken up; their luck from `seed`.
    pub(super) fn new(n: usize, bag: Bag, perks: Perks, seed: u32, combat: &mut Combat) -> Self {
        let seat = Self { n, bag, perks, vitals: Vitals::with(&perks), dice: Dice(seed.rotate_left(9 + n as u32) | 1), ..Self::default() };
        let arms = &mut combat.arms[n];
        arms.hands.reload_speed = perks.reload_speed();
        arms.melee = perks.melee();
        seat.take_up(combat, seat.first_armed());
        seat
    }

    /// How far the gun is lowered (patching up, or rummaging), 0–1.
    pub fn lowered(&self) -> f64 {
        if self.vitals.healing.is_some() || self.open.is_some() || self.throw_arc().is_some() { 1.0 } else { 0.0 }
    }

    /// Whether the pointer should be locked for looking about (not with
    /// the inventory up).
    pub fn wants_lock(&self) -> bool {
        self.open.is_none()
    }

    /// The dev's: whole again, the bleeding and poison gone.
    pub fn dev_heal(&mut self) {
        self.vitals.hp = self.vitals.max_hp;
        self.vitals.bleeding = 0;
        self.vitals.poison = 0.0;
    }

    /// A frame of what the keys do: looking, moving, patching up, the
    /// hands and what they do.
    pub(super) fn act(&mut self, ui: &mut Ui, cx: &mut AreaCx<()>, game: &mut Game, combat: &mut Combat, locked: bool, dt: f64) {
        let n = self.n;
        let open = self.open.is_some();
        let (feel, keys, toggle_crouch, toggle_sprint) = game.world.get_resource::<Settings>().map_or((Default::default(), Keys::default(), true, false), |s| (s.feel(), s.keys, s.toggle_crouch, s.toggle_sprint));
        self.keys = keys;
        self.bag_ui.slot_keys = keys.slot_names();
        // What's worn weighs on the sprint, the breath and the feet; the
        // armor worn has room for a plate or it hasn't.
        let (fast, breath, _) = crate::loot::gear::burden(self.bag.weight());
        game.set_load(n, fast);
        self.vitals.breath = breath;
        let (armor, most) = self.bag.armor();
        self.vitals.armor_room = most - armor;
        if let Some(mut view) = game.player_view_mut(n) {
            view.feel = feel;
        }
        if open {
            // The pointer is the inventory's.
        } else if locked {
            let motion = ui.state.locked_motion;
            if let Some(mut view) = game.player_view_mut(n) {
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
        let sprint = wants_sprint && combat.arms[n].sprint_allowed() && self.vitals.can_sprint();
        let jump = keys.pressed(ui, Action::Jump);
        // Patching up takes both hands and standing still; rummaging, a
        // slow walk at most.
        let (walk, sprint, jump) = if self.vitals.healing.is_some() {
            (Vec2::ZERO, false, false)
        } else if open {
            (walk * RUMMAGING_PACE, false, false)
        } else {
            (walk * (1.0 - (1.0 - AIMING_PACE) * combat.arms[n].hands.aim()), sprint, jump)
        };
        // Crouching, toggled or held: a hold flips it whenever the key and
        // the body disagree.
        let crouch = if toggle_crouch {
            keys.pressed(ui, Action::Crouch)
        } else {
            let crouched = game.player(n).is_some_and(|(b, _)| b.want_crouch);
            keys.held(ui, Action::Crouch) != crouched
        };
        if let Some(mut controls) = game.controls_mut(n) {
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
        if !busy && keys.pressed(ui, Action::FireMode) && combat.arms[n].hands.switch_fire() {
            combat.play(Sfx::Tick, 0.9);
        }
        self.switch_hands(ui, combat, !busy);
        // The sights up while the right button's held; a sprint takes them
        // down.
        let aim = locked && keys.held(ui, Action::Aim) && !wants_sprint;
        let trigger = if busy { Trigger::default() } else { Trigger { fire: firing, hold: holding, reload: keys.pressed(ui, Action::Reload), melee: striking, aim } };
        self.pull_rounds(combat);
        // Reloading draws on the rounds carried, of the kind the gun takes.
        let hands = &mut combat.arms[n].hands;
        let ammo = hands.spec().ammo;
        hands.spare = ammo.map_or(0, |kind| self.bag.count(kind));
        let spare = hands.spare;
        // Swings spend stamina; winded, they come slower.
        hands.swing_speed = if self.vitals.winded { WINDED_SWING } else { 1.0 };
        let spent = combat.frame(game, n, trigger, dt, &mut self.stats);
        self.vitals.spend(spent);
        let hands = &mut combat.arms[n].hands;
        if let Some(kind) = ammo {
            self.bag.remove(kind, spare - hands.spare);
        }
        // (The dev's: a magazine that never runs down.)
        if game.world.get_resource::<crate::dev::Cheats>().is_some_and(|c| c.ammo) {
            hands.mag = hands.spec().mag;
        }
        self.keep_rounds(combat);
    }

    /// What the dead and the fires did to them this frame: their `blows`,
    /// the bile they stand in (`poisoned`), what the fires and blasts did
    /// (`felt`); nothing hurts them with `god` on. How they died of it, if
    /// they did.
    pub(super) fn suffer(&mut self, game: &mut Game, combat: &mut Combat, blows: impl IntoIterator<Item = Blow>, poisoned: bool, felt: Felt, god: bool) -> Option<Outcome> {
        // Standing in a Spitter's bile.
        if poisoned {
            self.vitals.afflict(Affliction::Poison);
        }
        if god {
            self.dev_heal();
        }
        if self.blasted(game, combat, felt, god) {
            return Some(Outcome::Died(1.0));
        }
        for blow in blows {
            if god {
                continue;
            }
            self.stats.times_hit += 1;
            // Armor takes it first; while there's any, claws don't cut.
            let (armored, _) = self.bag.armor();
            let damage = (blow.damage - f64::from(self.bag.soak(blow.damage.round() as u32))).max(0.0);
            self.stats.damage_taken += damage.min(self.vitals.hp);
            if let Some(a) = blow.leaves
                && !(a == Affliction::Bleed && armored > 0)
            {
                self.vitals.afflict(a);
            }
            if self.vitals.hurt(damage) {
                let side = if self.stats.times_hit.is_multiple_of(2) { 1.0 } else { -1.0 };
                return Some(Outcome::Died(side));
            }
        }
        None
    }

    /// A frame of health, stamina, the kit being applied, and the count.
    /// Whether they bled out, or the poison did it.
    pub(super) fn live(&mut self, game: &mut Game, combat: &Combat, dt: f64) -> bool {
        let sprinting = game.player(self.n).is_some_and(|(b, _)| b.sprinting && b.speed_flat() > 0.5);
        let change = self.vitals.update(dt, sprinting);
        // Heavy gear, sprinting: footfalls the dead near hear.
        let (_, _, heard) = crate::loot::gear::burden(self.bag.weight());
        self.footfall_in -= dt;
        if sprinting
            && heard > 0.0
            && self.footfall_in <= 0.0
            && let Some((body, _)) = game.player(self.n)
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
            return true;
        }
        self.stats.seconds += dt;
        if let Some((body, _)) = game.player(self.n) {
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
        false
    }

    /// Their HUD: `prompt` for what's aimed at, and `busy`, how far through
    /// a job at hand (a way out, nailing boards) that isn't patching up or
    /// searching.
    pub(super) fn hud(&self, ui: &mut Ui, combat: &Combat, game: &mut Game, prompt: Option<(&'static str, String)>, busy: Option<f64>) {
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
        let arms = &combat.arms[self.n];
        let hands = &arms.hands;
        let weapon = match hands.spec().shot {
            Some(s) if s.select => format!("{}  ·  {}", hands.spec().name, if hands.full_auto() { "AUTO" } else { "SEMI" }),
            _ => hands.spec().name.to_string(),
        };
        hud::draw(
            ui,
            &Hud {
                weapon: &weapon,
                rounds: hands.spec().ammo.map(|kind| (hands.mag, self.bag.count(kind))),
                aim: hands.aim(),
                scope: hands.scoped(),
                marker: combat.fx.marker,
                hurt: arms.hurt,
                hp: v.hp,
                max_hp: v.max_hp,
                stamina: v.stamina,
                max_stamina: v.max_stamina,
                winded: v.winded,
                kits: &kits,
                heal: v.heal_progress().or(self.search.as_ref().map(loot::Search::progress)).or(busy),
                prompt: prompt.as_ref().map(|(key, text)| (if key.is_empty() { "" } else { interact.as_str() }, text.as_str())),
                note: self.note.map(|(n, _)| n),
                time,
                crosshair: game.world.get_resource::<Settings>().is_none_or(|s| s.crosshair),
                bleeding: v.bleeding,
                poison: v.poison,
                armor: self.bag.armor(),
            },
        );
    }
}
