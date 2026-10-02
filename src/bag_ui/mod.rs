//! The inventory screen, over the game while it goes on: the weapons in
//! their slots (`slots.rs`), the backpack and the pockets, and beside them
//! whatever is being searched. Things are dragged from place to place (R
//! turns what's held), right-clicked straight across (between the bag and
//! what's searched, or pack and pockets; a weapon into its empty slot, or
//! out of it), or dragged out onto the ground. Trading, there's a box of
//! things to be sold too, between the bag and the stash. A pad works it
//! with a cursor in the pointer's place (`cursor.rs`); playing together,
//! it's shown over its player's own pane.

mod cursor;
mod draw;
mod layout;
mod moves;
mod slots;
mod worn;

use std::collections::HashMap;

use lntrn_app::lntrn_render::ImageHandle;
use lntrn_math::{Rect, Vec2};
use lntrn_ui::{Key, Ui};

use crate::loot::bag::{Bag, Fit, Slot};
use crate::loot::gear::Wear;
use crate::loot::grid::{Grid, Item};
use crate::loot::{Kind, Stack};
use cursor::Cursor;
pub use cursor::Hands;
use moves::{Landing, landing, put_back, quick_move, release, sell_move};

/// Every kind of thing's picture, as it lies and turned.
#[derive(Default)]
pub struct Icons(pub HashMap<Kind, (ImageHandle, ImageHandle)>);

/// A cell's side, logical pixels, and the gaps about the grids (and, over
/// a player's pane, the lesser gaps: there's less room).
const CELL: f64 = 80.0;
const GAP: f64 = 60.0;
const PANE_GAP: f64 = 36.0;
const TITLE: f64 = 28.0;
/// Said when what a pad holds won't go where it's put.
const WONT_GO: &str = "WON'T GO THERE";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Which {
    Pack,
    Pockets,
    Loot,
    /// Trading: what's to be sold.
    Sell,
    Slot(Slot),
    /// A chest rig's grid, and a bandolier's (rounds only).
    Rig,
    Belt,
    Worn(Wear),
}

/// Where things are kept while the screen is up: the bag, what's being
/// searched (its name and grid), if anything, and trading, what's to be
/// sold.
pub struct Shelves<'a> {
    pub bag: &'a mut Bag,
    pub loot: Option<(&'static str, &'a mut Grid)>,
    pub sell: Option<&'a mut Grid>,
    /// What the player's perks make of what's worn.
    pub fit: Fit,
}

