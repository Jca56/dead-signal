//! Where everything goes on the bag screen, and how big a cell is. Left to
//! right: the weapon slots, what's worn, the bag (the backpack over the
//! pockets; or, where that makes for bigger cells, beside them), the rig
//! over the belt, and what's searched (or the stash). Trading, right of
//! the trader's offers: the bag, what's to be sold, the stash (no slots,
//! nothing worn). Cells are as big as they can be for it all to fit,
//! across and down (and in the hideout, for the stash to fit above the
//! way on).

use lntrn_math::{Rect, Vec2};
use lntrn_ui::Ui;

use super::{BELOW, BagUi, CELL, GAP, HIDEOUT_CELL, Mode, OFFERS_W, PANE_GAP, SELL_H, SELL_W, TITLE, Which, slots, worn};
use crate::loot::bag::Bag;

/// How far down the screen the grids' tops are, as a share of it: over
/// the whole window, and over a player's pane.
const LEAD: f64 = 0.17;
const PANE_LEAD: f64 = 0.04;
/// The room kept at the sides, and under the grids in a run (a pad's
/// hints are written there), logical pixels.
const SIDE: f64 = 60.0;
const PANE_SIDE: f64 = 24.0;
const FOOT: f64 = 70.0;

/// How the screen's laid out: a cell's side, and whether the pockets are
/// beside the pack (not under it).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Laid {
    pub cell: f64,
    pub beside: bool,
}

impl BagUi {
    /// What it's shown over: the player's pane, else the whole window.
    pub(super) fn screen(&self, ui: &Ui) -> Rect {
        self.pane.unwrap_or(ui.clip())
    }

    /// The gap between the grids.
    fn gap(&self, s: f64) -> f64 {
        if self.pane.is_some() { PANE_GAP * s } else { GAP * s }
    }

    /// Where the grids' tops are.
    fn top(&self, ui: &Ui) -> f64 {
        let screen = self.screen(ui);
        screen.min.y + screen.height() * if self.pane.is_some() { PANE_LEAD } else { LEAD } + TITLE * ui.m.scale * 1.6
    }

    /// How many cells across it all is, besides the gaps: `(cells, gaps)`.
    fn across(&self, bag: &Bag, loot: Option<(u8, u8)>, beside: bool) -> (f64, f64) {
        let bag_w = if beside { f64::from(bag.pack.w + bag.pockets.w) } else { f64::from(bag.pack.w.max(bag.pockets.w)) };
        let loot_w = loot.map_or(0.0, |(w, _)| f64::from(w));
        let gaps = 1.0 + f64::from(u8::from(loot.is_some())) + f64::from(u8::from(beside));
        if self.mode == Mode::Trade {
            return (bag_w + f64::from(SELL_W) + loot_w, gaps + 1.0);
        }
        let rig_w = f64::from(bag.rig.w.max(bag.belt.w));
        (slots::WIDTH + worn::WIDTH + bag_w + rig_w + loot_w, gaps + 1.0 + f64::from(u8::from(rig_w > 0.0)))
    }

    /// In a run, each column's height: its cells, and what else is in it
    /// (the space between boxes, a title), pixels.
    fn columns(&self, bag: &Bag, loot: Option<(u8, u8)>, beside: bool, s: f64) -> [(f64, f64); 5] {
        let (gap, title) = (self.gap(s), TITLE * s * 1.6);
        let (pack, pockets, rig, belt) = (f64::from(bag.pack.h), f64::from(bag.pockets.h), f64::from(bag.rig.h), f64::from(bag.belt.h));
        let between = title + gap * 0.5;
        [
            slots::height(s),
            worn::height(s),
            if beside {
                (pack.max(pockets), 0.0)
            } else if pack > 0.0 {
                (pack + pockets, between)
            } else {
                (pockets, title)
            },
            if rig > 0.0 { (rig + belt, between) } else { (belt, 0.0) },
            (loot.map_or(0.0, |(_, h)| f64::from(h)), 0.0),
        ]
    }

