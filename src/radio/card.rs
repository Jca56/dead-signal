//! The radio's card, drawn beside the handset while it's out: the signal
//! there is, and everything there is to call for, each with what it costs
//! in bars and its code of arrows, amber on dark as the handset's display
//! is; what costs points too, its price after its name. What there isn't
//! the signal for is dim, its cost in red (or the points: its price). As a
//! code's punched in, the lines it could still be stay lit, their arrows
//! so far bright, and the rest go dim; a wrong arrow (or a code there's
//! not the signal for) flashes it red; a whole code, and its line's lit across
//! while it's called in (and, a drop's, till its flare's thrown: the foot
//! says how). It keeps to the left of the handset, clear of
//! the middle of the view and of the health bottom left where there's
//! room for that, its lines as tall as the pane allows.

use lntrn_math::{Color, Rect, Vec2};
use lntrn_text::TextStyle;
use lntrn_ui::Ui;

use super::codes::{Arrow, ENTRIES, Entry};
use super::{Radio, WRONG_FOR, meter};
use crate::style;

const AMBER: Color = Color::rgb(0.98, 0.66, 0.18);
const LIT: Color = Color::rgb(1.0, 0.94, 0.72);
const DARK: Color = Color::rgb(0.05, 0.04, 0.02);
/// How much a line shows that the code so far isn't on the way to.
const DIMMED: f64 = 0.36;
/// The smallest its words are, pixels, whatever the pane.
const SMALLEST: f64 = 18.0;
/// A line's height at its shortest and tallest; and, with lines that
/// short and that tall, the header's, the foot's, and the space round it
/// all; times the scale.
const ROW: (f64, f64) = (26.0, 50.0);
const HEAD: (f64, f64) = (40.0, 52.0);
const FOOT: (f64, f64) = (28.0, 36.0);
const PAD: (f64, f64) = (10.0, 14.0);
const MARGIN: f64 = 16.0;
/// What's bottom left of a pane (health, armor, stamina): how far across
/// and up it reaches; and how far under the middle the dot and its ring
/// do.
const VITALS: (f64, f64) = (500.0, 185.0);
const MIDDLE: f64 = 30.0;
/// The most arrows a code's column holds.
const COLUMN: usize = 5;
/// How far after a name its price in points is, for each pixel of the
/// type's size.
const PRICE_GAP: f64 = 0.55;

/// A line's price in points, as it's written after its name (if it has
/// one).
fn price(e: &Entry) -> Option<String> {
    (e.points > 0).then(|| format!("[{}]", e.points))
}

/// What the foot of the card says: what dials, what puts it away, and
/// (a flare in hand, a strike to place) the trigger, which throws or
/// sends it.
pub struct Hints {
    pub dial: String,
    pub away: String,
    pub throw: String,
}

/// Where the card goes, and how tall each of its lines is.
#[derive(Clone, Copy, Debug)]
struct Lay {
    card: Rect,
    row: f64,
}

/// How big the words are in a line `row` tall.
fn type_size(row: f64, s: f64) -> f64 {
    (row * 0.56).clamp(SMALLEST, 28.0 * s)
}

/// How wide a code's column is, its lines `row` tall.
fn column(row: f64) -> f64 {
    row * 0.8 * COLUMN as f64
}

/// How wide the cost at the head of a line is, with the gap after it.
fn cost_wide(row: f64) -> f64 {
    row * 0.9
}

/// What's round the lines, theirs `row` tall: how tall the header is, and
/// the foot, and how much space is left round it all (the shorter the
/// lines, the less of each: there's little room).
fn chrome(row: f64, s: f64) -> (f64, f64, f64) {
    let t = ((row / s - ROW.0) / (ROW.1 - ROW.0)).clamp(0.0, 1.0);
    let at = |(least, most): (f64, f64)| (least + (most - least) * t) * s;
    (at(HEAD), at(FOOT), at(PAD))
}

