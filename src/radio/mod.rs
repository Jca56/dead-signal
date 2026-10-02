//! The handheld radio every player carries in a holdout: pulled out, the
//! gun's put away and the handset comes up in the right hand in its
//! place; a code of arrows is punched in on it (`codes.rs`: what there is
//! to call for, and each one's code; `card.rs`: the card of them drawn
//! beside it); a whole code keys it, brought to the mouth with the talk
//! button pressed, and what was called for goes out as the button's let
//! go, paid for in signal (`signal.rs`: charged by kills; `meter.rs`: its
//! bars drawn); put away, it goes down and the gun comes back. A machine of
//! states, like the hands (`weapon`): it knows nothing of the world, and
//! says what's to be heard at the moments its clips show them. How it's
//! drawn is the viewmodel's (`viewmodel`), in the weapon's place.

pub mod card;
pub mod codes;
pub mod meter;
pub mod signal;

use codes::{Arrow, Call, Dial, Dialed};
use signal::Signal;

/// How long it takes to come up and to go down, and how long keying it
/// takes, seconds.
const RAISE: f64 = 0.28;
const LOWER: f64 = 0.16;
pub const KEY: f64 = 1.0;
/// Into keying it, when the talk button goes down and when it's let go.
const TALK_AT: f64 = 0.3;
const OVER_AT: f64 = 0.8;
/// How long its card takes to come up, and a wrong arrow shows on it.
const CARD_IN: f64 = 0.12;
pub const WRONG_FOR: f64 = 0.35;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum State {
    /// Not in hand.
    #[default]
    Away,
    /// Wanted, once the gun's put away.
    Wanted,
    Raise,
    Up,
    Key,
    Lower,
}

/// What's heard of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cue {
    /// It's switched on, coming up.
    On,
    /// The talk button down; and let go, what was called for sent.
    Talk,
    Over(Call),
}

/// The handset as it's drawn: which of its clips, how far into it (an
/// idle loops on the game's clock instead), and how far out of view, 0
/// (in hand) to 1 (away).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shown {
    pub clip: &'static str,
    pub t: Option<f64>,
    pub stowed: f64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Radio {
    state: State,
    /// Seconds into the state, and since it was pulled out.
    t: f64,
    since: f64,
    /// The code being punched in; what it came to, to be called in (it's
    /// keyed once it's up), or being called in now; and how long since a
    /// wrong arrow.
    dial: Dial,
    calling: Option<Call>,
    wrong: Option<f64>,
    /// What there is to call things in with.
    pub signal: Signal,
}

impl Radio {
    /// Whether it's out, or on its way out or away: the hands are its,
    /// not a weapon's.
    pub fn out(&self) -> bool {
        self.state != State::Away
    }

    /// Wanted or in hand (not on its way down): what was held stays put
    /// away, and the feet stand still to dial.
    pub fn holds(&self) -> bool {
        !matches!(self.state, State::Away | State::Lower)
    }

    /// Whether arrows are being taken: from the moment it's pulled out
    /// (the fingers needn't wait for it to come up), till a code's in.
    pub fn dialing(&self) -> bool {
        matches!(self.state, State::Wanted | State::Raise | State::Up) && self.calling.is_none()
    }

    /// What's punched in so far; what's being called in (or about to be);
    /// and how long since a wrong arrow, while that shows.
    pub fn dial(&self) -> &Dial {
        &self.dial
    }

    pub fn calling(&self) -> Option<Call> {
        self.calling
    }

    pub fn wrong(&self) -> Option<f64> {
        self.wrong.filter(|&t| t < WRONG_FOR)
    }

    /// How much its card shows, 0–1: up as soon as it's pulled out, gone
    /// as it goes down.
    pub fn card(&self) -> f64 {
        match self.state {
            State::Away => 0.0,
            State::Lower => 1.0 - (self.t / LOWER).min(1.0),
            _ => (self.since / CARD_IN).min(1.0),
        }
    }

