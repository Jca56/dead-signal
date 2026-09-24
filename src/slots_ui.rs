//! The save slots screen, off the title: a card a slot, side by side,
//! each its player at a glance (name, level, money, runs, what the stash
//! is worth) or EMPTY. A slot is played, renamed (typed right on its card:
//! Enter keeps it, Esc doesn't) or deleted (asked twice); an empty one
//! starts someone new, and asks their name. The app does what's asked
//! (`app/mod.rs`) and hands back how the slots look after.

use lntrn_math::{Color, Rect, Vec2};
use lntrn_text::TextStyle;
use lntrn_ui::{Key, Sense, Ui};

use crate::hideout::{GOLD, dollars, pressed};
use crate::profile::Profile;
use crate::profile::save::{DEV, NAME_MAX, clean_name};
use crate::{feedback, style};

/// A slot's player, at a glance.
#[derive(Clone, Debug, PartialEq)]
pub struct Summary {
    pub name: String,
    pub level: u32,
    pub money: u32,
    pub runs: u32,
    pub extractions: u32,
    pub worth: u32,
}

impl Summary {
    pub fn of(p: &Profile) -> Self {
        Self { name: p.name.clone(), level: crate::profile::xp::level(p.xp).0, money: p.money, runs: p.runs, extractions: p.extractions, worth: p.stash.value() }
    }
}

/// What's been asked of the slots.
#[derive(Clone, Debug, PartialEq)]
pub enum SlotEvent {
    /// Play this one (someone new, if it's empty).
    Chosen(u8),
    Renamed(u8, String),
    Deleted(u8),
    Closed,
}

pub struct SlotsScreen {
    /// Each slot shown (the DEV slot too, in developer mode), and its
    /// player, if it has one.
    cards: Vec<(u8, Option<Summary>)>,
    /// The slot being named, and the name so far.
    renaming: Option<(u8, String)>,
    /// The slot asked to be deleted, waiting to be sure.
    deleting: Option<u8>,
}

/// A card's height, and a button's, logical pixels.
const CARD_H: f64 = 620.0;
const BUTTON_H: f64 = 68.0;

impl SlotsScreen {
    pub fn new(cards: Vec<(u8, Option<Summary>)>) -> Self {
        Self { cards, renaming: None, deleting: None }
    }

    /// How the slots look now.
    pub fn refresh(&mut self, cards: Vec<(u8, Option<Summary>)>) {
        self.cards = cards;
    }

    /// Ask the name of `slot`'s player, starting from `name`.
    pub fn rename(&mut self, slot: u8, name: &str) {
        self.renaming = Some((slot, name.to_string()));
        self.deleting = None;
    }

    /// A frame of it, `current` the slot being played, `active` unless a
    /// fade is running. What was asked, if anything.
    pub fn frame(&mut self, ui: &mut Ui, current: u8, active: bool) -> Option<SlotEvent> {
        let s = ui.m.scale;
        let screen = ui.clip();
        ui.draw.rect(screen, Color::rgba(0.02, 0.02, 0.02, 0.82));
        let heading = TextStyle::new((60.0 * s) as f32).bold().family(style::FONT);
        let top = screen.min.y + screen.height() * 0.035;
        let left = screen.min.x + 80.0 * s;
        ui.text_at("SAVE SLOTS", &heading, Vec2::new(left, top), screen.width(), style::BONE);
        ui.draw.rect(Rect::from_min_size(Vec2::new(left + 4.0 * s, top + f64::from(heading.line_height()) + 4.0 * s), Vec2::new(160.0 * s, 5.0 * s)), style::SIGNAL);

        let mut event = None;
        if active && let Some(e) = self.type_name(ui) {
            event = Some(e);
        }
        let gap = 40.0 * s;
        let n = self.cards.len().max(1) as f64;
        let width = (screen.max.x - 80.0 * s - left - gap * (n - 1.0)) / n;
        let y = screen.min.y + screen.height() * 0.17;
        let slots: Vec<u8> = self.cards.iter().map(|(slot, _)| *slot).collect();
        for (i, slot) in slots.into_iter().enumerate() {
            let card = Rect::from_min_size(Vec2::new(left + i as f64 * (width + gap), y), Vec2::new(width, CARD_H * s));
            if let Some(e) = self.card(ui, slot, card, slot == current, active) {
                event = Some(e);
            }
        }

        // The way back.
        let item = TextStyle::new((50.0 * s) as f32).bold().family(style::FONT);
        let h = f64::from(item.line_height()) + 24.0 * s;
        let back_w = ui.measure("BACK", &item) + 80.0 * s;
        let back = Rect::from_min_size(Vec2::new(screen.max.x - 80.0 * s - back_w, screen.max.y - 60.0 * s - h), Vec2::new(back_w, h));
        let hit = ui.interact(ui.id("slots back"), back, Sense::CLICK);
        let lit = active && hit.hovered;
        ui.draw.rect(back, if lit { Color::rgba(1.0, 1.0, 1.0, 0.18) } else { Color::rgba(1.0, 1.0, 1.0, 0.08) });
        ui.draw.stroke_rect(back, 2.0 * s, 0.0, if lit { style::BONE } else { style::DIM });
        ui.text_at("BACK", &item, Vec2::new(back.min.x + 40.0 * s, back.min.y + 12.0 * s), back_w, style::BONE);
        if feedback::button("slots back", lit, active && hit.clicked) {
            event = Some(SlotEvent::Closed);
        }
        if active && self.renaming.is_none() && ui.state.take_key(|k| k.key == Key::Escape).is_some() {
            // Esc backs out of a delete first, then off the screen.
            if self.deleting.take().is_none() {
                event = Some(SlotEvent::Closed);
            }
        }
        event
    }

