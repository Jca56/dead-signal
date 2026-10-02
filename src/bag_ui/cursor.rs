//! The bag screen worked with a pad: a cursor that steps from cell to
//! cell and box to box, standing in for the pointer (a thing picked up
//! rides with it, its top-left cell under the cursor), and which of the
//! two, pad or mouse, is at the screen.

use lntrn_math::{Rect, Vec2};
use lntrn_ui::Ui;

use super::{BagUi, Landing, Shelves, Which, cell_under, corner_cell, landing, slots, worn};
use crate::input::steer::Steer;
use crate::loot::bag::Slot;

/// The mouse moved this far in a frame, it's the mouse at the screen.
const MOUSED: f64 = 3.0;
/// How much dearer a step off to the side is than one straight on,
/// picking where the cursor goes off the edge of a grid.
const ASIDE: f64 = 2.0;

/// What a player has at the screen: the mouse (if it's theirs), and what
/// their pad asks.
#[derive(Clone, Copy, Debug)]
pub struct Hands {
    pub mouse: bool,
    pub pad: Option<Steer>,
}

impl Hands {
    /// The mouse alone.
    pub const MOUSE: Hands = Hands { mouse: true, pad: None };
}

/// Where the pad's cursor is: a cell of a grid, or a box (a slot's, or
/// what's worn's: its one cell).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Cursor {
    pub which: Which,
    pub x: u8,
    pub y: u8,
}

impl Cursor {
    fn first(which: Which) -> Cursor {
        Cursor { which, x: 0, y: 0 }
    }

    /// Where it is on screen: its cell, or its box.
    pub fn rect(self, places: &[(Which, Rect)], cell: f64) -> Option<Rect> {
        let r = places.iter().find(|(w, _)| *w == self.which)?.1;
        Some(match self.which {
            Which::Slot(_) | Which::Worn(_) => r,
            _ => Rect::from_min_size(r.min + Vec2::new(f64::from(self.x), f64::from(self.y)) * cell, Vec2::splat(cell)),
        })
    }
}

/// How many cells across and down `which` is: a grid's own, a box one.
fn size(shelves: &mut Shelves, which: Which) -> (u8, u8) {
    match which {
        Which::Slot(_) | Which::Worn(_) => (1, 1),
        _ => shelves.grid(which).map_or((0, 0), |g| (g.w, g.h)),
    }
}

/// Whether `c` is somewhere on the screen.
fn there(shelves: &mut Shelves, places: &[(Which, Rect)], c: Cursor) -> bool {
    let (w, h) = size(shelves, c.which);
    places.iter().any(|(which, _)| *which == c.which) && c.x < w && c.y < h
}

/// The cursor a step `(dx, dy)` on from `from`: the next cell of its grid
/// (past the rest of whatever it's on, with nothing `held`); or, off the
/// grid's edge (or from a box), the nearest cell or box lying that way.
/// Where it was, with nothing that way.
pub(super) fn stepped(shelves: &mut Shelves, places: &[(Which, Rect)], cell: f64, from: Cursor, (dx, dy): (i32, i32), held: bool) -> Cursor {
    let mut edge = from;
    if let Some(g) = shelves.grid(from.which) {
        let on = if held { None } else { g.at(from.x, from.y) };
        let inside = |x: i32, y: i32| x >= 0 && y >= 0 && x < i32::from(g.w) && y < i32::from(g.h);
        let (mut x, mut y) = (i32::from(from.x) + dx, i32::from(from.y) + dy);
        while on.is_some() && inside(x, y) && g.at(x as u8, y as u8) == on {
            (x, y) = (x + dx, y + dy);
        }
        if inside(x, y) {
            return Cursor { which: from.which, x: x as u8, y: y as u8 };
        }
        edge = Cursor { which: from.which, x: (x - dx) as u8, y: (y - dy) as u8 };
    }
    let Some(here) = edge.rect(places, cell) else { return from };
    let mut best: Option<(f64, Cursor)> = None;
    for &(which, _) in places.iter().filter(|(w, _)| *w != from.which) {
        let (w, h) = size(shelves, which);
        for c in (0..h).flat_map(|y| (0..w).map(move |x| Cursor { which, x, y })) {
            let Some(r) = c.rect(places, cell) else { continue };
            // (Wholly past this one's edge, that way.)
            let past = match (dx, dy) {
                (1, _) => r.min.x >= here.max.x - 1.0,
                (-1, _) => r.max.x <= here.min.x + 1.0,
                (_, 1) => r.min.y >= here.max.y - 1.0,
                _ => r.max.y <= here.min.y + 1.0,
            };
            let to = r.center() - here.center();
            let (along, aside) = if dx != 0 { (to.x.abs(), to.y.abs()) } else { (to.y.abs(), to.x.abs()) };
            let cost = along + aside * ASIDE;
            if past && best.is_none_or(|(b, _)| cost < b) {
                best = Some((cost, c));
            }
        }
    }
    best.map_or(from, |(_, c)| c)
}

