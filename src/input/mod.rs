//! A player's controls, whatever they hold: the keys and mouse (bound in
//! `settings::keys`), a pad (`pad.rs`), or, playing alone, both at once.
//! A frame's actions are read through the player's [`Input`] the way the
//! keys always were: held, or pressed this frame (a press taken, so
//! nothing else acts on it too). How a stick turns the view, and aim
//! assist's drag on it, is in `look.rs`; a pad at a screen of cells (the
//! bag), in `steer.rs`.

pub mod look;
pub mod pad;
pub mod steer;

use lntrn_math::Vec2;
use lntrn_ui::Ui;

use crate::settings::keys::{Action, Bind, Keys};
use pad::{Control, PadBinds, PadFrame};

/// The mouse moved this far (counts) in a frame, it's the mouse they're
/// using.
const MOUSED: f64 = 2.0;
/// A control that's one thing tapped and another held: held this long,
/// it's the other.
const HOLD: f64 = 0.4;

/// What a player plays with: the keyboard and mouse, one pad (by its id),
/// or, playing alone, the lot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Device {
    Keys,
    Pad(u32),
    All,
}

impl Device {
    /// Whether the keyboard and mouse are among it.
    pub fn has_keys(self) -> bool {
        matches!(self, Device::Keys | Device::All)
    }
}

/// What a player's hands are on this frame: whether the keyboard and mouse
/// are theirs, and what their pad did.
#[derive(Clone, Copy, Debug, Default)]
pub struct Feed {
    pub keys: bool,
    pub pad: PadFrame,
}

#[derive(Clone, Debug, Default)]
pub struct Input {
    /// The keys and mouse, as bound, if they're this player's; whether the
    /// pointer's locked (the mouse is looking, not pointing: its buttons
    /// count).
    keys: Option<Keys>,
    locked: bool,
    /// What their pad (or pads) did this frame, and its layout.
    pad: PadFrame,
    binds: PadBinds,
    /// Whether a pad was the last thing they touched: the HUD names its
    /// buttons then.
    pub on_pad: bool,
    /// Something's in front of them to use: a pad's X is interact, not
    /// reload.
    prompting: bool,
    /// How long the look stick's been held hard over (its turn builds).
    pub hard_over: f64,
    /// The control that's the bag tapped and the map held: how long it's
    /// been down (while that's still to be told), and which it turned out
    /// to be this frame.
    timing: Option<f64>,
    tapped: bool,
    long: bool,
    /// The map's up: that control puts it away, tapped or held.
    mapped: bool,
    /// The bag's up: their pad's working it ([`Input::steer`]), and none
    /// of its controls are the game's.
    rummaging: bool,
    steering: steer::Repeat,
}

impl Input {
    /// This frame's: the keys and mouse (`keys`, the pointer `locked`) if
    /// they're this player's, and what their pads did (`pad`), `dt` on
    /// from the last.
    pub fn update(&mut self, ui: &Ui, keys: Option<Keys>, locked: bool, pad: PadFrame, dt: f64) {
        let s = &ui.state;
        let typed = keys.is_some() && (!s.keys_down.is_empty() || s.pressed || s.right_pressed || s.middle_pressed || s.wheel.y != 0.0 || s.locked_motion.length() > MOUSED);
        if pad.touched {
            self.on_pad = true;
        } else if typed {
            self.on_pad = false;
        }
        self.keys = keys;
        self.locked = locked;
        self.pad = pad;
        // A tap's told from a hold when it's let go, or held long enough.
        (self.tapped, self.long) = (false, false);
        let Some(c) = self.two_way() else {
            self.timing = None;
            return;
        };
        if pad.went_down(c) {
            self.timing = Some(0.0);
        }
        if let Some(t) = self.timing {
            let t = t + dt;
            self.timing = Some(t);
            if !pad.held(c) {
                (self.tapped, self.timing) = (true, None);
            } else if t >= HOLD {
                (self.long, self.timing) = (true, None);
            }
        }
    }

    /// The control that's both the bag and the map, if one is.
    fn two_way(&self) -> Option<Control> {
        self.binds.get(Action::Inventory).filter(|c| self.binds.get(Action::Map) == Some(*c))
    }

    /// The map's up (or away): up, the control that's the bag tapped and
    /// the map held puts it away either way.
    pub fn set_mapped(&mut self, mapped: bool) {
        self.mapped = mapped;
    }

    /// The bag's up (or away): up, their pad's the bag's.
    pub fn set_rummaging(&mut self, rummaging: bool) {
        self.rummaging = rummaging;
    }

