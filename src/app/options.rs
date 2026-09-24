//! The settings screen as the app has it (over the title or a paused
//! run), and the camera each screen sees through.

use lntrn_math::Vec3;
use lntrn_ui::{AreaCx, ShellRequest, Ui};

use super::{DeadSignal, Screen, TITLE_EYE, TITLE_FOV, TITLE_LOOK};
use crate::head;
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
        if let Some(mut view) = self.game.player_view_mut() {
            view.feel = s.feel();
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

    /// Where the camera is this frame.
    pub(super) fn place_camera(&mut self, time: f64) {
        match self.screen {
            Screen::Title | Screen::Hideout | Screen::Loading => {
                // A slow drift, never quite still.
                let drift = Vec3::new((time * 0.05).sin() * 4.0, (time * 0.07).sin() * 0.5, (time * 0.04).cos() * 2.5);
                let mut eye = TITLE_EYE + drift;
                if let Some(h) = self.game.ground().height_at(eye.x, eye.z) {
                    eye.y = eye.y.max(h + 2.0);
                }
                self.camera.position = eye;
                self.camera.roll = 0.0;
                self.camera.fov_y = TITLE_FOV.to_radians();
                self.camera.look_at(TITLE_LOOK + Vec3::new((time * 0.09).sin() * 1.5, 0.0, 0.0));
            }
            Screen::Run => {
                let alpha = self.game.alpha();
                if let Some((body, view)) = self.game.player() {
                    self.camera.position = head::eye_position(&view, &body, alpha);
                    (self.camera.yaw, self.camera.pitch) = view.aim();
                    self.camera.fov_y = view.fov_y();
                    self.camera.roll = 0.0;
                    if let Some(ending) = &self.run.ending {
                        ending.fall(&mut self.camera);
                    }
                }
            }
        }
    }
}
