//! Every sound made from scratch: noise, oscillators, filters and
//! envelopes, rendered once into a buffer each.

use super::{RATE, Sfx};

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

pub(super) fn synth(sfx: Sfx) -> Vec<f32> {
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
        Sfx::Blast => {
            // The pistol's shot, bigger: a harder crack, a deeper thump
            // and a long low roll after it.
            let (mut body, mut tail) = (Svf::default(), Svf::default());
            render(1.1, 1.0, |t, n| {
                let x = n.next();
                let crack = body.run(x, 2600.0, 0.7).2 * env(t, 0.0005, 0.01);
                let thump = (std::f32::consts::TAU * sweep_phase(t, 120.0, 32.0, 0.05)).sin() * env(t, 0.001, 0.12);
                let rumble = tail.run(x, 380.0, 1.1).0 * env(t, 0.006, 0.34);
                (crack * 1.1 + thump * 1.4 + rumble * 3.2).tanh()
            })
        }
        Sfx::Pump => {
            // Back: a scrape and a clack; home: a harder clack.
            let (mut scrape, mut clack) = (Svf::default(), Svf::default());
            render(0.34, 0.7, |t, n| {
                let x = n.next();
                let pull = scrape.run(x, 900.0 + 2500.0 * t, 0.8).1 * env(t, 0.01, 0.04) * f32::from(t < 0.1);
                let knock = |at: f32, f: &mut Svf, x: f32, low: f32| {
                    let s = t - at;
                    if s > 0.0 { f.run(x, 1900.0, 0.3).1 * env(s, 0.0003, 0.014) + sine(s, low) * env(s, 0.001, 0.04) * 0.7 } else { 0.0 }
                };
                pull * 0.5 + knock(0.08, &mut clack, x, 170.0) * 0.8 + knock(0.2, &mut clack, x, 140.0)
            })
        }
        Sfx::ShellIn => {
            let mut f = Svf::default();
            render(0.12, 0.5, |t, n| {
                let x = n.next();
                let slide = f.run(x, 1600.0 + 4000.0 * t, 0.6).1 * env(t, 0.005, 0.03) * 0.5;
                let click = f.run(x, 3200.0, 0.3).1 * env((t - 0.05).max(0.0), 0.0003, 0.006) * f32::from(t > 0.05);
                slide + click + sine(t, 260.0) * env(t, 0.002, 0.03) * 0.4
            })
        }
        Sfx::RifleShot => {
            // A sharp, high crack, a hard thump, and a long echo off the
            // hills that rolls back twice.
            let (mut body, mut tail) = (Svf::default(), Svf::default());
            render(1.6, 1.0, |t, n| {
                let x = n.next();
                let crack = body.run(x, 5200.0, 0.5).2 * env(t, 0.0003, 0.006);
                let thump = (std::f32::consts::TAU * sweep_phase(t, 180.0, 50.0, 0.03)).sin() * env(t, 0.001, 0.08);
                let echo = |at: f32| env((t - at).max(0.0), 0.02, 0.25) * f32::from(t > at);
                let roll = tail.run(x, 520.0, 1.0).0 * (env(t, 0.004, 0.2) + 0.5 * echo(0.45) + 0.25 * echo(0.9));
                (crack * 1.2 + thump * 1.1 + roll * 2.8).tanh()
            })
        }
        Sfx::Bolt => {
            // Up, back, forward, down: four clicks and a slide either way.
            let (mut slide, mut click) = (Svf::default(), Svf::default());
            render(0.45, 0.65, |t, n| {
                let x = n.next();
                let mut out = 0.0;
                for (at, low) in [(0.0, 300.0), (0.1, 220.0), (0.24, 200.0), (0.33, 260.0)] {
                    let s = t - at;
                    if s > 0.0 {
                        out += click.run(x, 2800.0, 0.3).1 * env(s, 0.0003, 0.008) + sine(s, low) * env(s, 0.001, 0.02) * 0.4;
                    }
                }
                let sliding = (t > 0.1 && t < 0.2) || (t > 0.24 && t < 0.3);
                out + slide.run(x, 1400.0, 0.7).1 * 0.25 * f32::from(sliding)
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
        Sfx::Rummage => {
            // Things shoved about: a few scuffs of cloth and wood, knocks
            // under them.
            let (mut f, mut g) = (Svf::default(), Svf::default());
            render(0.45, 0.5, |t, n| {
                let x = n.next();
                let scuffs = [(0.0, 1.0), (0.12, 0.7), (0.26, 0.85)];
                let scuff: f32 = scuffs.iter().map(|&(at, gain)| if t >= at { env(t - at, 0.01, 0.05) * gain } else { 0.0 }).sum();
                let knock = if t >= 0.2 { sine(t - 0.2, 180.0) * env(t - 0.2, 0.001, 0.03) * 0.5 } else { 0.0 };
                f.run(x, 1300.0, 0.8).1 * scuff + g.run(x, 400.0, 1.0).0 * scuff * 0.4 + knock
            })
        }
        Sfx::Unlock => {
            // A key turning: a scrape, the click, the shackle springing.
            let (mut f, mut g) = (Svf::default(), Svf::default());
            render(0.4, 0.6, |t, n| {
                let x = n.next();
                let turn = f.run(x, 2800.0, 0.6).1 * env(t, 0.02, 0.05) * f32::from(t < 0.14) * 0.5;
                let click = if t >= 0.15 { g.run(x, 3500.0, 0.2).1 * env(t - 0.15, 0.0003, 0.006) + sine(t - 0.15, 1900.0) * env(t - 0.15, 0.0005, 0.02) * 0.4 } else { 0.0 };
                let spring = if t >= 0.24 { sine(t - 0.24, 950.0) * env(t - 0.24, 0.0008, 0.06) * 0.5 } else { 0.0 };
                turn + click + spring
            })
        }
        Sfx::Static => {
            // A radio opening: a burst of hiss, chirps bleeding through.
            let (mut f, mut g) = (Svf::default(), Svf::default());
            render(0.9, 0.45, |t, n| {
                let x = n.next();
                let hiss = f.run(x, 3200.0, 0.7).1 * (0.6 + 0.4 * sine(t, 23.0).abs());
                let chirp = sine(t, 900.0 + 700.0 * sine(t, 3.0)) * (0.5 + 0.5 * sine(t, 11.0)).powi(6) * 0.4;
                let shape = (t / 0.03).min(1.0) * (1.0 - ((t - 0.75) / 0.15).clamp(0.0, 1.0));
                (hiss + g.run(chirp, 1400.0, 0.5).1) * shape
            })
        }
        Sfx::Rotor => {
            // One beat of a chopper's blades: a chop of low air.
            let mut f = Svf::default();
            render(0.22, 0.8, |t, n| f.run(n.next(), 180.0, 0.9).0 * env(t, 0.004, 0.06) * 2.0 + sine(t, 62.0) * env(t, 0.003, 0.05) * 0.6)
        }
        Sfx::Crank => {
            // A starter grinding at a dead engine: whine and chug.
            let (mut f, mut g) = (Svf::default(), Svf::default());
            render(0.5, 0.6, |t, n| {
                let x = n.next();
                let whine = g.run(pulse(t, 140.0 + 20.0 * sine(t, 2.0)), 900.0, 0.8).1 * 0.5;
                let chug = f.run(x, 300.0, 1.0).0 * (0.5 + 0.5 * sine(t, 9.0)).powi(3) * 1.4;
                (whine + chug) * (t / 0.02).min(1.0) * (1.0 - ((t - 0.42) / 0.08).clamp(0.0, 1.0))
            })
        }
        Sfx::Engine => {
            // It catches: a cough, then a rough idle revving up and away.
            let mut f = Svf::default();
            let mut phase = 0.0f32;
            let dt = 1.0 / RATE as f32;
            render(2.2, 0.8, move |t, n| {
                let rpm = 28.0 + 40.0 * (t / 1.2).min(1.0);
                phase = (phase + rpm * dt).fract();
                let firing = (1.0 - phase).powi(5);
                let body = f.run(firing + n.next() * 0.2, 250.0 + 300.0 * (t / 1.5).min(1.0), 1.0).0;
                let cough = if t < 0.15 { n.next() * env(t, 0.002, 0.05) } else { 0.0 };
                (body * 2.2 + cough) * (t / 0.05).min(1.0) * (1.0 - ((t - 1.7) / 0.5).clamp(0.0, 1.0))
            })
        }
        Sfx::Safe => {
            // Out: a low, warm chord swelling up from under the static.
            render(4.0, 0.6, |t, n| {
                let swell = (t / 1.5).min(1.0) * (-(t - 1.5).max(0.0) / 1.4).exp();
                let chord: f32 = [110.0, 164.8, 220.0, 277.2].iter().map(|&f| sine(t, f)).sum::<f32>() * 0.25;
                (chord + n.next() * 0.03 * (1.0 - t / 4.0)) * swell
            })
        }
        Sfx::Rattle => {
            // A locked door shaken: chain-link and a padlock knocking.
            let mut f = Svf::default();
            render(0.35, 0.55, |t, n| {
                let shakes = (0.5 + 0.5 * sine(t, 16.0)).powi(4) * env(t, 0.01, 0.12);
                f.run(n.next(), 2400.0, 0.4).1 * shakes + sine(t, 740.0) * shakes * 0.25
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::ALL;

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
}
