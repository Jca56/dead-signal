//! How a run ends: dying (the eye drops to the ground, rolling onto its
//! side; YOU DIED in blood red) or getting out (EXTRACTED in the radio's
//! green, and how). Either way the screen goes black, the words come up
//! across a band and slowly grow, and then the run's numbers beside a way
//! on: another run, or the title. Under the way on, what was carried:
//! got out, or lost.

use lntrn_math::{Color, Rect, Vec2};
use lntrn_text::TextStyle;
use lntrn_ui::{Key, Ui};

use crate::bag_ui::Icons;
use crate::camera::Camera;
use crate::exits::Way;
use crate::exits::hud::SIGNAL_GREEN;
use crate::loot::Stack;
use crate::loot::bag::Bag;
use crate::menu::SideMenu;
use crate::profile::xp::Earned;
use crate::stats::{Group, Stats};
use crate::style;

/// When each part starts, seconds after the end.
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
    /// To the hideout, to see what's left and pack for the next run.
    Hideout,
    Title,
}

/// How it ended.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Outcome {
    /// Dead, fallen over to this side (-1 or 1).
    Died(f64),
    Extracted(Way),
}

pub struct Ending {
    t: f64,
    pub outcome: Outcome,
    stats: Stats,
    /// What was carried at the end and got out, or was lost (dead: all
    /// but the pockets), and what it was worth; and, dead, what the
    /// pockets kept safe.
    loot: Vec<Stack>,
    value: u32,
    kept: Vec<Stack>,
    /// What the run earned, and the XP there was before it.
    earned: Earned,
    xp_before: u32,
    menu: SideMenu<After>,
}

