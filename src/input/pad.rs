//! The pads, as the game has them: polled once a frame (`lntrn-sys` finds
//! them as they come and go), each one's frame (its sticks with their dead
//! zones taken out, what's held and what went down), the layout that turns
//! its buttons into actions ([`PadBinds`]), what the buttons are called on
//! screen, the menus driven by the d-pad, and the rumble.

use std::time::Duration;

use lntrn_math::Vec2;
use lntrn_sys::gamepad::{Button, Gamepads, PadEvent, PadState, dead_zone};
use lntrn_ui::{Key, KeyPress, Modifiers, Ui};

use crate::settings::keys::Action;

/// A stick's dead zone: nothing within the inner, all of it from the outer.
const DEAD: f64 = 0.15;
const FULL: f64 = 0.95;
/// A trigger counts as pulled past this, and as let go under that.
const PULLED: f64 = 0.55;
const LET_GO: f64 = 0.4;
/// A stick pushed this far counts as touched.
const TOUCHED: f64 = 0.25;
/// Flicked this far, the left stick is an arrow key in the menus; it has
/// to come back under the other before it can be again.
const FLICK: f64 = 0.6;
const REARM: f64 = 0.3;

/// Every button, in `lntrn-sys`'s order.
const BUTTONS: [Button; 15] = [
    Button::South,
    Button::East,
    Button::West,
    Button::North,
    Button::LeftBumper,
    Button::RightBumper,
    Button::LeftStick,
    Button::RightStick,
    Button::Select,
    Button::Start,
    Button::Guide,
    Button::Up,
    Button::Down,
    Button::Left,
    Button::Right,
];

/// Something on a pad that can be bound: a button, or a trigger.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control {
    Button(Button),
    LeftTrigger,
    RightTrigger,
}

impl Control {
    fn bit(self) -> u32 {
        match self {
            Control::Button(b) => 1 << b as u32,
            Control::LeftTrigger => 1 << 20,
            Control::RightTrigger => 1 << 21,
        }
    }
}

/// Which action each control is, a pad's way: Call of Duty's layout, kept
/// as a table so it can be rebound. The left stick walks and the right
/// looks (`look.rs`); X is both reload and interact, and which it is goes
/// by whether something's in front of the player to use ([`super::Input`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PadBinds {
    actions: [Option<Control>; Action::ALL.len()],
    /// On to the next weapon carried, and pausing.
    pub next_weapon: Control,
    pub pause: Control,
}

impl Default for PadBinds {
    fn default() -> Self {
        let b = |b| Some(Control::Button(b));
        let actions = Action::ALL.map(|a| match a {
            Action::Forward | Action::Back | Action::Left | Action::Right => None,
            Action::Sprint => b(Button::LeftStick),
            Action::Jump => b(Button::South),
            Action::Crouch => b(Button::East),
            Action::Fire => Some(Control::RightTrigger),
            Action::Aim => Some(Control::LeftTrigger),
            Action::Reload | Action::Interact => b(Button::West),
            Action::Bash => b(Button::RightStick),
            Action::FireMode => b(Button::Up),
            // (Y goes through them all.)
            Action::Primary | Action::Sidearm | Action::Melee => None,
            Action::Bandage => b(Button::Left),
            Action::Medkit => b(Button::Right),
            Action::Plate => b(Button::Down),
            Action::Throw => b(Button::RightBumper),
            Action::NextThrowable => b(Button::LeftBumper),
            Action::Inventory => None,
            Action::Map => b(Button::Select),
        });
        Self { actions, next_weapon: Control::Button(Button::North), pause: Control::Button(Button::Start) }
    }
}

impl PadBinds {
    /// The control `a` is on, if it's on one.
    pub fn get(&self, a: Action) -> Option<Control> {
        Action::ALL.iter().position(|&x| x == a).and_then(|i| self.actions[i])
    }
}

/// Whose names a pad's buttons go by.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Labels {
    #[default]
    Xbox,
    PlayStation,
    Nintendo,
}

impl Labels {
    /// By who made the pad (its USB vendor id).
    fn of(vendor: u16) -> Labels {
        match vendor {
            0x054c => Labels::PlayStation,
            0x057e => Labels::Nintendo,
            _ => Labels::Xbox,
        }
    }

