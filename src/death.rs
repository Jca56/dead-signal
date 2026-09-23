//! Dying: the fall (the eye drops to the ground, rolling onto its side),
//! the fade to black, YOU DIED across a dark band in blood red, slowly
//! growing, and then the run's numbers beside a way out: another run, or
//! the title.

use lntrn_math::{Color, Rect, Vec2};
use lntrn_text::TextStyle;
use lntrn_ui::{Key, Ui};

use crate::camera::Camera;
use crate::menu::SideMenu;
use crate::stats::Stats;
use crate::style;

/// When each part starts, seconds after death.
const FALL_FOR: f64 = 1.1;
const FADE_FROM: f64 = 0.7;
const FADE_FOR: f64 = 1.3;
const WORDS_AT: f64 = 2.2;
const WORDS_IN: f64 = 1.6;
/// Past this the words can be skipped.
const SKIPPABLE: f64 = 3.2;
const STATS_AT: f64 = 6.0;
/// How far the eye falls, and how far over it rolls and tips.
const DROP: f64 = 1.35;
const ROLL: f64 = 1.3;
const TIP: f64 = 0.25;

/// What the player chose.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum After {
    Again,
    Title,
}

pub struct Death {
    t: f64,
    stats: Stats,
    /// Which way over the body rolls: -1 or 1.
    side: f64,
    menu: SideMenu<After>,
}

fn ease(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl Death {
    pub fn new(stats: Stats, side: f64) -> Self {
        Self { t: 0.0, stats, side: side.signum(), menu: SideMenu::new("YOU DIED", &[("TRY AGAIN", After::Again), ("TITLE", After::Title)]) }
    }

    pub fn update(&mut self, dt: f64) {
        self.t += dt;
    }

    /// Whether the numbers are up (and the pointer should be free).
    pub fn showing_stats(&self) -> bool {
        self.t >= STATS_AT
    }

    /// Whether the YOU DIED card has just come up this frame (for its sound).
    pub fn words_begin(&self, dt: f64) -> bool {
        self.t >= WORDS_AT && self.t - dt < WORDS_AT
    }

    /// The eye as it falls: dropped, rolled, tipped.
    pub fn fall(&self, camera: &mut Camera) {
        let f = ease(self.t / FALL_FOR);
        // A fall: slow to start, then all at once.
        let drop = f * f;
        camera.position.y -= DROP * drop;
        camera.roll = self.side * ROLL * f;
        camera.pitch -= TIP * f;
    }

    /// Draw whatever of it shows now; the choice made, once there is one.
    pub fn draw(&mut self, ui: &mut Ui, active: bool) -> Option<After> {
        let s = ui.m.scale;
        let screen = ui.clip();
        let black = ease((self.t - FADE_FROM) / FADE_FOR);
        if black > 0.0 {
            ui.draw.rect(screen, Color::rgba(0.0, 0.0, 0.0, black));
        }
        if self.t < STATS_AT {
            if self.t >= SKIPPABLE && active && (ui.state.pressed || ui.state.take_key(|k| matches!(k.key, Key::Enter | Key::Space | Key::Escape)).is_some()) {
                self.t = STATS_AT;
            }
            let shown = ease((self.t - WORDS_AT) / WORDS_IN);
            if shown > 0.0 {
                // A band a little lighter than the black, and the words.
                let band_h = screen.height() * 0.22;
                let band = Rect::from_min_size(Vec2::new(screen.min.x, screen.center().y - band_h * 0.5), Vec2::new(screen.width(), band_h));
                ui.draw.rect(band, Color::rgba(0.09, 0.02, 0.02, 0.9 * shown));
                let grow = 1.0 + 0.06 * ((self.t - WORDS_AT) / 4.0).min(1.0);
                let words = TextStyle::new((120.0 * s * grow) as f32).family(style::SERIF);
                let w = ui.measure("YOU DIED", &words);
                let h = f64::from(words.line_height());
                let at = Vec2::new(screen.center().x - w * 0.5, screen.center().y - h * 0.5);
                ui.text_at("YOU DIED", &words, at, screen.width(), Color::rgba(0.62, 0.07, 0.05, shown));
            }
            return None;
        }
        let shown = ease((self.t - STATS_AT) / 0.5);
        let chosen = self.menu.draw(ui, active && shown >= 1.0);
        self.numbers(ui, screen, shown);
        chosen
    }

    /// The run's numbers in two columns across the right of the screen.
    fn numbers(&self, ui: &mut Ui, screen: Rect, shown: f64) {
        let s = ui.m.scale;
        let title = TextStyle::new((35.0 * s) as f32).bold().family(style::FONT);
        let line = TextStyle::new((28.0 * s) as f32).family(style::FONT);
        let value = TextStyle::new((28.0 * s) as f32).bold().family(style::FONT);
        let (title_h, line_h) = (f64::from(title.line_height()), f64::from(line.line_height()));
        let fade = |c: Color| Color::rgba(c.r, c.g, c.b, c.a * shown);
        let col_w = 480.0 * s;
        let gap = 70.0 * s;
        let left = screen.max.x - 2.0 * col_w - gap - 90.0 * s;
        let top = screen.min.y + screen.height() * 0.16;
        let groups = self.stats.groups();
        // SURVIVAL and SHOOTING on the left; the rest on the right.
        for (column, range) in [(0usize, 0..2usize), (1, 2..groups.len())] {
            let x = left + column as f64 * (col_w + gap);
            let mut y = top;
            for g in &groups[range] {
                ui.text_at(g.title, &title, Vec2::new(x, y), col_w, fade(style::SIGNAL));
                y += title_h + 10.0 * s;
                for (label, v) in &g.lines {
                    ui.text_at(label, &line, Vec2::new(x, y), col_w, fade(style::DIM));
                    let w = ui.measure(v, &value);
                    ui.text_at(v, &value, Vec2::new(x + col_w - w, y), col_w, fade(style::BONE));
                    y += line_h + 6.0 * s;
                }
                y += 30.0 * s;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lntrn_math::Vec3;

    #[test]
    fn the_eye_falls_rolls_and_the_numbers_come_last() {
        let mut d = Death::new(Stats::default(), -1.0);
        let mut cam = Camera::new(Vec3::new(0.0, 1.7, 0.0));
        d.fall(&mut cam);
        assert!((cam.position.y - 1.7).abs() < 1e-9 && cam.roll == 0.0, "nothing yet at 0");
        for _ in 0..90 {
            d.update(1.0 / 60.0);
        }
        let mut cam = Camera::new(Vec3::new(0.0, 1.7, 0.0));
        d.fall(&mut cam);
        assert!(cam.position.y < 0.5 && cam.roll < -1.0, "down and over: {} {}", cam.position.y, cam.roll);
        assert!(!d.showing_stats());
        let mut came = false;
        for _ in 0..(60.0 * 6.0) as usize {
            d.update(1.0 / 60.0);
            came |= d.words_begin(1.0 / 60.0);
        }
        assert!(came, "the words came up once");
        assert!(d.showing_stats());
    }
}