/// How wide the card is with its lines `row` tall: a cost, its longest
/// name (`names` wide for each pixel of its type's size), a gap, and a
/// code's column, or its foot's words (`foot` wide) if they're wider.
fn width(row: f64, s: f64, names: f64, foot: f64) -> f64 {
    (cost_wide(row) + names * type_size(row, s) + row * 0.52 + column(row)).max(foot) + 2.0 * chrome(row, s).2
}

/// The card over `pane` of `window`, at scale `s` (`names` and `foot` as
/// [`width`] takes them): beside the handset, its foot on the pane's (or
/// on top of the health, where it's over that), its lines as tall as
/// there's room for. Where that would have it over the middle of the view
/// it goes under the middle instead, if it fits there with shorter lines;
/// else to the left of the middle, if that's clear of the health; else
/// it's over the middle after all.
fn lay(pane: Rect, window: Rect, s: f64, names: f64, foot: f64) -> Lay {
    let sees = crate::render::viewmodel_sees(window.width() / window.height().max(1.0), pane.width() / pane.height().max(1.0));
    let mid = pane.center();
    let (margin, clear) = (MARGIN * s, MIDDLE * s);
    let lines = ENTRIES.len() as f64;
    // Its right side at `right`, no higher than `ceiling`: the card with
    // the tallest lines that fit under that, and whether any did. (Shorter
    // lines make a narrower card, which may clear the health and so have
    // more room: each height's tried.)
    let fit = |right: f64, ceiling: f64| {
        let at = |row: f64| {
            let left = (right - width(row, s, names, foot)).max(pane.min.x + margin);
            let floor = if left < pane.min.x + VITALS.0 * s { pane.max.y - VITALS.1 * s } else { pane.max.y - margin };
            let (head, foot, pad) = chrome(row, s);
            let tall = head + foot + 2.0 * pad + row * lines;
            let top = floor - tall;
            (Lay { card: Rect::from_min_size(Vec2::new(left, top.max(pane.min.y + margin)), Vec2::new(right - left, tall)), row }, top >= ceiling - 1e-9)
        };
        let steps = ((ROW.1 - ROW.0) * s).ceil() as u32;
        (0..=steps).map(|k| at((ROW.1 * s - f64::from(k)).max(ROW.0 * s))).find(|(_, fits)| *fits).unwrap_or((at(ROW.0 * s).0, false))
    };
    let right = (mid.x + crate::viewmodel::radio_side(sees) * pane.width() * 0.5 - margin).min(pane.max.x - margin);
    let anywhere = pane.min.y + margin;
    let beside = fit(right, anywhere).0;
    if beside.card.min.x >= mid.x + clear || right <= mid.x - clear {
        return beside;
    }
    let (under, fits) = fit(right, mid.y + clear);
    if fits {
        return under;
    }
    let aside = fit(mid.x - clear, anywhere).0;
    if aside.card.min.x >= pane.min.x + VITALS.0 * s { aside } else { beside }
}

/// `c` at `a` of itself.
fn faded(c: Color, a: f64) -> Color {
    Color::rgba(c.r, c.g, c.b, c.a * a)
}

/// An arrow `size` across about `at`.
fn arrow(ui: &mut Ui, at: Vec2, size: f64, which: Arrow, colour: Color) {
    let d = match which {
        Arrow::Up => Vec2::new(0.0, -1.0),
        Arrow::Right => Vec2::new(1.0, 0.0),
        Arrow::Down => Vec2::new(0.0, 1.0),
        Arrow::Left => Vec2::new(-1.0, 0.0),
    };
    let across = Vec2::new(-d.y, d.x);
    let neck = at + d * (size * 0.04);
    ui.draw.line(at - d * (size * 0.48), neck, size * 0.26, colour);
    ui.draw.triangle(at + d * (size * 0.5), neck + across * (size * 0.42), neck - across * (size * 0.42), colour);
}

