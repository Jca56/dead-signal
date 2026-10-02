//! Drawing the inventory screen: every grid and slot with what's in it,
//! what's held under the pointer and where it would land, a thing's tile,
//! and the tooltip over what's pointed at. Worked by a pad: its cursor,
//! and what its buttons do, along the foot.

use lntrn_math::{Color, Rect, Vec2};
use lntrn_text::TextStyle;
use lntrn_ui::Ui;

use super::{BagUi, Icons, Landing, Mode, Shelves, TITLE, Which, cell_under, corner_cell, footprint, landing, slots, worn};
use crate::input::pad::Labels;
use crate::input::steer;
use crate::loot::Stack;
use crate::loot::bag::Slot;
use crate::loot::grid::Item;
use crate::style;

/// A pad's cursor, and its buttons' names in the hints.
const CURSOR: Color = Color::rgb(1.0, 0.82, 0.25);

impl BagUi {
    /// The screen, pointed at at `p`: by the mouse, or (`pad`: whose names
    /// its buttons go by) a pad's cursor.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw(&self, ui: &mut Ui, shelves: &mut Shelves, icons: &Icons, places: &[(Which, Rect)], cell: f64, p: Vec2, pad: Option<Labels>) {
        let s = ui.m.scale;
        let screen = self.screen(ui);
        ui.draw.rect(screen, Color::rgba(0.0, 0.0, 0.0, 0.45));
        let title = TextStyle::new((TITLE * s) as f32).bold().family(style::FONT);
        let loot_name = shelves.loot.as_ref().map(|(name, _)| *name);
        for &(which, r) in places {
            let name = match which {
                Which::Pack if shelves.bag.pack.w == 0 => "NO BACKPACK",
                Which::Pack => "BACKPACK",
                Which::Sell => "SELL",
                Which::Pockets => "POCKETS",
                Which::Rig if shelves.bag.rig.w == 0 => continue,
                Which::Rig => "RIG",
                Which::Belt if shelves.bag.belt.w == 0 => continue,
                Which::Belt => "BELT · AMMO",
                Which::Worn(wear) => {
                    if wear == crate::loot::gear::Wear::ALL[0] {
                        ui.text_at("WORN", &title, Vec2::new(r.min.x, r.min.y - f64::from(title.line_height()) - 10.0 * s), r.width() * 2.0, style::BONE);
                    }
                    worn::draw(ui, icons, wear, shelves.bag.worn(wear), r, cell);
                    continue;
                }
                Which::Loot => loot_name.unwrap_or(""),
                Which::Slot(slot) => {
                    // The column's name over the first.
                    if slot == Slot::ALL[0] {
                        ui.text_at("WEAPONS", &title, Vec2::new(r.min.x, r.min.y - f64::from(title.line_height()) - 10.0 * s), r.width() * 2.0, style::BONE);
                    }
                    let i = Slot::ALL.iter().position(|&s| s == slot).unwrap_or(0);
                    slots::draw(ui, icons, slot, &self.slot_keys[i], shelves.bag.slot(slot), r, cell);
                    continue;
                }
            };
            ui.text_at(name, &title, Vec2::new(r.min.x, r.min.y - f64::from(title.line_height()) - 10.0 * s), r.width() * 2.0, if which == Which::Loot { style::SIGNAL } else { style::BONE });
            if r.width() > 0.0 {
                ui.draw.rect(r.expand(6.0 * s), Color::rgba(0.05, 0.05, 0.05, 0.85));
            }
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
        // The value of what's carried, under the pack (trading, the sale's
        // button says what's to be sold would fetch).
        if let Some(&(_, pack)) = places.iter().find(|(w, _)| *w == Which::Pack) {
            let style = TextStyle::new((26.0 * s) as f32).bold().family(style::FONT);
            let text = format!("VALUE  ${}", shelves.bag.value());
            let w = ui.measure(&text, &style);
            // (Level with the pockets' top, at the pack's right; the pockets
            // beside the pack, under them.)
            let pockets = places.iter().find(|(w, _)| *w == Which::Pockets).map_or(pack, |(_, r)| *r);
            let at = if pack.height() > 0.0 && pockets.min.x >= pack.max.x { Vec2::new(pockets.min.x, pockets.max.y + 12.0 * s) } else { Vec2::new(pack.max.x - w, pockets.min.y.max(pack.max.y)) };
            ui.text_at(&text, &style, at, w + 10.0, style::BONE);
        }
        let under = places.iter().find(|(_, r)| r.contains(p)).copied();
        match self.held {
            Some(h) => {
                // Where it would land, green (fresh, or onto a stack with
                // room) or red, then the thing itself.
                let spot = under.and_then(|(which, r)| match which {
                    Which::Slot(slot) => Some((r, slots::takes(h.item.stack, slot, shelves.bag.slot(slot)))),
                    Which::Worn(wear) => Some((r, worn::takes(h.item.stack, wear))),
                    _ => {
                        let g = shelves.grid(which)?;
                        let (x, y) = corner_cell(r, p - h.grab, cell);
                        let fits = which != Which::Belt || h.item.stack.kind.is_ammo();
                        Some(match landing(g, h.item, (x, y), cell_under(r, p, cell)) {
                            _ if !fits => (footprint(r, x, y, h.item.shape(), cell), false),
                            Landing::Put(x, y) => (footprint(r, x, y, h.item.shape(), cell), true),
                            Landing::Merge(i, _) | Landing::Load(i, _) => (footprint(r, i32::from(g.items[i].x), i32::from(g.items[i].y), g.items[i].shape(), cell), true),
                            Landing::Blocked => (footprint(r, x, y, h.item.shape(), cell), false),
                        })
                    }
                });
                if let Some((spot, ok)) = spot {
                    ui.draw.rect(spot, if ok { Color::rgba(0.3, 0.8, 0.3, 0.3) } else { Color::rgba(0.9, 0.2, 0.15, 0.3) });
                    if pad.is_some() {
                        ui.draw.stroke_rect(spot, 4.0 * s, 0.0, CURSOR);
                    }
                }
                tile(ui, icons, h.item, p - h.grab, cell, 0.85);
            }
            None => {
                // What's pointed at: the thing, and all of it (a pad's
                // cursor goes round it, or the empty cell or box).
                let hovered = under.and_then(|(which, r)| match which {
                    Which::Slot(slot) => shelves.bag.slot(slot).map(|stack| (stack, r)),
                    Which::Worn(wear) => shelves.bag.worn(wear).map(|stack| (stack, r)),
                    _ => {
                        let (cx, cy) = cell_under(r, p, cell);
                        let g = shelves.grid(which)?;
                        let item = g.items[g.at(cx as u8, cy as u8)?];
                        Some((item.stack, footprint(r, i32::from(item.x), i32::from(item.y), item.shape(), cell)))
                    }
                });
                if pad.is_some()
                    && let Some(at) = hovered.map(|(_, r)| r).or_else(|| self.cursor.and_then(|c| c.rect(places, cell)))
                {
                    ui.draw.stroke_rect(at, 4.0 * s, 0.0, CURSOR);
                }
                if let Some((stack, r)) = hovered {
                    // (Beside the thing, by a pad; under the pointer.)
                    tooltip(ui, stack, if pad.is_some() { r.max - Vec2::splat(12.0 * s) } else { p }, screen);
                }
            }
        }
        if let Some(labels) = pad {
            let name = |c| labels.name(c);
            let drop = (self.mode == Mode::Run).then_some((name(steer::ACROSS), "DROP"));
            let list: Vec<(&str, &str)> = match self.held {
                Some(_) => [Some((name(steer::PICK), "PUT DOWN")), Some((name(steer::TURN), "TURN")), drop, Some((name(steer::BACK), "PUT BACK"))].into_iter().flatten().collect(),
                None => vec![(name(steer::PICK), "PICK UP"), (name(steer::ACROSS), "EQUIP / MOVE"), (name(steer::BACK), "CLOSE")],
            };
            hints(ui, screen, &list);
        }
    }
}

