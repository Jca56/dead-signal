//! What's worn, on the bag screen: a column of five boxes (head, chest,
//! back, legs, belt), each holding what's on. Gear dropped on its own box is
//! put on (everything carried laid out again for the grids it brings or
//! takes away; what was worn there goes where the new came from); taken
//! off by dragging it out. Either is refused, with a word, when what's
//! carried wouldn't fit.

use lntrn_math::{Color, Rect, Vec2};
use lntrn_text::TextStyle;
use lntrn_ui::Ui;

use super::{Held, Icons, Shelves, Which, put_back, slots};
use crate::loot::Stack;
use crate::loot::gear::Wear;
use crate::loot::grid::Item;
use crate::style;

/// A box, cells across and down, and the space between boxes, logical
/// pixels.
pub const WIDTH: f64 = 2.6;
const HEIGHT: f64 = 1.5;
const SPACE: f64 = 12.0;

/// Every box, down from `at`.
pub fn layout(at: Vec2, cell: f64, s: f64) -> Vec<(Which, Rect)> {
    Wear::ALL
        .iter()
        .enumerate()
        .map(|(i, &wear)| {
            let top = at + Vec2::new(0.0, i as f64 * (HEIGHT * cell + SPACE * s));
            (Which::Worn(wear), Rect::from_min_size(top, Vec2::new(WIDTH, HEIGHT) * cell))
        })
        .collect()
}

/// The box of `wear` at `r`, and what's on.
pub fn draw(ui: &mut Ui, icons: &Icons, wear: Wear, stack: Option<Stack>, r: Rect, cell: f64) {
    let s = ui.m.scale;
    ui.draw.rect(r.expand(6.0 * s), Color::rgba(0.05, 0.05, 0.05, 0.85));
    ui.draw.rect(r.shrink(2.0 * s), Color::rgba(1.0, 1.0, 1.0, 0.05));
    let label = TextStyle::new((20.0 * s) as f32).bold().family(style::FONT);
    ui.text_at(wear.name(), &label, r.min + Vec2::new(10.0 * s, 6.0 * s), r.width(), style::DIM);
    if let Some(stack) = stack {
        let (at, size) = slots::tile(r, stack, cell);
        super::draw::tile(ui, icons, Item { stack, x: 0, y: 0, turned: false }, at, size, 1.0);
    }
}

/// Whether `stack` is worn on `wear`.
pub fn takes(stack: Stack, wear: Wear) -> bool {
    stack.kind.gear().is_some_and(|g| g.wear == wear)
}

/// Let go of `h` over `wear`: put on, what was worn there going where `h`
/// came from (else anywhere carried); or, if it isn't worn there, or
/// what's carried wouldn't fit, back where it was, with a word why. How
/// many landed.
pub fn release(shelves: &mut Shelves, h: Held, wear: Wear) -> (u32, Option<&'static str>) {
    if !takes(h.item.stack, wear) {
        put_back(shelves, h);
        return (0, None);
    }
    let before = shelves.bag.clone();
    match shelves.bag.wear(h.item.stack, shelves.fit) {
        Err(why) => {
            put_back(shelves, h);
            (0, Some(why))
        }
        Ok(None) => (1, None),
        Ok(Some(old)) => {
            let was = h.was;
            let home = match h.from {
                Which::Slot(_) | Which::Worn(_) => false,
                from => shelves.grid(from).is_some_and(|g| g.put(old, i32::from(was.x), i32::from(was.y), was.turned) || g.place(old).count == 0),
            };
            if home || shelves.bag.add(old).count == 0 {
                (1, None)
            } else {
                // Nowhere for what was worn: as it was.
                *shelves.bag = before;
                put_back(shelves, h);
                (0, Some(crate::loot::bag::NO_ROOM))
            }
        }
    }
}

/// Right-clicked: gear in a grid put on (what was worn there taking its
/// place); worn, taken off into what's searched (or the stash), else the
/// bag. How many moved, and a word if it couldn't.
pub fn quick(shelves: &mut Shelves, from: Which, index: usize) -> (u32, Option<&'static str>) {
    if let Which::Worn(_) = from {
        let Some(item) = shelves.take(from, index) else { return (0, Some(crate::loot::bag::NO_ROOM)) };
        let rest = match shelves.loot.as_mut() {
            Some((_, g)) => g.place(item.stack),
            None => shelves.bag.add(item.stack),
        };
        if rest.count == 0 {
            return (1, None);
        }
        let _ = shelves.bag.wear(item.stack, shelves.fit);
        return (0, Some("NO ROOM TO TAKE IT OFF"));
    }
    let Some(item) = shelves.take(from, index) else { return (0, None) };
    let Some(wear) = item.stack.kind.gear().map(|g| g.wear) else {
        put_back(shelves, Held { item, from, was: item, grab: Vec2::ZERO });
        return (0, None);
    };
    release(shelves, Held { item, from, was: item, grab: Vec2::ZERO }, wear)
}
