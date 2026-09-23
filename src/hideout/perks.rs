//! The perks page: every perk a card (its name, its ranks, what it does
//! now and what the next rank would, a button to take it and one to give
//! it back), and the points there are to spend.

use lntrn_math::{Color, Rect, Vec2};
use lntrn_text::TextStyle;
use lntrn_ui::{Sense, Ui};

use crate::profile::Profile;
use crate::profile::perks::{self, ALL, RANKS};
use crate::style;

/// The ranks' colour, taken and not.
const TAKEN: Color = Color::rgb(0.78, 0.66, 0.30);

/// A frame of the page, `active` unless a fade is running. A word to flash,
/// if something couldn't be done.
pub fn page(ui: &mut Ui, profile: &mut Profile, active: bool) -> Option<&'static str> {
    let s = ui.m.scale;
    let screen = ui.clip();
    let big = TextStyle::new((40.0 * s) as f32).bold().family(style::FONT);
    let name = TextStyle::new((32.0 * s) as f32).bold().family(style::FONT);
    let line = TextStyle::new((24.0 * s) as f32).family(style::FONT);
    let button = TextStyle::new((26.0 * s) as f32).bold().family(style::FONT);
    let points = profile.points_left();
    let head = format!("{points} PERK POINT{} TO SPEND", if points == 1 { "" } else { "S" });
    let top = screen.min.y + screen.height() * 0.19;
    let hw = ui.measure(&head, &big);
    ui.text_at(&head, &big, Vec2::new(screen.center().x - hw * 0.5, top), hw + 4.0, if points > 0 { style::SIGNAL } else { style::DIM });

    // Four across, two down (narrower if the screen is).
    let gap = 28.0 * s;
    let card_w = (470.0 * s).min((screen.width() - 160.0 * s - 3.0 * gap) / 4.0);
    let card_h = 270.0 * s;
    let grid_w = 4.0 * card_w + 3.0 * gap;
    let left = screen.center().x - grid_w * 0.5;
    let first_y = top + f64::from(big.line_height()) + 30.0 * s;
    let mut note = None;
    for (i, perk) in ALL.into_iter().enumerate() {
        let at = Vec2::new(left + (i % 4) as f64 * (card_w + gap), first_y + (i / 4) as f64 * (card_h + gap));
        let card = Rect::from_min_size(at, Vec2::new(card_w, card_h));
        let rank = profile.perks.rank(perk);
        ui.draw.rect(card, Color::rgba(0.05, 0.05, 0.05, 0.9));
        ui.draw.stroke_rect(card, 2.0 * s, 0.0, if rank > 0 { TAKEN } else { Color::rgba(1.0, 1.0, 1.0, 0.15) });
        let pad = 22.0 * s;
        let mut y = at.y + pad;
        ui.text_at(perk.name(), &name, Vec2::new(at.x + pad, y), card_w, style::BONE);
        // Its ranks: a pip each, filled when taken.
        for k in 0..RANKS {
            let pip = Rect::from_min_size(Vec2::new(at.x + card_w - pad - f64::from(RANKS - k) * 30.0 * s, y + 6.0 * s), Vec2::splat(22.0 * s));
            if k < rank {
                ui.draw.rect(pip, TAKEN);
            }
            ui.draw.stroke_rect(pip, 2.0 * s, 0.0, TAKEN);
        }
        y += f64::from(name.line_height()) + 14.0 * s;
        let now = if rank == 0 { "Not taken".to_string() } else { perk.does(rank) };
        ui.text_at(&now, &line, Vec2::new(at.x + pad, y), card_w - 2.0 * pad, if rank == 0 { style::DIM } else { style::BONE });
        y += f64::from(line.line_height()) + 6.0 * s;
        let next = if rank >= RANKS { "MAXED".to_string() } else { format!("Next: {}", perk.does(rank + 1)) };
        ui.text_at(&next, &line, Vec2::new(at.x + pad, y), card_w - 2.0 * pad, if rank >= RANKS { TAKEN } else { style::DIM });

        // Take a rank, give one back.
        let bh = f64::from(button.line_height()) + 20.0 * s;
        let by = at.y + card_h - pad - bh;
        if rank < RANKS {
            let cost = perks::cost(rank);
            let label = format!("+ RANK  ({cost} PT{})", if cost == 1 { "" } else { "S" });
            let can = points >= cost;
            let w = ui.measure(&label, &button) + 36.0 * s;
            if pressed(ui, &format!("raise {i}"), Rect::from_min_size(Vec2::new(at.x + pad, by), Vec2::new(w, bh)), &label, &button, can, true, active) {
                profile.raise(perk);
            }
        }
        if rank > 0 {
            let w = ui.measure("REFUND", &button) + 36.0 * s;
            if pressed(ui, &format!("lower {i}"), Rect::from_min_size(Vec2::new(at.x + card_w - pad - w, by), Vec2::new(w, bh)), "REFUND", &button, true, false, active) && !profile.lower(perk) {
                note = Some("MAKE ROOM IN THE STASH FIRST");
            }
        }
    }
    note
}

/// A button: drawn (bright when it can be used, primary in red), and
/// whether it was clicked.
#[allow(clippy::too_many_arguments)]
pub(super) fn pressed(ui: &mut Ui, id: &str, r: Rect, label: &str, style_: &TextStyle, can: bool, primary: bool, active: bool) -> bool {
    let s = ui.m.scale;
    let hit = ui.interact(ui.id(id), r, Sense::CLICK);
    let lit = active && can && hit.hovered;
    let fill = match (can, primary) {
        (false, _) => Color::rgba(1.0, 1.0, 1.0, 0.04),
        (true, true) if lit => style::SIGNAL,
        (true, true) => Color::rgb(0.55, 0.10, 0.07),
        (true, false) if lit => Color::rgba(1.0, 1.0, 1.0, 0.18),
        (true, false) => Color::rgba(1.0, 1.0, 1.0, 0.08),
    };
    ui.draw.rect(r, fill);
    ui.draw.stroke_rect(r, 2.0 * s, 0.0, if lit { style::BONE } else { style::DIM });
    ui.text_at(label, style_, Vec2::new(r.min.x + 18.0 * s, r.min.y + 10.0 * s), r.width(), if can { style::BONE } else { style::DIM });
    active && can && hit.clicked
}