    fn start(&mut self, state: State) {
        (self.state, self.t) = (state, 0.0);
    }

    /// Pull it out (once the gun's away).
    pub fn pull(&mut self) {
        if self.state == State::Away {
            *self = Self { state: State::Wanted, signal: self.signal, ..Self::default() };
        }
    }

    /// Put it away, from wherever it's got to (a code half in is
    /// forgotten; one not yet sent isn't). Whether it was in view, and is
    /// on its way down now.
    pub fn put_away(&mut self) -> bool {
        match self.state {
            State::Away | State::Lower => return false,
            State::Wanted => {
                self.drop_it();
                return false;
            }
            // (Half up, it goes down from there.)
            State::Raise => {
                let up = (self.t / RAISE).min(1.0);
                self.start(State::Lower);
                self.t = (1.0 - up) * LOWER;
            }
            State::Up | State::Key => self.start(State::Lower),
        }
        self.dial.clear();
        self.calling = None;
        true
    }

    /// Away at once, wherever it was (they're down and out): its signal's
    /// kept.
    pub fn drop_it(&mut self) {
        *self = Self { signal: self.signal, ..Self::default() };
    }

    /// What was just dialled can't be called in (there's not the signal
    /// for it): forgotten, and shown as a wrong arrow is.
    pub fn refuse(&mut self) {
        self.dial.clear();
        self.calling = None;
        self.wrong = Some(0.0);
    }

    /// Punch `arrow` in, if arrows are being taken; what it came to. A
    /// whole code's called in: it's keyed, as soon as it's up.
    pub fn press(&mut self, arrow: Arrow) -> Option<Dialed> {
        if !self.dialing() {
            return None;
        }
        let dialed = self.dial.press(arrow);
        match dialed {
            Dialed::On => {}
            Dialed::Wrong => self.wrong = Some(0.0),
            Dialed::Called(call) => self.calling = Some(call),
        }
        Some(dialed)
    }

    /// Move on by `dt`, the hands `empty` (the gun put away) or not; what's
    /// heard.
    pub fn update(&mut self, empty: bool, dt: f64) -> Vec<Cue> {
        let mut cues = Vec::new();
        let before = self.t;
        self.t += dt;
        self.since += dt;
        self.wrong = self.wrong.map(|t| t + dt);
        let crossed = |at: f64| before < at && self.t >= at;
        match self.state {
            State::Away => {}
            State::Wanted if empty => {
                self.start(State::Raise);
                cues.push(Cue::On);
            }
            State::Wanted => {}
            State::Raise if self.t >= RAISE => self.start(State::Up),
            // A code's in: brought to the mouth, the talk button pressed.
            State::Up if self.calling.is_some() => self.start(State::Key),
            State::Key => {
                if crossed(TALK_AT) {
                    cues.push(Cue::Talk);
                }
                if let Some(call) = self.calling.filter(|_| crossed(OVER_AT)) {
                    cues.push(Cue::Over(call));
                }
                if self.t >= KEY {
                    self.start(State::Up);
                    self.dial.clear();
                    self.calling = None;
                }
            }
            State::Lower if self.t >= LOWER => self.start(State::Away),
            State::Raise | State::Up | State::Lower => {}
        }
        cues
    }

    /// How it's drawn, while it's in view.
    pub fn shown(&self) -> Option<Shown> {
        let idle = |stowed: f64| Shown { clip: "Idle", t: None, stowed };
        match self.state {
            State::Away | State::Wanted => None,
            State::Raise => Some(idle(1.0 - (self.t / RAISE).min(1.0))),
            State::Up => Some(idle(0.0)),
            State::Key => Some(Shown { clip: "Key", t: Some(self.t), stowed: 0.0 }),
            State::Lower => Some(idle((self.t / LOWER).min(1.0))),
        }
    }
}

#[cfg(test)]
mod tests;