/// A pad's buttons and what each does, along the foot of `screen`.
fn hints(ui: &mut Ui, screen: Rect, list: &[(&str, &str)]) {
    let s = ui.m.scale;
    let style = TextStyle::new((24.0 * s) as f32).bold().family(style::FONT);
    let (space, between, pad) = (10.0 * s, 36.0 * s, 18.0 * s);
    let widths: Vec<(f64, f64)> = list.iter().map(|(button, word)| (ui.measure(button, &style), ui.measure(word, &style))).collect();
    let whole = widths.iter().map(|(b, w)| b + space + w).sum::<f64>() + between * list.len().saturating_sub(1) as f64;
    let high = f64::from(style.line_height());
    let mut at = Vec2::new(screen.center().x - whole * 0.5, screen.max.y - 20.0 * s - high);
    ui.draw.rect(Rect::from_min_size(at - Vec2::new(pad, 8.0 * s), Vec2::new(whole + pad * 2.0, high + 16.0 * s)), Color::rgba(0.0, 0.0, 0.0, 0.7));
    for ((button, word), (bw, ww)) in list.iter().zip(widths) {
        ui.text_at(button, &style, at, bw + 4.0, CURSOR);
        at.x += bw + space;
        ui.text_at(word, &style, at, ww + 4.0, style::BONE);
        at.x += ww + between;
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
    // How many; for a gun, the rounds in it; for armor, its points left.
    let text = match item.stack.most() {
        Some(most) => Some(format!("{}/{most}", item.stack.loaded)),
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
/// the pointer, kept on `screen`.
fn tooltip(ui: &mut Ui, stack: Stack, p: Vec2, screen: Rect) {
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
    // What's worn: where, its armor, its pockets or grid, how heavy.
    if let Some(g) = stack.kind.gear() {
        let mut bits = vec![format!("WORN ON {}", g.wear.name())];
        if g.armor > 0 {
            bits.push(format!("ARMOR {}/{}", stack.loaded, g.armor));
        }
        if let Some((w, h)) = g.grid {
            bits.push(if g.ammo_only { format!("{w}×{h} FOR AMMO") } else { format!("{w}×{h} GRID") });
        }
        if g.pockets != (0, 0) {
            bits.push("BIGGER POCKETS".to_string());
        }
        bits.push(["LIGHT", "MEDIUM", "HEAVY"][usize::from(g.weight.min(2))].to_string());
        lines.push((bits.join("  ·  "), &line, style::BONE));
    }
    lines.push((worth, &line, style::BONE));
    let pad = 14.0 * s;
    let w = lines.iter().map(|(t, st, _)| ui.measure(t, st)).fold(0.0, f64::max) + pad * 2.0;
    let h: f64 = lines.iter().map(|(_, st, _)| f64::from(st.line_height()) + 4.0 * s).sum::<f64>() + pad * 2.0;
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
