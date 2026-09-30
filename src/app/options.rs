//! The settings screen as the app has it (over the title or a paused
//! run).

use lntrn_ui::{AreaCx, ShellRequest, Ui};

use super::DeadSignal;
use crate::settings::{self, Field, Settings};

impl DeadSignal {
    /// A frame of the settings screen, over whatever's behind it: what's
    /// changed takes at once (the window, the sound, the UI's scale once
    /// its slider's let go), and it's all saved on the way out. Whether it
    /// was closed.
    pub(super) fn settings_frame(&mut self, ui: &mut Ui, cx: &mut AreaCx<()>, active: bool) -> bool {
        let Some(screen) = &mut self.settings else { return true };
        let mut s = self.game.world.resource::<Settings>().clone();
        let was = s.clone();
        let closed = screen.frame(ui, &mut s, active);
        if s.fullscreen != was.fullscreen {
            self.fullscreen = s.fullscreen;
            cx.request(ShellRequest::Fullscreen(s.fullscreen));
        }
        if s.vsync != was.vsync {
            cx.request(ShellRequest::Vsync(s.vsync));
        }
        if s.levels() != was.levels() {
            self.combat.set_mix(s.levels());
        }
        // (Paused, the view behind shows a new field of view at once.)
        for (seat, _, _) in self.game.players() {
            if let Some(mut view) = self.game.player_view_mut(seat) {
                view.feel = s.feel();
            }
        }
        if !screen.holding(Field::UiScale) {
            self.ui_scale = s.ui_scale;
        }
        if closed {
            settings::store(&s);
            self.settings = None;
        }
        *self.game.world.resource_mut::<Settings>() = s;
        closed
    }
}
