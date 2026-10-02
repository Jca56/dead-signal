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
/// it's the other (the bag and the map; reloading, and what's used by
/// holding).
const HOLD: f64 = 0.4;
const USE_HOLD: f64 = 0.22;

/// What's in front of a player to use, as a pad's X has it (X reloads
/// too): nothing; something a press uses (X is that press, not a reload);
/// or something used by holding (X held is that, and tapped, still a
/// reload).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Prompt {
    #[default]
    None,
    Press,
    Hold,
}

/// A control told apart by how long it's down: a tap (let go soon), or a
/// hold.
#[derive(Clone, Copy, Debug, Default)]
struct Timed {
    /// How long it's been down, while which it is is still to be told.
    since: Option<f64>,
    /// Which it turned out to be this frame, if it did; and whether it's
    /// down still, past a tap.
    tapped: bool,
    long: bool,
    holding: bool,
}

impl Timed {
    /// A frame of control `c` on `pad`, `dt` on from the last: a hold
    /// after `after` seconds down.
    fn step(&mut self, pad: &PadFrame, c: Control, after: f64, dt: f64) {
        (self.tapped, self.long) = (false, false);
        if pad.went_down(c) {
            (self.since, self.holding) = (Some(0.0), false);
        }
        self.holding &= pad.held(c);
        let Some(t) = self.since else { return };
        self.since = Some(t + dt);
        if !pad.held(c) {
            (self.tapped, self.since) = (true, None);
        } else if t + dt >= after {
            (self.long, self.holding, self.since) = (true, true, None);
        }
    }
}

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
    /// What's in front of them to use, and what was as a pad's X last
    /// went down (what that press is for).
    prompt: Prompt,
    began: Prompt,
    /// How long the look stick's been held hard over (its turn builds).
    pub hard_over: f64,
    /// The control that's the bag tapped and the map held, and the one
    /// that reloads and uses: how long each has been down.
    view: Timed,
    using: Timed,
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
        match self.two_way() {
            Some(c) => self.view.step(&pad, c, HOLD, dt),
            None => self.view = Timed::default(),
        }
        match self.shared() {
            Some(c) => {
                if pad.went_down(c) {
                    self.began = self.prompt;
                }
                self.using.step(&pad, c, USE_HOLD, dt);
            }
            None => self.using = Timed::default(),
        }
    }

    /// The control that's both the bag and the map, if one is.
    fn two_way(&self) -> Option<Control> {
        self.binds.get(Action::Inventory).filter(|c| self.binds.get(Action::Map) == Some(*c))
    }

    /// The control that both reloads and uses what's in front of them, if
    /// one does (and the pad's not the bag's).
    fn shared(&self) -> Option<Control> {
        self.binds.get(Action::Reload).filter(|c| !self.rummaging && self.binds.get(Action::Interact) == Some(*c))
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

    /// What's in front of them to use (a prompt's up): a pad's X uses it
    /// (pressed, or held) instead of reloading.
    pub fn set_prompt(&mut self, prompt: Prompt) {
        self.prompt = prompt;
    }

    /// Whether the keyboard and mouse are theirs.
    pub fn has_keys(&self) -> bool {
        self.keys.is_some()
    }

    /// The pad control `a` is on this frame, where it has one of its own:
    /// none at all while the pad's the bag's, and none for the controls
    /// two actions share (told apart in [`Input::pad_pressed`] and
    /// [`Input::pad_held`]).
    fn control(&self, a: Action) -> Option<Control> {
        match a {
            _ if self.rummaging => None,
            Action::Inventory | Action::Map if self.two_way().is_some() => None,
            Action::Reload | Action::Interact if self.shared().is_some() => None,
            _ => self.binds.get(a),
        }
    }

    /// Whether `a`'s button went down on the pad this frame (a press
    /// taken). Where two share a control: the bag's is a tap of it and the
    /// map's a hold; a reload is a press with nothing in front of them, or
    /// a tap with something there that's used by holding, and using that
    /// is a press, or the hold.
    pub fn pad_pressed(&mut self, a: Action) -> bool {
        let take = std::mem::take::<bool>;
        match (a, self.shared()) {
            (Action::Inventory, _) if self.two_way().is_some() => !self.mapped && take(&mut self.view.tapped),
            (Action::Map, _) if self.two_way().is_some() => take(&mut self.view.long) || (self.mapped && take(&mut self.view.tapped)),
            (Action::Reload, Some(c)) => match self.began {
                Prompt::None => self.pad.take(c),
                Prompt::Press => false,
                Prompt::Hold => take(&mut self.using.tapped),
            },
            (Action::Interact, Some(c)) => match self.began {
                Prompt::None => false,
                Prompt::Press => self.pad.take(c),
                Prompt::Hold => take(&mut self.using.long),
            },
            _ => self.control(a).is_some_and(|c| self.pad.take(c)),
        }
    }

    /// Whether `a`'s button is held down on the pad. The one that reloads
    /// and uses is using while something's in front of them: from the
    /// press that used it, else once it's been down longer than a tap.
    fn pad_held(&self, a: Action) -> bool {
        match (a, self.shared()) {
            (Action::Interact, Some(c)) => self.prompt != Prompt::None && self.pad.held(c) && (self.began == Prompt::Press || self.using.holding),
            (Action::Reload, Some(c)) => self.prompt == Prompt::None && self.pad.held(c),
            _ => self.control(a).is_some_and(|c| self.pad.held(c)),
        }
    }

    /// Whether a bind counts now: a mouse button only while the pointer's
    /// locked.
    fn counts(&self, b: Bind) -> bool {
        self.locked || !matches!(b, Bind::Mouse(_))
    }

    /// Whether `a`'s key or button is held down.
    pub fn held(&self, ui: &Ui, a: Action) -> bool {
        self.pad_held(a) || self.keys.is_some_and(|k| self.counts(k.get(a)) && k.held(ui, a))
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
            i.set_prompt(Prompt::Press);
            i.update(ui, Some(Keys::default()), true, x, 0.016);
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
    fn where_x_is_held_to_use_a_tap_of_it_still_reloads() {
        let mut h = Harness::new(800.0, 600.0);
        h.frame(|ui| {
            let (held, rest) = (pad::frame_holding(&[Button::West]), PadFrame::default());
            let down = held.merge(pad::frame_with(&[Button::West]));
            // At a window to be boarded up, X tapped: a reload, as it's
            // let go; nothing's nailed.
            let mut i = Input::default();
            i.set_prompt(Prompt::Hold);
            i.update(ui, None, true, down, 0.016);
            assert!(!i.pressed(ui, Action::Reload) && !i.held(ui, Action::Interact), "not yet: it may be a hold");
            i.update(ui, None, true, held, 0.05);
            assert!(!i.held(ui, Action::Interact));
            i.update(ui, None, true, rest, 0.016);
            assert!(i.pressed(ui, Action::Reload) && !i.pressed(ui, Action::Interact));
            assert!(!i.pressed(ui, Action::Reload), "once");
            // Held: the boards are nailed while it's down, and no reload
            // as it's let go.
            i.update(ui, None, true, down, 0.016);
            i.update(ui, None, true, held, 0.25);
            assert!(i.held(ui, Action::Interact) && i.pressed(ui, Action::Interact) && !i.pressed(ui, Action::Reload));
            i.update(ui, None, true, held, 1.0);
            assert!(i.held(ui, Action::Interact));
            i.update(ui, None, true, rest, 0.016);
            assert!(!i.held(ui, Action::Interact) && !i.pressed(ui, Action::Reload));
            // The aim swung off the window before the tap was let go:
            // still a reload.
            i.update(ui, None, true, down, 0.016);
            i.set_prompt(Prompt::None);
            i.update(ui, None, true, rest, 0.016);
            assert!(i.pressed(ui, Action::Reload));
            // Nothing in front of them: a reload at once; held on up to a
            // window, the boards go on.
            i.update(ui, None, true, down, 0.016);
            assert!(i.pressed(ui, Action::Reload));
            i.set_prompt(Prompt::Hold);
            i.update(ui, None, true, held, 0.3);
            assert!(i.held(ui, Action::Interact) && !i.pressed(ui, Action::Reload));
            // Something a press uses (a gun on the wall): used at once.
            i.update(ui, None, true, rest, 0.016);
            i.set_prompt(Prompt::Press);
            i.update(ui, None, true, down, 0.016);
            assert!(i.pressed(ui, Action::Interact) && i.held(ui, Action::Interact) && !i.pressed(ui, Action::Reload));
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
