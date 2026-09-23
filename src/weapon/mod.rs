//! The hands as a machine of states: ready, firing, reloading, striking,
//! and bringing a weapon up or putting it away. What's in them is any
//! weapon (`spec.rs`), bare fists among them. They're fed the frame's
//! presses and say what happened (a shot, a click, the magazine out, the
//! blow landing) at the moments the weapon's clips show them. They know
//! nothing of the world: the caller turns what they say into rays, sounds
//! and hits, and says what to take up once they're empty.

mod spec;
#[cfg(test)]
mod tests;

pub use spec::{Spec, Weapon};

use crate::loot::bag::Slot;

/// Which clip shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Clip {
    Idle,
    Fire,
    Reload,
    Bash,
    /// Coming up into view, going down out of it, and gone (waiting to be
    /// told what to take up next).
    Draw,
    Holster,
    Stowed,
}

impl Clip {
    /// Its name in the weapon's viewmodel (the hands come up and go down
    /// in their idle).
    pub fn name(self) -> &'static str {
        match self {
            Clip::Idle | Clip::Draw | Clip::Holster | Clip::Stowed => "Idle",
            Clip::Fire => "Fire",
            Clip::Reload => "Reload",
            Clip::Bash => "Bash",
        }
    }
}

/// What happened this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    Shoot,
    DryFire,
    MagOut,
    MagIn,
    SlideRack,
    Swing,
    Strike,
}

/// The frame's presses.
#[derive(Clone, Copy, Debug, Default)]
pub struct Trigger {
    pub fire: bool,
    pub reload: bool,
    pub melee: bool,
}

#[derive(Clone, Debug)]
pub struct Hands {
    pub weapon: Weapon,
    /// The slot it came from; none for bare fists.
    pub held: Option<Slot>,
    pub mag: u32,
    /// Rounds carried to reload with.
    pub spare: u32,
    clip: Clip,
    /// Seconds into the clip.
    t: f64,
    /// Until it may fire again.
    gap: f64,
    /// How fast a reload goes, a multiple of the usual (quick hands).
    pub reload_speed: f64,
    /// What to take up once what's held is put away (none: bare fists).
    next: Option<Option<Slot>>,
}

impl Default for Hands {
    fn default() -> Self {
        Self { weapon: Weapon::Fists, held: None, mag: 0, spare: 0, clip: Clip::Idle, t: 0.0, gap: 0.0, reload_speed: 1.0, next: None }
    }
}

impl Hands {
    pub fn spec(&self) -> &'static Spec {
        self.weapon.spec()
    }

    /// The clip showing and how far into it.
    pub fn clip(&self) -> (Clip, f64) {
        (self.clip, self.t)
    }

    /// Busy with something a shot can't cut short.
    pub fn busy(&self) -> bool {
        !matches!(self.clip, Clip::Idle | Clip::Fire)
    }

    /// How far out of view, 0 (in hand) to 1 (put away).
    pub fn stowed_amount(&self) -> f64 {
        let spec = self.spec();
        match self.clip {
            Clip::Draw => 1.0 - (self.t / spec.draw).min(1.0),
            Clip::Holster => (self.t / spec.holster).min(1.0),
            Clip::Stowed => 1.0,
            _ => 0.0,
        }
    }

    /// Room in the magazine and rounds to fill it with.
    fn can_reload(&self) -> bool {
        self.spec().reload.is_some() && self.mag < self.spec().mag && self.spare > 0
    }

    fn start(&mut self, clip: Clip) {
        self.clip = clip;
        self.t = 0.0;
    }

    /// Put what's held away, to take up what's in `to` (none: bare fists)
    /// next. Not mid-blow; nor for what's already in hand. Whether it's
    /// going.
    pub fn put_away(&mut self, to: Option<Slot>) -> bool {
        match self.clip {
            Clip::Bash => false,
            Clip::Holster | Clip::Stowed => {
                self.next = Some(to);
                true
            }
            _ if to == self.held => false,
            _ => {
                self.next = Some(to);
                self.start(Clip::Holster);
                true
            }
        }
    }

    /// Whether putting away is under way (or done), and what's to come up.
    pub fn switching(&self) -> Option<Option<Slot>> {
        matches!(self.clip, Clip::Holster | Clip::Stowed).then_some(self.next).flatten()
    }

    /// Put away, and waiting: what's to be taken up.
    pub fn stowed(&self) -> Option<Option<Slot>> {
        (self.clip == Clip::Stowed).then_some(self.next).flatten()
    }

    /// Take up `weapon` (from `held`, `mag` rounds in it) and bring it up.
    pub fn take_up(&mut self, held: Option<Slot>, weapon: Weapon, mag: u32) {
        self.weapon = weapon;
        self.held = held;
        self.mag = mag.min(weapon.spec().mag);
        self.gap = 0.0;
        self.next = None;
        self.start(Clip::Draw);
    }

    /// Move on by `dt` with this frame's presses; what happened.
    pub fn update(&mut self, input: Trigger, dt: f64) -> Vec<Act> {
        let mut acts = Vec::new();
        let spec = self.spec();
        let before = self.t;
        // A reload runs quicker in quick hands (the animation with it).
        self.t += if self.clip == Clip::Reload { dt * self.reload_speed } else { dt };
        self.gap = (self.gap - dt).max(0.0);
        let crossed = |at: f64| before < at && self.t >= at;
        match self.clip {
            Clip::Reload => {
                let reload = spec.reload.expect("only a gun reloads");
                acts.extend(reload.marks.iter().filter(|(at, _)| crossed(*at)).map(|(_, act)| *act));
                if self.t >= reload.time {
                    let taken = (spec.mag - self.mag).min(self.spare);
                    self.mag += taken;
                    self.spare -= taken;
                    self.start(Clip::Idle);
                }
            }
            Clip::Bash => {
                for (at, act) in [(spec.bash.swing_at, Act::Swing), (spec.bash.strike_at, Act::Strike)] {
                    if crossed(at) {
                        acts.push(act);
                    }
                }
                if self.t >= spec.bash.time {
                    self.start(Clip::Idle);
                }
            }
            Clip::Fire if spec.shot.is_none_or(|s| self.t >= s.time) => self.start(Clip::Idle),
            Clip::Draw if self.t >= spec.draw => self.start(Clip::Idle),
            Clip::Holster if self.t >= spec.holster => self.start(Clip::Stowed),
            _ => {}
        }
        if self.busy() {
            return acts;
        }
        if input.melee {
            self.start(Clip::Bash);
        } else if input.reload && self.can_reload() {
            self.start(Clip::Reload);
        } else if input.fire && self.gap <= 0.0 {
            match spec.shot {
                // No gun: the trigger strikes.
                None => self.start(Clip::Bash),
                Some(_) if self.mag > 0 => {
                    self.mag -= 1;
                    self.gap = spec.shot.map_or(0.0, |s| s.gap);
                    self.start(Clip::Fire);
                    acts.push(Act::Shoot);
                }
                Some(_) => {
                    acts.push(Act::DryFire);
                    if self.can_reload() {
                        self.start(Clip::Reload);
                    }
                }
            }
        }
        acts
    }
}
