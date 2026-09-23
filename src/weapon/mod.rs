//! The hands as a machine of states: ready, firing, reloading, striking,
//! and bringing a weapon up or putting it away; and a gun raised to the
//! eye or not (only while ready or firing). What's in them is any
//! weapon (`spec.rs`), bare fists among them. They're fed the frame's
//! presses and say what happened (a shot, a click, the magazine out, the
//! blow landing) at the moments the weapon's clips show them. They know
//! nothing of the world: the caller turns what they say into rays, sounds
//! and hits, and says what to take up once they're empty.

mod spec;
#[cfg(test)]
mod tests;

pub use spec::{Falloff, Reload, Spec, Weapon};

use crate::loot::bag::Slot;

/// How far up the sights are before a scope's view comes in.
const SCOPE_FROM: f64 = 0.8;

/// Which clip shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Clip {
    Idle,
    Fire,
    /// A magazine out and in; or, a round at a time, getting ready, a
    /// round in (again and again), and done.
    Reload,
    ReloadStart,
    ReloadShell,
    ReloadEnd,
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
            Clip::ReloadStart => "ReloadStart",
            Clip::ReloadShell => "ReloadShell",
            Clip::ReloadEnd => "ReloadEnd",
            Clip::Bash => "Bash",
        }
    }

    fn reloading(self) -> bool {
        matches!(self, Clip::Reload | Clip::ReloadStart | Clip::ReloadShell | Clip::ReloadEnd)
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
    /// A pump racked back and home; a bolt worked; a round (a shell)
    /// pushed in.
    Pump,
    Bolt,
    ShellIn,
    Swing,
    Strike,
}

/// The frame's presses.
#[derive(Clone, Copy, Debug, Default)]
pub struct Trigger {
    pub fire: bool,
    pub reload: bool,
    pub melee: bool,
    /// Held: the sights up.
    pub aim: bool,
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
    /// How far the sights are raised to the eye, 0–1.
    aim: f64,
    /// The trigger cut a reload short: fire as soon as it's done.
    fire_after: bool,
}

impl Default for Hands {
    fn default() -> Self {
        Self { weapon: Weapon::Fists, held: None, mag: 0, spare: 0, clip: Clip::Idle, t: 0.0, gap: 0.0, reload_speed: 1.0, next: None, aim: 0.0, fire_after: false }
    }
}

impl Hands {
    pub fn spec(&self) -> &'static Spec {
        self.weapon.spec()
    }

    /// How far the sights are up, 0–1, eased in and out.
    pub fn aim(&self) -> f64 {
        self.aim * self.aim * (3.0 - 2.0 * self.aim)
    }

    /// How far into the scope's view, 0–1 (none but for a scoped gun, the
    /// last of the way up).
    pub fn scoped(&self) -> f64 {
        if self.spec().shot.is_some_and(|s| s.scope) { ((self.aim() - SCOPE_FROM) / (1.0 - SCOPE_FROM)).clamp(0.0, 1.0) } else { 0.0 }
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

    fn start_reload(&mut self) {
        match self.spec().reload {
            Some(Reload::Magazine { .. }) => self.start(Clip::Reload),
            Some(Reload::Rounds { .. }) => self.start(Clip::ReloadStart),
            None => {}
        }
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
        self.aim = 0.0;
        self.fire_after = false;
        self.start(Clip::Draw);
    }

    /// Move on by `dt` with this frame's presses; what happened.
    pub fn update(&mut self, input: Trigger, dt: f64) -> Vec<Act> {
        let mut acts = Vec::new();
        let spec = self.spec();
        let before = self.t;
        // A reload runs quicker in quick hands (the animation with it).
        self.t += if self.clip.reloading() { dt * self.reload_speed } else { dt };
        self.gap = (self.gap - dt).max(0.0);
        let crossed = |at: f64| before < at && self.t >= at;
        let marks = |marks: &[(f64, Act)]| marks.iter().filter(|(at, _)| crossed(*at)).map(|(_, act)| *act).collect::<Vec<Act>>();
        let room = |h: &Self| h.mag < spec.mag && h.spare > 0;
        match (self.clip, spec.reload) {
            (Clip::Reload, Some(Reload::Magazine { time, marks: m })) => {
                acts.extend(marks(m));
                if self.t >= time {
                    let taken = (spec.mag - self.mag).min(self.spare);
                    self.mag += taken;
                    self.spare -= taken;
                    self.start(Clip::Idle);
                }
            }
            (Clip::ReloadStart, Some(Reload::Rounds { start, start_marks, .. })) => {
                acts.extend(marks(start_marks));
                if self.t >= start {
                    self.start(Clip::ReloadShell);
                }
            }
            (Clip::ReloadShell, Some(Reload::Rounds { each, insert_at, .. })) => {
                if crossed(insert_at) && room(self) {
                    self.mag += 1;
                    self.spare -= 1;
                    acts.push(Act::ShellIn);
                }
                if self.t >= each {
                    self.start(if room(self) { Clip::ReloadShell } else { Clip::ReloadEnd });
                }
            }
            (Clip::ReloadEnd, Some(Reload::Rounds { end, end_marks, .. })) => {
                acts.extend(marks(end_marks));
                if self.t >= end {
                    self.start(Clip::Idle);
                }
            }
            (Clip::Bash, _) => {
                for (at, act) in [(spec.bash.swing_at, Act::Swing), (spec.bash.strike_at, Act::Strike)] {
                    if crossed(at) {
                        acts.push(act);
                    }
                }
                if self.t >= spec.bash.time {
                    self.start(Clip::Idle);
                }
            }
            (Clip::Fire, _) => {
                acts.extend(marks(spec.shot.map_or(&[], |s| s.marks)));
                if spec.shot.is_none_or(|s| self.t >= s.time) {
                    self.start(Clip::Idle);
                }
            }
            (Clip::Draw, _) if self.t >= spec.draw => self.start(Clip::Idle),
            (Clip::Holster, _) if self.t >= spec.holster => self.start(Clip::Stowed),
            _ => {}
        }
        // The trigger cuts a round-at-a-time reload short, to fire what's
        // in as soon as the gun's ready.
        if input.fire && matches!(self.clip, Clip::ReloadStart | Clip::ReloadShell) && self.mag > 0 {
            self.fire_after = true;
            self.start(Clip::ReloadEnd);
        }
        // The sights come up (a gun, ready or firing) and go down.
        if let Some(shot) = spec.shot {
            let up = input.aim && !self.busy();
            let step = dt / shot.aim_time.max(1e-3);
            self.aim = if up { (self.aim + step).min(1.0) } else { (self.aim - step).max(0.0) };
        }
        if self.busy() {
            return acts;
        }
        let fire = input.fire || std::mem::take(&mut self.fire_after);
        if input.melee {
            self.start(Clip::Bash);
        } else if input.reload && self.can_reload() {
            self.start_reload();
        } else if fire && self.gap <= 0.0 {
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
                        self.start_reload();
                    }
                }
            }
        }
        acts
    }
}