/// Draw `radio`'s card over `pane` of `window`, for someone with `purse`
/// points to spend (none: points are no matter).
pub fn draw(ui: &mut Ui, pane: Rect, window: Rect, radio: &Radio, purse: Option<u32>, hints: &Hints) {
    let s = ui.m.scale;
    let shows = radio.card();
    let words = |row: f64| TextStyle::new(type_size(row, s) as f32).bold().family(style::FONT);
    // As wide as its longest name (with its price, if it has one), a
    // code's column, and the foot's words.
    let tallest = words(ROW.1 * s);
    let tall = type_size(ROW.1 * s, s);
    let named = |ui: &mut Ui, e: &Entry| ui.measure(e.name, &tallest) + price(e).map_or(0.0, |p| PRICE_GAP * tall + ui.measure(&p, &tallest));
    let names = ENTRIES.iter().map(|e| named(ui, e)).fold(0.0, f64::max) / tall;
    let small = TextStyle::new((SMALLEST.max(18.0 * s)) as f32).bold().family(style::FONT);
    // (A flare in hand, the foot says how it's thrown; the card's as wide
    // for either.)
    let dialling = format!("{}  DIAL      {}  PUT AWAY", hints.dial, hints.away);
    let throwing = format!("HOLD {}  AIM      LET GO  THROW", hints.throw);
    let placing = format!("LOOK  MARK IT      {}  SEND IT", hints.throw);
    let foot_wide = ui.measure(&dialling, &small).max(ui.measure(&throwing, &small)).max(ui.measure(&placing, &small));
    let marking = radio.flare().or(radio.strike());
    let foot = match (radio.flare(), radio.strike()) {
        (Some(_), _) => throwing,
        (None, Some(_)) => placing,
        (None, None) => dialling,
    };
    let Lay { card, row } = lay(pane, window, s, names, foot_wide);
    let text = words(row);
    let (head_tall, foot_tall, pad) = chrome(row, s);

    // A wrong arrow: red, fading back to amber.
    let wrong = radio.wrong().map_or(0.0, |t| 1.0 - t / WRONG_FOR);
    let edge = Color::rgb(AMBER.r + (style::SIGNAL.r - AMBER.r) * wrong, AMBER.g + (style::SIGNAL.g - AMBER.g) * wrong, AMBER.b + (style::SIGNAL.b - AMBER.b) * wrong);
    // (Solid: the night's lamps and marks would read through its words.)
    ui.draw.rounded_rect(card, 10.0 * s, Color::rgba(DARK.r, DARK.g, DARK.b, shows));
    ui.draw.stroke_rect(card, 2.0 * s, 10.0 * s, faded(edge, (0.55 + 0.45 * wrong) * shows));

    // The header, ruled off.
    let head = TextStyle::new((type_size(row, s) * 1.15) as f32).bold().family(style::FONT);
    let title = match (marking, radio.calling()) {
        (Some(_), _) if radio.flare().is_none() => "MARK THE TARGET",
        (Some(_), _) => "THROW THE FLARE",
        (None, Some(_)) => "CALLING IN",
        (None, None) => "CALL IN",
    };
    ui.text_at(title, &head, Vec2::new(card.min.x + pad, card.min.y + pad + (head_tall - f64::from(head.line_height())) * 0.4), card.width(), faded(edge, shows));
    let rule = card.min.y + pad + head_tall - 8.0 * s;
    // The signal there is, at the header's right.
    let bars = head_tall - 22.0 * s;
    meter::draw(ui, Vec2::new(card.max.x - pad - meter::width(bars), rule - 7.0 * s), bars, &radio.signal, AMBER, shows);
    ui.draw.hline(card.min.x + pad, card.max.x - pad, rule, 2.0 * s, faded(edge, 0.45 * shows));

    // Each line: its name, and its code.
    let dial = radio.dial();
    let step = row * 0.8;
    let codes = card.max.x - pad - column(row);
    for (i, e) in ENTRIES.iter().enumerate() {
        let top = card.min.y + pad + head_tall + row * i as f64;
        let mid = top + row * 0.5;
        // (Being called in, or its flare still to throw: lit across.)
        let chosen = radio.calling().or(marking);
        let called = chosen == Some(e.call);
        // (Lit: what the code so far could still be, and there's the
        // signal for, and the points.)
        let (bars, poor) = (radio.signal.has(e.cost), purse.is_some_and(|p| p < e.points));
        let afford = bars && !poor;
        let live = match chosen {
            Some(_) => called,
            None => afford && dial.begins(e.code),
        };
        let a = shows * if live { 1.0 } else { DIMMED };
        if called {
            let bar = Rect::from_min_size(Vec2::new(card.min.x + pad * 0.5, top + 2.0 * s), Vec2::new(card.width() - pad, row - 4.0 * s));
            ui.draw.rounded_rect(bar, 5.0 * s, faded(AMBER, 0.92 * shows));
        }
        let ink = if called { DARK } else { AMBER };
        // What it costs, in bars (red, where there aren't that many).
        let cost = e.cost.to_string();
        let name_at = card.min.x + pad + cost_wide(row);
        let line = mid - f64::from(text.line_height()) * 0.5;
        let short = !bars && !called;
        let cost_at = card.min.x + pad + (cost_wide(row) * 0.6 - ui.measure(&cost, &text)) * 0.5;
        ui.text_at(&cost, &text, Vec2::new(cost_at, line), cost_wide(row), if short { faded(style::SIGNAL, shows) } else { faded(ink, a) });
        ui.text_at(e.name, &text, Vec2::new(name_at, line), codes - name_at, faded(ink, a));
        // What it costs in points, after its name (red, where they've not
        // that many).
        if let Some(price) = price(e) {
            let at = name_at + ui.measure(e.name, &text) + PRICE_GAP * type_size(row, s);
            ui.text_at(&price, &text, Vec2::new(at, line), codes - at, if poor && !called { faded(style::SIGNAL, shows) } else { faded(ink, a) });
        }
        for (k, &which) in e.code.iter().enumerate() {
            // (Punched in already, on a line it's on the way to: bright.)
            let colour = if called { DARK } else if live && k < dial.len() { LIT } else { faded(AMBER, 0.8) };
            arrow(ui, Vec2::new(codes + step * (k as f64 + 0.5), mid), row * 0.6, which, faded(colour, a));
        }
    }

    // The foot: what dials, and what puts it away.
    let at = Vec2::new(card.min.x + pad, card.max.y - pad - foot_tall + (foot_tall - f64::from(small.line_height())) * 0.7);
    ui.text_at(&foot, &small, at, card.width(), faded(AMBER, 0.6 * shows));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A longest name's width for each pixel of its type's size, and a
    /// foot's (as the game's own type has them).
    const NAMES: f64 = 12.3;
    const FOOT_WIDE: f64 = 330.0;

    fn rect(x: f64, y: f64, w: f64, h: f64) -> Rect {
        Rect::from_min_size(Vec2::new(x, y), Vec2::new(w, h))
    }

    /// How far across the handset's near side is, in `pane` of `window`.
    fn handset(pane: Rect, window: Rect) -> f64 {
        let sees = crate::render::viewmodel_sees(window.width() / window.height(), pane.width() / pane.height());
        pane.center().x + crate::viewmodel::radio_side(sees) * pane.width() * 0.5
    }

    #[test]
    fn alone_it_s_beside_the_handset_under_the_middle_its_lines_near_their_tallest() {
        let window = rect(0.0, 0.0, 1920.0, 1080.0);
        let l = lay(window, window, 1.0, NAMES, FOOT_WIDE);
        assert!(l.row >= 46.0 && l.row <= ROW.1, "{l:?}");
        assert!(l.card.min.x > VITALS.0 && l.card.max.x < handset(window, window), "{:?}", l.card);
        assert!(l.card.max.y <= 1080.0 - MARGIN + 1e-9 && l.card.min.y > 540.0 + MIDDLE, "{:?}", l.card);
        assert!((l.card.width() - width(l.row, 1.0, NAMES, FOOT_WIDE)).abs() < 1e-9);
    }

    #[test]
    fn where_there_s_no_room_under_the_middle_it_s_to_the_left_of_it() {
        // One above the other: too short a pane to be under the middle,
        // so it's left of it (clear of the health), its lines as tall as
        // fit between the two.
        let window = rect(0.0, 0.0, 1920.0, 1080.0);
        let pane = rect(0.0, 540.0, 1920.0, 540.0);
        let l = lay(pane, window, 1.0, NAMES, FOOT_WIDE);
        assert!(l.row > 30.0 && type_size(l.row, 1.0) >= SMALLEST, "{l:?}");
        assert!(l.card.min.x >= VITALS.0 && l.card.max.x <= 960.0 - MIDDLE + 1e-9, "{:?}", l.card);
        assert!(l.card.min.y >= pane.min.y + MARGIN - 1e-9 && l.card.max.y <= 1080.0 - MARGIN + 1e-9, "{:?}", l.card);
    }

    #[test]
    fn where_it_s_over_the_middle_its_lines_shorten_till_it_s_under_it() {
        // A small window: the card would reach up over the dot.
        let window = rect(0.0, 0.0, 1280.0, 720.0);
        let l = lay(window, window, 1.0, NAMES, FOOT_WIDE);
        assert!(l.card.min.x < 640.0 && l.card.max.x > 640.0, "across the middle: {:?}", l.card);
        assert!(l.row < ROW.1 && l.row >= ROW.0 && l.card.min.y >= 360.0 + MIDDLE - 1e-9, "{l:?}");
        assert!(type_size(l.row, 1.0) >= SMALLEST);
        // Side by side: over the health too, which it keeps above.
        let window = rect(0.0, 0.0, 1920.0, 1080.0);
        let pane = rect(960.0, 0.0, 960.0, 1080.0);
        let l = lay(pane, window, 1.0, NAMES, FOOT_WIDE);
        assert!(l.card.min.x >= pane.min.x + MARGIN && l.card.max.x < handset(pane, window), "{:?}", l.card);
        assert!(l.card.max.y <= 1080.0 - VITALS.1 + 1e-9 && l.card.min.y >= 540.0 + MIDDLE - 1e-9, "{l:?}");
    }

    #[test]
    fn in_a_pane_too_small_for_any_of_that_it_s_over_the_middle_in_the_pane_and_clear_of_the_health() {
        let window = rect(0.0, 0.0, 1280.0, 720.0);
        for pane in [rect(0.0, 0.0, 1280.0, 360.0), rect(0.0, 360.0, 1280.0, 360.0), rect(640.0, 0.0, 640.0, 720.0)] {
            let l = lay(pane, window, 1.0, NAMES, FOOT_WIDE);
            assert!(l.row >= ROW.0 && type_size(l.row, 1.0) >= SMALLEST);
            assert!(l.card.min.x >= pane.min.x && l.card.max.x <= pane.max.x && l.card.min.y >= pane.min.y && l.card.max.y <= pane.max.y, "{:?} in {pane:?}", l.card);
            let on_health = l.card.min.x < pane.min.x + VITALS.0 && l.card.max.y > pane.max.y - VITALS.1 + 1e-9;
            assert!(!on_health, "{:?} in {pane:?}", l.card);
        }
    }

    #[test]
    fn it_draws_whatever_the_radio_s_doing() {
        let mut h = lntrn_ui::testing::Harness::new(1280.0, 720.0);
        let hints = Hints { dial: "WASD".into(), away: "Q".into(), throw: "MOUSE LEFT".into() };
        let mut radio = Radio::default();
        radio.pull();
        for arrow in [Arrow::Down, Arrow::Down, Arrow::Left, Arrow::Up, Arrow::Right, Arrow::Right] {
            radio.update(true, 0.2);
            radio.press(arrow);
            h.frame(|ui| {
                let window = ui.clip();
                draw(ui, window, window, &radio, Some(900), &hints);
                assert!(!ui.draw.is_empty());
            });
        }
        assert!(radio.calling().is_some());
    }
}