    /// A cell's side with the pockets `beside` the pack or under it, what's
    /// searched (the stash) `loot` big: as big as fits across (and in a
    /// run, down; in the hideout, the stash above the way on).
    fn cell(&self, ui: &Ui, bag: &Bag, loot: Option<(u8, u8)>, beside: bool) -> f64 {
        let s = ui.m.scale;
        let screen = self.screen(ui);
        let side = if self.pane.is_some() { PANE_SIDE * s } else { SIDE * s };
        let (from, to) = if self.mode == Mode::Trade { self.trade_span(ui) } else { (screen.min.x + side, screen.max.x - side) };
        let (cells, gaps) = self.across(bag, loot, beside);
        let fits_across = (to - from - gaps * self.gap(s)) / cells.max(1.0);
        if self.mode == Mode::Run {
            let room = screen.max.y - self.top(ui) - FOOT * s;
            let fits_down = self.columns(bag, loot, beside, s).into_iter().filter(|(cells, _)| *cells > 0.0).map(|(cells, more)| (room - more) / cells).fold(f64::MAX, f64::min);
            return (CELL * s).min(fits_across).min(fits_down);
        }
        let top = screen.height() * LEAD + TITLE * s * 1.6;
        let rows = f64::from(loot.map_or(1, |(_, h)| h.max(1)));
        (HIDEOUT_CELL * s).min((screen.height() - top - BELOW * s) / rows).min(fits_across)
    }

    /// How it's laid out: the pockets under the pack, unless (in a run)
    /// beside it makes for bigger cells.
    pub(super) fn laid(&self, ui: &Ui, bag: &Bag, loot: Option<(u8, u8)>) -> Laid {
        let under = self.cell(ui, bag, loot, false);
        if self.mode != Mode::Run || bag.pack.h == 0 || bag.pockets.h == 0 {
            return Laid { cell: under, beside: false };
        }
        let beside = self.cell(ui, bag, loot, true);
        if beside > under + 0.5 { Laid { cell: beside, beside: true } } else { Laid { cell: under, beside: false } }
    }

    /// Trading: what the offers leave, left to right.
    pub(super) fn trade_span(&self, ui: &Ui) -> (f64, f64) {
        let (s, screen) = (ui.m.scale, self.screen(ui));
        (screen.min.x + (80.0 + OFFERS_W) * s + GAP * s, screen.max.x - 80.0 * s)
    }

    /// The grids' and slots' places on screen: which, and its rect.
    pub(super) fn layout(&self, ui: &Ui, bag: &Bag, loot: Option<(u8, u8)>, laid: Laid) -> Vec<(Which, Rect)> {
        let s = ui.m.scale;
        let screen = self.screen(ui);
        let cell = laid.cell;
        let (gap, title) = (self.gap(s), TITLE * s * 1.6);
        let size = |(w, h): (u8, u8)| Vec2::new(f64::from(w) * cell, f64::from(h) * cell);
        let (cells, gaps) = self.across(bag, loot, laid.beside);
        let whole = cells * cell + gaps * gap;
        let trade = self.mode == Mode::Trade;
        let centre = if trade { (self.trade_span(ui).0 + self.trade_span(ui).1) * 0.5 } else { screen.center().x };
        let top = self.top(ui);
        let mut x = centre - whole * 0.5;
        let mut out = Vec::new();
        if !trade {
            out.extend(slots::layout(Vec2::new(x, top), cell, s));
            x += slots::WIDTH * cell + gap;
            out.extend(worn::layout(Vec2::new(x, top), cell, s));
            x += worn::WIDTH * cell + gap;
        }
        // The bag: the backpack over the pockets, or beside them.
        let pack = Rect::from_min_size(Vec2::new(x, top), size((bag.pack.w, bag.pack.h)));
        out.push((Which::Pack, pack));
        if laid.beside {
            x += pack.width() + gap;
            out.push((Which::Pockets, Rect::from_min_size(Vec2::new(x, top), size((bag.pockets.w, bag.pockets.h)))));
            x += f64::from(bag.pockets.w) * cell + gap;
        } else {
            let below = if bag.pack.h > 0 { pack.max.y + title + gap * 0.5 } else { top + title };
            out.push((Which::Pockets, Rect::from_min_size(Vec2::new(x, below), size((bag.pockets.w, bag.pockets.h)))));
            x += f64::from(bag.pack.w.max(bag.pockets.w)) * cell + gap;
        }
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
