//! The title screen: the name and a menu down the left, over the scene.
//! Mouse or keys (W/S, arrows, Enter) both work.

use lntrn_math::{Color, Rect, Vec2};
use lntrn_text::TextStyle;
use lntrn_ui::{Key, Sense, Ui};

use crate::style;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    Play,
    Quit,
}

const ITEMS: [(&str, Choice); 2] = [("PLAY", Choice::Play), ("QUIT", Choice::Quit)];

#[derive(Default)]
pub struct TitleMenu {
    selected: usize,
}

impl TitleMenu {
    /// Draw the screen; the choice made this frame, if any. `active` is
    /// false while a fade is running, so nothing can be picked twice.
    pub fn draw(&mut self, ui: &mut Ui, active: bool) -> Option<Choice> {
        let s = ui.m.scale;
        let screen = ui.clip();
        shade_left(ui, screen);

        let left = screen.min.x + 120.0 * s;
        let title = TextStyle::new((100.0 * s) as f32).bold().family(style::FONT);
        let title_h = f64::from(title.line_height());
        let mut y = screen.min.y + screen.height() * 0.30;
        ui.text_at("DEAD SIGNAL", &title, Vec2::new(left, y), screen.width(), style::BONE);
        y += title_h + 10.0 * s;
        ui.draw.rect(Rect::from_min_size(Vec2::new(left + 5.0 * s, y), Vec2::new(200.0 * s, 5.0 * s)), style::SIGNAL);
        y += 60.0 * s;

        let item = TextStyle::new((50.0 * s) as f32).bold().family(style::FONT);
        let item_h = f64::from(item.line_height());
        let mut chosen = None;
        for (i, (label, choice)) in ITEMS.iter().enumerate() {
            let w = ui.measure(label, &item) + 60.0 * s;
            let rect = Rect::from_min_size(Vec2::new(left - 40.0 * s, y), Vec2::new(w + 40.0 * s, item_h));
            let r = ui.interact(ui.id(label), rect, Sense::CLICK);
            if active && r.hovered {
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
                self.selected = (self.selected + ITEMS.len() - 1) % ITEMS.len();
            }
            if down {
                self.selected = (self.selected + 1) % ITEMS.len();
            }
            if ui.state.take_key(|k| matches!(k.key, Key::Enter | Key::Char(' '))).is_some() {
                chosen = Some(ITEMS[self.selected].1);
            }
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
