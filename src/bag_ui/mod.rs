//! The inventory screen, over the game while it goes on: the weapons in
//! their slots (`slots.rs`), the backpack and the pockets, and beside them
//! whatever is being searched. Things are dragged from place to place (R
//! turns what's held), right-clicked straight across (between the bag and
//! what's searched, or pack and pockets; a weapon into its empty slot, or
//! out of it), or dragged out onto the ground.

mod slots;

use std::collections::HashMap;

use lntrn_app::lntrn_render::ImageHandle;
use lntrn_math::{Color, Rect, Vec2};
use lntrn_text::TextStyle;
use lntrn_ui::{Key, Ui};

use crate::loot::bag::{Bag, Slot};
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
    Slot(Slot),
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

#[derive(Default)]
pub struct BagUi {
    held: Option<Held>,
    /// In the hideout: the stash (the "loot" grid) on the left, a little
    /// smaller all over, and nothing thrown on any ground.
    hideout: bool,
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
        put_back(shelves, Held { item: Item { stack: h.item.stack.with_count(left), ..h.item }, ..h });
    }
    landed
}

/// A cell's side in the hideout (a 10 × 10 stash must fit a short screen).
const HIDEOUT_CELL: f64 = 70.0;

impl BagUi {
    /// The screen as the hideout has it.
    pub fn hideout() -> Self {
        Self { held: None, hideout: true }
    }

    /// A cell's side, logical pixels.
    fn cell(&self) -> f64 {
        if self.hideout { HIDEOUT_CELL } else { CELL }
    }

    /// The grids' and slots' places on screen: which, and its rect. In a
    /// run the slots on the left, then the bag, what's searched on the
    /// right; in the hideout the stash on the left, then the bag, the slots
    /// on the right.
    fn layout(&self, ui: &Ui, bag: &Bag, loot: Option<(u8, u8)>) -> Vec<(Which, Rect)> {
        let s = ui.m.scale;
        let screen = ui.clip();
        let (cell, gap, title) = (self.cell() * s, GAP * s, TITLE * s * 1.6);
        let (pack_dims, pocket_dims) = ((bag.pack.w, bag.pack.h), (bag.pockets.w, bag.pockets.h));
        let pack_w = f64::from(pack_dims.0) * cell;
        let loot_w = loot.map_or(0.0, |(w, _)| gap + f64::from(w) * cell);
        let slots_w = slots::WIDTH * cell + gap;
        let left = screen.center().x - (slots_w + pack_w + loot_w) * 0.5;
        let top = screen.min.y + screen.height() * 0.17 + title;
        let size = |(w, h): (u8, u8)| Vec2::new(f64::from(w) * cell, f64::from(h) * cell);
        let bag_x = if self.hideout { left + loot_w } else { left + slots_w };
        let slots_x = if self.hideout { bag_x + pack_w + gap } else { left };
        let pack = Rect::from_min_size(Vec2::new(bag_x, top), size(pack_dims));
        let pockets = Rect::from_min_size(Vec2::new(bag_x, pack.max.y + title + gap * 0.5), size(pocket_dims));
        let mut out = vec![(Which::Pack, pack), (Which::Pockets, pockets)];
        if let Some(dims) = loot {
            let x = if self.hideout { left } else { pack.max.x + gap };
            out.push((Which::Loot, Rect::from_min_size(Vec2::new(x, top), size(dims))));
        }
        out.extend(slots::layout(Vec2::new(slots_x, top), cell, s));
        out
    }
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
        let cell = self.cell() * s;
        let mut moved = Moved::default();
        let places = self.layout(ui, shelves.bag, shelves.loot.as_ref().map(|(_, g)| (g.w, g.h)));
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
                        None if self.hideout => put_back(shelves, h),
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
        let cell = self.cell() * s;
        let screen = ui.clip();
        ui.draw.rect(screen, Color::rgba(0.0, 0.0, 0.0, 0.45));
        let title = TextStyle::new((TITLE * s) as f32).bold().family(style::FONT);
        let loot_name = shelves.loot.as_ref().map(|(name, _)| *name);
        for &(which, r) in places {
            let name = match which {
                Which::Pack => "BACKPACK",
                Which::Pockets => "POCKETS",
                Which::Loot => loot_name.unwrap_or(""),
                Which::Slot(slot) => {
                    // The column's name over the first.
                    if slot == Slot::ALL[0] {
                        ui.text_at("WEAPONS", &title, Vec2::new(r.min.x, r.min.y - f64::from(title.line_height()) - 10.0 * s), r.width() * 2.0, style::BONE);
                    }
                    slots::draw(ui, icons, slot, shelves.bag.slot(slot), r, cell);
                    continue;
                }
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
                if let Some(&(Which::Slot(slot), r)) = places.iter().find(|(_, r)| r.contains(p)) {
                    let ok = slots::takes(h.item.stack, slot);
                    ui.draw.rect(r, if ok { Color::rgba(0.3, 0.8, 0.3, 0.3) } else { Color::rgba(0.9, 0.2, 0.15, 0.3) });
                } else if let Some(&(which, r)) = places.iter().find(|(_, r)| r.contains(p))
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
                let hovered = places.iter().find(|(_, r)| r.contains(p)).and_then(|&(which, r)| match which {
                    Which::Slot(slot) => shelves.bag.slot(slot),
                    _ => {
                        let (cx, cy) = cell_under(r, p, cell);
                        shelves.grid(which).and_then(|g| g.at(cx as u8, cy as u8).map(|i| g.items[i].stack))
                    }
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
    // How many; for a gun, the rounds in it.
    let text = match item.stack.magazine() {
        Some(mag) => Some(format!("{}/{mag}", item.stack.loaded)),
        None => (item.stack.count > 1).then(|| item.stack.count.to_string()),
    };
    if let Some(text) = text {
        let style = TextStyle::new((22.0 * s) as f32).bold().family(style::FONT);
        let tw = ui.measure(&text, &style);
        let th = f64::from(style.line_height());
        let at = Vec2::new(r.max.x - tw - 6.0 * s, r.max.y - th - 2.0 * s);
        ui.text_at(&text, &style, at + Vec2::new(2.0 * s, 2.0 * s), tw + 4.0, Color::rgba(0.0, 0.0, 0.0, 0.8 * alpha));
        ui.text_at(&text, &style, at, tw + 4.0, Color::rgba(style::BONE.r, style::BONE.g, style::BONE.b, alpha));
    }
}

/// Name, rarity and worth (and for a weapon, its slot and rounds), beside
/// the pointer.
fn tooltip(ui: &mut Ui, stack: Stack, p: Vec2) {
    let s = ui.m.scale;
    let def = stack.kind.def();
    let name = TextStyle::new((26.0 * s) as f32).bold().family(style::FONT);
    let line = TextStyle::new((22.0 * s) as f32).family(style::FONT);
    let worth = if stack.count > 1 { format!("${} each  ·  ${}", def.value, stack.value()) } else { format!("${}", def.value) };
    let mut lines = vec![(def.name.to_string(), &name, def.rarity.colour()), (def.rarity.name().to_string(), &line, style::DIM)];
    if let Some(slot) = Slot::of(stack.kind) {
        let rounds = stack.magazine().map_or(String::new(), |mag| format!("  ·  {}/{mag} ROUNDS", stack.loaded));
        lines.push((format!("{}{rounds}", slot.name()), &line, style::BONE));
    }
    lines.push((worth, &line, style::BONE));
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
