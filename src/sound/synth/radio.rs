//! The handheld radio, and what it calls down: the handset switched on,
//! its talk button pressed and let go, the tones of its dial and the buzz
//! of a wrong arrow; a plane going over, high up, and a crate coming to
//! ground; a boost coming on; a strafing run's plane, and its guns.

use super::{Svf, env, pulse, render, sine};
use crate::sound::{RATE, Sfx};

pub(super) fn make(sfx: Sfx) -> Vec<f32> {
    match sfx {
        Sfx::RadioOn => {
            // The handset switched on: its knob clicked round, and a
            // breath of hiss as the squelch opens and shuts.
            let (mut f, mut g) = (Svf::default(), Svf::default());
            render(0.3, 0.4, |t, n| {
                let x = n.next();
                let click = g.run(x, 2400.0, 0.5).1 * env(t, 0.0004, 0.006) + sine(t, 1300.0) * env(t, 0.0006, 0.012) * 0.5;
                let open = (t - 0.04).max(0.0);
                let hiss = f.run(x, 2900.0, 0.8).1 * env(open, 0.01, 0.07) * f32::from(t >= 0.04) * 0.55;
                click + hiss
            })
        }
        Sfx::RadioTalk => {
            // The talk button down: its click, and the carrier coming up
            // under a thin hiss.
            let (mut f, mut g) = (Svf::default(), Svf::default());
            render(0.22, 0.38, |t, n| {
                let x = n.next();
                let click = g.run(x, 3000.0, 0.4).1 * env(t, 0.0003, 0.005);
                let hiss = f.run(x, 3400.0, 0.9).1 * env(t, 0.015, 0.09) * 0.4;
                click + hiss + sine(t, 420.0) * env(t, 0.004, 0.05) * 0.25
            })
        }
        Sfx::RadioOver => {
            // Let go: the roger beep, two notes up, and the squelch's
            // tail after it.
            let mut f = Svf::default();
            render(0.42, 0.42, |t, n| {
                let note = |from: f32, hz: f32| if t >= from { sine(t - from, hz) * env(t - from, 0.004, 0.035) * f32::from(t - from < 0.09) } else { 0.0 };
                let tail = (t - 0.2).max(0.0);
                let hiss = f.run(n.next(), 3000.0, 0.8).1 * env(tail, 0.004, 0.05) * f32::from(t >= 0.2) * 0.5;
                note(0.0, 1250.0) + note(0.1, 1660.0) + hiss
            })
        }
        Sfx::DialUp | Sfx::DialRight | Sfx::DialDown | Sfx::DialLeft => {
            // A key of the handset: two notes at once, as a telephone's
            // are, each arrow its own pair, through a small speaker.
            let (low, high) = match sfx {
                Sfx::DialUp => (852.0, 1477.0),
                Sfx::DialRight => (770.0, 1336.0),
                Sfx::DialLeft => (697.0, 1209.0),
                _ => (620.0, 1075.0),
            };
            let mut f = Svf::default();
            render(0.11, 0.4, move |t, n| {
                let tone = sine(t, low) + sine(t, high) * 0.8;
                let shape = (t / 0.004).min(1.0) * (1.0 - ((t - 0.07) / 0.035).clamp(0.0, 1.0));
                f.run(tone + n.next() * 0.06, 1300.0, 0.9).1 * shape
            })
        }
        Sfx::DialWrong => {
            // A wrong arrow: the handset's low double buzz.
            let mut f = Svf::default();
            render(0.26, 0.42, move |t, n| {
                let on = f32::from(t < 0.09 || (0.13..0.24).contains(&t));
                let buzz = pulse(t, 148.0) + pulse(t, 151.0) * 0.6 + n.next() * 0.1;
                f.run(buzz, 900.0, 0.8).0 * on
            })
        }
        Sfx::Flyover => {
            // A transport going over, high up: its engines' drone coming
            // on, dropping in pitch as it passes, and dying away.
            let mut f = Svf::default();
            let (mut pa, mut pb) = (0.0f32, 0.0f32);
            let dt = 1.0 / RATE as f32;
            render(4.2, 0.6, move |t, n| {
                let pitch = 96.0 - 22.0 * ((t - 1.8) / 0.9).tanh();
                pa = (pa + pitch * dt).fract();
                pb = (pb + pitch * 1.507 * dt).fract();
                let drone = (pa * 2.0 - 1.0) + (pb * 2.0 - 1.0) * 0.6 + n.next() * 0.25;
                let swell = (t / 1.6).min(1.0).powi(2) * (-(t - 1.8).max(0.0) / 1.1).exp();
                f.run(drone, 420.0 + 500.0 * swell, 0.8).0 * swell * (0.8 + 0.2 * sine(t, 13.0))
            })
        }
        Sfx::Thud => {
            // A crate come to ground: wood and weight, a knock in it, the
            // dust after.
            let (mut f, mut g) = (Svf::default(), Svf::default());
            render(0.6, 0.8, move |t, n| {
                let x = n.next();
                let boom = sine(t, 62.0 - 20.0 * (t / 0.2).min(1.0)) * env(t, 0.003, 0.12);
                let knock = f.run(x, 520.0, 0.35).1 * env(t, 0.001, 0.035) * 0.8;
                let dust = g.run(x, 1800.0, 0.9).1 * env((t - 0.03).max(0.0), 0.02, 0.16) * 0.25;
                boom * 1.2 + knock + dust
            })
        }
        Sfx::Boost => {
            // A boost coming on: three notes climbing fast, the last rung
            // out, with a shimmer over it.
            render(0.75, 0.5, |t, n| {
                let note = |from: f32, hz: f32, rings: f32| if t >= from { (sine(t - from, hz) + sine(t - from, hz * 2.0) * 0.3) * env(t - from, 0.004, rings) } else { 0.0 };
                let shimmer = sine(t, 3136.0) * env((t - 0.2).max(0.0), 0.05, 0.2) * f32::from(t >= 0.2) * 0.12;
                note(0.0, 784.0, 0.06) + note(0.09, 988.0, 0.06) + note(0.18, 1175.0, 0.22) + shimmer + n.next() * 0.01
            })
        }
        Sfx::Jet => {
            // A strafing run coming in: a whine and a roar swelling for
            // three seconds, tearing overhead, and gone past.
            let (mut f, mut g) = (Svf::default(), Svf::default());
            let mut phase = 0.0f32;
            let dt = 1.0 / RATE as f32;
            render(5.0, 0.85, move |t, n| {
                let near = (-((t - 3.3) / 0.9).powi(2)).exp();
                let swell = (t / 3.3).min(1.0).powi(3) * 0.5 + near;
                let pitch = 560.0 - 240.0 * ((t - 3.3) / 0.35).tanh();
                phase = (phase + pitch * dt).fract();
                let whine = (phase * 2.0 - 1.0) * 0.18;
                let roar = f.run(n.next(), 300.0 + 900.0 * near, 0.9).0 * 1.4 + g.run(n.next(), 2400.0, 0.7).1 * 0.35 * near;
                (whine + roar) * swell
            })
        }
        Sfx::Brrt => {
            // Its guns: one long tearing burst, the rounds too fast to
            // tell apart, and the air settling after.
            let (mut f, mut g) = (Svf::default(), Svf::default());
            render(1.6, 0.95, move |t, n| {
                let on = f32::from(t < 1.05) * (t / 0.02).min(1.0);
                let x = n.next();
                let tear = pulse(t, 68.0) * 0.9 + pulse(t, 136.5) * 0.4;
                let body = f.run(tear + x * 0.5, 900.0, 0.7).0 * on;
                let crack = g.run(x, 3200.0, 0.6).1 * on * (0.5 + 0.5 * pulse(t, 68.0));
                let after = sine(t, 70.0) * env((t - 1.05).max(0.0), 0.005, 0.18) * f32::from(t >= 1.05) * 0.5;
                body + crack * 0.6 + after
            })
        }
        _ => Vec::new(),
    }
}
