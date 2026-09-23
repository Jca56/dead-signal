//! The player's level and how full it is: LEVEL n, a bar, the XP in it.
//! After a run it fills with what was earned, rolling over into the next
//! level as it goes.

use lntrn_math::{Color, Rect, Vec2};
use lntrn_text::TextStyle;
use lntrn_ui::Ui;

use crate::profile::xp;
use crate::style;

/// The bar's colour, and its glow as it fills.
const FILL: Color = Color::rgb(0.78, 0.66, 0.30);
const FILLING: Color = Color::rgb(0.95, 0.86, 0.50);

/// Draw it at `at`, `width` wide (logical pixels): `xp` before, `gained`
/// more, `shown` (0–1) of the gain filled in so far, all faded by `alpha`.
pub fn draw(ui: &mut Ui, at: Vec2, width: f64, before: u32, gained: u32, shown: f64, alpha: f64) {
    let s = ui.m.scale;
    let now = before + (f64::from(gained) * shown.clamp(0.0, 1.0)).round() as u32;
    let (level, into, need) = xp::level(now);
    let (was_level, was_into, _) = xp::level(before);
    let fade = |c: Color| Color::rgba(c.r, c.g, c.b, c.a * alpha);
    let big = TextStyle::new((34.0 * s) as f32).bold().family(style::FONT);
    let small = TextStyle::new((24.0 * s) as f32).bold().family(style::FONT);
    let label = format!("LEVEL {level}");
    ui.text_at(&label, &big, at, width, fade(style::BONE));
    let lw = ui.measure(&label, &big);
    let numbers = format!("{into} / {need} XP");
    let nw = ui.measure(&numbers, &small);
    let big_h = f64::from(big.line_height());
    ui.text_at(&numbers, &small, Vec2::new(at.x + width - nw, at.y + big_h - f64::from(small.line_height())), width, fade(style::DIM));
    if gained > 0 {
        let plus = format!("+{gained} XP");
        ui.text_at(&plus, &small, Vec2::new(at.x + lw + 20.0 * s, at.y + big_h - f64::from(small.line_height())), width, fade(FILLING));
    }
    let bar = Rect::from_min_size(Vec2::new(at.x, at.y + big_h + 8.0 * s), Vec2::new(width, 22.0 * s));
    ui.draw.rect(bar, fade(Color::rgba(0.0, 0.0, 0.0, 0.6)));
    let part = |x: u32| bar.width() * f64::from(x) / f64::from(need);
    // What was there before (none of it, if a level has been passed since),
    // then what's been added, brighter.
    let base = if level == was_level { was_into } else { 0 };
    ui.draw.rect(Rect::from_min_size(bar.min, Vec2::new(part(base), bar.height())), fade(FILL));
    if into > base {
        ui.draw.rect(Rect::from_min_size(bar.min + Vec2::new(part(base), 0.0), Vec2::new(part(into - base), bar.height())), fade(FILLING));
    }
    ui.draw.stroke_rect(bar, 2.0 * s, 0.0, fade(Color::rgba(0.0, 0.0, 0.0, 0.9)));
}
