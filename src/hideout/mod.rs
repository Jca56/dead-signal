//! The hideout, between runs, in three tabs: the bag to take into the next
//! run (and its weapons) on the left and the stash on the right, things
//! dragged (or right-clicked) between them; trading (`trader.rs`); and the perks
//! (`perks.rs`). The player's level and money over it all, and the way on
//! (back to the title, or straight into a run with what's packed).

mod perks;
mod trader;

pub(crate) use perks::pressed;

use lntrn_math::{Color, Rect, Vec2};
use lntrn_text::TextStyle;
use lntrn_ui::{Key, Sense, Ui};

use crate::bag_ui::{BagUi, Icons, Mode, Shelves};
use crate::loot::grid::Grid;
use crate::profile::Profile;
use crate::style;

/// Where the hideout was left for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Leave {
    Back,
    Play,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tab {
    Stash,
    Trader,
    Perks,
}

pub struct Hideout {
    grids: BagUi,
    /// Trading: the bag, the stash and what's to be sold, and that.
    trade_grids: BagUi,
    sell: Grid,
    tab: Tab,
    /// A word flashed at the top ("MAKE ROOM IN THE STASH"), and till when.
    note: Option<(String, f64)>,
}

impl Default for Hideout {
    fn default() -> Self {
        Self { grids: BagUi::new(Mode::Hideout), trade_grids: BagUi::new(Mode::Trade), sell: trader::sell_box(), tab: Tab::Stash, note: None }
    }
}

impl Hideout {
    /// Write the weapon slots' keys (the player's) over them.
    pub fn set_slot_keys(&mut self, keys: [String; 3]) {
        self.grids.slot_keys = keys;
    }

    /// Put down whatever is held, and put back what was to be sold
    /// (leaving the page, or the hideout).
    pub fn let_go(&mut self, profile: &mut Profile) {
        let mut shelves = Shelves { bag: &mut profile.loadout, loot: Some(("STASH", &mut profile.stash)), sell: None };
        self.grids.let_go(&mut shelves);
        let mut shelves = Shelves { bag: &mut profile.loadout, loot: Some(("STASH", &mut profile.stash)), sell: Some(&mut self.sell) };
        self.trade_grids.let_go(&mut shelves);
        trader::put_back(profile, &mut self.sell);
    }

