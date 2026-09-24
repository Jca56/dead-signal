//! Where a sound is heard from. Each kind carries only so far: full close
//! by, fading to nothing at the edge of its range, losing its high end as
//! it goes, and quieter and duller again with something solid in the way.
//! The dead are many, so only so many of their voices, and of their feet,
//! sound at once: the nearest.

use lntrn_math::Vec3;

use super::Sfx;

/// Closer than this, a sound is heard whole.
const NEAR: f64 = 1.5;
/// The cutoff close by, and at the edge of hearing, Hz.
const OPEN: f64 = 16_000.0;
const DULL: f64 = 900.0;
/// Behind something: this much of the gain, the cutoff this much lower,
/// though never below the floor.
const BLOCKED_GAIN: f32 = 0.4;
const BLOCKED_DULL: f64 = 0.25;
const BLOCKED_FLOOR: f64 = 350.0;
/// How many of a crowd sound at once.
pub const CROWD: usize = 6;

/// Sounds the dead make in numbers, capped together.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Crowd {
    Voices,
    Feet,
}

impl Sfx {
    /// How far it carries, metres.
    pub fn range(self) -> f64 {
        match self {
            Sfx::Shuffle => 8.0,
            Sfx::Groan | Sfx::Gurgle => 20.0,
            Sfx::Snarl => 30.0,
            Sfx::Shriek => 45.0,
            Sfx::Retch => 30.0,
            Sfx::Spit => 35.0,
            Sfx::Splat => 25.0,
            Sfx::Swell => 30.0,
            Sfx::Burst => 70.0,
            Sfx::Flesh => 40.0,
            _ => 60.0,
        }
    }

    /// Which crowd it's one of, if any.
    pub fn crowd(self) -> Option<Crowd> {
        match self {
            Sfx::Groan | Sfx::Snarl | Sfx::Shriek | Sfx::Retch | Sfx::Gurgle => Some(Crowd::Voices),
            Sfx::Shuffle => Some(Crowd::Feet),
            _ => None,
        }
    }
}

/// How a sound is to be played: its gain in each ear, how muffled (a
/// low-pass cutoff, Hz), and how far off it is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Heard {
    pub left: f32,
    pub right: f32,
    pub cutoff: f32,
    pub distance: f32,
}

impl Heard {
    /// Right at the listener: both ears, nothing taken off (no cutoff).
    pub fn here(gain: f32) -> Self {
        Self { left: gain, right: gain, cutoff: f32::INFINITY, distance: 0.0 }
    }
}

/// How `sfx` at `at` sounds to an ear at `ear` (its right along `right`),
/// with something solid between them or not. Nothing, out of range.
pub fn place(sfx: Sfx, gain: f32, at: Vec3, ear: Vec3, right: Vec3, blocked: bool) -> Option<Heard> {
    let to = at - ear;
    let d = to.length();
    let range = sfx.range();
    if d >= range {
        return None;
    }
    let x = ((d - NEAR) / (range - NEAR)).clamp(0.0, 1.0);
    let mut g = gain * ((1.0 - x) * (1.0 - x)) as f32;
    let mut cutoff = OPEN * (DULL / OPEN).powf(x);
    if blocked {
        g *= BLOCKED_GAIN;
        cutoff = (cutoff * BLOCKED_DULL).max(BLOCKED_FLOOR);
    }
    // An equal-power pan.
    let side = if d > 1e-6 { (to.dot(right) / d).clamp(-1.0, 1.0) } else { 0.0 };
    let angle = (side + 1.0) * std::f64::consts::FRAC_PI_4;
    let g = g * std::f32::consts::SQRT_2;
    Some(Heard { left: g * angle.cos() as f32, right: g * angle.sin() as f32, cutoff: cutoff as f32, distance: d as f32 })
}

/// What to do with a crowd's newcomer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Admit<T> {
    Play,
    /// Play it, and let this farther one go.
    Instead(T),
    Skip,
}

/// Who of each crowd is sounding, and how far off.
pub struct Crowds<T> {
    voices: Vec<(T, f32)>,
    feet: Vec<(T, f32)>,
}

