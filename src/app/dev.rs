//! The dev tools as the app has them (`dev/`): whether they're on (the DEV
//! slot, in developer mode), F1 for the panel, doing what it asks, and the
//! readout in the corner.

use lntrn_math::{Vec2, Vec3};
use lntrn_ui::{AreaCx, Key, ShellRequest, Ui};

use super::{DeadSignal, Screen};
use crate::dev::{self, Cheats, DevAction};
use crate::loot::Stack;
use crate::profile::Profile;
use crate::profile::save::DEV;
use crate::settings::Settings;
use crate::zombie::{self, brain::Zombie, looks::Theme};

impl DeadSignal {
    /// Whether the dev tools are there: the DEV slot, in developer mode.
    pub(super) fn dev_on(&self) -> bool {
        self.saves.slot == DEV && self.game.world.resource::<Settings>().dev_mode
    }

    /// The dev tools for a frame, before the screen's own: off (and every
    /// cheat with them) unless they're on; F1 opens and shuts the panel
    /// (in the hideout or a run), and while it's up, it's all there is.
    /// Whether the panel took the frame.
    pub(super) fn dev_frame(&mut self, ui: &mut Ui, cx: &mut AreaCx<()>, active: bool) -> bool {
        if !self.dev_on() {
            *self.game.world.resource_mut::<Cheats>() = Cheats::default();
            self.dev_open = false;
            return false;
        }
        let can = matches!(self.screen, Screen::Hideout | Screen::Run) && self.run.ending.is_none() && !self.paused;
        if !self.dev_open && can && active && ui.state.take_key(|k| !k.repeat && k.key == Key::F(1)).is_some() {
            self.dev_open = true;
            cx.request(ShellRequest::LockPointer(false));
        }
        if !self.dev_open || !can {
            self.dev_open = false;
            return false;
        }
        let places: Vec<String> = self.map.as_ref().map_or_else(Vec::new, |m| m.sites.iter().map(|s| s.kind.label().to_string()).collect());
        let mut cheats = *self.game.world.resource::<Cheats>();
        let in_run = self.screen == Screen::Run;
        let asked = self.dev.frame(ui, &self.icons, &mut cheats, &places, in_run, active);
        *self.game.world.resource_mut::<Cheats>() = cheats;
        if let Some(action) = asked.action {
            self.dev_do(action);
        }
        if asked.closed {
            self.dev_open = false;
            if in_run && self.run.wants_lock() {
                cx.request(ShellRequest::LockPointer(true));
            }
        }
        true
    }

