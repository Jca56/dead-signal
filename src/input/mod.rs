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
/// it's the other (the bag, and the map or the radio; reloading, and
/// what's used by holding).
const HOLD: f64 = 0.4;
const USE_HOLD: f64 = 0.22;
const BLADE_HOLD: f64 = 0.3;

/// What a pad's weapon button asks: the other gun (as it goes down), or
/// the blade (held on).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Swap {
    Guns,
    Blade,
}

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
    /// The weapon button: how long it's been down.
    swap: Timed,
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
        self.swap.step(&pad, self.binds.next_weapon, BLADE_HOLD, dt);
    }

    /// The control that's both the bag and the map, if one is.
    fn two_way(&self) -> Option<Control> {
        self.binds.get(Action::Inventory).filter(|c| self.binds.get(Action::Map) == Some(*c))
    }

    /// Whether `a` is what that control is held: the map, or (asked for
    /// where there's no map: a holdout) the radio.
    fn on_hold(&self, a: Action) -> bool {
        matches!(a, Action::Map | Action::Radio) && self.two_way().is_some_and(|c| self.binds.get(a) == Some(c))
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
            Action::Inventory if self.two_way().is_some() => None,
            _ if self.on_hold(a) => None,
            Action::Reload | Action::Interact if self.shared().is_some() => None,
            _ => self.binds.get(a),
        }
    }

    /// Whether `a`'s button went down on the pad this frame (a press
    /// taken). Where two share a control: the bag's is a tap of it and the
    /// map's (or the radio's) a hold; a reload is a press with nothing in front of them, or
    /// a tap with something there that's used by holding, and using that
    /// is a press, or the hold.
    pub fn pad_pressed(&mut self, a: Action) -> bool {
        let take = std::mem::take::<bool>;
        match (a, self.shared()) {
            (Action::Inventory, _) if self.two_way().is_some() => !self.mapped && take(&mut self.view.tapped),
            (Action::Map, _) if self.on_hold(a) => take(&mut self.view.long) || (self.mapped && take(&mut self.view.tapped)),
            (Action::Radio, _) if self.on_hold(a) => take(&mut self.view.long),
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

    /// What a pad's Y asks this frame (taken): the other gun as it goes
    /// down, and (still down a moment on) the blade instead.
    pub fn swap(&mut self) -> Option<Swap> {
        if self.rummaging {
            None
        } else if std::mem::take(&mut self.swap.long) {
            Some(Swap::Blade)
        } else {
            self.pad.take(self.binds.next_weapon).then_some(Swap::Guns)
        }
    }

    /// Whether a pad's Y is down and may yet be held for the blade: what
    /// it's put away for isn't settled.
    pub fn swapping(&self) -> bool {
        !self.rummaging && self.swap.since.is_some()
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
mod tests;
