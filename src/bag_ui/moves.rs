//! Things moved about the bag screen: where something let go of would
//! land in a grid, letting go of it there, sending a thing straight
//! across (and trading, into what's to be sold), and putting what's held
//! back where it came from.

use lntrn_math::Vec2;

use super::{Held, Shelves, Which};
use crate::loot::bag::Slot;
use crate::loot::grid::{Grid, Item};

/// Where something held would go, let go of over a grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Landing {
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
pub(super) fn landing(grid: &Grid, held: Item, (x, y): (i32, i32), (cx, cy): (i32, i32)) -> Landing {
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
pub(super) fn release(shelves: &mut Shelves, h: Held, which: Which, at: (i32, i32), over: (i32, i32)) -> u32 {
    // A bandolier holds rounds, nothing else.
    if which == Which::Belt && !h.item.stack.kind.is_ammo() {
        put_back(shelves, h);
        return 0;
    }
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

/// Send the thing at `index` in `from` straight across: out of what's
/// searched into the bag, into what's searched from the bag, or (nothing
/// searched) between pack and pockets. What won't fit stays. How many went.
pub(super) fn quick_move(shelves: &mut Shelves, from: Which, index: usize) -> u32 {
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
pub(super) fn sell_move(shelves: &mut Shelves, from: Which, index: usize) {
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
pub(super) fn put_back(shelves: &mut Shelves, h: Held) {
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
        Which::Worn(wear) => shelves.bag.worn(wear).is_none() && shelves.bag.wear(stack, shelves.fit).is_ok(),
        _ => shelves.grid(h.from).is_some_and(|g| g.put(stack, i32::from(h.was.x), i32::from(h.was.y), h.was.turned)),
    };
    if !back {
        shelves.bag.add(stack);
    }
}
