//! The inventory screen, over the game while it goes on: the weapons in
//! their slots (`slots.rs`), the backpack and the pockets, and beside them
//! whatever is being searched. Things are dragged from place to place (R
//! turns what's held), right-clicked straight across (between the bag and
//! what's searched, or pack and pockets; a weapon into its empty slot, or
//! out of it), or dragged out onto the ground. Trading, there's a box of
//! things to be sold too, between the bag and the stash.

mod draw;
mod slots;

use std::collections::HashMap;

use lntrn_app::lntrn_render::ImageHandle;
use lntrn_math::{Rect, Vec2};
use lntrn_ui::{Key, Ui};

use crate::loot::bag::{Bag, Slot};
use crate::loot::grid::{Grid, Item};
use crate::loot::{Kind, Stack};

/// Every kind of thing's picture, as it lies and turned.
#[derive(Default)]
pub struct Icons(pub HashMap<Kind, (ImageHandle, ImageHandle)>);

/// A cell's side, logical pixels, and the gaps about the grids.
const CELL: f64 = 80.0;
const GAP: f64 = 60.0;
const TITLE: f64 = 28.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Which {
    Pack,
    Pockets,
    Loot,
    /// Trading: what's to be sold.
    Sell,
    Slot(Slot),
}

/// Where things are kept while the screen is up: the bag, what's being
/// searched (its name and grid), if anything, and trading, what's to be
/// sold.
pub struct Shelves<'a> {
    pub bag: &'a mut Bag,
    pub loot: Option<(&'static str, &'a mut Grid)>,
    pub sell: Option<&'a mut Grid>,
}

impl Shelves<'_> {
    fn grid(&mut self, which: Which) -> Option<&mut Grid> {
        match which {
            Which::Pack => Some(&mut self.bag.pack),
            Which::Pockets => Some(&mut self.bag.pockets),
            Which::Loot => self.loot.as_mut().map(|(_, g)| &mut **g),
            Which::Sell => self.sell.as_deref_mut(),
            Which::Slot(_) => None,
        }
    }

    /// Take the thing at `index` of `which` out (a slot's only thing is 0).
    fn take(&mut self, which: Which, index: usize) -> Option<Item> {
        match which {
            Which::Slot(slot) => self.bag.slot_mut(slot).take().map(|stack| Item { stack, x: 0, y: 0, turned: false }),
            _ => self.grid(which).map(|g| g.take(index)),
        }
    }
}

/// What's held under the pointer: the thing (as it would lie), where it
/// came from (to go back to), and where on it the pointer holds it.
#[derive(Clone, Copy, Debug)]
struct Held {
    item: Item,
    from: Which,
    was: Item,
    grab: Vec2,
}

/// Where the screen is shown.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    /// In a run: the bag, and what's searched.
    #[default]
    Run,
    /// In the hideout: the stash (the "loot" grid) where what's searched
    /// is in a run, and nothing thrown on any ground.
    Hideout,
    /// Trading: right of the trader's offers, the bag, what's to be sold,
    /// and the stash.
    Trade,
}

pub struct BagUi {
    held: Option<Held>,
    mode: Mode,
    /// The weapon slots' keys, as written over them.
    pub slot_keys: [String; 3],
}

impl Default for BagUi {
    fn default() -> Self {
        Self::new(Mode::Run)
    }
}

/// What came of a frame: what was thrown on the ground, and what was taken
/// out of what's searched into the bag.
#[derive(Default)]
pub struct Moved {
    pub dropped: Vec<Stack>,
    pub taken: Vec<Stack>,
}

/// Where something held would go, let go of over a grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Landing {
    /// Laid down fresh with its top-left cell at `(x, y)`.
    Put(i32, i32),
    /// Onto the stack at `index` of its own kind, `room` more fitting.
    Merge(usize, u32),
    /// Rounds into the gun at `index`, `room` more fitting in it.
    Load(usize, u32),
    /// Nowhere: in the way of something, or a stack already full.
    Blocked,
}

/// Where `held` goes in `grid` with its top-left at `(x, y)` and the
/// pointer over cell `(cx, cy)`: into a gun under the pointer that takes
/// it, with room; laid down if it fits; onto a stack of its kind under the
/// pointer with room left; or not at all.
fn landing(grid: &Grid, held: Item, (x, y): (i32, i32), (cx, cy): (i32, i32)) -> Landing {
    let under = (cx >= 0 && cy >= 0).then(|| grid.at(cx as u8, cy as u8)).flatten();
    if let Some(i) = under {
        let room = grid.items[i].stack.room_for(held.stack);
        if room > 0 {
            return Landing::Load(i, room);
        }
    }
    if grid.fits(held.stack.kind, x, y, held.turned, None) {
        return Landing::Put(x, y);
    }
    match grid.at(cx as u8, cy as u8) {
        Some(i) if grid.items[i].stack.kind == held.stack.kind => {
            let room = held.stack.kind.def().stack.saturating_sub(grid.items[i].stack.count);
            if room > 0 { Landing::Merge(i, room) } else { Landing::Blocked }
        }
        _ => Landing::Blocked,
    }
}

