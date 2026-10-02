//! A pad at a screen of cells (the bag): which way its d-pad, or its left
//! stick pushed over, steps a cursor (held, it steps again and again), and
//! what its face buttons ask: A to pick a thing up and put it down, X to
//! send it straight across (holding something, to drop it), Y to turn it,
//! B to put it back, or the screen away.

use lntrn_sys::gamepad::Button;

use super::pad::{Control, Labels, PadFrame};

/// Held, a step comes again after this long, then this often.
const FIRST: f64 = 0.32;
const AGAIN: f64 = 0.11;
/// The stick steps pushed this far over.
const PUSHED: f64 = 0.6;

pub const PICK: Control = Control::Button(Button::South);
pub const ACROSS: Control = Control::Button(Button::West);
pub const TURN: Control = Control::Button(Button::North);
pub const BACK: Control = Control::Button(Button::East);

/// What a pad asks of the screen in a frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Steer {
    /// A step of the cursor: x right, y down.
    pub step: (i32, i32),
    pub pick: bool,
    pub across: bool,
    pub turn: bool,
    pub back: bool,
    /// Whose names its buttons go by, for the hints.
    pub labels: Labels,
}

impl Steer {
    /// Whether it asked anything.
    pub fn any(&self) -> bool {
        self.step != (0, 0) || self.pick || self.across || self.turn || self.back
    }
}

/// A direction held, stepping again and again.
#[derive(Clone, Copy, Debug, Default)]
pub struct Repeat {
    way: (i32, i32),
    wait: f64,
}

impl Repeat {
    /// Which way `pad` points: the d-pad, else the left stick's stronger
    /// axis, pushed well over.
    fn way(pad: &PadFrame) -> (i32, i32) {
        let held = |b| i32::from(pad.held(Control::Button(b)));
        let dpad = (held(Button::Right) - held(Button::Left), held(Button::Down) - held(Button::Up));
        if dpad != (0, 0) {
            return dpad;
        }
        let stick = pad.left;
        if stick.length() < PUSHED {
            (0, 0)
        } else if stick.x.abs() > stick.y.abs() {
            (stick.x.signum() as i32, 0)
        } else {
            (0, -stick.y.signum() as i32)
        }
    }

    /// The step `pad` asks for this frame, `dt` on from the last: one as
    /// it's first pointed a way, and more while it's held there.
    fn step(&mut self, pad: &PadFrame, dt: f64) -> (i32, i32) {
        let way = Self::way(pad);
        if way != self.way {
            (self.way, self.wait) = (way, FIRST);
            return way;
        }
        if way == (0, 0) {
            return way;
        }
        self.wait -= dt;
        if self.wait > 0.0 {
            return (0, 0);
        }
        self.wait += AGAIN;
        way
    }

    /// What `pad` asks this frame (its presses taken).
    pub fn read(&mut self, pad: &mut PadFrame, dt: f64) -> Steer {
        // (A d-pad tapped between frames still steps.)
        let tapped = |b| i32::from(pad.went_down(Control::Button(b)));
        let quick = (tapped(Button::Right) - tapped(Button::Left), tapped(Button::Down) - tapped(Button::Up));
        let step = match self.step(pad, dt) {
            (0, 0) if Self::way(pad) == (0, 0) => quick,
            step => step,
        };
        Steer { step, pick: pad.take(PICK), across: pad.take(ACROSS), turn: pad.take(TURN), back: pad.take(BACK), labels: pad.labels }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::pad::frame_with;

    #[test]
    fn a_direction_held_steps_once_then_again_and_again() {
        let mut pad = PadFrame::default();
        pad.left = lntrn_math::Vec2::new(0.0, -0.9);
        let mut r = Repeat::default();
        assert_eq!(r.read(&mut pad, 0.016).step, (0, 1), "the stick pulled back steps down");
        assert_eq!(r.read(&mut pad, 0.016).step, (0, 0), "and not again at once");
        let steps: i32 = (0..60).map(|_| r.read(&mut pad, 1.0 / 60.0).step.1).sum();
        assert!((6..=8).contains(&steps), "a second held: {steps} more");
        let mut rest = PadFrame::default();
        assert_eq!(r.read(&mut rest, 0.016).step, (0, 0));
        assert_eq!(r.read(&mut pad, 0.016).step, (0, 1), "let go and pushed again, at once");
    }

    #[test]
    fn a_tap_of_the_d_pad_between_frames_still_steps() {
        let mut pad = frame_with(&[Button::Left, Button::South]);
        let s = Repeat::default().read(&mut pad, 0.016);
        assert_eq!((s.step, s.pick, s.across), ((-1, 0), true, false));
        assert!(!Repeat::default().read(&mut pad, 0.016).pick, "a press is taken once");
    }
}
