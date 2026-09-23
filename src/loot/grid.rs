//! A grid of cells that things are laid in, each taking its own shape
//! (turned on its side if it's put that way), none overlapping. Things of
//! a kind that stacks gather together, up to their limit, in one place.

use super::{Kind, Stack};

/// Something laid in a grid: what, whose top-left cell, and whether it's
/// turned (across becomes down).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Item {
    pub stack: Stack,
    pub x: u8,
    pub y: u8,
    pub turned: bool,
}

/// How many cells across and down `kind` takes, turned or not.
pub fn shape(kind: Kind, turned: bool) -> (u8, u8) {
    let (w, h) = kind.def().size;
    if turned { (h, w) } else { (w, h) }
}

impl Item {
    pub fn shape(&self) -> (u8, u8) {
        shape(self.stack.kind, self.turned)
    }

    fn covers(&self, x: u8, y: u8) -> bool {
        let (w, h) = self.shape();
        x >= self.x && x < self.x + w && y >= self.y && y < self.y + h
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grid {
    pub w: u8,
    pub h: u8,
    pub items: Vec<Item>,
}

impl Grid {
    pub fn new(w: u8, h: u8) -> Self {
        Self { w, h, items: Vec::new() }
    }

    /// Which thing covers the cell, if any.
    pub fn at(&self, x: u8, y: u8) -> Option<usize> {
        self.items.iter().position(|i| i.covers(x, y))
    }

    /// Whether `kind` would lie at `(x, y)`, turned or not, inside the grid
    /// and over nothing (but the thing `ignore`, which is being moved).
    pub fn fits(&self, kind: Kind, x: i32, y: i32, turned: bool, ignore: Option<usize>) -> bool {
        let (w, h) = shape(kind, turned);
        if x < 0 || y < 0 || x + i32::from(w) > i32::from(self.w) || y + i32::from(h) > i32::from(self.h) {
            return false;
        }
        let (x, y) = (x as u8, y as u8);
        (y..y + h).all(|cy| (x..x + w).all(|cx| self.at(cx, cy).is_none_or(|i| Some(i) == ignore)))
    }

    /// Lay `stack` at `(x, y)` if it fits there. Whether it did.
    pub fn put(&mut self, stack: Stack, x: i32, y: i32, turned: bool) -> bool {
        if stack.count == 0 || !self.fits(stack.kind, x, y, turned, None) {
            return false;
        }
        self.items.push(Item { stack, x: x as u8, y: y as u8, turned });
        true
    }

    /// Where `kind` would first fit, reading across then down, as it lies
    /// and then turned.
    pub fn space_for(&self, kind: Kind) -> Option<(i32, i32, bool)> {
        for turned in [false, true] {
            for y in 0..i32::from(self.h) {
                for x in 0..i32::from(self.w) {
                    if self.fits(kind, x, y, turned, None) {
                        return Some((x, y, turned));
                    }
                }
            }
        }
        None
    }

    /// Top up stacks of `stack`'s kind already here. What's left over.
    pub fn top_up(&mut self, mut stack: Stack) -> Stack {
        let most = stack.kind.def().stack;
        for i in self.items.iter_mut().filter(|i| i.stack.kind == stack.kind) {
            let room = most.saturating_sub(i.stack.count).min(stack.count);
            i.stack.count += room;
            stack.count -= room;
        }
        stack
    }

    /// Lay `stack` in fresh places, a full stack at a time. What's left.
    pub fn place(&mut self, mut stack: Stack) -> Stack {
        let most = stack.kind.def().stack;
        while stack.count > 0 {
            let Some((x, y, turned)) = self.space_for(stack.kind) else { break };
            let n = stack.count.min(most);
            self.put(Stack::new(stack.kind, n), x, y, turned);
            stack.count -= n;
        }
        stack
    }

    /// Take the thing at `index` out.
    pub fn take(&mut self, index: usize) -> Item {
        self.items.remove(index)
    }

    pub fn count(&self, kind: Kind) -> u32 {
        self.items.iter().filter(|i| i.stack.kind == kind).map(|i| i.stack.count).sum()
    }

    /// Take up to `n` of `kind` out, from the smallest stacks first (so
    /// full ones stay full). How many were taken.
    pub fn remove(&mut self, kind: Kind, n: u32) -> u32 {
        let mut left = n;
        while left > 0 {
            let Some(i) = self.items.iter().enumerate().filter(|(_, i)| i.stack.kind == kind).min_by_key(|(_, i)| i.stack.count).map(|(i, _)| i) else { break };
            let took = self.items[i].stack.count.min(left);
            self.items[i].stack.count -= took;
            left -= took;
            if self.items[i].stack.count == 0 {
                self.items.remove(i);
            }
        }
        n - left
    }

    /// What everything here is worth.
    pub fn value(&self) -> u32 {
        self.items.iter().map(|i| i.stack.value()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn things_take_their_shape_and_never_overlap() {
        let mut g = Grid::new(6, 4);
        assert!(g.put(Stack::one(Kind::Battery), 0, 0, false), "2×2 in a corner");
        assert!(!g.put(Stack::one(Kind::Watch), 1, 1, false), "on the battery");
        assert!(g.put(Stack::one(Kind::Watch), 2, 1, false));
        assert!(!g.put(Stack::one(Kind::Medkit), 5, 0, false), "2 wide off the right edge");
        assert!(g.put(Stack::one(Kind::Medkit), 5, 0, true), "but turned it's 1 wide");
        assert_eq!(g.at(5, 1), Some(2), "and it reaches down a cell");
        assert!(!g.fits(Kind::Watch, 5, 1, false, None));
        assert!(g.fits(Kind::Watch, 5, 1, false, Some(2)), "unless that's what is moving");
    }

    #[test]
    fn stacks_gather_up_to_their_limit_then_spill_into_new_places() {
        let mut g = Grid::new(2, 2);
        let left = g.place(Stack::new(Kind::Rounds, 50));
        assert_eq!((left.count, g.items.len()), (0, 2), "a 30 and a 20");
        let left = g.top_up(Stack::new(Kind::Rounds, 15));
        assert_eq!((left.count, g.count(Kind::Rounds)), (5, 60), "topped the 20 up to 30");
        let left = g.place(Stack::new(Kind::Rounds, 70));
        assert_eq!(left.count, 10, "two places left, 30 each");
        assert_eq!(g.remove(Kind::Rounds, 45), 45);
        assert_eq!(g.count(Kind::Rounds), 75);
    }

    #[test]
    fn a_full_grid_turns_things_to_fit_or_leaves_them() {
        let mut g = Grid::new(3, 2);
        g.put(Stack::one(Kind::Battery), 0, 0, false);
        // A 2×1 medkit only fits the last column stood on end.
        assert_eq!(g.space_for(Kind::Medkit), Some((2, 0, true)));
        assert_eq!(g.place(Stack::one(Kind::Medkit)).count, 0);
        assert_eq!(g.place(Stack::one(Kind::Watch)).count, 1, "no room left");
        assert_eq!(g.value(), 180 + 60);
    }
}