    /// What `c` is called on this pad, as the HUD writes it.
    pub fn name(self, c: Control) -> &'static str {
        use Button as B;
        let face = |xbox, ps, nintendo| match self {
            Labels::Xbox => xbox,
            Labels::PlayStation => ps,
            Labels::Nintendo => nintendo,
        };
        match c {
            Control::LeftTrigger => face("LT", "L2", "ZL"),
            Control::RightTrigger => face("RT", "R2", "ZR"),
            Control::Button(b) => match b {
                B::South => face("A", "CROSS", "B"),
                B::East => face("B", "CIRCLE", "A"),
                B::West => face("X", "SQUARE", "Y"),
                B::North => face("Y", "TRIANGLE", "X"),
                B::LeftBumper => face("LB", "L1", "L"),
                B::RightBumper => face("RB", "R1", "R"),
                B::LeftStick => face("LS", "L3", "L-STICK"),
                B::RightStick => face("RS", "R3", "R-STICK"),
                B::Select => face("VIEW", "SHARE", "−"),
                B::Start => face("MENU", "OPTIONS", "+"),
                B::Guide => "HOME",
                B::Up => "D-PAD ↑",
                B::Down => "D-PAD ↓",
                B::Left => "D-PAD ←",
                B::Right => "D-PAD →",
            },
        }
    }
}

/// What a pad (or several, held by one player) did this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PadFrame {
    /// The sticks, their dead zones taken out: x right, y up (pushed away).
    pub left: Vec2,
    pub right: Vec2,
    held: u32,
    pressed: u32,
    /// Whose names its buttons go by.
    pub labels: Labels,
    /// Whether anything on it was touched this frame.
    pub touched: bool,
}

impl PadFrame {
    pub fn held(&self, c: Control) -> bool {
        self.held & c.bit() != 0
    }

    /// Whether `c` went down this frame.
    pub fn went_down(&self, c: Control) -> bool {
        self.pressed & c.bit() != 0
    }

    /// Whether `c` went down this frame; taken, so nothing else acts on it
    /// too.
    pub fn take(&mut self, c: Control) -> bool {
        let down = self.went_down(c);
        self.pressed &= !c.bit();
        down
    }

    /// This one and `other` in one player's hands: whatever either does,
    /// each stick as far as the further of the two, named as the one
    /// touched.
    pub fn merge(self, other: PadFrame) -> PadFrame {
        let further = |a: Vec2, b: Vec2| if b.length() > a.length() { b } else { a };
        PadFrame {
            left: further(self.left, other.left),
            right: further(self.right, other.right),
            held: self.held | other.held,
            pressed: self.pressed | other.pressed,
            labels: if other.touched && !self.touched { other.labels } else { self.labels },
            touched: self.touched || other.touched,
        }
    }
}

/// A pad's frame from how it is now (`state`), the buttons that went down
/// since the last (`down`: a tap in between still counts), and whether its
/// triggers were pulled at the last (`pulled`, kept up to date).
fn frame(state: &PadState, down: &[Button], pulled: &mut [bool; 2], labels: Labels) -> PadFrame {
    let stick = |s: [f64; 2]| {
        let [x, y] = dead_zone(s, DEAD, FULL);
        Vec2::new(x, y)
    };
    let mut f = PadFrame { left: stick(state.left), right: stick(state.right), labels, ..PadFrame::default() };
    for b in BUTTONS.into_iter().filter(|&b| state.held(b)) {
        f.held |= Control::Button(b).bit();
    }
    for &b in down {
        f.pressed |= Control::Button(b).bit();
    }
    for (i, (value, c)) in [(state.left_trigger, Control::LeftTrigger), (state.right_trigger, Control::RightTrigger)].into_iter().enumerate() {
        let was = pulled[i];
        let now = if was { value > LET_GO } else { value > PULLED };
        if now {
            f.held |= c.bit();
        }
        if now && !was {
            f.pressed |= c.bit();
        }
        pulled[i] = now;
    }
    f.touched = f.held != 0 || f.pressed != 0 || f.left.length() > TOUCHED || f.right.length() > TOUCHED;
    f
}

/// A frame with nothing held but `down` gone down on it.
#[cfg(test)]
pub(super) fn frame_with(down: &[Button]) -> PadFrame {
    frame(&PadState::default(), down, &mut [false; 2], Labels::Xbox)
}

/// The left stick in a menu: the arrow key a flick up or down is (once a
/// flick), and whether it's back near the middle, ready for the next.
fn flick(stick: Vec2, armed: bool) -> (Option<Key>, bool) {
    if !armed {
        return (None, stick.length() < REARM);
    }
    if stick.y > FLICK {
        (Some(Key::ArrowUp), false)
    } else if stick.y < -FLICK {
        (Some(Key::ArrowDown), false)
    } else {
        (None, true)
    }
}

