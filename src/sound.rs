//! The game's sounds, made in code: noise, oscillators and envelopes,
//! rendered once at start into buffers, and played on a stream of our own
//! through `lntrn-audio`'s mixer. A sound in the world is quieter with
//! distance and sits left or right of the listener.

use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

use lntrn_audio::{Audio, Mixer, Output, Source, Spec};
use lntrn_core::{log_error, log_info};
use lntrn_math::Vec3;

const RATE: u32 = 48_000;

/// Every sound the game makes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sfx {
    Shot,
    DryFire,
    MagOut,
    MagIn,
    SlideRack,
    Whoosh,
    HitWood,
    HitDirt,
    HitStone,
    Ding,
    /// The hitmarker's tick.
    Confirm,
    /// The dead: a moan, a snarl on seeing you or striking, a bullet into
    /// one, its last gurgle, its feet dragging.
    Groan,
    Snarl,
    Flesh,
    Gurgle,
    Shuffle,
    /// The player: a heart thumping when hurt badly, the drone under YOU
    /// DIED, something picked up, a bandage torn and wound.
    Heartbeat,
    Died,
    Pickup,
    Heal,
}

const ALL: [Sfx; 20] = [
    Sfx::Shot,
    Sfx::DryFire,
    Sfx::MagOut,
    Sfx::MagIn,
    Sfx::SlideRack,
    Sfx::Whoosh,
    Sfx::HitWood,
    Sfx::HitDirt,
    Sfx::HitStone,
    Sfx::Ding,
    Sfx::Confirm,
    Sfx::Groan,
    Sfx::Snarl,
    Sfx::Flesh,
    Sfx::Gurgle,
    Sfx::Shuffle,
    Sfx::Heartbeat,
    Sfx::Died,
    Sfx::Pickup,
    Sfx::Heal,
];

struct Play {
    sfx: Sfx,
    left: f32,
    right: f32,
}

/// The game's sound: a stream and what it has to play. Silent (and says
/// so once) when there is no device to play on.
pub struct Sound {
    tx: Option<Sender<Play>>,
    _out: Option<Output>,
}

impl Sound {
    pub fn new() -> Self {
        let bank: Arc<Vec<Arc<Audio>>> = Arc::new(ALL.iter().map(|&s| Arc::new(Audio::new(RATE, 1, synth(s)))).collect());
        let (tx, rx): (Sender<Play>, Receiver<Play>) = channel();
        let mut mixer = Mixer::new(Spec::STEREO_48K);
        mixer.set_master(0.8);
        let render = move |out: &mut [f32]| {
            while let Ok(p) = rx.try_recv() {
                let i = ALL.iter().position(|&s| s == p.sfx).unwrap_or(0);
                mixer.play(Panned { audio: Arc::clone(&bank[i]), pos: 0, left: p.left, right: p.right });
            }
            mixer.render(out);
        };
        match Output::open(Spec::STEREO_48K, render) {
            Ok(out) => {
                log_info!("sound: {} effects, {:.0} ms latency", ALL.len(), out.latency() * 1000.0);
                Self { tx: Some(tx), _out: Some(out) }
            }
            Err(e) => {
                log_error!("sound: no output ({e}); the game is silent");
                Self { tx: None, _out: None }
            }
        }
    }

    /// Play a sound at the listener (the gun in hand, a click).
    pub fn play(&self, sfx: Sfx, gain: f32) {
        self.send(sfx, gain, gain);
    }

    /// Play a sound at `at`, heard from `ear` with `right` pointing to the
    /// listener's right: fainter with distance, panned to its side.
    pub fn play_at(&self, sfx: Sfx, gain: f32, at: Vec3, ear: Vec3, right: Vec3) {
        let (l, r) = placed(gain, at, ear, right);
        self.send(sfx, l, r);
    }

    fn send(&self, sfx: Sfx, left: f32, right: f32) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(Play { sfx, left, right });
        }
    }
}

/// Left and right gains for a sound at `at` heard at `ear`: the inverse of
/// distance past a few metres, and an equal-power pan.
fn placed(gain: f32, at: Vec3, ear: Vec3, right: Vec3) -> (f32, f32) {
    let to = at - ear;
    let d = to.length();
    let fall = (1.0 / (1.0 + (d - 2.0).max(0.0) / 6.0)) as f32;
    let side = if d > 1e-6 { (to.dot(right) / d).clamp(-1.0, 1.0) } else { 0.0 };
    let angle = (side + 1.0) * std::f64::consts::FRAC_PI_4;
    let g = gain * fall * std::f32::consts::SQRT_2;
    (g * angle.cos() as f32, g * angle.sin() as f32)
}

