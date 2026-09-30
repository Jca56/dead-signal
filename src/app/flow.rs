//! Going between screens: the fade through black, and what each screen
//! needs as it's shown (a run begun with its player in place, the
//! loadout committed; the title's scene put back).

use lntrn_ui::{AreaCx, ShellRequest};

use super::{DeadSignal, FADE, Screen, Then};
use crate::zombie;

impl DeadSignal {
    pub(super) fn fade_to(&mut self, then: Then) {
        if self.fading_to.is_none() {
            self.fading_to = Some(then);
            self.fade_seconds = FADE;
        }
    }

    /// Move the fade along; what to do now that it is black, if anything.
    pub(super) fn step_fade(&mut self, dt: f64) -> Option<Then> {
        let step = dt / self.fade_seconds.max(1e-3);
        match self.fading_to {
            Some(then) => {
                self.black = (self.black + step).min(1.0);
                if self.black >= 1.0 {
                    self.fading_to = None;
                    self.fade_seconds = FADE;
                    return Some(then);
                }
            }
            None => self.black = (self.black - step).max(0.0),
        }
        None
    }

    /// The screen is black: change what is behind it.
    pub(super) fn show(&mut self, screen: Screen, cx: &mut AreaCx<()>) {
        let from = std::mem::replace(&mut self.screen, screen);
        self.paused = false;
        self.leaving = false;
        self.was_locked = false;
        match screen {
            Screen::Loading => self.start_loading(),
            Screen::Run => {
                // Each player in, side by side (a run with the ways out is
                // played alone).
                let (at, yaw) = self.spawn_point();
                let players = if self.arena.is_some() { self.devices.len() } else { 1 };
                self.game.despawn_players();
                for seat in 0..players {
                    let p = self.beside(at, yaw, seat);
                    self.game.spawn_player(seat, p.x, p.z, yaw);
                }
                // Together, each seen by the others.
                if players > 1 {
                    crate::survivor::dress(&mut self.game.world);
                }
                self.map_open = false;
                if let Some(vm) = &mut self.viewmodel {
                    vm.reset(self.game.players().len());
                }
                zombie::clear(&mut self.game.world);
                zombie::spit::clear(&mut self.game.world);
                crate::throw::clear(&mut self.game.world);
                if let Some(arena) = self.arena.take() {
                    // A holdout: nothing of the profile's goes in.
                    self.settle_run();
                    self.run.start_holdout(&mut self.game, &mut self.combat, arena, self.profile.best_round, players);
                    self.black = 1.0;
                    cx.request(ShellRequest::LockPointer(true));
                    return;
                }
                // The loadout goes in with the player. On disk it's already
                // as good as lost (all but the pockets) till they're out:
                // quitting mid-run is no way round dying.
                self.settle_run();
                let loadout = self.profile.take_loadout();
                let mut committed = self.profile.clone();
                committed.loadout.pockets = loadout.pockets.clone();
                self.saves.store(&committed);
                if let Some(map) = &self.map {
                    self.run.start(&mut self.game, &mut self.combat, loadout, self.profile.xp, self.profile.perks, map);
                }
                self.in_run = true;
                // In from black, off the loading screen.
                self.black = 1.0;
                cx.request(ShellRequest::LockPointer(true));
            }
            Screen::Hideout => {
                if from == Screen::Run {
                    self.leave_run();
                }
            }
            Screen::Title => {
                self.leave_run();
                self.title_menu.reset();
            }
        }
    }
}