/// A shake of the pad: its big motor and its small one (0–1 each), and for
/// how long.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rumble {
    pub strong: f64,
    pub weak: f64,
    pub seconds: f64,
}

impl Rumble {
    /// A shot, kicking the view up `kick` degrees: the harder the gun
    /// kicks, the harder and longer it thumps.
    pub fn shot(kick: f64) -> Rumble {
        Rumble { strong: (kick / 6.0).clamp(0.1, 1.0) * 0.85, weak: (0.2 + kick / 10.0).min(0.7), seconds: 0.06 + kick * 0.015 }
    }

    /// A blow landing on the one holding it.
    pub fn blow() -> Rumble {
        Rumble { strong: 0.75, weak: 0.5, seconds: 0.22 }
    }

    /// A blow of theirs landing on one of the dead.
    pub fn struck() -> Rumble {
        Rumble { strong: 0.2, weak: 0.55, seconds: 0.07 }
    }

    /// A blast, `share` of it (0–1: as near as it gets).
    pub fn blast(share: f64) -> Rumble {
        let share = share.clamp(0.0, 1.0);
        Rumble { strong: share, weak: share * 0.8, seconds: 0.25 + 0.35 * share }
    }

    /// This and `other` both: the stronger of each.
    pub fn add(&mut self, other: Rumble) {
        self.strong = self.strong.max(other.strong);
        self.weak = self.weak.max(other.weak);
        self.seconds = self.seconds.max(other.seconds);
    }
}

/// Every pad plugged in or paired, and what each did this frame.
pub struct Pads {
    pads: Gamepads,
    /// Each pad's frame, and whether its triggers were pulled, by id.
    frames: Vec<(u32, PadFrame, [bool; 2])>,
    /// The left stick's flick in the menus: whether it's ready.
    armed: bool,
}

impl Pads {
    /// The pads there now, and a watch for more.
    pub fn open() -> Self {
        Self { pads: Gamepads::open(), frames: Vec::new(), armed: true }
    }

    /// Take in what the pads did since the last frame.
    pub fn poll(&mut self) {
        let events = self.pads.poll();
        let mut next = Vec::new();
        for pad in self.pads.pads() {
            let id = pad.id();
            let mut pulled = self.frames.iter().find(|f| f.0 == id).map_or([false; 2], |f| f.2);
            let down: Vec<Button> = events
                .iter()
                .filter_map(|e| match *e {
                    PadEvent::Pressed(p, b) if p == id => Some(b),
                    _ => None,
                })
                .collect();
            let f = frame(pad.state(), &down, &mut pulled, Labels::of(pad.vendor_id()));
            next.push((id, f, pulled));
        }
        self.frames = next;
    }

    /// Every pad's frame at once: one player, holding whichever they pick
    /// up.
    pub fn all(&self) -> PadFrame {
        let mut frames = self.frames.iter().map(|f| f.1);
        let first = frames.next().unwrap_or_default();
        frames.fold(first, PadFrame::merge)
    }

    /// Shake every pad that can.
    pub fn rumble(&mut self, r: Rumble) {
        if r.seconds <= 0.0 {
            return;
        }
        let ids: Vec<u32> = self.pads.pads().iter().filter(|p| p.can_rumble()).map(|p| p.id()).collect();
        for id in ids {
            // (A pad gone mid-shake is let go at the next poll.)
            if let Some(pad) = self.pads.get_mut(id) {
                let _ = pad.rumble(r.strong, r.weak, Duration::from_secs_f64(r.seconds));
            }
        }
    }

