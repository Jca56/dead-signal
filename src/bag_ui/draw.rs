//! Drawing the inventory screen: every grid and slot with what's in it,
//! what's held under the pointer and where it would land, a thing's tile,
//! and the tooltip over what's pointed at.

use lntrn_math::{Color, Rect, Vec2};
use lntrn_text::TextStyle;
use lntrn_ui::Ui;

use super::{BagUi, Icons, Landing, Mode, Shelves, TITLE, Which, cell_under, corner_cell, footprint, landing, slots};
use crate::loot::Stack;
use crate::loot::bag::Slot;
use crate::loot::grid::Item;
use crate::style;

impl BagUi {
    pub(super) fn draw(&self, ui: &mut Ui, shelves: &mut Shelves, icons: &Icons, places: &[(Which, Rect)], cell: f64) {
        let s = ui.m.scale;
        let trade = self.mode == Mode::Trade;
        let screen = ui.clip();
        ui.draw.rect(screen, Color::rgba(0.0, 0.0, 0.0, 0.45));
        let title = TextStyle::new((TITLE * s) as f32).bold().family(style::FONT);
        let loot_name = shelves.loot.as_ref().map(|(name, _)| *name);
        for &(which, r) in places {
            let name = match which {
                Which::Pack if trade => "SELL",
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
        // The value of what's carried, under the pack; trading, what's to
        // be sold would fetch.
        if let Some(&(_, pack)) = places.iter().find(|(w, _)| *w == Which::Pack) {
            let style = TextStyle::new((26.0 * s) as f32).bold().family(style::FONT);
            let text = if trade {
                let paid: u32 = shelves.bag.pack.items.iter().map(|i| crate::profile::trade::sell_price(i.stack)).sum();
                format!("SPARKS PAYS  ${paid}")
            } else {
                format!("VALUE  ${}", shelves.bag.value())
            };
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

/// A thing drawn at `at`: its rarity's colour behind and round it, its
/// picture, how many.
pub(super) fn tile(ui: &mut Ui, icons: &Icons, item: Item, at: Vec2, cell: f64, alpha: f64) {
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
pub(super) fn tooltip(ui: &mut Ui, stack: Stack, p: Vec2) {
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