/// A mono buffer played into both channels at its own gains.
struct Panned {
    audio: Arc<Audio>,
    pos: usize,
    left: f32,
    right: f32,
}

impl Source for Panned {
    fn spec(&self) -> Spec {
        Spec::STEREO_48K
    }

    fn fill(&mut self, out: &mut [f32]) -> usize {
        let frames = (out.len() / 2).min(self.audio.samples.len() - self.pos);
        for (i, frame) in out.chunks_exact_mut(2).take(frames).enumerate() {
            let s = self.audio.samples[self.pos + i];
            frame[0] = s * self.left;
            frame[1] = s * self.right;
        }
        self.pos += frames;
        frames
    }
}

// ---- synthesis -------------------------------------------------------------------

/// A little noise source, the same every run.
struct Noise(u32);

impl Noise {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0 as f32 / u32::MAX as f32 * 2.0 - 1.0
    }
}

/// A resonant filter: low, band and high pass from one state (Chamberlin).
#[derive(Default)]
struct Svf {
    low: f32,
    band: f32,
}

impl Svf {
    /// Filter `x` at `freq` Hz with damping `q` (smaller rings more):
    /// (low, band, high).
    fn run(&mut self, x: f32, freq: f32, q: f32) -> (f32, f32, f32) {
        let f = 2.0 * (std::f32::consts::PI * freq.min(RATE as f32 / 6.0) / RATE as f32).sin();
        self.low += f * self.band;
        let high = x - self.low - q * self.band;
        self.band += f * high;
        (self.low, self.band, high)
    }
}

fn env(t: f32, attack: f32, decay: f32) -> f32 {
    if t < attack { t / attack.max(1e-6) } else { (-(t - attack) / decay).exp() }
}

/// Render `seconds` of sound, one sample at a time from `f(t, noise)`, and
/// bring it to a sensible peak.
fn render(seconds: f32, peak: f32, mut f: impl FnMut(f32, &mut Noise) -> f32) -> Vec<f32> {
    let n = (seconds * RATE as f32) as usize;
    let mut noise = Noise(0x9E37_79B9);
    let mut out: Vec<f32> = (0..n).map(|i| f(i as f32 / RATE as f32, &mut noise)).collect();
    let max = out.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    if max > 1e-6 {
        let k = peak / max;
        out.iter_mut().for_each(|s| *s = (*s * k).clamp(-1.0, 1.0));
    }
    // A few milliseconds of fade at the end, so nothing stops with a click.
    let fade = (0.004 * RATE as f32) as usize;
    for (i, s) in out.iter_mut().rev().take(fade).enumerate() {
        *s *= i as f32 / fade as f32;
    }
    out
}

/// A buzzy pulse at `freq` Hz: a throat, before its mouth shapes it.
fn pulse(t: f32, freq: f32) -> f32 {
    let phase = (t * freq).fract();
    if phase < 0.3 { 1.0 } else { -0.43 }
}

/// A dead voice: a buzz sliding from `from` to `to` Hz, wobbling, breathy by
/// `rasp`, through two mouth shapes, swelled by `shape(t)`.
fn voice(seconds: f32, peak: f32, from: f32, to: f32, rasp: f32, shape: impl Fn(f32) -> f32) -> Vec<f32> {
    let (mut a, mut b) = (Svf::default(), Svf::default());
    let mut phase = 0.0f32;
    let dt = 1.0 / RATE as f32;
    render(seconds, peak, move |t, n| {
        let freq = from + (to - from) * (t / seconds) + 4.0 * sine(t, 5.5);
        phase = (phase + freq * dt).fract();
        let buzz = if phase < 0.3 { 1.0 } else { -0.43 };
        let x = buzz + n.next() * rasp;
        (a.run(x, 520.0, 0.3).1 + b.run(x, 1150.0, 0.35).1 * 0.6) * shape(t)
    })
}

fn sine(t: f32, freq: f32) -> f32 {
    (std::f32::consts::TAU * freq * t).sin()
}

/// A pitch that falls from `from` to `to` Hz with time constant `tau`: its
/// phase, for `sin(TAU * phase)`.
fn sweep_phase(t: f32, from: f32, to: f32, tau: f32) -> f32 {
    to * t + (from - to) * tau * (1.0 - (-t / tau).exp())
}

