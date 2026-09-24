//! Where everything goes on the bag screen, and how big a cell is. Left to
//! right: the weapon slots, what's worn, the bag (the backpack over the
//! pockets), the rig over the belt, and what's searched (or the stash).
//! Trading, right of the trader's offers: the bag, what's to be sold, the
//! stash (no slots, nothing worn). Cells are as big as they can be for it
//! all to fit (and in the hideout, for the stash to fit above the way on).

use lntrn_math::{Rect, Vec2};
use lntrn_ui::Ui;

use super::{BagUi, CELL, GAP, HIDEOUT_CELL, BELOW, Mode, OFFERS_W, SELL_H, SELL_W, TITLE, Which, slots, worn};
use crate::loot::bag::Bag;

impl BagUi {
    /// How many cells across it all is, besides the gaps: `(cells, gaps)`.
    fn across(&self, bag: &Bag, loot: Option<(u8, u8)>) -> (f64, f64) {
        let bag_w = f64::from(bag.pack.w.max(bag.pockets.w));
        let loot_w = loot.map_or(0.0, |(w, _)| f64::from(w));
        let gaps = 1.0 + f64::from(u8::from(loot.is_some()));
        if self.mode == Mode::Trade {
            return (bag_w + f64::from(SELL_W) + loot_w, gaps + 1.0);
        }
        let rig_w = f64::from(bag.rig.w.max(bag.belt.w));
        (slots::WIDTH + worn::WIDTH + bag_w + rig_w + loot_w, gaps + 1.0 + f64::from(u8::from(rig_w > 0.0)))
    }

    /// A cell's side on screen, what's searched (the stash) `loot` big: as
    /// big as fits across (and in the hideout, down: the stash above the
    /// way on).
    pub(super) fn cell(&self, ui: &Ui, bag: &Bag, loot: Option<(u8, u8)>) -> f64 {
        let s = ui.m.scale;
        let screen = ui.clip();
        let (from, to) = if self.mode == Mode::Trade { self.trade_span(ui) } else { (screen.min.x + 60.0 * s, screen.max.x - 60.0 * s) };
        let (cells, gaps) = self.across(bag, loot);
        let fits_across = (to - from - gaps * GAP * s) / cells.max(1.0);
        if self.mode == Mode::Run {
            return (CELL * s).min(fits_across);
        }
        let top = screen.height() * 0.17 + TITLE * s * 1.6;
        let rows = f64::from(loot.map_or(1, |(_, h)| h.max(1)));
        (HIDEOUT_CELL * s).min((screen.height() - top - BELOW * s) / rows).min(fits_across)
    }

    /// Trading: what the offers leave, left to right.
    pub(super) fn trade_span(&self, ui: &Ui) -> (f64, f64) {
        let (s, screen) = (ui.m.scale, ui.clip());
        (screen.min.x + (80.0 + OFFERS_W) * s + GAP * s, screen.max.x - 80.0 * s)
    }

    /// The grids' and slots' places on screen: which, and its rect.
    pub(super) fn layout(&self, ui: &Ui, bag: &Bag, loot: Option<(u8, u8)>, cell: f64) -> Vec<(Which, Rect)> {
        let s = ui.m.scale;
        let screen = ui.clip();
        let (gap, title) = (GAP * s, TITLE * s * 1.6);
        let size = |(w, h): (u8, u8)| Vec2::new(f64::from(w) * cell, f64::from(h) * cell);
        let (cells, gaps) = self.across(bag, loot);
        let whole = cells * cell + gaps * gap;
        let trade = self.mode == Mode::Trade;
        let centre = if trade { (self.trade_span(ui).0 + self.trade_span(ui).1) * 0.5 } else { screen.center().x };
        let top = screen.min.y + screen.height() * 0.17 + title;
        let mut x = centre - whole * 0.5;
        let mut out = Vec::new();
        if !trade {
            out.extend(slots::layout(Vec2::new(x, top), cell, s));
            x += slots::WIDTH * cell + gap;
            out.extend(worn::layout(Vec2::new(x, top), cell, s));
            x += worn::WIDTH * cell + gap;
        }
        // The bag: the backpack over the pockets.
        let bag_w = f64::from(bag.pack.w.max(bag.pockets.w)) * cell;
        let pack = Rect::from_min_size(Vec2::new(x, top), size((bag.pack.w, bag.pack.h)));
        let below = if bag.pack.h > 0 { pack.max.y + title + gap * 0.5 } else { top + title };
        out.push((Which::Pack, pack));
        out.push((Which::Pockets, Rect::from_min_size(Vec2::new(x, below), size((bag.pockets.w, bag.pockets.h)))));
        x += bag_w + gap;
        if trade {
            out.push((Which::Sell, Rect::from_min_size(Vec2::new(x, top), size((SELL_W, SELL_H)))));
            x += f64::from(SELL_W) * cell + gap;
        } else if bag.rig.w.max(bag.belt.w) > 0 {
            // The rig over the belt.
            let rig = Rect::from_min_size(Vec2::new(x, top), size((bag.rig.w, bag.rig.h)));
            let below = if bag.rig.h > 0 { rig.max.y + title + gap * 0.5 } else { top };
            out.push((Which::Rig, rig));
            out.push((Which::Belt, Rect::from_min_size(Vec2::new(x, below), size((bag.belt.w, bag.belt.h)))));
            x += f64::from(bag.rig.w.max(bag.belt.w)) * cell + gap;
        }
        if let Some(dims) = loot {
            out.push((Which::Loot, Rect::from_min_size(Vec2::new(x, top), size(dims))));
        }
        out
    }
}
