//! Screenshots: F12 asks the window for its finished frame back (the
//! world, the HUD, a menu: all that's on screen). When it comes it's
//! written as a PNG into the Pictures folder's `Screenshots`, named for
//! the game and the time, off the frame's thread (a big frame takes a
//! moment to pack); a note at the foot of the screen says it's saved, or
//! that it couldn't be.

use std::ffi::{c_char, c_int, c_long};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use lntrn_core::{log_error, log_info};
use lntrn_image::Image;
use lntrn_math::{Color, Rect, Vec2};
use lntrn_sys::dirs::{self, UserDir};
use lntrn_text::TextStyle;
use lntrn_ui::Ui;

use crate::style;

/// The folder they go in, under the Pictures folder; and what each is
/// named for.
const FOLDER: &str = "Screenshots";
const NAMED: &str = "dead-signal";
/// How long the note stays, and how long it takes to fade at the end.
const NOTE_FOR: Duration = Duration::from_millis(2600);
const FADES: f64 = 0.5;
const SAVED: &str = "SCREENSHOT SAVED";
const FAILED: &str = "COULDN'T SAVE THE SCREENSHOT";
/// One asked for that hasn't come back in this long isn't coming (the
/// window's frames can't be read back here), and what's said then.
const GIVEN_UP: Duration = Duration::from_millis(1500);
const NONE_CAME: &str = "COULDN'T TAKE A SCREENSHOT";

pub struct Screenshots {
    /// How each one being written came out: its file's name, or why not.
    done: Receiver<Result<String, String>>,
    send: Sender<Result<String, String>>,
    /// The word on the last one (and whether it's good news), and till
    /// when it stays.
    note: Option<(String, bool, Instant)>,
    /// When one was asked for that hasn't come back yet.
    asked: Option<Instant>,
}

impl Default for Screenshots {
    fn default() -> Self {
        let (send, done) = mpsc::channel();
        Self { done, send, note: None, asked: None }
    }
}

impl Screenshots {
    /// One's about to be taken: the note's put away, so it's not in it.
    pub fn about_to(&mut self) {
        self.note = None;
        self.asked = Some(Instant::now());
    }

    /// A frame's come back: off it goes to be written.
    pub fn save(&mut self, image: Image) {
        self.asked = None;
        let send = self.send.clone();
        let made = std::thread::Builder::new().name("screenshot".to_string()).spawn(move || {
            let _ = send.send(write(&image, SystemTime::now()));
        });
        if let Err(e) = made {
            let _ = self.send.send(Err(format!("no thread to write it on: {e}")));
        }
    }

    /// A frame of it: what's been written since the last is noted, and
    /// the note drawn at the foot of the screen, over everything, till
    /// its time's up.
    pub fn frame(&mut self, ui: &mut Ui) {
        let now = Instant::now();
        for came in self.done.try_iter() {
            self.note = Some(match came {
                Ok(name) => {
                    log_info!("screenshot: {name}");
                    (format!("{SAVED}  ·  {name}"), true, now + NOTE_FOR)
                }
                Err(why) => {
                    log_error!("screenshot: {why}");
                    (FAILED.to_string(), false, now + NOTE_FOR)
                }
            });
        }
        if self.asked.is_some_and(|at| now.duration_since(at) > GIVEN_UP) {
            log_error!("screenshot: no frame came back (see the log's `surface:` and `readback:` lines)");
            (self.asked, self.note) = (None, Some((NONE_CAME.to_string(), false, now + NOTE_FOR)));
        }
        self.note = self.note.take().filter(|n| n.2 > now);
        let Some((text, good, till)) = &self.note else { return };
        let s = ui.m.scale;
        let alpha = (till.duration_since(now).as_secs_f64() / FADES).clamp(0.0, 1.0);
        let style = TextStyle::new((24.0 * s) as f32).bold().family(style::FONT);
        let (w, h) = (ui.measure(text, &style), f64::from(style.line_height()));
        let screen = ui.clip();
        let at = Vec2::new(screen.center().x - w * 0.5, screen.max.y - 110.0 * s - h);
        let pad = Vec2::new(22.0 * s, 10.0 * s);
        ui.draw.rect(Rect::new(at - pad, at + Vec2::new(w, h) + pad), Color::rgba(0.0, 0.0, 0.0, 0.72 * alpha));
        let c = if *good { style::BONE } else { style::SIGNAL };
        ui.text_at(text, &style, at, w + 8.0, Color::rgba(c.r, c.g, c.b, alpha));
    }
}

/// Write `image`, taken at `when`, into the screenshots folder: its file's
/// name, or why it couldn't be written.
fn write(image: &Image, when: SystemTime) -> Result<String, String> {
    let dir = dirs::user_dir(UserDir::Pictures).ok_or("no home to find Pictures under")?.join(FOLDER);
    write_in(&dir, image, when)
}

/// Write `image`, taken at `when`, into `dir` (made if it's not there).
fn write_in(dir: &Path, image: &Image, when: SystemTime) -> Result<String, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let seconds = when.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64);
    let path = free(dir, &stamp(local(seconds)));
    std::fs::write(&path, lntrn_image::encode_png(image)).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned()))
}

