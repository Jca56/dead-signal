//! The hideout, between runs: the stash on the left, the bag to take into
//! the next run on the right, things dragged (or right-clicked) between
//! them; the player's level over it all; and the way on (back to the
//! title, or straight into a run with what's packed).

use lntrn_math::{Color, Rect, Vec2};
use lntrn_text::TextStyle;
use lntrn_ui::{Key, Sense, Ui};

use crate::bag_ui::{BagUi, Icons, Shelves};
use crate::profile::Profile;
use crate::style;

/// Where the hideout was left for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Leave {
    Back,
    Play,
}

pub struct Hideout {
    grids: BagUi,
}

impl Default for Hideout {
    fn default() -> Self {
        Self { grids: BagUi::hideout() }
    }
}

impl Hideout {
    /// Put down whatever is held (leaving).
    pub fn let_go(&mut self, profile: &mut Profile) {
        let mut shelves = Shelves { bag: &mut profile.loadout, loot: Some(("STASH", &mut profile.stash)) };
        self.grids.let_go(&mut shelves);
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
        let stash_worth = format!("STASH ${}", profile.stash.value());
        let st = TextStyle::new((28.0 * s) as f32).bold().family(style::FONT);
        let sw = ui.measure(&stash_worth, &st);
        ui.text_at(&stash_worth, &st, Vec2::new(screen.max.x - 80.0 * s - sw, top + 16.0 * s), sw + 4.0, style::DIM);

        let mut leave = None;
        {
            let mut shelves = Shelves { bag: &mut profile.loadout, loot: Some(("STASH", &mut profile.stash)) };
            self.grids.frame(ui, &mut shelves, icons);
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