    /// What their pad asks of the bag this frame (`dt` on from the last).
    pub fn steer(&mut self, dt: f64) -> steer::Steer {
        self.steering.read(&mut self.pad, dt)
    }

    /// Whether something's in front of them to use (a prompt's up): a
    /// pad's X interacts with it instead of reloading.
    pub fn set_prompting(&mut self, prompting: bool) {
        self.prompting = prompting;
    }

    /// Whether the keyboard and mouse are theirs.
    pub fn has_keys(&self) -> bool {
        self.keys.is_some()
    }

    /// The pad control `a` is on this frame: where one control is both
    /// reload and interact, it's interact while something's in front of
    /// them and reload while there isn't.
    /// (None at all while the pad's the bag's; and none for the bag and
    /// the map on one control, told apart by how long it's held.)
    fn control(&self, a: Action) -> Option<Control> {
        let shared = self.binds.get(Action::Reload) == self.binds.get(Action::Interact);
        match a {
            _ if self.rummaging => None,
            Action::Inventory | Action::Map if self.two_way().is_some() => None,
            Action::Reload if shared && self.prompting => None,
            Action::Interact if shared && !self.prompting => None,
            _ => self.binds.get(a),
        }
    }

    /// Whether `a`'s button went down on the pad this frame (a press
    /// taken): the bag's is a tap of its control and the map's a hold,
    /// where they share one.
    pub fn pad_pressed(&mut self, a: Action) -> bool {
        match a {
            Action::Inventory if self.two_way().is_some() => !self.mapped && std::mem::take(&mut self.tapped),
            Action::Map if self.two_way().is_some() => std::mem::take(&mut self.long) || (self.mapped && std::mem::take(&mut self.tapped)),
            _ => self.control(a).is_some_and(|c| self.pad.take(c)),
        }
    }

    /// Whether a bind counts now: a mouse button only while the pointer's
    /// locked.
    fn counts(&self, b: Bind) -> bool {
        self.locked || !matches!(b, Bind::Mouse(_))
    }

    /// Whether `a`'s key or button is held down.
    pub fn held(&self, ui: &Ui, a: Action) -> bool {
        self.control(a).is_some_and(|c| self.pad.held(c)) || self.keys.is_some_and(|k| self.counts(k.get(a)) && k.held(ui, a))
    }

    /// Whether `a`'s key or button went down this frame (a press is taken,
    /// so nothing else acts on it too).
    pub fn pressed(&mut self, ui: &mut Ui, a: Action) -> bool {
        self.pad_pressed(a) || self.keys.is_some_and(|k| self.counts(k.get(a)) && k.pressed(ui, a))
    }

    /// Whether `a`'s key went down on the keyboard (a press taken): not a
    /// mouse button, nor the pad.
    pub fn key_pressed(&self, ui: &mut Ui, a: Action) -> bool {
        self.keys.is_some_and(|k| matches!(k.get(a), Bind::Key(_)) && k.pressed(ui, a))
    }

    /// Which way they're walking: x right, y forward, each -1 to 1 (the
    /// keys all the way, the stick as far as it's pushed).
    pub fn walk(&self, ui: &Ui) -> Vec2 {
        let axis = |neg, pos| f64::from(i8::from(self.held(ui, pos)) - i8::from(self.held(ui, neg)));
        let stick = if self.rummaging { Vec2::ZERO } else { self.pad.left };
        let w = Vec2::new(axis(Action::Left, Action::Right), axis(Action::Back, Action::Forward)) + stick;
        Vec2::new(w.x.clamp(-1.0, 1.0), w.y.clamp(-1.0, 1.0))
    }

    /// How far the mouse moved to look this frame (counts), while it's
    /// theirs and looking.
    pub fn mouse(&self, ui: &Ui) -> Vec2 {
        if self.keys.is_some() && self.locked { ui.state.locked_motion } else { Vec2::ZERO }
    }

    /// The look stick: x right, y up.
    pub fn stick(&self) -> Vec2 {
        self.pad.right
    }

    /// The mouse wheel's turn this frame, pixels, while it's theirs.
    pub fn wheel(&self, ui: &Ui) -> f64 {
        if self.keys.is_some() { ui.state.wheel.y } else { 0.0 }
    }

    /// Whether they asked for the next weapon on (a pad's Y): taken.
    pub fn next_weapon(&mut self) -> bool {
        !self.rummaging && self.pad.take(self.binds.next_weapon)
    }