impl BagUi {
    /// The screen's just been put up: by a pad (its cursor shown from the
    /// first) or not, the cursor where it starts (on what's searched, if
    /// anything is; else the bag).
    pub fn opened(&mut self, by_pad: bool) {
        (self.by_pad, self.cursor, self.pointer_was) = (by_pad, None, None);
    }

    /// Which is at the screen this frame, going by the last one touched:
    /// what the pad asks, if it's the pad. (One taking over from the other
    /// with something held, it goes back where it came from.)
    pub(super) fn at_it(&mut self, ui: &Ui, shelves: &mut Shelves, hands: Hands) -> Option<Steer> {
        let p = ui.state.pointer;
        let moused = hands.mouse && (ui.state.pressed || ui.state.right_pressed || self.pointer_was.is_some_and(|was| (p - was).length() > MOUSED));
        self.pointer_was = Some(p);
        let by_pad = if !hands.mouse || hands.pad.is_some_and(|pad| pad.any()) {
            true
        } else if moused {
            false
        } else {
            self.by_pad
        };
        if by_pad != self.by_pad {
            self.let_go(shelves);
            self.by_pad = by_pad;
        }
        by_pad.then(|| hands.pad.unwrap_or_default())
    }

    /// The cursor, somewhere that's on the screen: where it was, else on
    /// what's searched, the pack, the pockets, or the first thing there.
    pub(super) fn settle(&mut self, shelves: &mut Shelves, places: &[(Which, Rect)]) -> Cursor {
        if let Some(c) = self.cursor
            && there(shelves, places, c)
        {
            return c;
        }
        let firsts = [Which::Loot, Which::Pack, Which::Pockets].into_iter().chain(places.iter().map(|(w, _)| *w));
        let c = firsts.map(Cursor::first).find(|c| there(shelves, places, *c)).unwrap_or(Cursor::first(Which::Slot(Slot::ALL[0])));
        self.cursor = Some(c);
        c
    }

    /// The cursor moved by `step`, across then down.
    pub(super) fn step(&mut self, shelves: &mut Shelves, places: &[(Which, Rect)], cell: f64, step: (i32, i32)) {
        let mut c = self.settle(shelves, places);
        for way in [(step.0, 0), (0, step.1)].into_iter().filter(|w| *w != (0, 0)) {
            c = stepped(shelves, places, cell, c, way, self.held.is_some());
        }
        self.cursor = Some(c);
    }

    /// Where the cursor points on screen (the middle of its cell or box);
    /// what's held rides with it: its top-left cell under the cursor in a
    /// grid, its middle over a box.
    pub(super) fn aim(&mut self, shelves: &mut Shelves, places: &[(Which, Rect)], cell: f64) -> Vec2 {
        let c = self.settle(shelves, places);
        let at = c.rect(places, cell).map_or(Vec2::ZERO, |r| r.center());
        if let Some(h) = &mut self.held {
            let (w, hh) = h.item.shape();
            h.grab = match c.which {
                Which::Slot(_) | Which::Worn(_) => Vec2::new(f64::from(w), f64::from(hh)) * cell * 0.5,
                _ => Vec2::splat(cell * 0.5),
            };
        }
        at
    }

    /// Whether what's held would go where it's pointed (`p`), put down
    /// there. (A pad keeps hold of what won't; a mouse lets it spring
    /// back.)
    pub(super) fn lands(&self, shelves: &mut Shelves, places: &[(Which, Rect)], cell: f64, p: Vec2) -> bool {
        let (Some(h), Some(&(which, r))) = (self.held, places.iter().find(|(_, r)| r.contains(p))) else { return false };
        match which {
            Which::Slot(slot) => slots::takes(h.item.stack, slot, shelves.bag.slot(slot)),
            Which::Worn(wear) => worn::takes(h.item.stack, wear),
            Which::Belt if !h.item.stack.kind.is_ammo() => false,
            _ => shelves.grid(which).is_some_and(|g| landing(g, h.item, corner_cell(r, p - h.grab, cell), cell_under(r, p, cell)) != Landing::Blocked),
        }
    }
}