fn ease(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl Ending {
    pub fn new(outcome: Outcome, stats: Stats, bag: &Bag, earned: Earned, xp_before: u32) -> Self {
        let menu = match outcome {
            Outcome::Died(_) => SideMenu::new("YOU DIED", &[("HIDEOUT", After::Hideout), ("TITLE", After::Title)]),
            Outcome::Extracted(_) => SideMenu::new("EXTRACTED", &[("HIDEOUT", After::Hideout), ("TITLE", After::Title)]),
        };
        let (loot, kept): (Vec<Stack>, Vec<Stack>) = match outcome {
            Outcome::Extracted(_) => (bag.everything().collect(), Vec::new()),
            Outcome::Died(_) => {
                // All but the pockets: the weapons, what's worn, the pack,
                // the rig and the belt.
                let pockets: Vec<Stack> = bag.pockets.items.iter().map(|i| i.stack).collect();
                let mut lost: Vec<Stack> = bag.everything().collect();
                for p in &pockets {
                    if let Some(i) = lost.iter().position(|s| s == p) {
                        lost.remove(i);
                    }
                }
                (lost, pockets)
            }
        };
        let value = loot.iter().map(|s| s.value()).sum();
        Self { t: 0.0, outcome, stats, loot, value, kept, earned, xp_before, menu }
    }

    pub fn update(&mut self, dt: f64) {
        self.t += dt;
    }

    /// Whether the numbers are up (and the pointer should be free).
    pub fn showing_stats(&self) -> bool {
        self.t >= STATS_AT
    }

    /// Whether the words have just come up this frame (for their sound).
    pub fn words_begin(&self, dt: f64) -> bool {
        self.t >= WORDS_AT && self.t - dt < WORDS_AT
    }

    /// The eye, if it falls: dropped, rolled, tipped.
    pub fn fall(&self, camera: &mut Camera) {
        let Outcome::Died(side) = self.outcome else { return };
        let f = ease(self.t / FALL_FOR);
        // A fall: slow to start, then all at once.
        let drop = f * f;
        camera.position.y -= DROP * drop;
        camera.roll = side * ROLL * f;
        camera.pitch -= TIP * f;
    }

    /// The words, their colour, and the band's.
    fn words(&self) -> (&'static str, Color, Color) {
        match self.outcome {
            Outcome::Died(_) => ("YOU DIED", Color::rgb(0.62, 0.07, 0.05), Color::rgb(0.09, 0.02, 0.02)),
            Outcome::Extracted(_) => ("EXTRACTED", SIGNAL_GREEN, Color::rgb(0.02, 0.07, 0.03)),
        }
    }

    /// Draw whatever of it shows now; the choice made, once there is one.
    pub fn draw(&mut self, ui: &mut Ui, active: bool, icons: &Icons) -> Option<After> {
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
                let (text, colour, band_colour) = self.words();
                let band_h = screen.height() * 0.22;
                let band = Rect::from_min_size(Vec2::new(screen.min.x, screen.center().y - band_h * 0.5), Vec2::new(screen.width(), band_h));
                ui.draw.rect(band, Color::rgba(band_colour.r, band_colour.g, band_colour.b, 0.9 * shown));
                let grow = 1.0 + 0.06 * ((self.t - WORDS_AT) / 4.0).min(1.0);
                let words = TextStyle::new((120.0 * s * grow) as f32).family(style::SERIF);
                let w = ui.measure(text, &words);
                let h = f64::from(words.line_height());
                let at = Vec2::new(screen.center().x - w * 0.5, screen.center().y - h * 0.5);
                ui.text_at(text, &words, at, screen.width(), Color::rgba(colour.r, colour.g, colour.b, shown));
                if let Outcome::Extracted(way) = self.outcome {
                    let how = match way {
                        Way::Radio => "BY RADIO",
                        Way::Road => "BY ROAD",
                        Way::Truck => "BY TRUCK",
                    };
                    let small = TextStyle::new((30.0 * s) as f32).bold().family(style::FONT);
                    let hw = ui.measure(how, &small);
                    ui.text_at(how, &small, Vec2::new(screen.center().x - hw * 0.5, at.y + h + 4.0 * s), hw + 4.0, Color::rgba(style::BONE.r, style::BONE.g, style::BONE.b, shown));
                }
            }
            return None;
        }
        let shown = ease((self.t - STATS_AT) / 0.5);
        let chosen = self.menu.draw(ui, active && shown >= 1.0);
        self.numbers(ui, screen, shown);
        self.carried(ui, screen, shown, icons);
        // The level, filling with what was earned (if it was kept).
        let filled = ease((self.t - STATS_AT - 0.6) / 1.8);
        let at = Vec2::new(screen.min.x + 120.0 * s, screen.min.y + screen.height() * 0.09);
        crate::levelbar::draw(ui, at, 620.0 * s, self.xp_before, self.earned.banked(), filled, shown);
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
        let mut groups = self.stats.groups();
        // What it earned, last; struck off if it was lost.
        let mut lines: Vec<(&'static str, String)> = self.earned.lines.iter().map(|(l, n)| (*l, format!("+{n}"))).collect();
        lines.push(("Total", if self.earned.kept { format!("+{}", self.earned.total()) } else { "LOST".into() }));
        groups.push(Group { title: if self.earned.kept { "XP" } else { "XP · LOST" }, lines });
        // SURVIVAL, SHOOTING and LOOT on the left; the rest on the right.
        for (column, range) in [(0usize, 0..3usize), (1, 3..groups.len())] {
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

    /// What was carried, under the way on: got out with, or lost and (in
    /// the pockets) kept.
    fn carried(&self, ui: &mut Ui, screen: Rect, shown: f64, icons: &Icons) {
        let s = ui.m.scale;
        let got_out = matches!(self.outcome, Outcome::Extracted(_));
        let heading = format!("{}  ${}", if got_out { "GOT OUT WITH" } else { "LOST" }, self.value);
        let at = Vec2::new(screen.min.x + 120.0 * s, screen.min.y + screen.height() * 0.58);
        let y = row(ui, &heading, if got_out { SIGNAL_GREEN } else { style::SIGNAL }, &self.loot, at, shown, icons);
        if !self.kept.is_empty() {
            let worth: u32 = self.kept.iter().map(|s| s.value()).sum();
            row(ui, &format!("KEPT · POCKETS  ${worth}"), SIGNAL_GREEN, &self.kept, Vec2::new(at.x, y + 24.0 * s), shown, icons);
        }
    }
}

/// A heading and under it each of `stacks`' picture and how many, from
/// `at`; where it ends, down the screen.
fn row(ui: &mut Ui, heading: &str, colour: Color, stacks: &[Stack], at: Vec2, shown: f64, icons: &Icons) -> f64 {
    let s = ui.m.scale;
    let title = TextStyle::new((35.0 * s) as f32).bold().family(style::FONT);
    let (left, mut y) = (at.x, at.y);
    ui.text_at(heading, &title, Vec2::new(left, y), ui.clip().width(), Color::rgba(colour.r, colour.g, colour.b, shown));
    y += f64::from(title.line_height()) + 12.0 * s;
    if stacks.is_empty() {
        let line = TextStyle::new((28.0 * s) as f32).family(style::FONT);
        ui.text_at("Nothing.", &line, Vec2::new(left, y), ui.clip().width(), Color::rgba(style::DIM.r, style::DIM.g, style::DIM.b, shown));
        return y + f64::from(line.line_height());
    }
    let tile = 64.0 * s;
    let per_row = 10;
    let count = TextStyle::new((20.0 * s) as f32).bold().family(style::FONT);
    for (i, stack) in stacks.iter().enumerate() {
        let at = Vec2::new(left + (i % per_row) as f64 * (tile + 8.0 * s), y + (i / per_row) as f64 * (tile + 8.0 * s));
        let r = Rect::from_min_size(at, Vec2::splat(tile));
        let c = stack.kind.def().rarity.colour();
        ui.draw.rect(r, Color::rgba(c.r * 0.35, c.g * 0.35, c.b * 0.35, 0.85 * shown));
        ui.draw.stroke_rect(r, 2.0 * s, 0.0, Color::rgba(c.r, c.g, c.b, shown));
        if let Some(&(flat, _)) = icons.0.get(&stack.kind) {
            // A long thing fitted into a square, keeping its shape.
            let aspect = f64::from(flat.width) / f64::from(flat.height.max(1));
            let inner = r.shrink(5.0 * s);
            let size = if aspect >= 1.0 { Vec2::new(inner.width(), inner.width() / aspect) } else { Vec2::new(inner.height() * aspect, inner.height()) };
            ui.draw.image(Rect::from_center_size(inner.center(), size), flat, 0.0, Color::rgba(1.0, 1.0, 1.0, shown));
        }
        if stack.count > 1 {
            let n = stack.count.to_string();
            let w = ui.measure(&n, &count);
            ui.text_at(&n, &count, Vec2::new(r.max.x - w - 5.0 * s, r.max.y - f64::from(count.line_height()) - 2.0 * s), w + 4.0, Color::rgba(style::BONE.r, style::BONE.g, style::BONE.b, shown));
        }
    }
    y + stacks.len().div_ceil(per_row) as f64 * (tile + 8.0 * s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lntrn_math::Vec3;

    #[test]
    fn the_eye_falls_rolls_and_the_numbers_come_last() {
        let mut d = Ending::new(Outcome::Died(-1.0), Stats::default(), &Bag::with_rounds(), Earned::default(), 0);
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

    #[test]
    fn getting_out_keeps_the_eye_up_and_counts_what_was_carried() {
        let bag = Bag::with_rounds();
        let mut e = Ending::new(Outcome::Extracted(Way::Radio), Stats::default(), &bag, Earned::default(), 0);
        for _ in 0..90 {
            e.update(1.0 / 60.0);
        }
        let mut cam = Camera::new(Vec3::new(0.0, 1.7, 0.0));
        e.fall(&mut cam);
        assert!((cam.position.y - 1.7).abs() < 1e-9 && cam.roll == 0.0, "the extracted don't fall");
        assert_eq!((e.loot.len(), e.value), (1, bag.value()), "the pocket's rounds");
    }

    #[test]
    fn dead_the_pockets_are_kept_not_lost() {
        use crate::loot::bag::Slot;
        use crate::loot::{Kind, Stack};
        let mut bag = Bag::with_rounds();
        bag.pack.place(Stack::new(Kind::Rounds, 24));
        bag.pack.place(Stack::one(Kind::Watch));
        *bag.slot_mut(Slot::Sidearm) = Some(Stack::gun(Kind::Pistol, 5));
        let e = Ending::new(Outcome::Died(1.0), Stats::default(), &bag, Earned::default(), 0);
        // The pocket's 24 rounds are kept; the pack's 24 (just the same) and
        // the rest are lost.
        assert_eq!(e.kept, vec![Stack::new(Kind::Rounds, 24)]);
        assert_eq!(e.loot.len(), 3, "{:?}", e.loot);
        assert!(e.loot.contains(&Stack::new(Kind::Rounds, 24)) && e.loot.contains(&Stack::gun(Kind::Pistol, 5)));
        assert_eq!(e.value, bag.value() - Stack::new(Kind::Rounds, 24).value());
        // Got out: it all came, nothing set apart.
        let e = Ending::new(Outcome::Extracted(Way::Radio), Stats::default(), &bag, Earned::default(), 0);
        assert_eq!((e.loot.len(), e.kept.len(), e.value), (4, 0, bag.value()));
    }
}