fn synth(sfx: Sfx) -> Vec<f32> {
    match sfx {
        Sfx::Shot => {
            let (mut body, mut tail) = (Svf::default(), Svf::default());
            render(0.55, 0.95, |t, n| {
                let x = n.next();
                let crack = body.run(x, 3500.0, 0.6).2 * env(t, 0.0005, 0.006);
                let thump = (std::f32::consts::TAU * sweep_phase(t, 160.0, 45.0, 0.03)).sin() * env(t, 0.001, 0.07);
                let rumble = tail.run(x, 700.0, 1.2).0 * env(t, 0.004, 0.18);
                (crack * 0.9 + thump * 1.0 + rumble * 2.5).tanh()
            })
        }
        Sfx::DryFire => {
            let mut f = Svf::default();
            render(0.06, 0.45, |t, n| f.run(n.next(), 4000.0, 0.3).1 * env(t, 0.0003, 0.004) + sine(t, 2300.0) * env(t, 0.0005, 0.008) * 0.4)
        }
        Sfx::MagOut => {
            let mut f = Svf::default();
            render(0.18, 0.5, |t, n| {
                let x = n.next();
                let click = f.run(x, 3000.0, 0.25).1 * (env(t, 0.0003, 0.005) + env((t - 0.05).max(0.0), 0.0003, 0.006) * f32::from(t > 0.05));
                let scrape = x * 0.15 * env(t, 0.01, 0.05);
                click + scrape
            })
        }
        Sfx::MagIn => {
            let mut f = Svf::default();
            render(0.16, 0.6, |t, n| f.run(n.next(), 2200.0, 0.3).1 * env(t, 0.0003, 0.008) + sine(t, 150.0) * env(t, 0.001, 0.03) * 0.8)
        }
        Sfx::SlideRack => {
            let (mut scrape, mut clack) = (Svf::default(), Svf::default());
            render(0.3, 0.6, |t, n| {
                let x = n.next();
                let pull = scrape.run(x, 1200.0 + 3000.0 * t, 0.8).1 * env(t, 0.02, 0.05) * f32::from(t < 0.12);
                let snap = t - 0.13;
                let hit = if snap > 0.0 { clack.run(x, 2600.0, 0.25).1 * env(snap, 0.0003, 0.012) + sine(snap, 210.0) * env(snap, 0.001, 0.03) * 0.6 } else { 0.0 };
                pull * 0.6 + hit
            })
        }
        Sfx::Whoosh => {
            let mut f = Svf::default();
            render(0.32, 0.5, |t, n| {
                let centre = 350.0 + 1400.0 * (std::f32::consts::PI * (t / 0.32).min(1.0)).sin();
                f.run(n.next(), centre, 0.9).1 * (std::f32::consts::PI * (t / 0.32)).sin().powi(2)
            })
        }
        Sfx::HitWood => {
            let mut f = Svf::default();
            render(0.2, 0.7, |t, n| sine(t, 190.0) * env(t, 0.001, 0.035) + f.run(n.next(), 900.0, 0.5).1 * env(t, 0.0005, 0.015) * 0.8)
        }
        Sfx::HitDirt => {
            let mut f = Svf::default();
            render(0.22, 0.55, |t, n| f.run(n.next(), 500.0, 1.0).0 * env(t, 0.002, 0.05))
        }
        Sfx::HitStone => {
            let mut f = Svf::default();
            render(0.15, 0.6, |t, n| f.run(n.next(), 5000.0, 0.4).2 * env(t, 0.0003, 0.006) + sine(t, 420.0) * env(t, 0.0005, 0.02) * 0.5)
        }
        Sfx::Ding => {
            // A steel plate: a few inharmonic partials ringing down at
            // their own rates, and the strike on top.
            let partials = [(620.0, 1.3, 1.0), (1711.0, 0.8, 0.55), (3348.0, 0.45, 0.35), (5536.0, 0.25, 0.2)];
            let mut f = Svf::default();
            render(1.6, 0.75, |t, n| {
                let ring: f32 = partials.iter().map(|&(freq, decay, amp)| sine(t, freq) * env(t, 0.0008, decay) * amp).sum();
                ring + f.run(n.next(), 6000.0, 0.4).2 * env(t, 0.0002, 0.004) * 0.6
            })
        }
        Sfx::Confirm => render(0.05, 0.3, |t, _| sine(t, 1500.0) * env(t, 0.001, 0.012)),
        Sfx::Groan => voice(1.4, 0.7, 88.0, 72.0, 0.35, |t| (std::f32::consts::PI * (t / 1.4).min(1.0)).sin().powf(0.7)),
        Sfx::Snarl => voice(0.6, 0.85, 130.0, 170.0, 0.8, |t| env(t, 0.03, 0.3)),
        Sfx::Flesh => {
            let mut f = Svf::default();
            render(0.16, 0.8, |t, n| sine(t, 85.0) * env(t, 0.001, 0.05) + f.run(n.next(), 650.0, 0.6).1 * env(t, 0.0005, 0.03) * 0.9)
        }
        Sfx::Gurgle => {
            let mut f = Svf::default();
            let mut g = Svf::default();
            render(1.1, 0.7, |t, n| {
                // Bubbles: a low voice, chopped and wet.
                let chop = (0.5 + 0.5 * sine(t, 9.0 + 5.0 * t)).powi(3);
                let wet = f.run(n.next(), 420.0 + 260.0 * sine(t, 3.3), 0.25).1;
                let throat = g.run(pulse(t, 70.0 - 18.0 * t), 500.0, 0.5).1;
                (wet * 0.8 + throat * 0.6) * chop * env(t, 0.02, 0.5)
            })
        }
        Sfx::Shuffle => {
            let mut f = Svf::default();
            render(0.14, 0.4, |t, n| f.run(n.next(), 900.0, 1.2).1 * (std::f32::consts::PI * (t / 0.14)).sin())
        }
        Sfx::Heartbeat => render(0.6, 0.8, |t, _| {
            // Lub, and a quieter dub.
            let beat = |at: f32, gain: f32| if t >= at { (std::f32::consts::TAU * sweep_phase(t - at, 70.0, 40.0, 0.05)).sin() * env(t - at, 0.004, 0.06) * gain } else { 0.0 };
            beat(0.0, 1.0) + beat(0.22, 0.6)
        }),
        Sfx::Died => {
            // Two low buzzes a hair apart, beating slowly, through a dark
            // filter: a drone that swells in and dies away.
            let mut f = Svf::default();
            let (mut pa, mut pb) = (0.0f32, 0.0f32);
            let dt = 1.0 / RATE as f32;
            render(4.0, 0.7, move |t, n| {
                pa = (pa + 55.0 * dt).fract();
                pb = (pb + 55.7 * dt).fract();
                let saw = (pa * 2.0 - 1.0) + (pb * 2.0 - 1.0) + n.next() * 0.1;
                let swell = (t / 1.2).min(1.0) * (-(t - 1.2).max(0.0) / 1.6).exp();
                f.run(saw, 260.0 + 140.0 * (t / 4.0), 0.7).0 * swell
            })
        }
        Sfx::Pickup => {
            let mut f = Svf::default();
            render(0.18, 0.5, |t, n| f.run(n.next(), 1800.0, 0.9).1 * env(t, 0.004, 0.04) + sine(t, 520.0) * env(t, 0.002, 0.05) * 0.3)
        }
        Sfx::Heal => {
            // A strip torn off, and wound round.
            let mut f = Svf::default();
            render(0.7, 0.55, |t, n| {
                let x = n.next();
                let rip = f.run(x, 2500.0 + 2000.0 * (t / 0.25).min(1.0), 0.5).1 * f32::from(t < 0.25) * (0.6 + 0.4 * sine(t, 60.0).abs());
                let wind = x * 0.12 * f32::from(t > 0.3) * (0.5 + 0.5 * sine(t, 7.0)) * env((t - 0.3).max(0.0), 0.05, 0.3);
                rip + wind
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sound_is_made_and_in_range() {
        for &s in &ALL {
            let b = synth(s);
            assert!(b.len() > RATE as usize / 50, "{s:?} is {} samples", b.len());
            assert!(b.iter().all(|x| x.is_finite() && x.abs() <= 1.0), "{s:?} out of range");
            let peak = b.iter().fold(0.0f32, |m, x| m.max(x.abs()));
            assert!(peak > 0.2, "{s:?} peaks at {peak}");
            assert!(b.last().unwrap().abs() < 1e-3, "{s:?} ends with a click");
        }
    }

    #[test]
    fn far_is_quiet_and_right_is_right() {
        let ear = Vec3::ZERO;
        let right = Vec3::new(1.0, 0.0, 0.0);
        let (l, r) = placed(1.0, Vec3::new(5.0, 0.0, 0.0), ear, right);
        assert!(r > 0.9 && l < 0.05, "hard right: {l} {r}");
        let (l2, r2) = placed(1.0, Vec3::new(0.0, 0.0, -50.0), ear, right);
        assert!((l2 - r2).abs() < 1e-6 && l2 < 0.15, "far and ahead: {l2}");
        let (l3, _) = placed(1.0, Vec3::new(0.0, 0.0, -1.0), ear, right);
        assert!((l3 - 1.0).abs() < 1e-6, "close and ahead is full: {l3}");
    }
}
