//! Who's playing, with what, as the app has it: the lobby over the title
//! (`lobby.rs`); what each player's hands are on; the last thing touched
//! (whoever picks HOLDOUT is the first player); where each starts; and
//! each player's pad kept: one gone mid-run pauses it, and the next pad to
//! come (the same one back, or another) is theirs.

use lntrn_math::Vec3;
use lntrn_ui::{AreaCx, Ui};

use super::{DeadSignal, Screen, Then};
use crate::input::{Device, Feed};
use crate::lobby::Went;
use crate::world::Solid;

/// How far apart players start, side by side.
const APART: f64 = 1.2;

impl DeadSignal {
    /// What was touched this frame, a pad or the keyboard and mouse.
    pub(super) fn note_device(&mut self, ui: &Ui) {
        if let Some(id) = self.pads.pressed_by() {
            self.last_device = Device::Pad(id);
        } else if !ui.state.keys.is_empty() || ui.state.pressed {
            self.last_device = Device::Keys;
        }
    }

    /// A frame of the lobby: begun, or backed out of (the title menu
    /// again). Whether it's closed.
    pub(super) fn lobby_frame(&mut self, ui: &mut Ui, active: bool) -> bool {
        let Some(lobby) = &mut self.lobby else { return true };
        match lobby.frame(ui, &self.pads, active) {
            Some(Went::Start(devices)) => {
                self.devices = devices;
                self.holdout = true;
                self.lobby = None;
                self.fade_to(Then::Show(Screen::Loading));
                true
            }
            Some(Went::Back) => {
                self.lobby = None;
                true
            }
            None => false,
        }
    }

    /// Each player's pad still there: a player's gone (mid-run, it
    /// pauses), the next spare pad is theirs.
    pub(super) fn keep_pads(&mut self, cx: &mut AreaCx<()>) {
        let mut spare = self.pads.ids().into_iter().filter(|id| !self.devices.contains(&Device::Pad(*id))).collect::<Vec<_>>().into_iter();
        let mut lost = false;
        for device in &mut self.devices {
            if let Device::Pad(id) = *device
                && !self.pads.connected(id)
            {
                match spare.next() {
                    Some(next) => *device = Device::Pad(next),
                    None => lost = true,
                }
            }
        }
        if lost && self.screen == Screen::Run && !self.paused && self.run.ending.is_none() {
            self.pause(cx);
        }
    }

    /// What each player's hands are on this frame, by seat.
    pub(super) fn feeds(&self) -> Vec<Feed> {
        self.devices.iter().map(|&d| Feed { keys: d.has_keys(), pad: self.pads.feed(d) }).collect()
    }

    /// Each player's pad shaken for what happened to them this frame (if
    /// rumble's on).
    pub(super) fn shake_pads(&mut self) {
        let rumble = self.game.world.resource::<crate::settings::Settings>().rumble;
        for (arms, &device) in self.combat.arms.iter_mut().zip(&self.devices) {
            let r = std::mem::take(&mut arms.rumble);
            if rumble {
                self.pads.rumble(device, r);
            }
        }
    }

    /// Where player `seat` starts, the first starting at `at` facing
    /// `yaw`: beside them (to the right, else the left, else behind),
    /// where there's room.
    pub(super) fn beside(&self, at: Vec3, yaw: f64, seat: usize) -> Vec3 {
        if seat == 0 {
            return at;
        }
        let (s, c) = yaw.sin_cos();
        let (right, back) = (Vec3::new(c, 0.0, -s), Vec3::new(s, 0.0, c));
        let apart = APART * seat as f64;
        let solid = &self.game.world.resource::<Solid>().0;
        [right * apart, right * -apart, back * apart].into_iter().map(|off| at + off).find(|&p| solid.fits(crate::player::capsule(false), p + Vec3::new(0.0, 0.05, 0.0))).unwrap_or(at)
    }
}
