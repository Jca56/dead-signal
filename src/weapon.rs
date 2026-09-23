//! The pistol as a machine of states: ready, firing, reloading, striking.
//! It is fed the frame's presses and says what happened (a shot, a click,
//! the magazine out, the blow landing) at the moments the animations in
//! `arms.glb` show them. It knows nothing of the world: the caller turns
//! what it says into rays, sounds and hits.

/// Rounds in a magazine, and spare ones carried at the start of a run.
pub const MAG: u32 = 12;
pub const START_SPARE: u32 = 24;
/// The quickest it fires again, seconds: none, so it fires as fast as the
/// trigger is pulled (one shot a click).
const FIRE_GAP: f64 = 0.0;
/// How long each animation runs, and when things happen in them.
const FIRE_TIME: f64 = 0.2;
const RELOAD_TIME: f64 = 1.4;
const MAG_OUT_AT: f64 = 0.2;
const MAG_IN_AT: f64 = 0.95;
const RACK_AT: f64 = 1.12;
const MELEE_TIME: f64 = 0.5;
const SWING_AT: f64 = 0.05;
/// Frame 9 of 30 a second: the blow lands.
pub const STRIKE_AT: f64 = 8.0 / 30.0;

/// Which animation shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Clip {
    Idle,
    Fire,
    Reload,
    Melee,
}

