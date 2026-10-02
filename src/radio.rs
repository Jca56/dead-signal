//! The handheld radio every player carries in a holdout: pulled out, the
//! gun's put away and the handset comes up in the right hand in its
//! place; keyed, it's brought to the mouth and the talk button pressed;
//! put away, it goes down and the gun comes back. A machine of states,
//! like the hands (`weapon`): it knows nothing of the world, and says
//! what's to be heard at the moments its clips show them. How it's drawn
//! is the viewmodel's (`viewmodel`), in the weapon's place.

/// How long it takes to come up and to go down, and how long keying it
/// takes, seconds.
const RAISE: f64 = 0.28;
const LOWER: f64 = 0.16;
pub const KEY: f64 = 1.0;
/// Into keying it, when the talk button goes down and when it's let go.
const TALK_AT: f64 = 0.3;
const OVER_AT: f64 = 0.8;

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
    /// The talk button down; and let go, the message sent.
    Talk,
    Over,
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
    /// Seconds into the state.
    t: f64,
}

impl Radio {
    /// Whether it's out, or on its way out or away: the hands are its,
    /// not a weapon's.
    pub fn out(&self) -> bool {
        self.state != State::Away
    }

    /// Wanted or in hand (not on its way down): what was held stays put
    /// away.
    pub fn holds(&self) -> bool {
        !matches!(self.state, State::Away | State::Lower)
    }

    /// Up in the hand, ready to be keyed.
    pub fn ready(&self) -> bool {
        self.state == State::Up
    }

    fn start(&mut self, state: State) {
        (self.state, self.t) = (state, 0.0);
    }

    /// Pull it out (once the gun's away).
    pub fn pull(&mut self) {
        if self.state == State::Away {
            self.start(State::Wanted);
        }
    }

    /// Put it away, from wherever it's got to. Whether it was in view,
    /// and is on its way down now.
    pub fn put_away(&mut self) -> bool {
        match self.state {
            State::Away | State::Lower => return false,
            State::Wanted => {
                self.start(State::Away);
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
        true
    }

    /// Key it: brought to the mouth, the talk button pressed. Whether it
    /// was (it's up, and not being keyed already).
    pub fn key(&mut self) -> bool {
        let ready = self.ready();
        if ready {
            self.start(State::Key);
        }
        ready
    }

    /// Move on by `dt`, the hands `empty` (the gun put away) or not; what's
    /// heard.
    pub fn update(&mut self, empty: bool, dt: f64) -> Vec<Cue> {
        let mut cues = Vec::new();
        let before = self.t;
        self.t += dt;
        let crossed = |at: f64| before < at && self.t >= at;
        match self.state {
            State::Away => {}
            State::Wanted if empty => {
                self.start(State::Raise);
                cues.push(Cue::On);
            }
            State::Wanted => {}
            State::Raise if self.t >= RAISE => self.start(State::Up),
            State::Key => {
                for (at, cue) in [(TALK_AT, Cue::Talk), (OVER_AT, Cue::Over)] {
                    if crossed(at) {
                        cues.push(cue);
                    }
                }
                if self.t >= KEY {
                    self.start(State::Up);
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
mod tests {
    use super::*;

    const DT: f64 = 1.0 / 60.0;

    /// `radio` run on for `seconds`, the hands empty: everything heard.
    fn run(radio: &mut Radio, seconds: f64) -> Vec<Cue> {
        (0..(seconds / DT).round() as u32).flat_map(|_| radio.update(true, DT)).collect()
    }

    #[test]
    fn it_waits_for_the_gun_to_be_put_away_then_comes_up() {
        let mut r = Radio::default();
        assert!(!r.out() && r.shown().is_none());
        r.pull();
        assert!(r.out() && r.shown().is_none(), "wanted, not yet in view");
        assert!(r.update(false, 1.0).is_empty() && r.shown().is_none(), "the gun's still in hand");
        assert_eq!(r.update(true, DT), [Cue::On]);
        assert_eq!(r.shown().map(|s| s.stowed), Some(1.0), "from out of view");
        run(&mut r, RAISE * 0.5);
        assert!(r.shown().is_some_and(|s| s.stowed > 0.3 && s.stowed < 0.7));
        run(&mut r, RAISE);
        assert!(r.ready());
        assert_eq!(r.shown(), Some(Shown { clip: "Idle", t: None, stowed: 0.0 }));
    }

    #[test]
    fn keyed_it_talks_and_is_ready_again() {
        let mut r = Radio::default();
        assert!(!r.key(), "not while it's away");
        r.pull();
        run(&mut r, RAISE + 0.1);
        assert!(r.key() && !r.key(), "once at a time");
        assert_eq!(r.shown().map(|s| s.clip), Some("Key"));
        assert_eq!(run(&mut r, KEY + 0.1), [Cue::Talk, Cue::Over]);
        assert!(r.ready());
    }

    #[test]
    fn put_away_it_goes_down_from_where_it_is_and_the_hands_are_free() {
        let mut r = Radio::default();
        // Wanted and not yet up: just not wanted.
        r.pull();
        r.put_away();
        assert!(!r.out());
        // Half up: down from half way, in half the time.
        r.pull();
        run(&mut r, DT + RAISE * 0.5);
        let half = r.shown().map(|s| s.stowed).unwrap();
        r.put_away();
        assert!(r.out() && (r.shown().unwrap().stowed - half).abs() < 0.05);
        run(&mut r, LOWER * 0.5 + 2.0 * DT);
        assert!(!r.out() && r.shown().is_none());
        // Up, and mid-word: down all the way.
        r.pull();
        run(&mut r, RAISE + 0.1);
        r.key();
        run(&mut r, 0.4);
        r.put_away();
        assert_eq!(r.shown().map(|s| s.stowed), Some(0.0));
        assert!(run(&mut r, LOWER + DT).is_empty(), "cut short: nothing more's heard");
        assert!(!r.out());
    }
}
