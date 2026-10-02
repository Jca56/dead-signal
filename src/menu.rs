//! A menu down the left of the screen, over the scene: a heading, a red
//! rule, a word of warning if there is one, and the items. The title
//! screen and the pause screen are both one. Mouse or keys (W/S, arrows,
//! Enter) both work: the pointer picks an item out as it moves over it
//! (one left lying on an item doesn't hold the keys', or a pad's, there).

use lntrn_math::{Color, Rect, Vec2};
use lntrn_text::TextStyle;
use lntrn_ui::{Key, Sense, Ui};

use crate::{feedback, style};

/// A menu of `T` choices under a heading.
pub struct SideMenu<T: Copy + 'static> {
    heading: &'static str,
    items: &'static [(&'static str, T)],
    /// Lines under the rule, in red.
    warning: &'static [&'static str],
    /// A line under the rule, in bone (who's playing, say).
    pub subtitle: Option<String>,
    selected: usize,
    /// Where the pointer was last frame.
    pointer: Option<Vec2>,
}

impl<T: Copy + 'static> SideMenu<T> {
    pub fn new(heading: &'static str, items: &'static [(&'static str, T)]) -> Self {
        Self { heading, items, warning: &[], subtitle: None, selected: 0, pointer: None }
    }

    /// The same, with `lines` of warning over the items.
    pub fn warning(self, lines: &'static [&'static str]) -> Self {
        Self { warning: lines, ..self }
    }

    /// Put the highlight back on the first item.
    pub fn reset(&mut self) {
        self.selected = 0;
    }

    /// Draw the menu; the choice made this frame, if any. `active` is
    /// false while a fade is running, so nothing can be picked twice.
    pub fn draw(&mut self, ui: &mut Ui, active: bool) -> Option<T> {
        let s = ui.m.scale;
        let screen = ui.clip();
        shade_left(ui, screen);

        let left = screen.min.x + 120.0 * s;
        let title = TextStyle::new((100.0 * s) as f32).bold().family(style::FONT);
        let title_h = f64::from(title.line_height());
        let mut y = screen.min.y + screen.height() * 0.30;
        ui.text_at(self.heading, &title, Vec2::new(left, y), screen.width(), style::BONE);
        y += title_h + 10.0 * s;
        ui.draw.rect(Rect::from_min_size(Vec2::new(left + 5.0 * s, y), Vec2::new(200.0 * s, 5.0 * s)), style::SIGNAL);
        y += 60.0 * s;
        let warning = TextStyle::new((36.0 * s) as f32).bold().family(style::FONT);
        if let Some(line) = &self.subtitle {
            ui.text_at(line, &warning, Vec2::new(left, y), screen.width(), style::BONE);
            y += f64::from(warning.line_height()) + 40.0 * s;
        }
        for line in self.warning {
            ui.text_at(line, &warning, Vec2::new(left, y), screen.width(), style::SIGNAL);
            y += f64::from(warning.line_height()) + 6.0 * s;
        }
        if !self.warning.is_empty() {
            y += 40.0 * s;
        }

        let item = TextStyle::new((50.0 * s) as f32).bold().family(style::FONT);
        let item_h = f64::from(item.line_height());
        let mut chosen = None;
        let was = self.selected;
        let moved = self.pointer.is_some_and(|p| (ui.state.pointer - p).length() > 0.5);
        self.pointer = Some(ui.state.pointer);
        for (i, (label, choice)) in self.items.iter().enumerate() {
            let w = ui.measure(label, &item) + 60.0 * s;
            let rect = Rect::from_min_size(Vec2::new(left - 40.0 * s, y), Vec2::new(w + 40.0 * s, item_h));
            let r = ui.interact(ui.id(label), rect, Sense::CLICK);
            if active && r.hovered && moved {
                self.selected = i;
            }
            if active && r.clicked {
                chosen = Some(*choice);
            }
            let on = self.selected == i;
            if on {
                ui.draw.rect(Rect::from_min_size(Vec2::new(left - 30.0 * s, y + item_h * 0.2), Vec2::new(10.0 * s, item_h * 0.6)), style::SIGNAL);
            }
            ui.text_at(label, &item, Vec2::new(left, y), screen.width(), if on { style::BONE } else { style::DIM });
            y += item_h + 20.0 * s;
        }

        if active {
            let up = ui.state.take_key(|k| matches!(k.key, Key::ArrowUp | Key::Char('w' | 'W'))).is_some();
            let down = ui.state.take_key(|k| matches!(k.key, Key::ArrowDown | Key::Char('s' | 'S'))).is_some();
            if up {
                self.selected = (self.selected + self.items.len() - 1) % self.items.len();
            }
            if down {
                self.selected = (self.selected + 1) % self.items.len();
            }
            if ui.state.take_key(|k| matches!(k.key, Key::Enter | Key::Char(' '))).is_some() {
                chosen = Some(self.items[self.selected].1);
            }
        }
        if chosen.is_some() {
            feedback::click();
        } else if self.selected != was {
            feedback::tick();
        }
        chosen
    }
}

/// Darken the left of the screen so the words read over any sky: thin
/// strips, edge to edge, each a shade lighter than the last.
fn shade_left(ui: &mut Ui, screen: Rect) {
    let width = screen.width() * 0.55;
    let strips = (width / 4.0).ceil().max(1.0) as usize;
    let mut x = screen.min.x;
    for k in 0..strips {
        let t = (k as f64 + 0.5) / strips as f64;
        let alpha = 0.6 * (1.0 - t) * (1.0 - t);
        let next = (screen.min.x + width * (k + 1) as f64 / strips as f64).round();
        ui.draw.rect(Rect::new(Vec2::new(x, screen.min.y), Vec2::new(next, screen.max.y)), Color::rgba(0.0, 0.0, 0.0, alpha));
        x = next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lntrn_ui::testing::Harness;

    #[test]
    fn a_pointer_left_lying_on_an_item_doesnt_hold_the_keys_there() {
        let mut h = Harness::new(1920.0, 1080.0);
        let mut menu = SideMenu::new("PAUSED", &[("RESUME", 0), ("SETTINGS", 1), ("QUIT", 2)]);
        h.frame(|ui| {
            menu.draw(ui, true);
        });
        // The pointer moved onto the last item picks it out.
        h.move_to(h.rect_of_label("QUIT").expect("the item").center());
        h.frame(|ui| {
            menu.draw(ui, true);
        });
        assert_eq!(menu.selected, 2);
        // Lying there, the keys (a pad's d-pad) move off it, and Enter (a
        // pad's A) picks what they moved to.
        h.key(Key::ArrowUp);
        h.frame(|ui| {
            menu.draw(ui, true);
        });
        h.key(Key::Enter);
        let mut chosen = None;
        h.frame(|ui| chosen = menu.draw(ui, true));
        assert_eq!((menu.selected, chosen), (1, Some(1)));
        // A click on it still picks it.
        let mut clicked = None;
        h.click_at(h.rect_of_label("QUIT").expect("the item").center(), |ui| clicked = clicked.or(menu.draw(ui, true)));
        assert_eq!(clicked, Some(2));
    }
}
