//! The signal's meter as it's drawn, on the radio's card and by the
//! points: bars rising left to right, as on the handset's display and the
//! signs, each lit as far as it's charged, and green once it's whole (a
//! bar to spend).

use lntrn_math::{Color, Rect, Vec2};
use lntrn_ui::Ui;

use super::signal::{BARS, Signal};

/// A bar's width and the gap to the next, as shares of the meter's height;
/// and how tall the first is.
const BAR: f64 = 0.26;
const GAP: f64 = 0.13;
const FIRST: f64 = 0.36;
/// A whole bar's green.
const WHOLE: Color = Color::rgb(0.30, 0.92, 0.36);

/// How wide a meter `tall` is.
pub fn width(tall: f64) -> f64 {
    tall * (BAR * f64::from(BARS) + GAP * f64::from(BARS - 1))
}

/// Draw `signal`'s meter, `tall`, its bottom left corner at `at`: whole
/// bars green, the one being charged in `colour` as far up as it's got,
/// the rest only their troughs. All of it at `shows` of itself.
pub fn draw(ui: &mut Ui, at: Vec2, tall: f64, signal: &Signal, colour: Color, shows: f64) {
    let faded = |c: Color, a: f64| Color::rgba(c.r, c.g, c.b, c.a * a * shows);
    for i in 0..BARS {
        let high = tall * (FIRST + (1.0 - FIRST) * f64::from(i) / f64::from(BARS - 1));
        let x = at.x + tall * (BAR + GAP) * f64::from(i);
        let bar = |share: f64| Rect::from_min_size(Vec2::new(x, at.y - high * share), Vec2::new(tall * BAR, high * share));
        ui.draw.rect(bar(1.0), Color::rgba(0.0, 0.0, 0.0, 0.55 * shows));
        ui.draw.rect(bar(1.0), faded(colour, 0.16));
        let charged = (signal.bars() - f64::from(i)).clamp(0.0, 1.0);
        if charged > 0.0 {
            ui.draw.rect(bar(charged), if charged >= 1.0 { faded(WHOLE, 1.0) } else { faded(colour, 0.8) });
        }
    }
}