/// Let go of `h` over `which` (top-left at `at`, pointer over `over`): as
/// much as lands there does; the rest goes back where it came from. How
/// many landed.
fn release(shelves: &mut Shelves, h: Held, which: Which, at: (i32, i32), over: (i32, i32)) -> u32 {
    let grid = shelves.grid(which).expect("a grid on screen");
    let landed = match landing(grid, h.item, at, over) {
        Landing::Put(x, y) => {
            grid.put(h.item.stack, x, y, h.item.turned);
            h.item.stack.count
        }
        Landing::Merge(i, room) => {
            let n = room.min(h.item.stack.count);
            grid.items[i].stack.count += n;
            n
        }
        Landing::Load(i, room) => {
            let n = room.min(h.item.stack.count);
            grid.items[i].stack.loaded += n;
            n
        }
        Landing::Blocked => 0,
    };
    let left = h.item.stack.count - landed;
    if left > 0 {
        put_back(shelves, Held { item: Item { stack: h.item.stack.with_count(left), ..h.item }, ..h });
    }
    landed
}

/// A cell's side in the hideout, at most (a stash must fit a short
/// screen: a bigger one is drawn smaller).
const HIDEOUT_CELL: f64 = 70.0;
/// Room kept under the grids for the way on, logical pixels.
const BELOW: f64 = 180.0;
/// Trading: how wide the trader's offers are, at the left, and the box of
/// what's to be sold, cells across and down.
pub const OFFERS_W: f64 = 660.0;
pub const SELL_W: u8 = 6;
pub const SELL_H: u8 = 6;

impl BagUi {
    /// The screen as the hideout, or trading, has it.
    pub fn new(mode: Mode) -> Self {
        Self { held: None, mode, slot_keys: ["1", "2", "3"].map(String::from) }
    }

    /// A cell's side on screen, what's searched (the stash) `loot` big:
    /// in the hideout, small enough for the stash to fit above the way on
    /// (and trading, for everything to fit beside the offers).
    fn cell(&self, ui: &Ui, bag: &Bag, loot: Option<(u8, u8)>) -> f64 {
        let s = ui.m.scale;
        if self.mode == Mode::Run {
            return CELL * s;
        }
        let screen = ui.clip();
        let top = screen.height() * 0.17 + TITLE * s * 1.6;
        let rows = f64::from(loot.map_or(1, |(_, h)| h.max(1)));
        let cell = (HIDEOUT_CELL * s).min((screen.height() - top - BELOW * s) / rows);
        if self.mode != Mode::Trade {
            return cell;
        }
        let (x0, x1) = self.trade_span(ui);
        let cols = f64::from(bag.pack.w.max(bag.pockets.w) + SELL_W + loot.map_or(0, |(w, _)| w));
        cell.min((x1 - x0 - 2.0 * GAP * s) / cols)
    }

    /// Trading: what the offers leave, left to right.
    fn trade_span(&self, ui: &Ui) -> (f64, f64) {
        let (s, screen) = (ui.m.scale, ui.clip());
        (screen.min.x + (80.0 + OFFERS_W) * s + GAP * s, screen.max.x - 80.0 * s)
    }

