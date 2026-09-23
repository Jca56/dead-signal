//! The weapon slots on the bag screen: a column of three boxes, each with
//! its key and name, holding what's carried to hand there. A weapon is
//! dropped on its own slot (swapping with what's in it, which goes where
//! the new one came from); anything else, or on the wrong slot, goes back.

use lntrn_math::{Color, Rect, Vec2};
use lntrn_text::TextStyle;
use lntrn_ui::Ui;

use super::{Held, Icons, Shelves, Which, put_back};
use crate::loot::bag::Slot;
use crate::loot::grid::Item;
use crate::loot::Stack;
use crate::style;

/// A slot's box, cells across and down, and the space between boxes,
/// logical pixels.
pub const WIDTH: f64 = 3.5;
const HEIGHT: f64 = 1.7;
const SPACE: f64 = 18.0;
/// The band across a box's top its key and name are written in, a share
/// of a cell.
const LABEL: f64 = 0.42;

/// Every slot's box, down from `at`.
pub fn layout(at: Vec2, cell: f64, s: f64) -> Vec<(Which, Rect)> {
    Slot::ALL
        .iter()
        .enumerate()
        .map(|(i, &slot)| {
            let top = at + Vec2::new(0.0, i as f64 * (HEIGHT * cell + SPACE * s));
            (Which::Slot(slot), Rect::from_min_size(top, Vec2::new(WIDTH, HEIGHT) * cell))
        })
        .collect()
}

/// Where `stack` is drawn in the box `r`: its top-left corner, and the
/// cell size it's drawn at (as big as fits under the label, no bigger
/// than a grid's, in the middle).
pub fn tile(r: Rect, stack: Stack, cell: f64) -> (Vec2, f64) {
    let (w, h) = stack.kind.def().size;
    let room = Rect::new(Vec2::new(r.min.x, r.min.y + LABEL * cell), r.max).shrink(cell * 0.08);
    let size = (room.width() / f64::from(w)).min(room.height() / f64::from(h)).min(cell);
    let drawn = Vec2::new(f64::from(w), f64::from(h)) * size;
    (room.center() - drawn * 0.5, size)
}

/// The box of `slot` at `r`, and what's in it.
pub fn draw(ui: &mut Ui, icons: &Icons, slot: Slot, stack: Option<Stack>, r: Rect, cell: f64) {
    let s = ui.m.scale;
    ui.draw.rect(r.expand(6.0 * s), Color::rgba(0.05, 0.05, 0.05, 0.85));
    ui.draw.rect(r.shrink(2.0 * s), Color::rgba(1.0, 1.0, 1.0, 0.05));
    let label = TextStyle::new((22.0 * s) as f32).bold().family(style::FONT);
    let key = slot.key().to_string();
    let at = r.min + Vec2::new(10.0 * s, 6.0 * s);
    let kw = ui.measure(&key, &label);
    ui.text_at(&key, &label, at, kw + 4.0, style::SIGNAL);
    ui.text_at(slot.name(), &label, at + Vec2::new(kw + 12.0 * s, 0.0), r.width(), style::DIM);
    if let Some(stack) = stack {
        let (at, size) = tile(r, stack, cell);
        super::draw::tile(ui, icons, Item { stack, x: 0, y: 0, turned: false }, at, size, 1.0);
    }
}

/// Whether `stack` goes in `slot` (swapping with what's there).
pub fn takes(stack: Stack, slot: Slot) -> bool {
    Slot::of(stack.kind) == Some(slot)
}

/// Let go of `h` over `slot`: in it, what was there going where `h` came
/// from; or, if it doesn't go there (or what was there has nowhere to go),
/// back where it was. How many landed.
pub fn release(shelves: &mut Shelves, h: Held, slot: Slot) -> u32 {
    if !takes(h.item.stack, slot) {
        put_back(shelves, h);
        return 0;
    }
    let Some(old) = shelves.bag.slot_mut(slot).replace(h.item.stack) else { return 1 };
    let was = h.was;
    let went = match h.from {
        Which::Slot(_) => false,
        from => shelves.grid(from).is_some_and(|g| g.put(old, i32::from(was.x), i32::from(was.y), was.turned) || g.place(old).count == 0),
    };
    if !went {
        *shelves.bag.slot_mut(slot) = Some(old);
        put_back(shelves, h);
        return 0;
    }
    1
}
