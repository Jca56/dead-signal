//! The holdout's lobby: who's playing, and with what. Whoever picked
//! HOLDOUT is the first player; a second joins with a press on something
//! of their own (A on a pad, or Enter or Space on the keyboard), and
//! leaves again with B (or Esc). START (or A, or Enter, from anyone in)
//! begins, and one alone plays with everything, as ever. BACK (or the
//! first player's B or Esc) goes back to the title.

use lntrn_math::{Color, Rect, Vec2};
use lntrn_sys::gamepad::Button;
use lntrn_text::TextStyle;
use lntrn_ui::{Key, Sense, Ui};

use crate::input::Device;
use crate::input::pad::{Control, Pads};
use crate::{feedback, style};

/// The most who can play at once.
pub const MOST: usize = 2;

/// What the lobby came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Went {
    /// Begin, with each player's device, by seat.
    Start(Vec<Device>),
    Back,
}

pub struct Lobby {
    /// Who's in, by seat: what each plays with.
    seats: Vec<Device>,
}

impl Lobby {
    /// A lobby with the one who opened it (with `first`) in.
    pub fn new(first: Device) -> Self {
        Self { seats: vec![first] }
    }

    /// Who's playing and with what, as it begins: one alone plays with
    /// everything.
    fn players(&self) -> Vec<Device> {
        if self.seats.len() == 1 { vec![Device::All] } else { self.seats.clone() }
    }

    /// `device` joins, if there's room and it isn't in yet.
    fn join(&mut self, device: Device) {
        if self.seats.len() < MOST && !self.seats.contains(&device) {
            self.seats.push(device);
            feedback::click();
        }
    }

    /// Whoever's at `seat` presses cancel: the first goes back, the
    /// others leave.
    fn cancel(&mut self, seat: usize) -> Option<Went> {
        if seat == 0 {
            return Some(Went::Back);
        }
        self.seats.remove(seat);
        feedback::click();
        None
    }

    /// The pads and the keyboard this frame: joining, leaving, beginning.
    fn heard(&mut self, ui: &mut Ui, pads: &Pads) -> Option<Went> {
        let mut went = None;
        for id in pads.ids() {
            let pad = pads.frame(id);
            let down = |b| pad.went_down(Control::Button(b));
            match self.seats.iter().position(|&d| d == Device::Pad(id)) {
                None if down(Button::South) || down(Button::Start) => self.join(Device::Pad(id)),
                Some(_) if down(Button::South) || down(Button::Start) => went = Some(Went::Start(self.players())),
                Some(seat) if down(Button::East) => went = went.or(self.cancel(seat)),
                _ => {}
            }
        }
        let confirm = ui.state.take_key(|k| !k.repeat && matches!(k.key, Key::Enter | Key::Space | Key::Char(' '))).is_some();
        let cancel = ui.state.take_key(|k| k.key == Key::Escape).is_some();
        let keys = self.seats.iter().position(|&d| d == Device::Keys);
        if confirm {
            match keys {
                None => self.join(Device::Keys),
                Some(_) => went = Some(Went::Start(self.players())),
            }
        }
        if cancel {
            // (From the keyboard, not in: someone at it wants out.)
            went = went.or(self.cancel(keys.unwrap_or(0)));
        }
        went
    }