    /// The grids' and slots' places on screen: which, and its rect. The
    /// slots on the left, then the bag (the pack over the pockets), what's
    /// searched (or the stash) on the right. Trading, right of the offers:
    /// the bag, what's to be sold, the stash (and no slots).
    fn layout(&self, ui: &Ui, bag: &Bag, loot: Option<(u8, u8)>, cell: f64) -> Vec<(Which, Rect)> {
        let s = ui.m.scale;
        let screen = ui.clip();
        let (gap, title) = (GAP * s, TITLE * s * 1.6);
        let size = |(w, h): (u8, u8)| Vec2::new(f64::from(w) * cell, f64::from(h) * cell);
        let (pack_dims, pocket_dims) = ((bag.pack.w, bag.pack.h), (bag.pockets.w, bag.pockets.h));
        let bag_w = f64::from(pack_dims.0.max(pocket_dims.0)) * cell;
        let loot_w = loot.map_or(0.0, |(w, _)| gap + f64::from(w) * cell);
        let top = screen.min.y + screen.height() * 0.17 + title;
        let trade = self.mode == Mode::Trade;
        let (before, after) = if trade { (0.0, gap + f64::from(SELL_W) * cell) } else { (slots::WIDTH * cell + gap, 0.0) };
        let centre = if trade { (self.trade_span(ui).0 + self.trade_span(ui).1) * 0.5 } else { screen.center().x };
        let left = centre - (before + bag_w + after + loot_w) * 0.5;
        let bag_x = left + before;
        let pack = Rect::from_min_size(Vec2::new(bag_x, top), size(pack_dims));
        let pockets = Rect::from_min_size(Vec2::new(bag_x, pack.max.y + title + gap * 0.5), size(pocket_dims));
        let mut out = vec![(Which::Pack, pack), (Which::Pockets, pockets)];
        if trade {
            out.push((Which::Sell, Rect::from_min_size(Vec2::new(bag_x + bag_w + gap, top), size((SELL_W, SELL_H)))));
        }
        if let Some(dims) = loot {
            out.push((Which::Loot, Rect::from_min_size(Vec2::new(bag_x + bag_w + after + gap, top), size(dims))));
        }
        if !trade {
            out.extend(slots::layout(Vec2::new(left, top), cell, s));
        }
        out
    }
}

impl BagUi {
    /// Where `which` is on screen, as the next frame will lay it out.
    pub fn rect_of(&self, ui: &Ui, shelves: &Shelves, which: Which) -> Option<Rect> {
        let loot = shelves.loot.as_ref().map(|(_, g)| (g.w, g.h));
        self.layout(ui, shelves.bag, loot, self.cell(ui, shelves.bag, loot)).into_iter().find(|(w, _)| *w == which).map(|(_, r)| r)
    }

    /// Whatever is held goes back where it came from (the screen closing).
    pub fn let_go(&mut self, shelves: &mut Shelves) {
        if let Some(h) = self.held.take() {
            put_back(shelves, h);
        }
    }

    /// A frame of the screen: what was thrown on the ground and what was
    /// taken.
    pub fn frame(&mut self, ui: &mut Ui, shelves: &mut Shelves, icons: &Icons) -> Moved {
        let loot = shelves.loot.as_ref().map(|(_, g)| (g.w, g.h));
        let cell = self.cell(ui, shelves.bag, loot);
        let mut moved = Moved::default();
        let places = self.layout(ui, shelves.bag, loot, cell);
        let p = ui.state.pointer;
        let under_item = places.iter().find(|(_, r)| r.contains(p)).and_then(|&(which, r)| match which {
            Which::Slot(slot) => shelves.bag.slot(slot).map(|_| (which, 0)),
            _ => {
                let (cx, cy) = cell_under(r, p, cell);
                shelves.grid(which).and_then(|g| g.at(cx as u8, cy as u8)).map(|i| (which, i))
            }
        });

        match self.held {
            None => {
                if ui.state.pressed
                    && let Some((which, i)) = under_item
                    && let Some(item) = shelves.take(which, i)
                {
                    let r = places.iter().find(|(w, _)| *w == which).map(|(_, r)| *r).unwrap_or(Rect::ZERO);
                    let at = match which {
                        Which::Slot(_) => slots::tile(r, item.stack, cell).0,
                        _ => r.min + Vec2::new(f64::from(item.x), f64::from(item.y)) * cell,
                    };
                    self.held = Some(Held { item, from: which, was: item, grab: p - at });
                } else if ui.state.right_pressed
                    && self.mode == Mode::Trade
                    && let Some((which, i)) = under_item
                {
                    // Trading, straight across into what's to be sold, or out
                    // of it back to the stash.
                    sell_move(shelves, which, i);
                } else if ui.state.right_pressed
                    && let Some((which, i)) = under_item
                {
                    let stack = shelves.grid(which).map(|g| g.items[i].stack);
                    let n = quick_move(shelves, which, i);
                    if which == Which::Loot
                        && n > 0
                        && let Some(stack) = stack
                    {
                        moved.taken.push(stack.with_count(n));
                    }
                }
            }
            Some(mut h) => {
                if ui.state.take_key(|k| !k.repeat && matches!(k.key, Key::Char(c) if c.eq_ignore_ascii_case(&'r'))).is_some() {
                    h.item.turned = !h.item.turned;
                    let (w, hh) = h.item.shape();
                    h.grab = Vec2::new(f64::from(w), f64::from(hh)) * cell * 0.5;
                    self.held = Some(h);
                }
                if !ui.state.down {
                    let h = self.held.take().expect("held");
                    match places.iter().find(|(_, r)| r.contains(p)) {
                        Some(&(which, r)) => {
                            let landed = match which {
                                Which::Slot(slot) => slots::release(shelves, h, slot),
                                _ => release(shelves, h, which, corner_cell(r, p - h.grab, cell), cell_under(r, p, cell)),
                            };
                            if h.from == Which::Loot && which != Which::Loot && landed > 0 {
                                moved.taken.push(h.item.stack.with_count(landed));
                            }
                        }
                        // Out onto the ground (there's none in the hideout).
                        None if self.mode != Mode::Run => put_back(shelves, h),
                        None => moved.dropped.push(h.item.stack),
                    }
                }
            }
        }
        self.draw(ui, shelves, icons, &places, cell);
        moved
    }
}