    /// The pads as keys: the pause button is Esc anywhere (it pauses a run,
    /// and goes back); in the `menus`, the d-pad (or the left stick,
    /// flicked) is the arrow keys, A is Enter and B is Esc.
    pub fn menu_keys(&mut self, ui: &mut Ui, menus: bool) {
        let pad = self.all();
        let (flicked, armed) = flick(pad.left, self.armed);
        self.armed = armed;
        let mut keys = Vec::new();
        if pad.went_down(PadBinds::default().pause) {
            keys.push(Key::Escape);
        }
        if menus {
            let down = |b| pad.went_down(Control::Button(b));
            if down(Button::Up) || flicked == Some(Key::ArrowUp) {
                keys.push(Key::ArrowUp);
            }
            if down(Button::Down) || flicked == Some(Key::ArrowDown) {
                keys.push(Key::ArrowDown);
            }
            if down(Button::South) {
                keys.push(Key::Enter);
            }
            if down(Button::East) {
                keys.push(Key::Escape);
            }
        }
        ui.state.keys.extend(keys.into_iter().map(|key| KeyPress { key, mods: Modifiers::default(), repeat: false, seq: u32::MAX }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pad() -> PadState {
        PadState::default()
    }

    /// A pad at rest but for its sticks and its right trigger.
    fn pushed(left: [f64; 2], right: [f64; 2], trigger: f64) -> PadState {
        let mut s = pad();
        s.left = left;
        s.right = right;
        s.right_trigger = trigger;
        s
    }

    #[test]
    fn a_trigger_counts_past_its_pull_and_stays_till_let_go() {
        let mut pulled = [false; 2];
        let pull = |v: f64| pushed([0.0; 2], [0.0; 2], v);
        let rt = Control::RightTrigger;
        let f = frame(&pull(0.5), &[], &mut pulled, Labels::Xbox);
        assert!(!f.held(rt), "not far enough");
        let f = frame(&pull(0.6), &[], &mut pulled, Labels::Xbox);
        assert!(f.held(rt) && f.went_down(rt));
        let f = frame(&pull(0.45), &[], &mut pulled, Labels::Xbox);
        assert!(f.held(rt) && !f.went_down(rt), "eased off a little, still held, not pulled again");
        let f = frame(&pull(0.3), &[], &mut pulled, Labels::Xbox);
        assert!(!f.held(rt), "let go");
    }

    #[test]
    fn a_tap_between_frames_still_counts_once_taken() {
        let mut f = frame(&pad(), &[Button::West], &mut [false; 2], Labels::Xbox);
        let x = Control::Button(Button::West);
        assert!(!f.held(x) && f.touched, "up again by the frame, but touched");
        assert!(f.take(x));
        assert!(!f.take(x), "taken once");
    }

    #[test]
    fn a_resting_stick_is_nothing_and_two_pads_make_one() {
        let still = frame(&pushed([0.1, -0.05], [0.0; 2], 0.0), &[], &mut [false; 2], Labels::Xbox);
        assert_eq!(still.left, Vec2::ZERO);
        assert!(!still.touched);
        let looking = frame(&pushed([0.0; 2], [0.0, 1.0], 0.0), &[Button::South], &mut [false; 2], Labels::PlayStation);
        let both = still.merge(looking);
        assert!(both.right.y > 0.99 && both.went_down(Control::Button(Button::South)));
        assert_eq!(both.labels, Labels::PlayStation, "named as the one touched");
    }

    #[test]
    fn a_flick_is_one_arrow_till_the_stick_comes_back() {
        let (key, armed) = flick(Vec2::new(0.0, 0.9), true);
        assert_eq!(key, Some(Key::ArrowUp));
        let (key, armed) = flick(Vec2::new(0.0, 0.9), armed);
        assert_eq!(key, None, "held up: once");
        let (_, armed) = flick(Vec2::new(0.0, 0.1), armed);
        assert_eq!(flick(Vec2::new(0.0, -0.8), armed).0, Some(Key::ArrowDown));
    }

    #[test]
    fn every_action_the_pad_has_is_on_its_own_control_but_reload_and_interact() {
        let b = PadBinds::default();
        for a in Action::ALL {
            for o in Action::ALL {
                let shared = a != o && b.get(a).is_some() && b.get(a) == b.get(o);
                assert!(!shared || matches!((a, o), (Action::Reload, Action::Interact) | (Action::Interact, Action::Reload)), "{a:?} and {o:?}");
            }
            assert!(b.get(a) != Some(b.next_weapon) && b.get(a) != Some(b.pause), "{a:?}");
        }
    }

    #[test]
    fn a_harder_kick_thumps_harder_and_shakes_add_up_to_the_stronger() {
        let (smg, rifle) = (Rumble::shot(0.8), Rumble::shot(6.0));
        assert!(rifle.strong > smg.strong * 3.0 && rifle.seconds > smg.seconds);
        let mut r = Rumble::struck();
        r.add(Rumble::blow());
        assert_eq!(r, Rumble { strong: 0.75, weak: 0.55, seconds: 0.22 });
    }
}
