//! The game's sounds, made in code: noise, oscillators and envelopes,
//! rendered once at start into buffers, and played on a stream of our own
//! through `lntrn-audio`'s mixer. A sound in the world is placed by
//! `place.rs`: how loud, how muffled, which side, whether heard at all.

mod place;
mod synth;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};

use lntrn_audio::{Audio, Mixer, Output, Source, Spec, VoiceId};
use lntrn_core::{log_error, log_info};
use lntrn_math::Vec3;

use place::{Admit, Crowds, Heard};

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
    /// Searching: a container gone through, a lock opened, a lock that
    /// won't.
    Rummage,
    Unlock,
    Rattle,
    /// Getting out: the radio opening, a chopper's blades, a starter
    /// grinding, an engine catching, and the chord of being safe.
    Static,
    Rotor,
    Crank,
    Engine,
    Safe,
}

const ALL: [Sfx; 28] = [
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
    Sfx::Rummage,
    Sfx::Unlock,
    Sfx::Rattle,
    Sfx::Static,
    Sfx::Rotor,
    Sfx::Crank,
    Sfx::Engine,
    Sfx::Safe,
];

struct Play {
    sfx: Sfx,
    heard: Heard,
}

/// The game's sound: a stream and what it has to play. Silent (and says
/// so once) when there is no device to play on.
pub struct Sound {
    tx: Option<Sender<Play>>,
    _out: Option<Output>,
}

impl Sound {
    pub fn new() -> Self {
        let bank: Arc<Vec<Arc<Audio>>> = Arc::new(ALL.iter().map(|&s| Arc::new(Audio::new(RATE, 1, synth::synth(s)))).collect());
        let (tx, rx): (Sender<Play>, Receiver<Play>) = channel();
        let mut mixer = Mixer::new(Spec::STEREO_48K);
        mixer.set_master(0.8);
        let mut crowds: Crowds<(VoiceId, Arc<AtomicBool>)> = Crowds::default();
        let render = move |out: &mut [f32]| {
            while let Ok(p) = rx.try_recv() {
                let crowd = p.sfx.crowd();
                if let Some(c) = crowd {
                    match crowds.admit(c, p.heard.distance, |(id, _)| mixer.is_playing(*id)) {
                        Admit::Play => {}
                        Admit::Instead((_, fade)) => fade.store(true, Ordering::Relaxed),
                        Admit::Skip => continue,
                    }
                }
                let i = ALL.iter().position(|&s| s == p.sfx).unwrap_or(0);
                let fade = Arc::new(AtomicBool::new(false));
                let id = mixer.play(Panned::new(Arc::clone(&bank[i]), p.heard, Arc::clone(&fade)));
                if let Some(c) = crowd {
                    crowds.add(c, (id, fade), p.heard.distance);
                }
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
        self.send(sfx, Heard::here(gain));
    }

    /// Play a sound at `at`, heard from `ear` with `right` pointing to the
    /// listener's right, `blocked` or not by something solid between.
    pub fn play_at(&self, sfx: Sfx, gain: f32, at: Vec3, ear: Vec3, right: Vec3, blocked: bool) {
        if let Some(heard) = place::place(sfx, gain, at, ear, right, blocked) {
            self.send(sfx, heard);
        }
    }

    fn send(&self, sfx: Sfx, heard: Heard) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(Play { sfx, heard });
        }
    }
}

/// How long a voice let go for a nearer one takes to fade, seconds.
const LET_GO: f32 = 0.03;

/// A mono buffer played into both channels at its own gains, through a
/// two-pole low-pass (the muffling), fading out early if told to.
struct Panned {
    audio: Arc<Audio>,
    pos: usize,
    left: f32,
    right: f32,
    /// The low-pass's coefficient and its two stages.
    a: f32,
    lp: [f32; 2],
    fade: Arc<AtomicBool>,
    fading: f32,
}

impl Panned {
    fn new(audio: Arc<Audio>, heard: Heard, fade: Arc<AtomicBool>) -> Self {
        let a = 1.0 - (-std::f32::consts::TAU * heard.cutoff / RATE as f32).exp();
        Self { audio, pos: 0, left: heard.left, right: heard.right, a, lp: [0.0; 2], fade, fading: 1.0 }
    }
}

impl Source for Panned {
    fn spec(&self) -> Spec {
        Spec::STEREO_48K
    }

    fn fill(&mut self, out: &mut [f32]) -> usize {
        let mut frames = (out.len() / 2).min(self.audio.samples.len() - self.pos);
        let letting_go = self.fade.load(Ordering::Relaxed);
        let step = 1.0 / (LET_GO * RATE as f32);
        for (i, frame) in out.chunks_exact_mut(2).take(frames).enumerate() {
            let s = self.audio.samples[self.pos + i];
            self.lp[0] += self.a * (s - self.lp[0]);
            self.lp[1] += self.a * (self.lp[0] - self.lp[1]);
            if letting_go {
                self.fading -= step;
                if self.fading <= 0.0 {
                    frames = i;
                    break;
                }
            }
            let s = self.lp[1] * self.fading;
            frame[0] = s * self.left;
            frame[1] = s * self.right;
        }
        self.pos += frames;
        frames
    }
}
