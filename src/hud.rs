//! What's drawn over a run: a dot at the middle, the hitmarker around it,
//! and the rounds left, big, bottom right.

use lntrn_math::{Color, Rect, Vec2};
use lntrn_text::TextStyle;
use lntrn_ui::Ui;

use crate::fx::Marker;
use crate::style;

/// Draw the HUD. `mag` is the rounds in the gun.
pub fn draw(ui: &mut Ui, mag: u32, marker: Option<Marker>) {
    let s = ui.m.scale;
    let screen = ui.clip();
    let mid = screen.center();

    // The dot, ringed dark so it shows against sky and ground alike.
    let dot = 5.0 * s;
    ui.draw.rect(Rect::from_min_size(mid - Vec2::new(dot, dot) * 0.5 - Vec2::new(2.0 * s, 2.0 * s), Vec2::new(dot + 4.0 * s, dot + 4.0 * s)), Color::rgba(0.0, 0.0, 0.0, 0.55));
    ui.draw.rect(Rect::from_min_size(mid - Vec2::new(dot, dot) * 0.5, Vec2::new(dot, dot)), style::BONE);

    // The hitmarker: four strokes on the diagonals, clear of the dot.
    if let Some(m) = marker {
        let (inner, outer, width, colour) = if m.beaten { (10.0 * s, 25.0 * s, 5.0 * s, style::SIGNAL) } else { (10.0 * s, 20.0 * s, 4.0 * s, style::BONE) };
        for (dx, dy) in [(1.0, 1.0), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] {
            let d = Vec2::new(dx, dy) * std::f64::consts::FRAC_1_SQRT_2;
            ui.draw.line(mid + d * inner, mid + d * outer, width, colour);
        }
    }

    // Rounds left, and the reserve (which never runs out, for now).
    let big = TextStyle::new((60.0 * s) as f32).bold().family(style::FONT);
    let small = TextStyle::new((35.0 * s) as f32).bold().family(style::FONT);
    let count = mag.to_string();
    let spare = " / ∞";
    let (w_count, w_spare) = (ui.measure(&count, &big), ui.measure(spare, &small));
    let right = screen.max.x - 60.0 * s;
    let base = screen.max.y - 50.0 * s;
    let colour = if mag == 0 { style::SIGNAL } else { style::BONE };
    let big_h = f64::from(big.line_height());
    let small_h = f64::from(small.line_height());
    ui.text_at(&count, &big, Vec2::new(right - w_spare - w_count, base - big_h), screen.width(), colour);
    ui.text_at(spare, &small, Vec2::new(right - w_spare, base - small_h - 5.0 * s), screen.width(), style::DIM);
}