    /// `a`'s key or button, as the HUD writes it: the pad's while that's
    /// what they last touched (or all they have).
    pub fn name(&self, a: Action) -> String {
        if (self.on_pad || self.keys.is_none())
            && let Some(c) = self.binds.get(a)
        {
            return self.pad.labels.name(c).to_string();
        }
        self.keys.map_or_else(String::new, |k| k.name(a))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lntrn_sys::gamepad::Button;
    use lntrn_ui::testing::Harness;

    #[test]
    fn x_reloads_but_interacts_with_something_in_front() {
        let mut h = Harness::new(800.0, 600.0);
        h.frame(|ui| {
            let x = pad::frame_with(&[Button::West]);
            let mut i = Input::default();
            i.update(ui, Some(Keys::default()), true, x, 0.016);
            assert!(i.pressed(ui, Action::Reload) && !i.pressed(ui, Action::Interact));
            i.update(ui, Some(Keys::default()), true, x, 0.016);
            i.set_prompting(true);
            assert!(!i.pressed(ui, Action::Reload) && i.pressed(ui, Action::Interact));
            assert_eq!(i.name(Action::Interact), "X", "named for the pad just pressed");
        });
    }

    #[test]
    fn a_mouse_button_counts_only_while_the_pointer_s_locked() {
        let mut h = Harness::new(800.0, 600.0);
        h.press();
        h.frame(|ui| {
            let mut i = Input::default();
            i.update(ui, Some(Keys::default()), false, PadFrame::default(), 0.016);
            assert!(!i.held(ui, Action::Fire), "unlocked, the mouse points");
            i.update(ui, Some(Keys::default()), true, PadFrame::default(), 0.016);
            assert!(i.held(ui, Action::Fire));
            assert_eq!(i.name(Action::Fire), "MOUSE LEFT");
        });
    }

    #[test]
    fn view_tapped_is_the_bag_and_held_is_the_map() {
        let mut h = Harness::new(800.0, 600.0);
        h.frame(|ui| {
            let (tap, held, rest) = (pad::frame_with(&[Button::Select]), pad::frame_holding(&[Button::Select]), PadFrame::default());
            let mut i = Input::default();
            // Down and up again between two frames: a tap, at once.
            i.update(ui, None, true, tap, 0.016);
            assert!(!i.pressed(ui, Action::Map) && i.pressed(ui, Action::Inventory));
            assert!(!i.pressed(ui, Action::Inventory), "taken once");
            // Held a little, then let go: a tap, as it's let go.
            i.update(ui, None, true, held.merge(tap), 0.016);
            assert!(!i.pressed(ui, Action::Inventory), "not while it's still down");
            i.update(ui, None, true, held, 0.1);
            i.update(ui, None, true, rest, 0.016);
            assert!(i.pressed(ui, Action::Inventory) && !i.pressed(ui, Action::Map));
            // Held on: the map, once, and no bag when it's let go.
            i.update(ui, None, true, held.merge(tap), 0.016);
            i.update(ui, None, true, held, 0.5);
            assert!(i.pressed(ui, Action::Map) && !i.pressed(ui, Action::Inventory));
            i.update(ui, None, true, held, 0.5);
            assert!(!i.pressed(ui, Action::Map), "once a hold");
            i.update(ui, None, true, rest, 0.016);
            assert!(!i.pressed(ui, Action::Inventory));
            // The map up, a tap puts it away (and doesn't put the bag up).
            i.set_mapped(true);
            i.update(ui, None, true, tap, 0.016);
            assert!(!i.pressed(ui, Action::Inventory) && i.pressed(ui, Action::Map));
            assert_eq!(i.name(Action::Inventory), "VIEW");
        });
    }

    #[test]
    fn the_bag_up_a_pad_s_controls_arent_the_game_s() {
        let mut h = Harness::new(800.0, 600.0);
        h.frame(|ui| {
            let mut pad = pad::frame_with(&[Button::Left, Button::South, Button::Select]);
            pad.left = Vec2::new(1.0, 0.0);
            let mut i = Input::default();
            i.set_rummaging(true);
            i.update(ui, None, true, pad, 0.016);
            assert_eq!(i.walk(ui), Vec2::ZERO, "the stick's the cursor's");
            assert!(!i.pressed(ui, Action::Bandage) && !i.pressed(ui, Action::Jump) && !i.next_weapon());
            let steer = i.steer(0.016);
            assert_eq!((steer.step, steer.pick), ((1, 0), true), "they're the bag's");
            assert!(i.pad_pressed(Action::Inventory), "and View still puts it away");
            i.set_rummaging(false);
            i.update(ui, None, true, pad, 0.016);
            assert!(i.pressed(ui, Action::Bandage) && i.walk(ui).x > 0.9);
        });
    }
}
