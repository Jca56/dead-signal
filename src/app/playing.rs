//! A run as the app has it: its frames (and pausing, and being asked
//! twice about walking out), and settling it into the profile once it's
//! over.

use lntrn_math::Color;
use lntrn_ui::{AreaCx, Key, ShellRequest, Ui};

use super::{DeadSignal, LeaveItem, PAUSE_DIM, PauseItem, Screen, Then};
use crate::ending::After;
use crate::player::Controls;
use crate::settings::screen::SettingsScreen;

impl DeadSignal {
    /// Out of a run (if in one) and back to the title's scene: the run
    /// settled, the player gone.
    pub(super) fn leave_run(&mut self) {
        self.settle_run();
        self.game.despawn_player();
        self.run.ending = None;
        self.show_title_scene();
    }

    /// A run that's over (or walked out on) settles into the profile, and
    /// the profile is saved. Walked out on counts as dead.
    pub(super) fn settle_run(&mut self) {
        if let Some((got_out, bag, xp)) = self.run.take_result() {
            self.profile.settle(got_out, bag, xp);
            self.in_run = false;
            self.saves.store(&self.profile);
        } else if self.in_run {
            let bag = self.run.abandon();
            self.profile.settle(false, bag, 0);
            self.in_run = false;
            self.saves.store(&self.profile);
        }
    }

    pub(super) fn pause(&mut self, cx: &mut AreaCx<()>) {
        self.paused = true;
        self.leaving = false;
        self.pause_menu.reset();
        *self.game.controls_mut() = Controls::default();
        cx.request(ShellRequest::LockPointer(false));
    }

    pub(super) fn resume(&mut self, cx: &mut AreaCx<()>) {
        self.paused = false;
        if self.run.wants_lock() {
            cx.request(ShellRequest::LockPointer(true));
        }
    }

    /// A run's frame: look, move, pause; or, dead, the way out.
    pub(super) fn run_frame(&mut self, ui: &mut Ui, cx: &mut AreaCx<()>, active: bool) {
        if self.run.ending.is_some() {
            match self.run.ending(ui, cx, &mut self.game, &mut self.combat, active, &self.icons) {
                Some(After::Hideout) => self.fade_to(Then::Show(Screen::Hideout)),
                Some(After::Title) => self.fade_to(Then::Show(Screen::Title)),
                None => {}
            }
            return;
        }
        let locked = ui.state.pointer_locked;
        // Losing the lock unasked (the window lost focus) pauses; the
        // inventory letting it go doesn't.
        if self.was_locked && !locked && !self.paused && active && self.run.wants_lock() {
            self.pause(cx);
        }
        self.was_locked = locked;
        if self.paused && self.settings.is_some() {
            ui.draw.rect(ui.clip(), Color::rgba(0.0, 0.0, 0.0, PAUSE_DIM));
            if self.settings_frame(ui, cx, active) {
                self.pause_menu.reset();
            }
            return;
        }
        if active && ui.state.take_key(|k| k.key == Key::Escape).is_some() && !self.run.shut_bag(&mut self.game, cx) {
            // Asked about leaving, Esc is staying.
            if self.leaving {
                self.leaving = false;
            } else if self.paused {
                self.resume(cx)
            } else {
                self.pause(cx)
            }
        }
        if self.paused {
            let screen = ui.clip();
            ui.draw.rect(screen, Color::rgba(0.0, 0.0, 0.0, PAUSE_DIM));
            if self.leaving {
                match self.leave_menu.draw(ui, active) {
                    Some(LeaveItem::Stay) => self.leaving = false,
                    Some(LeaveItem::Leave) => {
                        cx.request(ShellRequest::LockPointer(false));
                        self.fade_to(Then::Show(Screen::Title));
                    }
                    None => {}
                }
            } else {
                match self.pause_menu.draw(ui, active) {
                    Some(PauseItem::Resume) => self.resume(cx),
                    Some(PauseItem::Settings) => self.settings = Some(SettingsScreen::default()),
                    Some(PauseItem::ToTitle) => {
                        self.leaving = true;
                        self.leave_menu.reset();
                    }
                    None => {}
                }
            }
            return;
        }
        if !active {
            return;
        }
        self.run.play(ui, cx, &mut self.game, &mut self.combat, locked, &self.icons);
        self.map_screen(ui, active);
        // The run just ended: it's settled (and saved) at once.
        if self.run.ending.is_some() {
            self.settle_run();
        }
    }
}