impl Clip {
    /// Its name in `arms.glb`.
    pub fn name(self) -> &'static str {
        match self {
            Clip::Idle => "PistolIdle",
            Clip::Fire => "PistolFire",
            Clip::Reload => "PistolReload",
            Clip::Melee => "PistolMelee",
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
pub struct Pistol {
    pub mag: u32,
    /// Rounds carried to reload with.
    pub spare: u32,
    clip: Clip,
    /// Seconds into the clip.
    t: f64,
    /// Until it may fire again.
    gap: f64,
}

impl Default for Pistol {
    fn default() -> Self {
        Self { mag: MAG, spare: START_SPARE, clip: Clip::Idle, t: 0.0, gap: 0.0 }
    }
}

impl Pistol {
    /// The clip showing and how far into it.
    pub fn clip(&self) -> (Clip, f64) {
        (self.clip, self.t)
    }

    /// Busy with something a shot can't cut short.
    pub fn busy(&self) -> bool {
        matches!(self.clip, Clip::Reload | Clip::Melee)
    }

    /// Room in the magazine and rounds to fill it with.
    fn can_reload(&self) -> bool {
        self.mag < MAG && self.spare > 0
    }

    fn start(&mut self, clip: Clip) {
        self.clip = clip;
        self.t = 0.0;
    }

    /// Move on by `dt` with this frame's presses; what happened.
    pub fn update(&mut self, input: Trigger, dt: f64) -> Vec<Act> {
        let mut acts = Vec::new();
        let before = self.t;
        self.t += dt;
        self.gap = (self.gap - dt).max(0.0);
        let crossed = |at: f64| before < at && self.t >= at;
        match self.clip {
            Clip::Reload => {
                for (at, act) in [(MAG_OUT_AT, Act::MagOut), (MAG_IN_AT, Act::MagIn), (RACK_AT, Act::SlideRack)] {
                    if crossed(at) {
                        acts.push(act);
                    }
                }
                if self.t >= RELOAD_TIME {
                    let taken = (MAG - self.mag).min(self.spare);
                    self.mag += taken;
                    self.spare -= taken;
                    self.start(Clip::Idle);
                }
            }
            Clip::Melee => {
                for (at, act) in [(SWING_AT, Act::Swing), (STRIKE_AT, Act::Strike)] {
                    if crossed(at) {
                        acts.push(act);
                    }
                }
                if self.t >= MELEE_TIME {
                    self.start(Clip::Idle);
                }
            }
            Clip::Fire if self.t >= FIRE_TIME => self.start(Clip::Idle),
            _ => {}
        }
        if self.busy() {
            return acts;
        }
        if input.melee {
            self.start(Clip::Melee);
        } else if input.reload && self.can_reload() {
            self.start(Clip::Reload);
        } else if input.fire && self.gap <= 0.0 {
            if self.mag > 0 {
                self.mag -= 1;
                self.gap = FIRE_GAP;
                self.start(Clip::Fire);
                acts.push(Act::Shoot);
            } else {
                acts.push(Act::DryFire);
                if self.can_reload() {
                    self.start(Clip::Reload);
                }
            }
        }
        acts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f64 = 1.0 / 60.0;

    fn run(p: &mut Pistol, input: Trigger, frames: usize) -> Vec<Act> {
        let mut all = Vec::new();
        for i in 0..frames {
            // A press is one frame's.
            let now = if i == 0 { input } else { Trigger::default() };
            all.extend(p.update(now, DT));
        }
        all
    }

    const FIRE: Trigger = Trigger { fire: true, reload: false, melee: false };

    #[test]
    fn fires_as_fast_as_the_trigger_is_pulled() {
        let mut p = Pistol::default();
        assert_eq!(p.update(FIRE, DT), vec![Act::Shoot]);
        assert_eq!(p.update(FIRE, DT), vec![Act::Shoot], "the very next frame");
        assert!(p.update(Trigger::default(), DT).is_empty(), "and only when pulled");
        assert_eq!(p.mag, MAG - 2);
    }

    #[test]
    fn empty_it_clicks_and_reloads_itself() {
        let mut p = Pistol { mag: 0, ..Pistol::default() };
        let acts = run(&mut p, FIRE, (RELOAD_TIME * 60.0) as usize + 5);
        assert_eq!(acts, vec![Act::DryFire, Act::MagOut, Act::MagIn, Act::SlideRack]);
        assert_eq!((p.mag, p.spare), (MAG, START_SPARE - MAG));
        assert_eq!(p.clip().0, Clip::Idle);
    }

    #[test]
    fn a_reload_takes_only_what_is_carried() {
        let mut p = Pistol { mag: 4, spare: 5, ..Pistol::default() };
        run(&mut p, Trigger { reload: true, ..Default::default() }, (RELOAD_TIME * 60.0) as usize + 5);
        assert_eq!((p.mag, p.spare), (9, 0));
        // Nothing left to load: the reload key does nothing, an empty
        // trigger only clicks.
        run(&mut p, Trigger { reload: true, ..Default::default() }, 2);
        assert_eq!(p.clip().0, Clip::Idle);
        let mut p = Pistol { mag: 0, spare: 0, ..Pistol::default() };
        assert_eq!(run(&mut p, FIRE, 30), vec![Act::DryFire]);
        assert_eq!(p.clip().0, Clip::Idle);
    }

    #[test]
    fn a_reload_is_not_cut_short_by_the_trigger() {
        let mut p = Pistol { mag: 3, ..Pistol::default() };
        run(&mut p, Trigger { reload: true, ..Default::default() }, 10);
        assert_eq!(p.clip().0, Clip::Reload);
        assert!(p.update(FIRE, DT).is_empty(), "no shot mid-reload");
        assert_eq!(p.mag, 3);
    }

    #[test]
    fn a_full_magazine_does_not_reload() {
        let mut p = Pistol::default();
        run(&mut p, Trigger { reload: true, ..Default::default() }, 2);
        assert_eq!(p.clip().0, Clip::Idle);
    }

    #[test]
    fn the_blow_lands_on_its_frame() {
        let mut p = Pistol::default();
        let mut struck_at = None;
        for i in 0..40 {
            let input = if i == 0 { Trigger { melee: true, ..Default::default() } } else { Trigger::default() };
            if p.update(input, DT).contains(&Act::Strike) {
                struck_at = Some(p.t);
            }
        }
        let t = struck_at.expect("a strike");
        assert!((t - STRIKE_AT).abs() <= DT, "at {t}");
        assert_eq!(p.clip().0, Clip::Idle, "and back to the grip");
    }
}