impl<T: Clone> Default for Crowds<T> {
    fn default() -> Self {
        Self { voices: Vec::new(), feet: Vec::new() }
    }
}

impl<T: Clone> Crowds<T> {
    fn of(&mut self, crowd: Crowd) -> &mut Vec<(T, f32)> {
        match crowd {
            Crowd::Voices => &mut self.voices,
            Crowd::Feet => &mut self.feet,
        }
    }

    /// Whether one of `crowd` `distance` off gets to sound (those no longer
    /// `playing` are forgotten first).
    pub fn admit(&mut self, crowd: Crowd, distance: f32, playing: impl Fn(&T) -> bool) -> Admit<T> {
        let all = self.of(crowd);
        all.retain(|(t, _)| playing(t));
        if all.len() < CROWD {
            return Admit::Play;
        }
        let (far, far_d) = all.iter().enumerate().fold((0, f32::MIN), |best, (i, (_, d))| if *d > best.1 { (i, *d) } else { best });
        if distance < far_d { Admit::Instead(all.swap_remove(far).0) } else { Admit::Skip }
    }

    pub fn add(&mut self, crowd: Crowd, t: T, distance: f32) {
        self.of(crowd).push((t, distance));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RIGHT: Vec3 = Vec3::new(1.0, 0.0, 0.0);

    fn at(sfx: Sfx, d: f64, blocked: bool) -> Option<Heard> {
        place(sfx, 1.0, Vec3::new(0.0, 0.0, -d), Vec3::ZERO, RIGHT, blocked)
    }

    #[test]
    fn right_is_right_and_close_is_whole() {
        let h = place(Sfx::Groan, 1.0, Vec3::new(1.0, 0.0, 0.0), Vec3::ZERO, RIGHT, false).unwrap();
        assert!(h.right > 0.9 && h.left < 0.05, "hard right: {h:?}");
        let h = at(Sfx::Groan, 1.0, false).unwrap();
        assert!((h.left - 1.0).abs() < 1e-6 && (h.left - h.right).abs() < 1e-6, "close and ahead: {h:?}");
    }

    #[test]
    fn each_carries_only_so_far_duller_as_it_goes() {
        assert!(at(Sfx::Shuffle, 9.0, false).is_none(), "feet from 9 m");
        assert!(at(Sfx::Groan, 21.0, false).is_none(), "a groan from 21 m");
        assert!(at(Sfx::Snarl, 25.0, false).is_some());
        let near = at(Sfx::Groan, 3.0, false).unwrap();
        let mid = at(Sfx::Groan, 10.0, false).unwrap();
        let far = at(Sfx::Groan, 18.0, false).unwrap();
        assert!(near.left > mid.left && mid.left > far.left && far.left < 0.05, "{near:?} {mid:?} {far:?}");
        assert!(near.cutoff > mid.cutoff && mid.cutoff > far.cutoff && far.cutoff < 1500.0);
    }

    #[test]
    fn a_wall_between_muffles_it() {
        let open = at(Sfx::Groan, 6.0, false).unwrap();
        let walled = at(Sfx::Groan, 6.0, true).unwrap();
        assert!(walled.left < open.left * 0.5 && walled.cutoff < open.cutoff * 0.3);
    }

    #[test]
    fn the_nearest_of_a_crowd_are_heard() {
        let mut c = Crowds::default();
        for i in 0..CROWD {
            assert_eq!(c.admit(Crowd::Voices, 10.0 + i as f32, |_| true), Admit::Play);
            c.add(Crowd::Voices, i, 10.0 + i as f32);
        }
        assert_eq!(c.admit(Crowd::Voices, 30.0, |_| true), Admit::Skip, "farther than all");
        assert_eq!(c.admit(Crowd::Voices, 2.0, |_| true), Admit::Instead(CROWD - 1), "the farthest makes way");
        c.add(Crowd::Voices, 99, 2.0);
        assert_eq!(c.admit(Crowd::Feet, 30.0, |_| true), Admit::Play, "feet are counted apart");
        assert_eq!(c.admit(Crowd::Voices, 30.0, |&i| i != 0), Admit::Play, "one that ended makes room");
    }
}
