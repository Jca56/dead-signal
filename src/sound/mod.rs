//! The game's sounds, made in code: noise, oscillators and envelopes,
//! rendered once at start into buffers, and played on a stream of our own
//! through `lntrn-audio`'s mixer. A sound in the world is placed by
//! `place.rs`: how loud, how muffled, which side, whether heard at all.
//! The one thing not made in code is the music (`music.rs`), read from
//! its file on a thread of its own and let into the stream once it's
//! ready.

mod music;
mod place;
mod synth;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
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
    /// A shotgun going off; its pump racked; a shell pushed into it; a
    /// rifle's crack and its bolt worked.
    Blast,
    Pump,
    ShellIn,
    RifleShot,
    Bolt,
    /// The SMG's snap and the assault rifle's crack: short, for they come
    /// one on another.
    SmgShot,
    ArShot,
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
    /// A Ripper's scream on seeing you, and as it slashes.
    Shriek,
    /// A Spitter: its gurgling retch on seeing you, the glob hawked up,
    /// the glob landing, its swelling when it dies, and the burst.
    Retch,
    Spit,
    Splat,
    Swell,
    Burst,
    /// A Juggernaut: its roar before it charges, its footfalls charging,
    /// running into something, and a round turned by its plate.
    Bellow,
    Stomp,
    Slam,
    Clank,
    Flesh,
    Gurgle,
    Shuffle,
    /// The player: a heart thumping when hurt badly, the drone under YOU
    /// DIED, something picked up, a bandage torn and wound.
    Heartbeat,
    Died,
    Pickup,
    Heal,
    /// The menus: the highlight moving, and something pressed.
    Tick,
    Click,
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

const ALL: [Sfx; 47] = [
    Sfx::Shot,
    Sfx::Blast,
    Sfx::Pump,
    Sfx::ShellIn,
    Sfx::RifleShot,
    Sfx::Bolt,
    Sfx::SmgShot,
    Sfx::ArShot,
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
    Sfx::Shriek,
    Sfx::Retch,
    Sfx::Spit,
    Sfx::Splat,
    Sfx::Swell,
    Sfx::Burst,
    Sfx::Bellow,
    Sfx::Stomp,
    Sfx::Slam,
    Sfx::Clank,
    Sfx::Flesh,
    Sfx::Gurgle,
    Sfx::Shuffle,
    Sfx::Heartbeat,
    Sfx::Died,
    Sfx::Pickup,
    Sfx::Heal,
    Sfx::Tick,
    Sfx::Click,
    Sfx::Rummage,
    Sfx::Unlock,
    Sfx::Rattle,
    Sfx::Static,
    Sfx::Rotor,
    Sfx::Crank,
    Sfx::Engine,
    Sfx::Safe,
];

impl Sfx {
    /// Whether it's the dead's (their own volume), not the world's.
    fn of_the_dead(self) -> bool {
        matches!(self, Sfx::Groan | Sfx::Snarl | Sfx::Shriek | Sfx::Retch | Sfx::Swell | Sfx::Bellow | Sfx::Stomp | Sfx::Gurgle | Sfx::Shuffle)
    }
}

/// How loud each part of the sound is, 0–1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mix {
    pub master: f32,
    pub music: f32,
    pub effects: f32,
    pub zombies: f32,
}

/// The mix as the stream reads it, while the game sets it; and whether
/// the music's wanted now.
#[derive(Default)]
struct Levels {
    master: AtomicU32,
    music: AtomicU32,
    effects: AtomicU32,
    zombies: AtomicU32,
    music_on: AtomicBool,
}

impl Levels {
    fn set(&self, mix: Mix) {
        for (at, v) in [(&self.master, mix.master), (&self.music, mix.music), (&self.effects, mix.effects), (&self.zombies, mix.zombies)] {
            at.store(v.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
        }
    }

    fn get(at: &AtomicU32) -> f32 {
        f32::from_bits(at.load(Ordering::Relaxed))
    }
}

struct Play {
    sfx: Sfx,
    heard: Heard,
}

/// The game's sound: a stream and what it has to play. Silent (and says
/// so once) when there is no device to play on.
pub struct Sound {
    tx: Option<Sender<Play>>,
    levels: Arc<Levels>,
    _out: Option<Output>,
}

impl Sound {
    pub fn new() -> Self {
        let bank: Arc<Vec<Arc<Audio>>> = Arc::new(ALL.iter().map(|&s| Arc::new(Audio::new(RATE, 1, synth::synth(s)))).collect());
        let (tx, rx): (Sender<Play>, Receiver<Play>) = channel();
        let mut mixer = Mixer::new(Spec::STEREO_48K);
        let levels = Arc::new(Levels::default());
        levels.set(Mix { master: 0.8, music: 0.6, effects: 1.0, zombies: 1.0 });
        let heard_levels = Arc::clone(&levels);
        let songs = music::load();
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
                let id = mixer.play(Panned::new(Arc::clone(&bank[i]), p.heard, Arc::clone(&fade), Arc::clone(&heard_levels), p.sfx.of_the_dead()));
                if let Some(c) = crowd {
                    crowds.add(c, (id, fade), p.heard.distance);
                }
            }
            if let Ok(song) = songs.try_recv() {
                mixer.play(music::Music::new(song, Arc::clone(&heard_levels)));
            }
            mixer.set_master(Levels::get(&heard_levels.master));
            mixer.render(out);
        };
        match Output::open(Spec::STEREO_48K, render) {
            Ok(out) => {
                log_info!("sound: {} effects, {:.0} ms latency", ALL.len(), out.latency() * 1000.0);
                Self { tx: Some(tx), levels, _out: Some(out) }
            }
            Err(e) => {
                log_error!("sound: no output ({e}); the game is silent");
                Self { tx: None, levels, _out: None }
            }
        }
    }

    /// Set how loud everything is (at once, even what's playing).
    pub fn set_mix(&self, mix: Mix) {
        self.levels.set(mix);
    }

    /// Whether the music should be playing (it fades in and out).
    pub fn set_music(&self, on: bool) {
        self.levels.music_on.store(on, Ordering::Relaxed);
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
    /// The mix, and whether it's the dead's volume or the effects' that
    /// it plays at.
    levels: Arc<Levels>,
    dead: bool,
}

impl Panned {
    fn new(audio: Arc<Audio>, heard: Heard, fade: Arc<AtomicBool>, levels: Arc<Levels>, dead: bool) -> Self {
        let a = 1.0 - (-std::f32::consts::TAU * heard.cutoff / RATE as f32).exp();
        Self { audio, pos: 0, left: heard.left, right: heard.right, a, lp: [0.0; 2], fade, fading: 1.0, levels, dead }
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
        let level = Levels::get(if self.dead { &self.levels.zombies } else { &self.levels.effects });
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
            let s = self.lp[1] * self.fading * level;
            frame[0] = s * self.left;
            frame[1] = s * self.right;
        }
        self.pos += frames;
        frames
    }
}
