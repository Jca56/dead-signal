//! The inventory screen, over the game while it goes on: the backpack and
//! the pockets, and beside them whatever is being searched. Things are
//! dragged from place to place (R turns what's held), right-clicked
//! straight across (between the bag and what's searched, or pack and
//! pockets), or dragged out onto the ground.

use std::collections::HashMap;

use lntrn_app::lntrn_render::ImageHandle;
use lntrn_math::{Color, Rect, Vec2};
use lntrn_text::TextStyle;
use lntrn_ui::{Key, Ui};

use crate::loot::bag::Bag;
use crate::loot::grid::{Grid, Item};
use crate::loot::{Kind, Stack};
use crate::style;

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
}

/// Where things are kept while the screen is up: the bag, and what's
/// being searched (its name and grid), if anything.
pub struct Shelves<'a> {
    pub bag: &'a mut Bag,
    pub loot: Option<(&'static str, &'a mut Grid)>,
}

impl Shelves<'_> {
    fn grid(&mut self, which: Which) -> Option<&mut Grid> {
        match which {
            Which::Pack => Some(&mut self.bag.pack),
            Which::Pockets => Some(&mut self.bag.pockets),
            Which::Loot => self.loot.as_mut().map(|(_, g)| &mut **g),
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

#[derive(Default)]
pub struct BagUi {
    held: Option<Held>,
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
    /// Nowhere: in the way of something, or a stack already full.
    Blocked,
}

/// Where `held` goes in `grid` with its top-left at `(x, y)` and the
/// pointer over cell `(cx, cy)`: laid down if it fits, onto a stack of its
/// kind under the pointer with room left, or not at all.
fn landing(grid: &Grid, held: Item, (x, y): (i32, i32), (cx, cy): (i32, i32)) -> Landing {
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
        Landing::Blocked => 0,
    };
    let left = h.item.stack.count - landed;
    if left > 0 {
        put_back(shelves, Held { item: Item { stack: Stack::new(h.item.stack.kind, left), ..h.item }, ..h });
    }
    landed
}

/// The grids' places on screen: which, its rect, and a cell's size.
fn layout(ui: &Ui, loot: Option<(u8, u8)>) -> Vec<(Which, Rect)> {
    let s = ui.m.scale;
    let screen = ui.clip();
    let (cell, gap, title) = (CELL * s, GAP * s, TITLE * s * 1.6);
    let pack_w = f64::from(crate::loot::bag::PACK.0) * cell;
    let loot_w = loot.map_or(0.0, |(w, _)| gap + f64::from(w) * cell);
    let left = screen.center().x - (pack_w + loot_w) * 0.5;
    let top = screen.min.y + screen.height() * 0.17 + title;
    let size = |(w, h): (u8, u8)| Vec2::new(f64::from(w) * cell, f64::from(h) * cell);
    let pack = Rect::from_min_size(Vec2::new(left, top), size(crate::loot::bag::PACK));
    let pockets = Rect::from_min_size(Vec2::new(left, pack.max.y + title + gap * 0.5), size(crate::loot::bag::POCKETS));
    let mut out = vec![(Which::Pack, pack), (Which::Pockets, pockets)];
    if let Some(dims) = loot {
        out.push((Which::Loot, Rect::from_min_size(Vec2::new(pack.max.x + gap, top), size(dims))));
    }
    out
}

impl BagUi {
    /// Whatever is held goes back where it came from (the screen closing).
    pub fn let_go(&mut self, shelves: &mut Shelves) {
        if let Some(h) = self.held.take() {
            put_back(shelves, h);
        }
    }

    /// A frame of the screen: what was thrown on the ground and what was
    /// taken.
    pub fn frame(&mut self, ui: &mut Ui, shelves: &mut Shelves, icons: &Icons) -> Moved {
        let s = ui.m.scale;
        let cell = CELL * s;
        let mut moved = Moved::default();
        let places = layout(ui, shelves.loot.as_ref().map(|(_, g)| (g.w, g.h)));
        let p = ui.state.pointer;
        let under_item = places.iter().find(|(_, r)| r.contains(p)).and_then(|&(which, r)| {
            let (cx, cy) = cell_under(r, p, cell);
            shelves.grid(which).and_then(|g| g.at(cx as u8, cy as u8)).map(|i| (which, i))
        });

        match self.held {
            None => {
                if ui.state.pressed
                    && let Some((which, i)) = under_item
                    && let Some(g) = shelves.grid(which)
                {
                    let item = g.take(i);
                    let r = places.iter().find(|(w, _)| *w == which).map(|(_, r)| *r).unwrap_or(Rect::ZERO);
                    let at = r.min + Vec2::new(f64::from(item.x), f64::from(item.y)) * cell;
                    self.held = Some(Held { item, from: which, was: item, grab: p - at });
                } else if ui.state.right_pressed
                    && let Some((which, i)) = under_item
                {
                    let kind = shelves.grid(which).map(|g| g.items[i].stack.kind);
                    let n = quick_move(shelves, which, i);
                    if which == Which::Loot
                        && n > 0
                        && let Some(kind) = kind
                    {
                        moved.taken.push(Stack::new(kind, n));
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
                            let landed = release(shelves, h, which, corner_cell(r, p - h.grab, cell), cell_under(r, p, cell));
                            if h.from == Which::Loot && which != Which::Loot && landed > 0 {
                                moved.taken.push(Stack::new(h.item.stack.kind, landed));
                            }
                        }
                        None => moved.dropped.push(h.item.stack),
                    }
                }
            }
        }
        self.draw(ui, shelves, icons, &places);
        moved
    }

    fn draw(&self, ui: &mut Ui, shelves: &mut Shelves, icons: &Icons, places: &[(Which, Rect)]) {
        let s = ui.m.scale;
        let cell = CELL * s;
        let screen = ui.clip();
        ui.draw.rect(screen, Color::rgba(0.0, 0.0, 0.0, 0.45));
        let title = TextStyle::new((TITLE * s) as f32).bold().family(style::FONT);
        let loot_name = shelves.loot.as_ref().map(|(name, _)| *name);
        for &(which, r) in places {
            let name = match which {
                Which::Pack => "BACKPACK",
                Which::Pockets => "POCKETS",
                Which::Loot => loot_name.unwrap_or(""),
            };
            ui.text_at(name, &title, Vec2::new(r.min.x, r.min.y - f64::from(title.line_height()) - 10.0 * s), r.width() * 2.0, if which == Which::Loot { style::SIGNAL } else { style::BONE });
            ui.draw.rect(r.expand(6.0 * s), Color::rgba(0.05, 0.05, 0.05, 0.85));
            let Some(g) = shelves.grid(which) else { continue };
            for y in 0..g.h {
                for x in 0..g.w {
                    let c = Rect::from_min_size(r.min + Vec2::new(f64::from(x), f64::from(y)) * cell, Vec2::splat(cell)).shrink(2.0 * s);
                    ui.draw.rect(c, Color::rgba(1.0, 1.0, 1.0, 0.05));
                }
            }
            let items = g.items.clone();
            for item in &items {
                let at = r.min + Vec2::new(f64::from(item.x), f64::from(item.y)) * cell;
                tile(ui, icons, *item, at, cell, 1.0);
            }
        }
        // The value of what's carried, under the pack.
        if let Some(&(_, pack)) = places.iter().find(|(w, _)| *w == Which::Pack) {
            let style = TextStyle::new((26.0 * s) as f32).bold().family(style::FONT);
            let text = format!("VALUE  ${}", shelves.bag.value());
            let w = ui.measure(&text, &style);
            let pockets_top = places.iter().find(|(w, _)| *w == Which::Pockets).map_or(pack.max.y, |(_, r)| r.min.y);
            ui.text_at(&text, &style, Vec2::new(pack.max.x - w, pockets_top), w + 10.0, style::BONE);
        }
        let p = ui.state.pointer;
        match self.held {
            Some(h) => {
                // Where it would land, green (fresh, or onto a stack with
                // room) or red, then the thing itself.
                if let Some(&(which, r)) = places.iter().find(|(_, r)| r.contains(p))
                    && let Some(g) = shelves.grid(which)
                {
                    let (x, y) = corner_cell(r, p - h.grab, cell);
                    let (spot, ok) = match landing(g, h.item, (x, y), cell_under(r, p, cell)) {
                        Landing::Put(x, y) => (footprint(r, x, y, h.item.shape(), cell), true),
                        Landing::Merge(i, _) => (footprint(r, i32::from(g.items[i].x), i32::from(g.items[i].y), g.items[i].shape(), cell), true),
                        Landing::Blocked => (footprint(r, x, y, h.item.shape(), cell), false),
                    };
                    ui.draw.rect(spot, if ok { Color::rgba(0.3, 0.8, 0.3, 0.3) } else { Color::rgba(0.9, 0.2, 0.15, 0.3) });
                }
                tile(ui, icons, h.item, p - h.grab, cell, 0.85);
            }
            None => {
                let hovered = places.iter().find(|(_, r)| r.contains(p)).and_then(|&(which, r)| {
                    let (cx, cy) = cell_under(r, p, cell);
                    shelves.grid(which).and_then(|g| g.at(cx as u8, cy as u8).map(|i| g.items[i].stack))
                });
                if let Some(stack) = hovered {
                    tooltip(ui, stack, p);
                }
            }
        }
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

/// A thing drawn at `at`: its rarity's colour behind and round it, its
/// picture, how many.
fn tile(ui: &mut Ui, icons: &Icons, item: Item, at: Vec2, cell: f64, alpha: f64) {
    let s = ui.m.scale;
    let (w, h) = item.shape();
    let r = Rect::from_min_size(at, Vec2::new(f64::from(w), f64::from(h)) * cell).shrink(3.0 * s);
    let c = item.stack.kind.def().rarity.colour();
    ui.draw.rect(r, Color::rgba(c.r * 0.35, c.g * 0.35, c.b * 0.35, 0.85 * alpha));
    ui.draw.stroke_rect(r, 2.0 * s, 0.0, Color::rgba(c.r, c.g, c.b, alpha));
    if let Some(&(flat, turned)) = icons.0.get(&item.stack.kind) {
        ui.draw.image(r.shrink(4.0 * s), if item.turned { turned } else { flat }, 0.0, Color::rgba(1.0, 1.0, 1.0, alpha));
    }
    if item.stack.count > 1 {
        let style = TextStyle::new((22.0 * s) as f32).bold().family(style::FONT);
        let text = item.stack.count.to_string();
        let tw = ui.measure(&text, &style);
        let th = f64::from(style.line_height());
        let at = Vec2::new(r.max.x - tw - 6.0 * s, r.max.y - th - 2.0 * s);
        ui.text_at(&text, &style, at + Vec2::new(2.0 * s, 2.0 * s), tw + 4.0, Color::rgba(0.0, 0.0, 0.0, 0.8 * alpha));
        ui.text_at(&text, &style, at, tw + 4.0, Color::rgba(style::BONE.r, style::BONE.g, style::BONE.b, alpha));
    }
}

/// Name, rarity and worth, beside the pointer.
fn tooltip(ui: &mut Ui, stack: Stack, p: Vec2) {
    let s = ui.m.scale;
    let def = stack.kind.def();
    let name = TextStyle::new((26.0 * s) as f32).bold().family(style::FONT);
    let line = TextStyle::new((22.0 * s) as f32).family(style::FONT);
    let worth = if stack.count > 1 { format!("${} each  ·  ${}", def.value, stack.value()) } else { format!("${}", def.value) };
    let lines = [(def.name.to_string(), &name, def.rarity.colour()), (def.rarity.name().to_string(), &line, style::DIM), (worth, &line, style::BONE)];
    let pad = 14.0 * s;
    let w = lines.iter().map(|(t, st, _)| ui.measure(t, st)).fold(0.0, f64::max) + pad * 2.0;
    let h: f64 = lines.iter().map(|(_, st, _)| f64::from(st.line_height()) + 4.0 * s).sum::<f64>() + pad * 2.0;
    let screen = ui.clip();
    let mut at = p + Vec2::new(24.0 * s, 24.0 * s);
    at.x = at.x.min(screen.max.x - w - 10.0 * s);
    at.y = at.y.min(screen.max.y - h - 10.0 * s);
    let r = Rect::from_min_size(at, Vec2::new(w, h));
    ui.draw.rect(r, Color::rgba(0.03, 0.03, 0.03, 0.95));
    ui.draw.stroke_rect(r, 2.0 * s, 0.0, def.rarity.colour());
    let mut y = at.y + pad;
    for (text, st, colour) in &lines {
        ui.text_at(text, st, Vec2::new(at.x + pad, y), w, *colour);
        y += f64::from(st.line_height()) + 4.0 * s;
    }
}

/// Send the thing at `index` in `from` straight across: out of what's
/// searched into the bag, into what's searched from the bag, or (nothing
/// searched) between pack and pockets. What won't fit stays. How many went.
fn quick_move(shelves: &mut Shelves, from: Which, index: usize) -> u32 {
    let Some(item) = shelves.grid(from).map(|g| g.take(index)) else { return 0 };
    let left = match from {
        Which::Loot => shelves.bag.add(item.stack),
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

/// `h` back where it was; failing that, anywhere it goes in the bag.
fn put_back(shelves: &mut Shelves, h: Held) {
    let stack = h.item.stack;
    let back = shelves.grid(h.from).is_some_and(|g| g.put(stack, i32::from(h.was.x), i32::from(h.was.y), h.was.turned));
    if !back {
        shelves.bag.add(stack);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn right_click_sends_things_across_and_keeps_what_wont_fit() {
        let mut bag = Bag::default();
        let mut crate_ = Grid::new(4, 3);
        crate_.place(Stack::new(Kind::Rounds, 12));
        crate_.place(Stack::one(Kind::Watch));
        let mut shelves = Shelves { bag: &mut bag, loot: Some(("CRATE", &mut crate_)) };
        // Rounds out of the crate top up the pocket's 24 to 30, the rest
        // take a place of their own.
        assert_eq!(quick_move(&mut shelves, Which::Loot, 0), 12);
        assert_eq!(shelves.bag.count(Kind::Rounds), 36);
        assert_eq!(shelves.bag.pockets.items[0].stack.count, 30);
        // And back: the pocket's rounds go into the crate.
        let i = shelves.bag.pockets.items.iter().position(|i| i.stack.kind == Kind::Rounds).unwrap();
        quick_move(&mut shelves, Which::Pockets, i);
        assert_eq!(shelves.loot.as_ref().unwrap().1.count(Kind::Rounds), 30);
        // Nothing searched: the pack and pockets trade.
        let mut bag = Bag::default();
        let mut shelves = Shelves { bag: &mut bag, loot: None };
        quick_move(&mut shelves, Which::Pockets, 0);
        assert_eq!((shelves.bag.pack.count(Kind::Rounds), shelves.bag.pockets.count(Kind::Rounds)), (24, 0));
        // A full pack sends nothing, loses nothing.
        let mut bag = Bag::default();
        for _ in 0..6 {
            bag.pack.place(Stack::one(Kind::Battery));
        }
        let mut shelves = Shelves { bag: &mut bag, loot: None };
        assert_eq!(quick_move(&mut shelves, Which::Pockets, 0), 0);
        assert_eq!(shelves.bag.pockets.count(Kind::Rounds), 24);
    }

    #[test]
    fn a_stack_dropped_on_its_kind_tops_it_up_and_the_rest_goes_back() {
        // 28 rounds in the pack; 7 found in a crate, dragged onto them.
        let mut bag = Bag::default();
        bag.pockets.items.clear();
        bag.pack.put(Stack::new(Kind::Rounds, 28), 2, 1, false);
        let mut crate_ = Grid::new(4, 3);
        crate_.put(Stack::new(Kind::Rounds, 7), 1, 1, false);
        let mut shelves = Shelves { bag: &mut bag, loot: Some(("CRATE", &mut crate_)) };
        let item = shelves.grid(Which::Loot).unwrap().take(0);
        let held = Held { item, from: Which::Loot, was: item, grab: Vec2::ZERO };
        assert_eq!(landing(&shelves.bag.pack, item, (2, 1), (2, 1)), Landing::Merge(0, 2), "green: room for 2");
        assert_eq!(release(&mut shelves, held, Which::Pack, (2, 1), (2, 1)), 2);
        assert_eq!(shelves.bag.pack.items[0].stack.count, 30);
        let back = &shelves.loot.as_ref().unwrap().1.items[0];
        assert_eq!((back.stack.count, back.x, back.y), (5, 1, 1), "the 5 went back to the crate");
        // Onto a full stack: red, and nothing moves.
        let item = shelves.grid(Which::Loot).unwrap().take(0);
        assert_eq!(landing(&shelves.bag.pack, item, (2, 1), (2, 1)), Landing::Blocked);
        let held = Held { item, from: Which::Loot, was: item, grab: Vec2::ZERO };
        assert_eq!(release(&mut shelves, held, Which::Pack, (2, 1), (2, 1)), 0);
        assert_eq!(shelves.loot.as_ref().unwrap().1.count(Kind::Rounds), 5);
        // Onto something else: red.
        assert_eq!(landing(&shelves.bag.pack, Item { stack: Stack::one(Kind::Watch), ..item }, (2, 1), (2, 1)), Landing::Blocked);
    }
}
