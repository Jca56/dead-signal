//! The music: one song, read and decoded on a thread of its own at start
//! (the stream plays on without it till it's ready), then looped for good,
//! fading in when it's wanted and out when it isn't, and dipping to
//! nothing and back where the end meets the start.

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{Receiver, channel};

use lntrn_audio::{Audio, Source, Spec};
use lntrn_core::{log_error, log_info};

use super::Levels;

/// The song, in `assets/music`.
const SONG: &str = "dusty_resilience.mp3";
/// Seconds to fade all the way in or out, and to dip at the loop.
const FADE: f32 = 2.5;
const LOOP_DIP: f32 = 1.5;

/// The song, decoded, once it's ready.
pub(super) fn load() -> Receiver<Arc<Audio>> {
    let (tx, rx) = channel();
    let path = crate::assets::root().join("music").join(SONG);
    let spawned = std::thread::Builder::new().name("music".into()).spawn(move || {
        let started = std::time::Instant::now();
        let read = std::fs::read(&path).map_err(|e| e.to_string()).and_then(|bytes| lntrn_audio::decode(&bytes).map_err(|e| e.to_string()));
        match read {
            Ok(song) => {
                log_info!("music: {} ({:.0} s) ready in {:.2} s", path.display(), song.duration(), started.elapsed().as_secs_f64());
                let _ = tx.send(Arc::new(song));
            }
            Err(e) => log_error!("music: {}: {e}", path.display()),
        }
    });
    if let Err(e) = spawned {
        log_error!("music: no thread to read it on: {e}");
    }
    rx
}

/// The song looping, at the music's volume while it's wanted.
pub(super) struct Music {
    song: Arc<Audio>,
    /// Where in it, frames.
    pos: usize,
    /// How far faded in, 0–1.
    fade: f32,
    levels: Arc<Levels>,
}

impl Music {
    pub(super) fn new(song: Arc<Audio>, levels: Arc<Levels>) -> Self {
        Self { song, pos: 0, fade: 0.0, levels }
    }
}

impl Source for Music {
    fn spec(&self) -> Spec {
        Spec::new(self.song.sample_rate, 2)
    }

    fn fill(&mut self, out: &mut [f32]) -> usize {
        let frames = out.len() / 2;
        let total = self.song.frames();
        if total == 0 {
            out.fill(0.0);
            return frames;
        }
        let rate = self.song.sample_rate as f32;
        let wanted = self.levels.music_on.load(Ordering::Relaxed);
        let volume = Levels::get(&self.levels.music);
        let step = 1.0 / (FADE * rate);
        let dip = (LOOP_DIP * rate) as usize;
        let ch = self.song.channels.max(1) as usize;
        for frame in out.chunks_exact_mut(2) {
            self.fade = if wanted { (self.fade + step).min(1.0) } else { (self.fade - step).max(0.0) };
            // Quiet at the very end and start, so the loop never clicks.
            let edge = self.pos.min(total - 1 - self.pos).min(dip) as f32 / dip.max(1) as f32;
            let gain = volume * self.fade * self.fade * edge;
            let at = self.pos * ch;
            let (l, r) = (self.song.samples[at], self.song.samples[at + ch.min(2) - 1]);
            frame[0] = l * gain;
            frame[1] = r * gain;
            self.pos = (self.pos + 1) % total;
        }
        frames
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn levels(on: bool) -> Arc<Levels> {
        let l = Arc::new(Levels::default());
        l.set(super::super::Mix { master: 1.0, music: 1.0, effects: 1.0, zombies: 1.0 });
        l.music_on.store(on, Ordering::Relaxed);
        l
    }

    /// The loudest sample in `seconds` of `m`.
    fn loudest(m: &mut Music, seconds: f32) -> f32 {
        let mut buf = vec![0.0; (seconds * 48_000.0) as usize * 2];
        m.fill(&mut buf);
        buf.iter().fold(0.0f32, |a, s| a.max(s.abs()))
    }

    #[test]
    fn it_fades_in_when_wanted_and_out_when_not() {
        // Half a minute of a steady tone, stereo (clear of the loop's dip).
        let song = Arc::new(Audio::new(48_000, 2, vec![0.5; 48_000 * 30 * 2]));
        let l = levels(true);
        let mut m = Music::new(Arc::clone(&song), Arc::clone(&l));
        assert!(loudest(&mut m, 0.1) < 0.01, "starts from silence");
        loudest(&mut m, FADE + 0.5);
        assert!((loudest(&mut m, 0.5) - 0.5).abs() < 0.01, "all the way in");
        l.music_on.store(false, Ordering::Relaxed);
        loudest(&mut m, FADE + 0.1);
        assert!(loudest(&mut m, 0.5) < 1e-6, "all the way out");
        // Its volume, as set.
        l.music_on.store(true, Ordering::Relaxed);
        l.set(super::super::Mix { master: 1.0, music: 0.5, effects: 1.0, zombies: 1.0 });
        loudest(&mut m, FADE + 0.1);
        assert!((loudest(&mut m, 0.5) - 0.25).abs() < 0.01);
    }

    #[test]
    fn it_loops_quietly_through_the_seam() {
        let song = Arc::new(Audio::new(48_000, 2, vec![0.5; 48_000 * 10 * 2]));
        let mut m = Music::new(song, levels(true));
        m.fade = 1.0;
        m.pos = 48_000 * 10 - 10;
        assert!(loudest(&mut m, 0.0004) < 0.01, "quiet at the seam");
        assert!(m.pos < 48_000, "round to the start");
    }

    #[test]
    fn the_song_is_there_and_reads() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/music/dusty_resilience.mp3");
        let song = lntrn_audio::decode(&std::fs::read(path).expect("the song")).expect("an mp3 that decodes");
        assert!((song.duration() - 128.0).abs() < 1.0, "{} s", song.duration());
        assert_eq!(song.channels, 2);
        assert!(song.samples.iter().any(|s| s.abs() > 0.1), "not silent");
    }
}
