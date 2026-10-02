//! The guns' sounds (their shots, their actions worked) and what's thrown
//! and burns: a part of `synth`, with its makings.

use super::{Noise, Svf, env, render, sine, sweep_phase};
use crate::sound::Sfx;

/// One of the guns' sounds, or of what's thrown.
pub(super) fn make(sfx: Sfx) -> Vec<f32> {
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
        Sfx::SmgShot => {
            // The pistol's shot, clipped: a snappy crack, a little thump,
            // gone before the next.
            let (mut body, mut tail) = (Svf::default(), Svf::default());
            render(0.3, 0.8, |t, n| {
                let x = n.next();
                let crack = body.run(x, 4200.0, 0.6).2 * env(t, 0.0004, 0.005);
                let thump = (std::f32::consts::TAU * sweep_phase(t, 190.0, 60.0, 0.02)).sin() * env(t, 0.001, 0.045);
                let rumble = tail.run(x, 900.0, 1.1).0 * env(t, 0.003, 0.08);
                (crack * 0.9 + thump * 0.8 + rumble * 2.2).tanh()
            })
        }
        Sfx::ArShot => {
            // A hard, bright crack and a punch, a short roll after it.
            let (mut body, mut tail) = (Svf::default(), Svf::default());
            render(0.55, 0.95, |t, n| {
                let x = n.next();
                let crack = body.run(x, 4800.0, 0.5).2 * env(t, 0.0003, 0.006);
                let thump = (std::f32::consts::TAU * sweep_phase(t, 170.0, 48.0, 0.025)).sin() * env(t, 0.001, 0.06);
                let roll = tail.run(x, 600.0, 1.0).0 * env(t, 0.004, 0.14);
                (crack * 1.1 + thump * 1.0 + roll * 2.6).tanh()
            })
        }
        Sfx::LmgShot => {
            // Heavier than the rifle's: a duller crack, a deep punch, a
            // longer roll.
            let (mut body, mut tail) = (Svf::default(), Svf::default());
            render(0.6, 1.0, |t, n| {
                let x = n.next();
                let crack = body.run(x, 3600.0, 0.5).2 * env(t, 0.0003, 0.007);
                let thump = (std::f32::consts::TAU * sweep_phase(t, 150.0, 40.0, 0.03)).sin() * env(t, 0.001, 0.08);
                let roll = tail.run(x, 480.0, 1.0).0 * env(t, 0.004, 0.17);
                (crack * 1.0 + thump * 1.25 + roll * 2.8).tanh()
            })
        }
        Sfx::Flame => {
            // A breath of the stream: a roar of burning air, swelling and
            // gone (one on another, they're the stream).
            let (mut f, mut g) = (Svf::default(), Svf::default());
            render(0.4, 0.75, |t, n| {
                let x = n.next();
                let roar = f.run(x, 520.0 + 260.0 * (t * 37.0).sin(), 0.9).1 * env(t, 0.05, 0.2);
                let hiss = g.run(x, 2800.0, 0.7).2 * env(t, 0.03, 0.16) * 0.25;
                (roar * 2.0 + hiss).tanh()
            })
        }
        Sfx::Shatter => {
            // Glass going: a bright burst, and shards tinkling after.
            let (mut f, mut g) = (Svf::default(), Svf::default());
            render(0.7, 0.8, |t, n| {
                let x = n.next();
                let burst = f.run(x, 5500.0, 0.4).2 * env(t, 0.0005, 0.03);
                let tinkle: f32 = [3100.0, 4700.0, 6300.0, 3900.0].iter().enumerate().map(|(i, &fr)| sine(t, fr) * env((t - 0.05 * i as f32).max(0.0), 0.001, 0.06) * f32::from(t > 0.05 * i as f32)).sum();
                burst * 1.2 + tinkle * 0.2 + g.run(x, 900.0, 0.6).1 * env(t, 0.001, 0.02) * 0.6
            })
        }
        Sfx::Ignite => {
            // Whoomph: air drawn in and a low roar of flame.
            let mut f = Svf::default();
            render(1.0, 0.9, |t, n| {
                let x = n.next();
                let roar = f.run(x, 250.0 + 500.0 * (t / 0.15).min(1.0), 0.8).1 * env(t, 0.06, 0.35);
                roar * 2.2 + (std::f32::consts::TAU * sweep_phase(t, 70.0, 35.0, 0.1)).sin() * env(t, 0.03, 0.2) * 0.5
            })
        }
        Sfx::Crackle => {
            // A fire going: a low rush, and pops.
            let (mut f, mut g) = (Svf::default(), Svf::default());
            let mut pops = Noise(0x51C3_2A77);
            render(1.2, 0.5, move |t, n| {
                let x = n.next();
                let rush = f.run(x, 400.0, 1.0).1 * 0.6 * env(t, 0.1, 0.9);
                let pop = if pops.next() > 0.9985 { 1.0 } else { 0.0 };
                rush + g.run(pop + x * 0.02, 2600.0, 0.3).1 * 3.0
            })
        }
        Sfx::Beep => render(0.09, 0.45, |t, _| (sine(t, 2200.0) + sine(t, 4400.0) * 0.2) * env(t, 0.002, 0.05)),
        Sfx::Explosion => {
            // A great deep boom, the blast's crack on its front, and a long
            // roll after it, stuff raining down.
            let (mut body, mut tail, mut debris) = (Svf::default(), Svf::default(), Svf::default());
            render(2.5, 1.0, |t, n| {
                let x = n.next();
                let crack = body.run(x, 3000.0, 0.6).2 * env(t, 0.0008, 0.02);
                let boom = (std::f32::consts::TAU * sweep_phase(t, 90.0, 25.0, 0.15)).sin() * env(t, 0.002, 0.45);
                let roll = tail.run(x, 350.0, 1.0).0 * env(t, 0.01, 0.7);
                let rain = debris.run(x, 2200.0, 0.6).1 * env((t - 0.3).max(0.0), 0.1, 0.6) * f32::from(t > 0.3) * (0.4 + 0.6 * sine(t, 17.0).abs());
                (crack * 1.4 + boom * 1.6 + roll * 3.0 + rain * 0.5).tanh()
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
        _ => unreachable!("{sfx:?} isn't a gun's sound"),
    }
}