    /// A frame of it, `active` unless a fade is running. Where it's left
    /// for, once it is.
    pub fn frame(&mut self, ui: &mut Ui, profile: &mut Profile, icons: &Icons, active: bool) -> Option<Leave> {
        let s = ui.m.scale;
        let screen = ui.clip();
        ui.draw.rect(screen, Color::rgba(0.02, 0.02, 0.02, 0.72));
        // The heading and the level, across the top.
        let heading = TextStyle::new((60.0 * s) as f32).bold().family(style::FONT);
        let top = screen.min.y + screen.height() * 0.035;
        let left = screen.min.x + 80.0 * s;
        ui.text_at("HIDEOUT", &heading, Vec2::new(left, top), screen.width(), style::BONE);
        let hw = ui.measure("HIDEOUT", &heading);
        ui.draw.rect(Rect::from_min_size(Vec2::new(left + 4.0 * s, top + f64::from(heading.line_height()) + 4.0 * s), Vec2::new(160.0 * s, 5.0 * s)), style::SIGNAL);
        crate::levelbar::draw(ui, Vec2::new(left + hw + 80.0 * s, top + 8.0 * s), 560.0 * s, profile.xp, 0, 1.0, 1.0);
        // The money, and what the stash is worth.
        let money = trader::dollars(profile.money);
        let big = TextStyle::new((44.0 * s) as f32).bold().family(style::FONT);
        let mw = ui.measure(&money, &big);
        ui.text_at(&money, &big, Vec2::new(screen.max.x - 80.0 * s - mw, top), mw + 4.0, trader::GOLD);
        let stash_worth = format!("STASH WORTH {}", trader::dollars(profile.stash.value()));
        let st = TextStyle::new((24.0 * s) as f32).bold().family(style::FONT);
        let sw = ui.measure(&stash_worth, &st);
        ui.text_at(&stash_worth, &st, Vec2::new(screen.max.x - 80.0 * s - sw, top + f64::from(big.line_height()) + 4.0 * s), sw + 4.0, style::DIM);

        // The tabs.
        let tab_style = TextStyle::new((34.0 * s) as f32).bold().family(style::FONT);
        let mut tx = left;
        let tab_y = screen.min.y + screen.height() * 0.105;
        let points = profile.points_left();
        for (tab, label) in [(Tab::Stash, "STASH".to_string()), (Tab::Trader, "TRADER".to_string()), (Tab::Perks, if points > 0 { format!("PERKS ({points})") } else { "PERKS".to_string() })] {
            let w = ui.measure(&label, &tab_style) + 50.0 * s;
            let r = Rect::from_min_size(Vec2::new(tx, tab_y), Vec2::new(w, f64::from(tab_style.line_height()) + 18.0 * s));
            let id = match tab {
                Tab::Stash => "tab stash",
                Tab::Trader => "tab trader",
                Tab::Perks => "tab perks",
            };
            let hit = ui.interact(ui.id(id), r, Sense::CLICK);
            let on = self.tab == tab;
            ui.draw.rect(r, if on { Color::rgba(1.0, 1.0, 1.0, 0.14) } else if active && hit.hovered { Color::rgba(1.0, 1.0, 1.0, 0.08) } else { Color::rgba(0.0, 0.0, 0.0, 0.3) });
            if on {
                ui.draw.rect(Rect::from_min_size(Vec2::new(r.min.x, r.max.y - 4.0 * s), Vec2::new(r.width(), 4.0 * s)), style::SIGNAL);
            }
            ui.text_at(&label, &tab_style, Vec2::new(r.min.x + 25.0 * s, r.min.y + 9.0 * s), w, if on { style::BONE } else { style::DIM });
            if active && hit.clicked && !on {
                self.let_go(profile);
                self.tab = tab;
            }
            tx += w + 16.0 * s;
        }
        if let Some((note, until)) = &self.note {
            if ui.now() < *until {
                let nw = ui.measure(note, &tab_style);
                ui.text_at(note, &tab_style, Vec2::new(screen.max.x - 80.0 * s - nw, tab_y + 9.0 * s), nw + 4.0, style::SIGNAL);
            } else {
                self.note = None;
            }
        }

        let mut leave = None;
        match self.tab {
            Tab::Stash => {
                let mut shelves = Shelves { bag: &mut profile.loadout, loot: Some(("STASH", &mut profile.stash)), sell: None };
                self.grids.frame(ui, &mut shelves, icons);
            }
            Tab::Trader => {
                if let Some(note) = trader::page(ui, profile, &mut self.sell, &mut self.trade_grids, icons, active) {
                    self.note = Some((note, ui.now() + 2.5));
                }
            }
            Tab::Perks => {
                if let Some(note) = perks::page(ui, profile, active) {
                    self.note = Some((note.to_string(), ui.now() + 2.5));
                }
            }
        }
        // The way on: two big buttons, bottom right.
        let item = TextStyle::new((50.0 * s) as f32).bold().family(style::FONT);
        let h = f64::from(item.line_height()) + 24.0 * s;
        let mut x = screen.max.x - 80.0 * s;
        for (label, choice, primary) in [("PLAY", Leave::Play, true), ("BACK", Leave::Back, false)] {
            let w = ui.measure(label, &item) + 80.0 * s;
            x -= w;
            let r = Rect::from_min_size(Vec2::new(x, screen.max.y - 60.0 * s - h), Vec2::new(w, h));
            let hit = ui.interact(ui.id(label), r, Sense::CLICK);
            let lit = active && hit.hovered;
            let fill = if primary { if lit { style::SIGNAL } else { Color::rgb(0.55, 0.10, 0.07) } } else if lit { Color::rgba(1.0, 1.0, 1.0, 0.18) } else { Color::rgba(1.0, 1.0, 1.0, 0.08) };
            ui.draw.rect(r, fill);
            ui.draw.stroke_rect(r, 2.0 * s, 0.0, if lit { style::BONE } else { style::DIM });
            ui.text_at(label, &item, Vec2::new(r.min.x + 40.0 * s, r.min.y + 12.0 * s), w, style::BONE);
            if active && hit.clicked {
                leave = Some(choice);
            }
            x -= 30.0 * s;
        }
        if active && leave.is_none() {
            if ui.state.take_key(|k| k.key == Key::Escape).is_some() {
                leave = Some(Leave::Back);
            } else if ui.state.take_key(|k| k.key == Key::Enter).is_some() {
                leave = Some(Leave::Play);
            }
        }
        if leave.is_some() {
            self.let_go(profile);
        }
        leave
    }
}
