//! Where a frame's time goes, during a run: every few seconds a line in
//! `~/.lantern/log/dead-signal-perf.log` with the frame rate, the average
//! and worst frame, and the CPU time of each part of it (the simulation,
//! the run's own work and the HUD, handing the draws to the GPU), beside
//! how many of the dead are up.

use std::io::Write;
use std::time::{Duration, Instant};

/// Seconds a line covers.
const EVERY: f64 = 5.0;

#[derive(Clone, Copy, Debug)]
pub enum Phase {
    Sim,
    Play,
    Render,
}

#[derive(Default)]
pub struct Perf {
    /// The window's start, and the last frame's, in the UI's clock.
    since: Option<f64>,
    last: Option<f64>,
    frames: u32,
    worst: f64,
    spent: [Duration; 3],
    most: [Duration; 3],
    zombies: usize,
    most_zombies: usize,
}

impl Perf {
    /// A frame begins at `now` (seconds); `running` is whether it's a run
    /// being played (only those are counted).
    pub fn frame(&mut self, now: f64, running: bool) {
        if !running {
            *self = Self::default();
            return;
        }
        if let Some(last) = self.last {
            self.worst = self.worst.max(now - last);
            self.frames += 1;
        }
        self.last = Some(now);
        let since = *self.since.get_or_insert(now);
        if now - since >= EVERY && self.frames > 0 {
            self.write(now - since);
            let last = self.last;
            *self = Self::default();
            self.since = last;
            self.last = last;
        }
    }

    /// A part of this frame, begun at `started`, is done.
    pub fn done(&mut self, phase: Phase, started: Instant) {
        let took = started.elapsed();
        let i = phase as usize;
        self.spent[i] += took;
        self.most[i] = self.most[i].max(took);
    }

    pub fn zombies(&mut self, n: usize) {
        self.zombies = n;
        self.most_zombies = self.most_zombies.max(n);
    }

    fn write(&self, seconds: f64) {
        let n = f64::from(self.frames);
        let ms = |d: Duration| d.as_secs_f64() * 1000.0;
        let line = format!(
            "{} build | {:.0} fps | frame {:.1} ms, worst {:.1} ms | cpu sim {:.2} (max {:.1}), play {:.2} (max {:.1}), render {:.2} (max {:.1}) ms | dead up {} (most {})\n",
            if cfg!(debug_assertions) { "debug" } else { "release" },
            n / seconds,
            seconds * 1000.0 / n,
            self.worst * 1000.0,
            ms(self.spent[0]) / n,
            ms(self.most[0]),
            ms(self.spent[1]) / n,
            ms(self.most[1]),
            ms(self.spent[2]) / n,
            ms(self.most[2]),
            self.zombies,
            self.most_zombies,
        );
        let Some(home) = std::env::var_os("HOME") else { return };
        let dir = std::path::PathBuf::from(home).join(".lantern/log");
        let _ = std::fs::create_dir_all(&dir);
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(dir.join("dead-signal-perf.log")) {
            let _ = f.write_all(line.as_bytes());
        }
    }
}