/// The cell of the grid at `r` that `p` is over.
fn cell_under(r: Rect, p: Vec2, cell: f64) -> (i32, i32) {
    (((p.x - r.min.x) / cell).floor() as i32, ((p.y - r.min.y) / cell).floor() as i32)
}

/// The cell a thing's top-left corner at `corner` is nearest.
fn corner_cell(r: Rect, corner: Vec2, cell: f64) -> (i32, i32) {
    (((corner.x - r.min.x) / cell).round() as i32, ((corner.y - r.min.y) / cell).round() as i32)
}

/// The screen rect of a `shape` with its top-left at cell `(x, y)`.
fn footprint(r: Rect, x: i32, y: i32, (w, h): (u8, u8), cell: f64) -> Rect {
    Rect::from_min_size(r.min + Vec2::new(f64::from(x), f64::from(y)) * cell, Vec2::new(f64::from(w), f64::from(h)) * cell)
}

/// Send the thing at `index` in `from` straight across: out of what's
/// searched into the bag, into what's searched from the bag, or (nothing
/// searched) between pack and pockets. What won't fit stays. How many went.
fn quick_move(shelves: &mut Shelves, from: Which, index: usize) -> u32 {
    let Some(item) = shelves.take(from, index) else { return 0 };
    let searching = shelves.loot.is_some();
    let empty_slot = Slot::of(item.stack.kind).filter(|&s| shelves.bag.slot(s).is_none());
    let left = match (from, empty_slot) {
        (Which::Loot, _) => shelves.bag.add(item.stack),
        // Nothing searched: a weapon in the bag to its empty slot, and one
        // out of its slot into the bag.
        (Which::Pack | Which::Pockets, Some(slot)) if !searching => {
            *shelves.bag.slot_mut(slot) = Some(item.stack);
            item.stack.with_count(0)
        }
        (Which::Slot(_), _) if !searching => {
            let rest = shelves.bag.pack.place(item.stack);
            shelves.bag.pockets.place(rest)
        }
        _ => {
            let to = if shelves.loot.is_some() { Which::Loot } else if from == Which::Pack { Which::Pockets } else { Which::Pack };
            let g = shelves.grid(to).expect("somewhere to go");
            let rest = g.top_up(item.stack);
            g.place(rest)
        }
    };
    if left.count > 0 {
        put_back(shelves, Held { item: Item { stack: left, ..item }, from, was: item, grab: Vec2::ZERO });
    }
    item.stack.count - left.count
}

/// Trading: send the thing at `index` in `from` into what's to be sold, or
/// (from there) back to the stash. What won't fit stays.
fn sell_move(shelves: &mut Shelves, from: Which, index: usize) {
    let Some(item) = shelves.take(from, index) else { return };
    let to = if from == Which::Sell { Which::Loot } else { Which::Sell };
    let rest = match shelves.grid(to) {
        Some(g) => {
            let rest = g.top_up(item.stack);
            g.place(rest)
        }
        None => item.stack,
    };
    if rest.count > 0 {
        put_back(shelves, Held { item: Item { stack: rest, ..item }, from, was: item, grab: Vec2::ZERO });
    }
}

/// `h` back where it was; failing that, anywhere it goes in the bag.
fn put_back(shelves: &mut Shelves, h: Held) {
    let stack = h.item.stack;
    let back = match h.from {
        Which::Slot(slot) => {
            let place = shelves.bag.slot_mut(slot);
            let empty = place.is_none();
            if empty {
                *place = Some(stack);
            }
            empty
        }
        _ => shelves.grid(h.from).is_some_and(|g| g.put(stack, i32::from(h.was.x), i32::from(h.was.y), h.was.turned)),
    };
    if !back {
        shelves.bag.add(stack);
    }
}

#[cfg(test)]
mod tests;
