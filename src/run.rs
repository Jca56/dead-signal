//! A run, from the player's side: the keys turned into movement, shots,
//! healing and picking things up; health and stamina; the count of what
//! happened; and dying, when it comes to that.

use lntrn_math::{Vec2, Vec3};
use lntrn_ui::{AreaCx, Key, ShellRequest, Ui};

use crate::combat::Combat;
use crate::death::{After, Death};
use crate::hud::{self, Hud};
use crate::items::{self, Item};
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

#[derive(Default)]
pub struct Run {
    pub vitals: Vitals,
    pub stats: Stats,
    pub death: Option<Death>,
    heartbeat: f64,
    /// Where the player was last frame, for distance walked.
    last: Option<Vec3>,
    /// What the player could pick up right now.
    pickup: Option<(bevy_ecs::entity::Entity, Item)>,
    director: Director,
}

impl Run {
    /// A fresh run: whole, nothing counted, things lying in their spots,
    /// the first of the dead already out there.
    pub fn start(&mut self, game: &mut Game) {
        *self = Self::default();
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.subsec_nanos());
        items::scatter(&mut game.world, seed);
        if let Some((eye, forward)) = Self::watching(game) {
            self.director.begin(&mut game.world, eye, forward);
        }
    }

    /// Where the player's eye is and which way it looks, flat.
    fn watching(game: &mut Game) -> Option<(Vec3, Vec3)> {
        let (body, view) = game.player()?;
        Some((body.pos + Vec3::new(0.0, 1.6, 0.0), Vec3::new(-view.yaw.sin(), 0.0, -view.yaw.cos())))
    }

    /// How far the gun is lowered (while patching up), 0–1.
    pub fn lowered(&self) -> f64 {
        if self.vitals.healing.is_some() { 1.0 } else { 0.0 }
    }

    /// A frame of a run while alive: what the keys do, and what came of it.
    pub fn play(&mut self, ui: &mut Ui, cx: &mut AreaCx<()>, game: &mut Game, combat: &mut Combat, locked: bool) {
        let dt = game.clock().dt;
        if locked {
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
        // Patching up takes both hands and standing still.
        let rooted = self.vitals.healing.is_some();
        let (walk, sprint, jump) = if rooted { (Vec2::ZERO, false, false) } else { (walk, sprint, jump) };
        let crouch = pressed(ui, &['c']);
        {
            let mut controls = game.controls_mut();
            controls.walk = walk;
            controls.sprint = sprint;
            controls.jump |= jump;
            controls.crouch_toggle ^= crouch;
        }

        // Patching up: 3 a bandage, 4 a medkit, standing still. Firing,
        // striking or a blow stops it (the kit is kept).
        for (key, kit) in [('3', Kit::Bandage), ('4', Kit::Medkit)] {
            if pressed(ui, &[key]) && self.vitals.start_heal(kit) {
                combat.play(Sfx::Heal, 0.8);
            }
        }
        let firing = locked && ui.state.pressed;
        let striking = ui.state.middle_pressed || pressed(ui, &['v']);
        if self.vitals.healing.is_some() && (firing || striking) {
            self.vitals.interrupt();
        }
        let trigger = if self.vitals.healing.is_some() {
            Trigger::default()
        } else {
            Trigger { fire: firing, reload: pressed(ui, &['r']), melee: striking }
        };
        combat.frame(game, trigger, dt, &mut self.stats);

        // Taking what's in reach and looked at.
        self.pickup = game.player().and_then(|(body, view)| {
            let eye = crate::head::eye_position(&view, &body, game.alpha());
            let (yaw, pitch) = view.aim();
            let dir = Vec3::new(-yaw.sin() * pitch.cos(), pitch.sin(), -yaw.cos() * pitch.cos());
            items::in_view(&mut game.world, eye, dir)
        });
        if let Some((e, item)) = self.pickup
            && pressed(ui, &['e'])
        {
            game.world.despawn(e);
            match item {
                Item::Kit(kit) => self.vitals.take(kit),
                Item::Rounds(n) => {
                    combat.pistol.spare += n;
                    self.stats.rounds_found += n;
                }
            }
            combat.play(Sfx::Pickup, 0.8);
            self.pickup = None;
        }

        // Blows from the dead.
        let blows = combat.answer_the_dead(game, true);
        for _ in 0..blows {
            self.stats.times_hit += 1;
            self.stats.damage_taken += BLOW_DAMAGE.min(self.vitals.hp);
            if self.vitals.hurt(BLOW_DAMAGE) {
                let side = if self.stats.times_hit.is_multiple_of(2) { 1.0 } else { -1.0 };
                self.death = Some(Death::new(self.stats.clone(), side));
                *game.controls_mut() = Default::default();
                return;
            }
        }

        // More of the dead, as the kills mount.
        if let Some((eye, forward)) = Self::watching(game) {
            self.director.update(&mut game.world, self.stats.kills(), eye, forward, dt);
            self.stats.biggest_horde = self.director.peak as u32;
        }

        // Health, stamina, the kit being applied, the count.
        let sprinting = game.player().is_some_and(|(b, _)| b.sprinting && b.speed_flat() > 0.5);
        let change = self.vitals.update(dt, sprinting);
        if let Some((kit, healed)) = change.healed {
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
        self.hud(ui, combat, game.clock().time);
    }

    fn hud(&self, ui: &mut Ui, combat: &Combat, time: f64) {
        let v = &self.vitals;
        let pickup = self.pickup.map(|(_, item)| item.label());
        hud::draw(
            ui,
            &Hud {
                mag: combat.pistol.mag,
                spare: combat.pistol.spare,
                marker: combat.fx.marker,
                hurt: combat.hurt,
                hp: v.hp,
                stamina: v.stamina,
                winded: v.winded,
                bandages: v.bandages,
                medkits: v.medkits,
                heal: v.heal_progress(),
                pickup: pickup.as_deref(),
                time,
            },
        );
    }

    /// A frame of dying: the fall, the words, the numbers. What the player
    /// chose, once they have.
    pub fn dying(&mut self, ui: &mut Ui, cx: &mut AreaCx<()>, game: &mut Game, combat: &mut Combat, active: bool) -> Option<After> {
        let dt = game.clock().dt;
        let death = self.death.as_mut()?;
        death.update(dt);
        if death.words_begin(dt) {
            combat.play(Sfx::Died, 1.0);
        }
        if death.showing_stats() && ui.state.pointer_locked {
            cx.request(ShellRequest::LockPointer(false));
        }
        combat.answer_the_dead(game, false);
        death.draw(ui, active)
    }
}