impl Shelves<'_> {
    fn grid(&mut self, which: Which) -> Option<&mut Grid> {
        match which {
            Which::Pack => Some(&mut self.bag.pack),
            Which::Pockets => Some(&mut self.bag.pockets),
            Which::Loot => self.loot.as_mut().map(|(_, g)| &mut **g),
            Which::Sell => self.sell.as_deref_mut(),
            Which::Rig => Some(&mut self.bag.rig),
            Which::Belt => Some(&mut self.bag.belt),
            Which::Slot(_) | Which::Worn(_) => None,
        }
    }

    /// Take the thing at `index` of `which` out (a slot's only thing is 0).
    fn take(&mut self, which: Which, index: usize) -> Option<Item> {
        match which {
            Which::Slot(slot) => self.bag.slot_mut(slot).take().map(|stack| Item { stack, x: 0, y: 0, turned: false }),
            Which::Worn(wear) => self.bag.take_off(wear, self.fit).ok().map(|stack| Item { stack, x: 0, y: 0, turned: false }),
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

/// What's pointing at the screen this frame: the mouse, or a pad's cursor
/// standing in for it (its A the button, held from one press to the next;
/// its X the right button).
#[derive(Clone, Copy, Debug)]
struct Point {
    at: Vec2,
    pressed: bool,
    down: bool,
    across: bool,
    turn: bool,
}

pub struct BagUi {
    held: Option<Held>,
    mode: Mode,
    /// The weapon slots' keys, as written over them.
    pub slot_keys: [String; 3],
    /// The part of the window it's shown over, when that's not all of it
    /// (a player's pane, playing together).
    pub pane: Option<Rect>,
    /// A pad's cursor, whether the pad's at the screen (not the mouse),
    /// and where the mouse was last.
    cursor: Option<Cursor>,
    by_pad: bool,
    pointer_was: Option<Vec2>,
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
    /// Why something wouldn't go (gear that won't go on or come off).
    pub said: Option<&'static str>,
    /// A pad asked for the screen to be put away.
    pub closed: bool,
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
        Self { held: None, mode, slot_keys: ["1", "2", "3"].map(String::from), pane: None, cursor: None, by_pad: false, pointer_was: None }
    }

    /// Where `which` is on screen, as the next frame will lay it out.
    pub fn rect_of(&self, ui: &Ui, shelves: &Shelves, which: Which) -> Option<Rect> {
        let loot = shelves.loot.as_ref().map(|(_, g)| (g.w, g.h));
        self.layout(ui, shelves.bag, loot, self.laid(ui, shelves.bag, loot)).into_iter().find(|(w, _)| *w == which).map(|(_, r)| r)
    }

    /// Whatever is held goes back where it came from (the screen closing).
    pub fn let_go(&mut self, shelves: &mut Shelves) {
        if let Some(h) = self.held.take() {
            put_back(shelves, h);
        }
    }

    /// A frame of the screen, worked by `hands`: what was thrown on the
    /// ground and what was taken.
    pub fn frame(&mut self, ui: &mut Ui, shelves: &mut Shelves, icons: &Icons, hands: Hands) -> Moved {
        let loot = shelves.loot.as_ref().map(|(_, g)| (g.w, g.h));
        let laid = self.laid(ui, shelves.bag, loot);
        let cell = laid.cell;
        let mut moved = Moved::default();
        let places = self.layout(ui, shelves.bag, loot, laid);
        // A pad's cursor is the pointer: B puts what's held back (or the
        // screen away), X throws what's held down, and A puts it down
        // where it'll go (it's kept hold of where it won't).
        let pad = self.at_it(ui, shelves, hands);
        let point = match pad {
            Some(pad) => {
                self.step(shelves, &places, cell, pad.step);
                let holding = self.held.is_some();
                if pad.back && holding {
                    self.let_go(shelves);
                } else if pad.back {
                    moved.closed = true;
                } else if pad.across
                    && self.mode == Mode::Run
                    && let Some(h) = self.held.take()
                {
                    moved.dropped.push(h.item.stack);
                }
                let at = self.aim(shelves, &places, cell);
                let put = pad.pick && self.held.is_some();
                let lands = put && self.lands(shelves, &places, cell, at);
                if put && !lands {
                    moved.said = Some(WONT_GO);
                }
                Point { at, pressed: pad.pick && !holding, down: !lands, across: pad.across && !holding, turn: pad.turn }
            }
            None => Point { at: ui.state.pointer, pressed: ui.state.pressed, down: ui.state.down, across: ui.state.right_pressed, turn: false },
        };
        let p = point.at;
        let under_item = places.iter().find(|(_, r)| r.contains(p)).and_then(|&(which, r)| match which {
            Which::Slot(slot) => shelves.bag.slot(slot).map(|_| (which, 0)),
            Which::Worn(wear) => shelves.bag.worn(wear).map(|_| (which, 0)),
            _ => {
                let (cx, cy) = cell_under(r, p, cell);
                shelves.grid(which).and_then(|g| g.at(cx as u8, cy as u8)).map(|i| (which, i))
            }
        });

        match self.held {
            None => {
                if point.pressed
                    && let Some((which, i)) = under_item
                    && let Some(item) = shelves.take(which, i)
                {
                    let r = places.iter().find(|(w, _)| *w == which).map(|(_, r)| *r).unwrap_or(Rect::ZERO);
                    let at = match which {
                        Which::Slot(_) => slots::tile(r, item.stack, cell, ui.m.scale).0,
                        _ => r.min + Vec2::new(f64::from(item.x), f64::from(item.y)) * cell,
                    };
                    self.held = Some(Held { item, from: which, was: item, grab: p - at });
                    // (A pad's cursor goes to its top-left cell, where it's
                    // held by.)
                    if pad.is_some() && shelves.grid(which).is_some() {
                        self.cursor = Some(Cursor { which, x: item.x, y: item.y });
                    }
                } else if point.pressed
                    && let Some((Which::Worn(_), _)) = under_item
                {
                    // It won't come off: what's in it has nowhere to go.
                    moved.said = Some(crate::loot::bag::NO_ROOM);
                } else if point.across
                    && self.mode == Mode::Trade
                    && let Some((which, i)) = under_item
                {
                    // Trading, straight across into what's to be sold, or out
                    // of it back to the stash.
                    sell_move(shelves, which, i);
                } else if point.across
                    && let Some((which, i)) = under_item
                    && (matches!(which, Which::Worn(_)) || shelves.grid(which).is_some_and(|g| g.items[i].stack.kind.gear().is_some()))
                {
                    // Gear: put on, or taken off.
                    let stack = shelves.grid(which).map(|g| g.items[i].stack);
                    let (n, said) = worn::quick(shelves, which, i);
                    moved.said = moved.said.or(said);
                    if which == Which::Loot
                        && n > 0
                        && let Some(stack) = stack
                    {
                        moved.taken.push(stack);
                    }
                } else if point.across
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
                if point.turn || (pad.is_none() && ui.state.take_key(|k| !k.repeat && matches!(k.key, Key::Char(c) if c.eq_ignore_ascii_case(&'r'))).is_some()) {
                    h.item.turned = !h.item.turned;
                    let (w, hh) = h.item.shape();
                    h.grab = Vec2::new(f64::from(w), f64::from(hh)) * cell * 0.5;
                    self.held = Some(h);
                }
                if !point.down {
                    let h = self.held.take().expect("held");
                    match places.iter().find(|(_, r)| r.contains(p)) {
                        Some(&(which, r)) => {
                            let landed = match which {
                                Which::Slot(slot) => slots::release(shelves, h, slot),
                                Which::Worn(wear) => {
                                    let (landed, said) = worn::release(shelves, h, wear);
                                    moved.said = moved.said.or(said);
                                    landed
                                }
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
        // (What a pad holds rides with its cursor, wherever that's got to.)
        let p = if pad.is_some() { self.aim(shelves, &places, cell) } else { p };
        self.draw(ui, shelves, icons, &places, cell, p, pad.map(|pad| pad.labels));
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

#[cfg(test)]
mod pad_tests;
#[cfg(test)]
mod tests;
