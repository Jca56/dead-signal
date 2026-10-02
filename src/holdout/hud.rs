//! The holdout's own over the run, top left: the round, chalked up in
//! blood (tally marks for the first five, then its number), flaring as a
//! new one begins and dim between; under it the points, and what was just
//! earned rising off them; under them the radio's signal, with the key
//! that pulls it out; in a breather, what the radio says is coming; and,
//! top middle, the boosts that are up (and the gunship), each with how
//! long it has left.

use lntrn_math::{Color, Rect, Vec2};
use lntrn_text::TextStyle;
use lntrn_ui::Ui;

use super::Holdout;
use crate::radio::codes::Call;
use crate::radio::signal::Signal;
use super::rounds::Wave;
use crate::style;

/// How long a "+50" stays up, rising and fading.
pub const POP_FOR: f64 = 1.1;
/// The round's red, and how long it flares as a round begins.
const BLOOD: Color = Color::rgb(0.66, 0.08, 0.06);
const FLARE: Color = Color::rgb(0.95, 0.85, 0.75);
const FLARE_FOR: f64 = 2.5;

/// The radio's signal's amber; and its boosts' colours.
const AMBER: Color = Color::rgb(0.98, 0.66, 0.18);
const DOUBLE: Color = Color::rgb(1.0, 0.82, 0.25);
const INSTAKILL: Color = Color::rgb(1.0, 0.30, 0.22);
const GUNSHIP: Color = Color::rgb(0.70, 0.84, 1.0);

/// The round and player `seat`'s points (and the others', smaller), over
/// `screen`: their own part of the window; and their radio's `signal`,
/// with what pulls the radio out.
pub fn draw(ui: &mut Ui, screen: Rect, h: &Holdout, seat: usize, signal: Option<(&Signal, &str)>) {
    boosts(ui, screen, h);
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
    // The radio's signal: its bars, and the key that pulls it out.
    if let Some((signal, key)) = signal {
        let bars = 34.0 * s;
        let foot = y + 54.0 * s + bars;
        crate::radio::meter::draw(ui, Vec2::new(left, foot), bars, signal, AMBER, 1.0);
        let kw = ui.measure(key, &small);
        ui.text_at(key, &small, Vec2::new(left + crate::radio::meter::width(bars) + 14.0 * s, foot - f64::from(small.line_height()) * 0.86), kw + 8.0, style::DIM);
        y += 46.0 * s;
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
    // Resting: when the next round comes, and what the radio says of it.
    if h.rounds.resting() {
        let words = if round == 0 { "GET READY".to_string() } else { format!("ROUND {} IN {}", round + 1, h.rounds.between.ceil() as u32) };
        let ww = ui.measure(&words, &small);
        ui.text_at(&words, &small, Vec2::new(left, y + 56.0 * s), ww + 8.0, style::DIM);
        if let Some(warning) = h.rounds.warning() {
            let wide = ui.measure(warning, &points);
            let beat = 0.6 + 0.4 * (h.rounds.between * 5.0).sin().abs();
            ui.text_at(warning, &points, Vec2::new(left, y + 56.0 * s + f64::from(small.line_height()) + 10.0 * s), wide + 8.0, Color::rgba(style::SIGNAL.r, style::SIGNAL.g, style::SIGNAL.b, beat));
        }
    } else if h.rounds.wave == Wave::Hounds {
        let ww = ui.measure("HELLHOUNDS", &small);
        ui.text_at("HELLHOUNDS", &small, Vec2::new(left, y + 56.0 * s), ww + 8.0, style::SIGNAL);
    }
}

/// The boosts that are up, across the top middle of `screen`: each its
/// name, the seconds it has left, and a bar running down under it;
/// blinking as it runs out.
fn boosts(ui: &mut Ui, screen: Rect, h: &Holdout) {
    use super::boosts::ENDING;
    let s = ui.m.scale;
    let colour = |call| match call {
        Call::Instakill => INSTAKILL,
        Call::Gunship => GUNSHIP,
        _ => DOUBLE,
    };
    let up: Vec<(String, f64, f64, Color)> = h.boosts.up().map(|(n, left, of, call)| (format!("{n}  {}", left.ceil() as u32), left, of, colour(call))).collect();
    // (Each as wide as there's room for, across a narrow pane; its words
    // no bigger than fit it.)
    let gap = 40.0 * s;
    let room = (screen.width() - 80.0 * s - gap * (up.len() as f64 - 1.0).max(0.0)) / (up.len() as f64).max(1.0);
    let wide = (330.0 * s).min(room);
    let name = TextStyle::new((38.0 * s * (wide / (330.0 * s)).max(0.55)) as f32).bold().family(style::FONT);
    let all = up.len() as f64 * wide + (up.len() as f64 - 1.0).max(0.0) * gap;
    let top = screen.min.y + 44.0 * s;
    for (i, (words, left, of, colour)) in up.into_iter().enumerate() {
        let x = screen.center().x - all * 0.5 + i as f64 * (wide + gap);
        let shows = if left < ENDING { 0.4 + 0.6 * (left * 7.0).sin().abs() } else { 1.0 };
        let c = Color::rgba(colour.r, colour.g, colour.b, shows);
        let w = ui.measure(&words, &name);
        ui.text_at(&words, &name, Vec2::new(x + (wide - w) * 0.5, top), w + 8.0, c);
        let under = top + f64::from(name.line_height()) + 6.0 * s;
        ui.draw.rect(Rect::from_min_size(Vec2::new(x, under), Vec2::new(wide, 8.0 * s)), Color::rgba(0.0, 0.0, 0.0, 0.55));
        ui.draw.rect(Rect::from_min_size(Vec2::new(x, under), Vec2::new(wide * (left / of).min(1.0), 8.0 * s)), c);
    }
}

fn mix(a: Color, b: Color, t: f64) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color::rgba(a.r + (b.r - a.r) * t, a.g + (b.g - a.g) * t, a.b + (b.b - a.b) * t, 1.0)
}