    /// A frame of the lobby over the title's scene. What it came to, once
    /// it has. `active` is false while a fade is running.
    pub fn frame(&mut self, ui: &mut Ui, pads: &Pads, active: bool) -> Option<Went> {
        let went = if active { self.heard(ui, pads) } else { None };
        let s = ui.m.scale;
        let screen = ui.clip();
        ui.draw.rect(screen, Color::rgba(0.02, 0.02, 0.02, 0.78));
        let left = screen.min.x + 120.0 * s;
        let heading = TextStyle::new((100.0 * s) as f32).bold().family(style::FONT);
        let mut y = screen.min.y + screen.height() * 0.12;
        ui.text_at("HOLDOUT", &heading, Vec2::new(left, y), screen.width(), style::BONE);
        y += f64::from(heading.line_height()) + 10.0 * s;
        ui.draw.rect(Rect::from_min_size(Vec2::new(left + 5.0 * s, y), Vec2::new(200.0 * s, 5.0 * s)), style::SIGNAL);
        y += 40.0 * s;
        let sub = TextStyle::new((36.0 * s) as f32).bold().family(style::FONT);
        ui.text_at("WHO'S PLAYING?", &sub, Vec2::new(left, y), screen.width(), style::DIM);
        y += f64::from(sub.line_height()) + 50.0 * s;

        // A card a seat, the empty one waiting.
        let card = Vec2::new(560.0 * s, 300.0 * s);
        let gap = 60.0 * s;
        for seat in 0..MOST {
            let at = Vec2::new(left + seat as f64 * (card.x + gap), y);
            self.card(ui, Rect::from_min_size(at, card), seat, pads);
        }
        y += card.y + 70.0 * s;

        // START and BACK, for the mouse.
        let item = TextStyle::new((50.0 * s) as f32).bold().family(style::FONT);
        let mut chosen = None;
        let mut x = left;
        for (label, what) in [("START", Went::Start(self.players())), ("BACK", Went::Back)] {
            let size = Vec2::new(ui.measure(label, &item) + 80.0 * s, f64::from(item.line_height()) + 30.0 * s);
            let r = Rect::from_min_size(Vec2::new(x, y), size);
            let hit = ui.interact(ui.id(label), r, Sense::CLICK);
            let lit = active && hit.hovered;
            ui.draw.rect(r, if lit { Color::rgba(1.0, 1.0, 1.0, 0.16) } else { Color::rgba(0.0, 0.0, 0.0, 0.45) });
            ui.draw.stroke_rect(r, 3.0 * s, 0.0, if label == "START" { style::SIGNAL } else { style::DIM });
            ui.text_at(label, &item, Vec2::new(r.min.x + 40.0 * s, r.min.y + 15.0 * s), size.x, if lit { style::BONE } else { style::DIM });
            if feedback::button(label, lit, active && hit.clicked) {
                chosen = Some(what);
            }
            x += size.x + 40.0 * s;
        }
        went.or(chosen)
    }

    /// Seat `seat`'s card: who, with what, and what to press.
    fn card(&self, ui: &mut Ui, r: Rect, seat: usize, pads: &Pads) {
        let s = ui.m.scale;
        let colour = style::player(seat);
        let name = TextStyle::new((48.0 * s) as f32).bold().family(style::FONT);
        let line = TextStyle::new((32.0 * s) as f32).bold().family(style::FONT);
        let hint = TextStyle::new((26.0 * s) as f32).bold().family(style::FONT);
        let pad = 30.0 * s;
        let width = r.width() - 2.0 * pad;
        ui.draw.rect(r, Color::rgba(0.05, 0.05, 0.05, 0.85));
        ui.draw.rect(Rect::from_min_size(r.min, Vec2::new(r.width(), 10.0 * s)), colour);
        let mut y = r.min.y + 40.0 * s;
        ui.text_at(&format!("PLAYER {}", seat + 1), &name, Vec2::new(r.min.x + pad, y), width, colour);
        y += f64::from(name.line_height()) + 24.0 * s;
        match self.seats.get(seat) {
            Some(&device) => {
                let with = match device {
                    Device::Keys => "KEYBOARD & MOUSE".to_string(),
                    Device::Pad(id) => pads.name(id),
                    Device::All => "EVERYTHING".to_string(),
                };
                ui.text_at(&with, &line, Vec2::new(r.min.x + pad, y), width, style::BONE);
                y += f64::from(line.line_height()) + 20.0 * s;
                let what = if seat == 0 { "A / ENTER TO START" } else { "B / ESC TO LEAVE" };
                ui.text_at(what, &hint, Vec2::new(r.min.x + pad, y), width, style::DIM);
            }
            None => {
                ui.text_at("PRESS A TO JOIN", &line, Vec2::new(r.min.x + pad, y), width, style::BONE);
                y += f64::from(line.line_height()) + 20.0 * s;
                ui.text_at("OR ENTER ON THE KEYBOARD", &hint, Vec2::new(r.min.x + pad, y), width, style::DIM);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_alone_plays_with_everything_and_two_with_their_own() {
        let mut lobby = Lobby::new(Device::Pad(3));
        assert_eq!(lobby.players(), vec![Device::All]);
        lobby.join(Device::Keys);
        lobby.join(Device::Keys);
        lobby.join(Device::Pad(5));
        assert_eq!(lobby.players(), vec![Device::Pad(3), Device::Keys], "no one twice, and no more than two");
        assert_eq!(lobby.cancel(1), None, "the second leaves");
        assert_eq!(lobby.players(), vec![Device::All]);
        assert_eq!(lobby.cancel(0), Some(Went::Back), "the first goes back");
    }
}