    /// Do what the panel asked.
    fn dev_do(&mut self, action: DevAction) {
        let player = self.game.player();
        match action {
            DevAction::Give(kind) => {
                let stack = match kind.weapon() {
                    Some(w) => Stack::gun(kind, w.spec().mag),
                    None => Stack::new(kind, kind.def().stack),
                };
                if self.screen == Screen::Run {
                    let rest = self.run.bag.add(stack);
                    if rest.count > 0
                        && let Some((body, view)) = player
                    {
                        crate::items::set_down(&mut self.game.world, rest, body.pos + Vec3::new(0.0, 1.0, 0.0), view.yaw);
                    }
                } else {
                    let rest = self.profile.stash.top_up(stack);
                    self.profile.stash.place(rest);
                    self.saves.store(&self.profile);
                }
            }
            DevAction::Spawn(kind, n) => {
                let Some((body, view)) = player else { return };
                let forward = Vec3::new(-view.yaw.sin(), 0.0, -view.yaw.cos());
                let side = Vec3::new(forward.z, 0.0, -forward.x);
                for i in 0..n {
                    let spread = f64::from(i) - f64::from(n.saturating_sub(1)) * 0.5;
                    let mut at = body.pos + forward * 10.0 + side * (spread * 1.6);
                    if let Some(h) = self.game.world.resource::<zombie::Nav>().0.as_ref().and_then(|nav| nav.height_at(at + Vec3::new(0.0, 2.0, 0.0))) {
                        at.y = h;
                    }
                    zombie::spawn_kind(&mut self.game.world, at, view.yaw + std::f64::consts::PI, kind, Theme::Drifter);
                }
            }
            DevAction::KillNear(reach) => {
                let Some((body, _)) = player else { return };
                let near: Vec<(bevy_ecs::entity::Entity, Vec3)> = self
                    .game
                    .world
                    .query::<(bevy_ecs::entity::Entity, &Zombie, &crate::player::Body)>()
                    .iter(&self.game.world)
                    .filter(|(_, z, b)| !z.dead() && (b.pos - body.pos).length() < reach)
                    .map(|(e, _, b)| (e, b.pos))
                    .collect();
                for (e, at) in near {
                    let impact = zombie::Impact { damage: 1.0e7, head: false, limb: false, blow: false, shove: 2.0, stumble: false, takedown: false };
                    zombie::hurt(&mut self.game.world, e, at - body.pos, body.pos, impact);
                }
            }
            DevAction::Teleport(i) => {
                let Some(site) = self.map.as_ref().and_then(|m| m.sites.get(i)) else { return };
                let middle = site.plot.world(Vec2::ZERO);
                let y = self.game.ground().height_at(middle.x, middle.y).unwrap_or(site.plot.height);
                self.game.teleport(Vec3::new(middle.x, y + 0.2, middle.y));
            }
            DevAction::RevealExits => {
                if let Some(mut exits) = self.game.world.get_resource_mut::<crate::exits::Exits>() {
                    for e in &mut exits.list {
                        e.found = true;
                    }
                }
            }
            DevAction::StartSurge => self.run.dev_surge(),
            DevAction::Heal => self.run.dev_heal(),
            DevAction::Money(n) => {
                self.profile.money += n;
                self.saves.store(&self.profile);
            }
            DevAction::Level => {
                let (level, _, _) = crate::profile::xp::level(self.profile.xp);
                self.profile.xp += crate::profile::xp::to_next(level);
                self.saves.store(&self.profile);
            }
            DevAction::Reset => {
                self.profile = Profile::new_player();
                self.saves.store(&self.profile);
            }
        }
    }

    /// The readout in the corner, in a run: what's up and about, how loud
    /// it's been, where the player is.
    pub(super) fn dev_readout(&mut self, ui: &mut Ui) {
        if !self.dev_on() || !self.dev.info || self.screen != Screen::Run {
            return;
        }
        let dt = self.game.clock().dt.max(1e-6);
        self.dev_fps += (1.0 / dt - self.dev_fps) * 0.05;
        let mut counts = [0usize; 4];
        for z in self.game.world.query::<&Zombie>().iter(&self.game.world).filter(|z| !z.dead()) {
            counts[z.kind as usize] += 1;
        }
        let heat = self.game.world.get_resource::<zombie::Heat>().map_or(0.0, |h| h.0);
        let at = self.game.player().map_or(Vec3::ZERO, |(b, _)| b.pos);
        let cheats = *self.game.world.resource::<Cheats>();
        let on: Vec<&str> = [(cheats.god, "GOD"), (cheats.ammo, "AMMO"), (cheats.ignored, "IGNORED"), (cheats.one_shot, "ONE-SHOT")].into_iter().filter(|(on, _)| *on).map(|(_, n)| n).collect();
        let lines = vec![
            format!("DEV · {:.0} FPS", self.dev_fps),
            format!("DEAD {}: {} shamblers, {} rippers, {} spitters, {} juggernauts", counts.iter().sum::<usize>(), counts[0], counts[1], counts[2], counts[3]),
            format!("HEAT {heat:.1}"),
            format!("AT {:.0}, {:.0}, {:.0}", at.x, at.y, at.z),
            format!("CHEATS {}", if on.is_empty() { "none".to_string() } else { on.join(", ") }),
        ];
        dev::draw_info(ui, &lines);
    }
}
