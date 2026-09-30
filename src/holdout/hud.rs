//! The holdout's own over the run, top left: the round, chalked up in
//! blood (tally marks for the first five, then its number), flaring as a
//! new one begins and dim between; under it the points, and what was just
//! earned rising off them.

use lntrn_math::{Color, Rect, Vec2};
use lntrn_text::TextStyle;
use lntrn_ui::Ui;

use super::Holdout;
use crate::style;

/// How long a "+50" stays up, rising and fading.
pub const POP_FOR: f64 = 1.1;
/// The round's red, and how long it flares as a round begins.
const BLOOD: Color = Color::rgb(0.66, 0.08, 0.06);
const FLARE: Color = Color::rgb(0.95, 0.85, 0.75);
const FLARE_FOR: f64 = 2.5;

/// The round and player `seat`'s points (and the others', smaller), over
/// `screen`: their own part of the window.
pub fn draw(ui: &mut Ui, screen: Rect, h: &Holdout, seat: usize) {
    let s = ui.m.scale;
    let left = screen.min.x + 60.0 * s;
    let top = screen.min.y + 50.0 * s;
    let round = h.rounds.round;
    // Bright as it begins, fading to its red; dim while resting.
    let colour = if h.rounds.resting() { mix(BLOOD, style::DIM, 0.4) } else { mix(FLARE, BLOOD, h.rounds.begun / FLARE_FOR) };
    let tall = 90.0 * s;
    if round == 0 {
        // Nothing yet.
    } else if round <= 5 {
        // Tally marks: four up, the fifth across them.
        let gap = 22.0 * s;
        let width = 9.0 * s;
        for k in 0..round.min(4) {
            let x = left + f64::from(k) * gap + width * 0.5;
            ui.draw.line(Vec2::new(x, top + tall), Vec2::new(x + 6.0 * s, top), width, colour);
        }
        if round == 5 {
            ui.draw.line(Vec2::new(left - 10.0 * s, top + tall * 0.75), Vec2::new(left + 4.0 * gap, top + tall * 0.2), width, colour);
        }
    } else {
        let big = TextStyle::new((96.0 * s) as f32).bold().family(style::SERIF);
        let text = round.to_string();
        let w = ui.measure(&text, &big);
        ui.text_at(&text, &big, Vec2::new(left, top + tall - f64::from(big.line_height()) * 0.9), w + 8.0, colour);
    }
    // The points, and what was just earned rising off them.
    let points = TextStyle::new((40.0 * s) as f32).bold().family(style::FONT);
    let small = TextStyle::new((28.0 * s) as f32).bold().family(style::FONT);
    let mut y = top + tall + 24.0 * s;
    let Some(wallet) = h.wallet(seat) else { return };
    let text = wallet.points.to_string();
    let w = ui.measure(&text, &points);
    ui.text_at(&text, &points, Vec2::new(left, y), w + 8.0, style::BONE);
    for (k, &(n, t)) in wallet.pops.iter().rev().take(6).enumerate() {
        let f = t / POP_FOR;
        let pop = format!("+{n}");
        let pw = ui.measure(&pop, &small);
        let at = Vec2::new(left + w + 20.0 * s + f64::from(k as u32) * 6.0 * s, y - f * 40.0 * s);
        let c = Color::rgba(0.95, 0.80, 0.30, 1.0 - f);
        ui.text_at(&pop, &small, at, pw + 8.0, c);
    }
    // Playing together: the others' points, in their colours.
    if h.wallets.len() > 1 {
        y += 50.0 * s;
        for (other, w) in h.wallets.iter().enumerate().filter(|&(other, _)| other != seat) {
            let line = format!("P{}  {}", other + 1, w.points);
            let lw = ui.measure(&line, &small);
            ui.text_at(&line, &small, Vec2::new(left, y), lw + 8.0, style::player(other));
            y += f64::from(small.line_height()) + 6.0 * s;
        }
    }
    // Resting: when the next round comes.
    if h.rounds.resting() {
        let words = if round == 0 { "GET READY".to_string() } else { format!("ROUND {} IN {}", round + 1, h.rounds.between.ceil() as u32) };
        let ww = ui.measure(&words, &small);
        ui.text_at(&words, &small, Vec2::new(left, y + 56.0 * s), ww + 8.0, style::DIM);
    }
}

fn mix(a: Color, b: Color, t: f64) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color::rgba(a.r + (b.r - a.r) * t, a.g + (b.g - a.g) * t, a.b + (b.b - a.b) * t, 1.0)
}