    /// Typing a name: letters and all go in, Backspace takes one off,
    /// Enter keeps it, Esc lets it go.
    fn type_name(&mut self, ui: &mut Ui) -> Option<SlotEvent> {
        let (slot, mut name) = self.renaming.take()?;
        while let Some(k) = ui.state.take_key(|_| true) {
            match k.key {
                Key::Escape => return None,
                Key::Enter => return Some(SlotEvent::Renamed(slot, clean_name(&name))),
                Key::Backspace => {
                    if name.pop().is_some() {
                        feedback::tick();
                    }
                }
                Key::Space | Key::Char(_) if name.chars().count() < NAME_MAX => {
                    let c = if let Key::Char(c) = k.key { c } else { ' ' };
                    if !c.is_control() {
                        name.push(c);
                        feedback::tick();
                    }
                }
                _ => {}
            }
        }
        self.renaming = Some((slot, name));
        None
    }

    /// One slot's card: who's in it and what can be done with it.
    fn card(&mut self, ui: &mut Ui, slot: u8, r: Rect, current: bool, active: bool) -> Option<SlotEvent> {
        let s = ui.m.scale;
        ui.draw.rect(r, Color::rgba(0.05, 0.05, 0.05, 0.85));
        ui.draw.stroke_rect(r, 2.0 * s, 0.0, if current { style::SIGNAL } else { Color::rgba(1.0, 1.0, 1.0, 0.15) });
        let small = TextStyle::new((26.0 * s) as f32).bold().family(style::FONT);
        let big = TextStyle::new((46.0 * s) as f32).bold().family(style::FONT);
        let line = TextStyle::new((30.0 * s) as f32).bold().family(style::FONT);
        let dim = TextStyle::new((26.0 * s) as f32).family(style::FONT);
        let button = TextStyle::new((30.0 * s) as f32).bold().family(style::FONT);
        let pad = 28.0 * s;
        let inner = r.width() - pad * 2.0;
        let mut y = r.min.y + pad;
        let (title, colour) = if slot == DEV { ("DEV SLOT · TEST TOOLS ON F1".to_string(), GOLD) } else { (format!("SLOT {slot}"), style::DIM) };
        ui.text_at(&title, &small, Vec2::new(r.min.x + pad, y), inner, colour);
        if current {
            let w = ui.measure("PLAYING", &small);
            ui.text_at("PLAYING", &small, Vec2::new(r.max.x - pad - w, y), w + 4.0, style::SIGNAL);
        }
        y += f64::from(small.line_height()) + 18.0 * s;

        // The name: typed into, while it's being named.
        let card = self.cards.iter().find(|(n, _)| *n == slot).and_then(|(_, c)| c.clone());
        let naming = self.renaming.as_ref().filter(|(n, _)| *n == slot).map(|(_, name)| name.clone());
        if let Some(name) = &naming {
            let field = Rect::from_min_size(Vec2::new(r.min.x + pad - 10.0 * s, y - 6.0 * s), Vec2::new(inner + 20.0 * s, f64::from(big.line_height()) + 12.0 * s));
            ui.draw.rect(field, Color::rgba(1.0, 1.0, 1.0, 0.1));
            ui.draw.stroke_rect(field, 2.0 * s, 0.0, style::BONE);
            let caret = if (ui.now() * 2.0).fract() < 0.5 { "|" } else { " " };
            ui.text_at(&format!("{name}{caret}"), &big, Vec2::new(r.min.x + pad, y), inner, style::BONE);
        } else {
            let unnamed = if slot == DEV { "DEV".to_string() } else { format!("SLOT {slot}") };
            let name = card.as_ref().map_or("EMPTY".to_string(), |c| if c.name.is_empty() { unnamed } else { c.name.clone() });
            ui.text_at(&name, &big, Vec2::new(r.min.x + pad, y), inner, if card.is_some() { style::BONE } else { style::DIM });
        }
        y += f64::from(big.line_height()) + 30.0 * s;
        if let Some(c) = &card {
            for (text, style_, colour) in [
                (format!("LEVEL {}", c.level), &line, style::BONE),
                (dollars(c.money), &line, GOLD),
                (format!("{} RUNS · {} EXTRACTED", c.runs, c.extractions), &dim, style::DIM),
                (format!("STASH WORTH {}", dollars(c.worth)), &dim, style::DIM),
            ] {
                ui.text_at(&text, style_, Vec2::new(r.min.x + pad, y), inner, colour);
                y += f64::from(style_.line_height()) + 14.0 * s;
            }
        } else if naming.is_none() {
            ui.text_at("Nobody here yet", &dim, Vec2::new(r.min.x + pad, y), inner, style::DIM);
        }

        // What can be done, down the bottom of the card.
        let bh = BUTTON_H * s;
        let at = |k: usize| Rect::from_min_size(Vec2::new(r.min.x + pad, r.max.y - pad - bh - k as f64 * (bh + 14.0 * s)), Vec2::new(inner, bh));
        let id = |what: &str| format!("slot {slot} {what}");
        if let Some(name) = naming {
            if pressed(ui, &id("keep name"), at(1), "KEEP NAME", &button, true, true, active) {
                self.renaming = None;
                return Some(SlotEvent::Renamed(slot, clean_name(&name)));
            }
            if pressed(ui, &id("cancel name"), at(0), "CANCEL", &button, true, false, active) {
                self.renaming = None;
            }
            return None;
        }
        if self.deleting == Some(slot) {
            ui.text_at("DELETE FOREVER?", &line, Vec2::new(r.min.x + pad, at(2).min.y - 10.0 * s), inner, style::SIGNAL);
            if pressed(ui, &id("delete forever"), at(1), "YES, DELETE", &button, true, true, active) {
                self.deleting = None;
                return Some(SlotEvent::Deleted(slot));
            }
            if pressed(ui, &id("keep"), at(0), "KEEP IT", &button, true, false, active) {
                self.deleting = None;
            }
            return None;
        }
        match card {
            None => {
                if pressed(ui, &id("new"), at(0), "NEW GAME", &button, true, true, active) {
                    self.rename(slot, "");
                    return Some(SlotEvent::Chosen(slot));
                }
            }
            Some(c) => {
                if !current && pressed(ui, &id("play"), at(2), "PLAY THIS SLOT", &button, true, true, active) {
                    return Some(SlotEvent::Chosen(slot));
                }
                if pressed(ui, &id("rename"), at(1), "RENAME", &button, true, false, active) {
                    self.rename(slot, &c.name);
                }
                if pressed(ui, &id("delete"), at(0), "DELETE", &button, true, false, active) {
                    self.deleting = Some(slot);
                    self.renaming = None;
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_summary_is_the_player_at_a_glance() {
        let mut p = Profile::new_player();
        p.name = "Alva".into();
        p.money = 2500;
        p.runs = 4;
        let c = Summary::of(&p);
        assert_eq!((c.name.as_str(), c.level, c.money, c.runs), ("Alva", 1, 2500, 4));
        assert!(c.worth > 0, "the starting stash is worth something");
    }
}