/// A file in `dir` named for the game and `stamp` that isn't there yet:
/// the second in one second is `…_2`, and so on.
fn free(dir: &Path, stamp: &str) -> PathBuf {
    let named = |n: u32| dir.join(if n < 2 { format!("{NAMED}_{stamp}.png") } else { format!("{NAMED}_{stamp}_{n}.png") });
    (1..).map(named).find(|p| !p.exists()).expect("a name not taken")
}

/// A moment, as the clock on the wall has it: year, month, day, hour,
/// minute, second.
type Moment = [i64; 6];

/// `2026-10-02_21-14-03`.
fn stamp([y, mo, d, h, mi, s]: Moment) -> String {
    format!("{y:04}-{mo:02}-{d:02}_{h:02}-{mi:02}-{s:02}")
}

/// C's broken-down time (`struct tm`, glibc's and musl's alike).
#[repr(C)]
struct Tm {
    sec: c_int,
    min: c_int,
    hour: c_int,
    mday: c_int,
    mon: c_int,
    year: c_int,
    wday: c_int,
    yday: c_int,
    isdst: c_int,
    gmtoff: c_long,
    zone: *const c_char,
}

unsafe extern "C" {
    fn localtime_r(time: *const i64, out: *mut Tm) -> *mut Tm;
}

/// `seconds` since 1970 as the clock on the wall here has it (in UTC, if
/// the system can't say).
fn local(seconds: i64) -> Moment {
    let mut tm = std::mem::MaybeUninit::<Tm>::zeroed();
    // SAFETY: `localtime_r` writes a whole `struct tm` into `tm` (64-bit
    // Linux: `time_t` is an i64) and hands back null if it couldn't, in
    // which case `tm` is left zeroed and isn't read.
    let filled = unsafe { !localtime_r(&seconds, tm.as_mut_ptr()).is_null() };
    if !filled {
        return utc(seconds);
    }
    // SAFETY: filled just above; every field is a plain integer or pointer.
    let tm = unsafe { tm.assume_init() };
    [i64::from(tm.year) + 1900, i64::from(tm.mon) + 1, i64::from(tm.mday), i64::from(tm.hour), i64::from(tm.min), i64::from(tm.sec)]
}

/// `seconds` since 1970, in UTC.
fn utc(seconds: i64) -> Moment {
    let (days, rest) = (seconds.div_euclid(86_400), seconds.rem_euclid(86_400));
    // Days to a date in the calendar as it is now, reckoned from a March
    // that starts each year (so the leap day is its last).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let of_era = z.rem_euclid(146_097);
    let year_of_era = (of_era - of_era / 1_460 + of_era / 36_524 - of_era / 146_096) / 365;
    let of_year = of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let m = (5 * of_year + 2) / 153;
    let (day, month) = (of_year - (153 * m + 2) / 5 + 1, if m < 10 { m + 3 } else { m - 9 });
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    [year, month, day, rest / 3600, rest % 3600 / 60, rest % 60]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shot_is_named_for_the_game_and_the_time() {
        assert_eq!(stamp(utc(0)), "1970-01-01_00-00-00");
        // 2026-10-02 21:14:03 UTC; a leap day; the last second of a year.
        assert_eq!(stamp(utc(1_790_975_643)), "2026-10-02_21-14-03");
        assert_eq!(stamp(utc(1_709_164_800)), "2024-02-29_00-00-00");
        assert_eq!(stamp(utc(1_735_689_599)), "2024-12-31_23-59-59");
        // The clock on the wall is within a day of UTC, and a real date.
        let [y, mo, d, h, ..] = local(1_790_975_643);
        assert!(y == 2026 && mo == 10 && (1..=3).contains(&d) && (0..24).contains(&h), "{y}-{mo}-{d} {h}h");
    }

    #[test]
    fn a_second_shot_in_a_second_gets_a_name_of_its_own() {
        let dir = std::env::temp_dir().join(format!("dead-signal-shots-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let first = free(&dir, "2026-10-02_21-14-03");
        assert_eq!(first.file_name().unwrap(), "dead-signal_2026-10-02_21-14-03.png");
        std::fs::write(&first, b"").unwrap();
        let second = free(&dir, "2026-10-02_21-14-03");
        assert_eq!(second.file_name().unwrap(), "dead-signal_2026-10-02_21-14-03_2.png");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_frame_s_written_as_a_picture_that_reads_back_the_same() {
        let dir = std::env::temp_dir().join(format!("dead-signal-written-{}", std::process::id()));
        let image = Image::new(3, 2, (0..24).map(|i| i * 10).collect());
        let when = UNIX_EPOCH + Duration::from_secs(1_790_975_643);
        let name = write_in(&dir, &image, when).expect("written");
        assert!(name.starts_with("dead-signal_2026-10-0") && name.ends_with(".png"), "{name}");
        let back = lntrn_image::decode(&std::fs::read(dir.join(&name)).unwrap()).expect("a picture");
        assert_eq!((back.width, back.height, &back.rgba), (3, 2, &image.rgba));
        // And again the same second: beside it, not over it.
        assert_ne!(write_in(&dir, &image, when).expect("written"), name);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
