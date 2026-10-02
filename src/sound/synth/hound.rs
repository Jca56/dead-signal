//! A Hellhound's voice, and the static it comes through: its bark, the
//! growl in its throat, the yelp it dies with, the pack's howl far off
//! (heard before it's seen), and the crack of a tear in the air.

use super::{Noise, Svf, env, render, sine, sweep_phase};
use crate::sound::{RATE, Sfx};

/// A throat: a buzz at `freq(t)` Hz and breath by `rasp`, through two
/// mouth shapes (`low` and `high` Hz), swelled by `shape(t)`.
#[allow(clippy::too_many_arguments)]
fn throat(seconds: f32, peak: f32, rasp: f32, low: f32, high: f32, freq: impl Fn(f32) -> f32, shape: impl Fn(f32) -> f32) -> Vec<f32> {
    let (mut a, mut b) = (Svf::default(), Svf::default());
    let mut phase = 0.0f32;
    let dt = 1.0 / RATE as f32;
    render(seconds, peak, move |t, n: &mut Noise| {
        phase = (phase + freq(t) * dt).fract();
        let buzz = if phase < 0.3 { 1.0 } else { -0.43 };
        let x = buzz + n.next() * rasp;
        (a.run(x, low, 0.3).1 + b.run(x, high, 0.35).1 * 0.7) * shape(t)
    })
}

pub(super) fn make(sfx: Sfx) -> Vec<f32> {
    match sfx {
        // Two sharp barks, the second lower: a snap of the jaws each.
        Sfx::Bark => throat(0.42, 0.9, 1.0, 700.0, 1900.0, |t| if t < 0.2 { 260.0 - 500.0 * t } else { 220.0 - 420.0 * (t - 0.2) }, |t| env(t, 0.008, 0.05) + env((t - 0.2).max(0.0), 0.008, 0.06) * f32::from(t > 0.2)),
        // Low in the throat, rolling.
        Sfx::Growl => throat(1.1, 0.6, 1.3, 300.0, 900.0, |t| 70.0 + 14.0 * sine(t, 3.0), |t| env(t, 0.15, 0.5) * (0.55 + 0.45 * sine(t, 27.0).abs())),
        // High, cut short, falling away.
        Sfx::Yelp => throat(0.45, 0.85, 0.6, 1100.0, 2600.0, |t| 900.0 - 1300.0 * t + 40.0 * sine(t, 18.0), |t| env(t, 0.01, 0.13)),
        // Far off and long: up, held, and down, a second voice under it.
        Sfx::Howl => {
            let (mut a, mut b) = (Svf::default(), Svf::default());
            let (mut p1, mut p2) = (0.0f32, 0.0f32);
            let dt = 1.0 / RATE as f32;
            render(2.6, 0.8, move |t, n| {
                let bend = 300.0 + 240.0 * (t / 0.6).min(1.0) - 210.0 * ((t - 1.5).max(0.0) / 1.1) + 7.0 * sine(t, 5.0);
                p1 = (p1 + bend * dt).fract();
                p2 = (p2 + bend * 0.76 * dt).fract();
                let voice = |p: f32| if p < 0.4 { 1.0 } else { -0.66 };
                let x = voice(p1) + voice(p2) * 0.5 + n.next() * 0.25;
                let shape = (t / 0.35).min(1.0) * (1.0 - ((t - 1.7) / 0.9).clamp(0.0, 1.0));
                (a.run(x, 620.0, 0.25).1 + b.run(x, 1300.0, 0.3).1 * 0.4) * shape
            })
        }
        // The air torn open: a crack, a falling buzz of static, a thump.
        _ => {
            let (mut f, mut g) = (Svf::default(), Svf::default());
            render(0.8, 1.0, |t, n| {
                let x = n.next();
                let crack = f.run(x, 4200.0, 0.5).2 * env(t, 0.0005, 0.03);
                let fizz = g.run(x * (0.5 + 0.5 * sine(t, 61.0).abs()), 2400.0 - 1500.0 * t, 0.4).1 * env(t, 0.004, 0.25);
                let thump = (std::f32::consts::TAU * sweep_phase(t, 120.0, 38.0, 0.08)).sin() * env(t, 0.002, 0.2);
                (crack * 1.3 + fizz * 0.9 + thump * 1.2).tanh()
            })
        }
    }
}
