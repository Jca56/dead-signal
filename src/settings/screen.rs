//! The settings screen, over the title or a paused run: tabs across the
//! top (controls, video, audio, HUD), a row a setting (its name, a word on
//! what it does, and a slider or an ON/OFF button), and the way back.
//! Changes take at once; the caller saves them on the way out. A slider is
//! dragged, clicked along, or turned with the wheel a step at a time.

use lntrn_math::{Color, Rect, Vec2};
use lntrn_text::TextStyle;
use lntrn_ui::{Key, Sense, Ui};

use super::{Field, Range, Settings};
use crate::hideout::pressed;
use crate::style;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tab {
    Controls,
    Video,
    Audio,
    Hud,
}

impl Tab {
    const ALL: [Tab; 4] = [Tab::Controls, Tab::Video, Tab::Audio, Tab::Hud];

    fn label(self) -> &'static str {
        match self {
            Tab::Controls => "CONTROLS",
            Tab::Video => "VIDEO",
            Tab::Audio => "AUDIO",
            Tab::Hud => "HUD",
        }
    }

    fn fields(self) -> &'static [Field] {
        match self {
            Tab::Controls => &[Field::Sensitivity, Field::AdsSensitivity],
            Tab::Video => &[Field::Fov, Field::Fullscreen, Field::Vsync],
            Tab::Audio => &[Field::Master, Field::Music, Field::Effects, Field::Zombies, Field::MusicInRuns],
            Tab::Hud => &[Field::Crosshair, Field::HeadBob, Field::UiScale],
        }
    }
}

/// A row's height, logical pixels; the slider's length and its knob.
const ROW: f64 = 118.0;
const TRACK: f64 = 620.0;
const KNOB: f64 = 34.0;

pub struct SettingsScreen {
    tab: Tab,
    /// The slider being dragged.
    held: Option<Field>,
}

impl Default for SettingsScreen {
    fn default() -> Self {
        Self { tab: Tab::Controls, held: None }
    }
}

impl SettingsScreen {
    /// Whether `field`'s slider is being dragged right now.
    pub fn holding(&self, field: Field) -> bool {
        self.held == Some(field)
    }

    /// A frame of it, `active` unless a fade is running. Whether it was
    /// closed (BACK, or Esc).
    pub fn frame(&mut self, ui: &mut Ui, settings: &mut Settings, active: bool) -> bool {
        let s = ui.m.scale;
        let screen = ui.clip();
        ui.draw.rect(screen, Color::rgba(0.02, 0.02, 0.02, 0.82));
        let heading = TextStyle::new((60.0 * s) as f32).bold().family(style::FONT);
        let top = screen.min.y + screen.height() * 0.035;
        let left = screen.min.x + 80.0 * s;
        ui.text_at("SETTINGS", &heading, Vec2::new(left, top), screen.width(), style::BONE);
        ui.draw.rect(Rect::from_min_size(Vec2::new(left + 4.0 * s, top + f64::from(heading.line_height()) + 4.0 * s), Vec2::new(160.0 * s, 5.0 * s)), style::SIGNAL);

        // The tabs.
        let tab_style = TextStyle::new((34.0 * s) as f32).bold().family(style::FONT);
        let mut tx = left;
        let tab_y = screen.min.y + screen.height() * 0.105;
        let tab_h = f64::from(tab_style.line_height()) + 18.0 * s;
        for tab in Tab::ALL {
            let label = tab.label();
            let w = ui.measure(label, &tab_style) + 50.0 * s;
            let r = Rect::from_min_size(Vec2::new(tx, tab_y), Vec2::new(w, tab_h));
            let hit = ui.interact(ui.id(label), r, Sense::CLICK);
            let on = self.tab == tab;
            ui.draw.rect(r, if on { Color::rgba(1.0, 1.0, 1.0, 0.14) } else if active && hit.hovered { Color::rgba(1.0, 1.0, 1.0, 0.08) } else { Color::rgba(0.0, 0.0, 0.0, 0.3) });
            if on {
                ui.draw.rect(Rect::from_min_size(Vec2::new(r.min.x, r.max.y - 4.0 * s), Vec2::new(r.width(), 4.0 * s)), style::SIGNAL);
            }
            ui.text_at(label, &tab_style, Vec2::new(r.min.x + 25.0 * s, r.min.y + 9.0 * s), w, if on { style::BONE } else { style::DIM });
            if active && hit.clicked {
                self.tab = tab;
                self.held = None;
            }
            tx += w + 16.0 * s;
        }

        // A row a setting.
        let name = TextStyle::new((34.0 * s) as f32).bold().family(style::FONT);
        let hint = TextStyle::new((22.0 * s) as f32).family(style::FONT);
        let value_style = TextStyle::new((32.0 * s) as f32).bold().family(style::FONT);
        let control_x = left + 760.0 * s;
        let mut y = tab_y + tab_h + 50.0 * s;
        if !ui.state.down {
            self.held = None;
        }
        for &field in self.tab.fields() {
            let row = Rect::from_min_size(Vec2::new(left, y), Vec2::new(screen.max.x - 80.0 * s - left, (ROW - 14.0) * s));
            ui.draw.rect(row, Color::rgba(0.05, 0.05, 0.05, 0.7));
            ui.text_at(field.label(), &name, Vec2::new(row.min.x + 24.0 * s, row.min.y + 16.0 * s), control_x - left, style::BONE);
            ui.text_at(field.hint(), &hint, Vec2::new(row.min.x + 24.0 * s, row.min.y + 24.0 * s + f64::from(name.line_height())), control_x - left - 40.0 * s, style::DIM);
            let mid = row.center().y;
            let value = settings.get(field);
            match field.range() {
                Range::Toggle => {
                    let label = field.show(value);
                    let r = Rect::from_min_size(Vec2::new(control_x, mid - 32.0 * s), Vec2::new(180.0 * s, 64.0 * s));
                    if pressed(ui, field.key(), r, &label, &value_style, true, value > 0.5, active) {
                        settings.set(field, 1.0 - value);
                    }
                }
                Range::Slider { min, max, step, .. } => {
                    let track = Rect::from_min_size(Vec2::new(control_x, mid - 4.0 * s), Vec2::new(TRACK * s, 8.0 * s));
                    let grab = Rect::from_min_size(Vec2::new(track.min.x - KNOB * s, mid - 36.0 * s), Vec2::new(track.width() + 2.0 * KNOB * s, 72.0 * s));
                    let hit = ui.interact(ui.id(field.key()), grab, Sense::DRAG);
                    if active && hit.pressed {
                        self.held = Some(field);
                    }
                    if active && self.held == Some(field) {
                        let t = ((ui.state.pointer.x - track.min.x) / track.width()).clamp(0.0, 1.0);
                        settings.set(field, min + t * (max - min));
                    } else if active && hit.hovered && ui.state.wheel.y != 0.0 {
                        settings.set(field, value + step * ui.state.wheel.y.signum());
                    }
                    let t = (settings.get(field) - min) / (max - min);
                    let lit = active && (hit.hovered || self.held == Some(field));
                    ui.draw.rect(track, Color::rgba(1.0, 1.0, 1.0, 0.15));
                    ui.draw.rect(Rect::from_min_size(track.min, Vec2::new(track.width() * t, track.height())), style::SIGNAL);
                    let knob = Rect::from_center_size(Vec2::new(track.min.x + track.width() * t, mid), Vec2::splat(KNOB * s));
                    ui.draw.rect(knob, if lit { style::BONE } else { Color::rgb(0.75, 0.73, 0.68) });
                    ui.draw.stroke_rect(knob, 2.0 * s, 0.0, Color::rgba(0.0, 0.0, 0.0, 0.6));
                    let shown = field.show(settings.get(field));
                    ui.text_at(&shown, &value_style, Vec2::new(track.max.x + 40.0 * s, mid - f64::from(value_style.line_height()) * 0.5), 200.0 * s, style::BONE);
                }
            }
            y += ROW * s;
        }

        // Defaults for this tab, and the way back.
        let item = TextStyle::new((50.0 * s) as f32).bold().family(style::FONT);
        let small = TextStyle::new((30.0 * s) as f32).bold().family(style::FONT);
        let h = f64::from(item.line_height()) + 24.0 * s;
        let bottom = screen.max.y - 60.0 * s - h;
        let back_w = ui.measure("BACK", &item) + 80.0 * s;
        let back = Rect::from_min_size(Vec2::new(screen.max.x - 80.0 * s - back_w, bottom), Vec2::new(back_w, h));
        let reset_label = format!("RESET {} TO DEFAULTS", self.tab.label());
        let reset_w = ui.measure(&reset_label, &small) + 40.0 * s;
        let reset = Rect::from_min_size(Vec2::new(left, bottom + (h - 64.0 * s) * 0.5), Vec2::new(reset_w, 64.0 * s));
        if pressed(ui, "reset tab", reset, &reset_label, &small, true, false, active) {
            let d = Settings::default();
            for &f in self.tab.fields() {
                settings.set(f, d.get(f));
            }
        }
        let mut closed = back_button(ui, back, &item, active);
        if active && ui.state.take_key(|k| k.key == Key::Escape).is_some() {
            closed = true;
        }
        if closed {
            self.held = None;
        }
        closed
    }
}

/// The big BACK button, bottom right (as the hideout's).
fn back_button(ui: &mut Ui, r: Rect, item: &TextStyle, active: bool) -> bool {
    let s = ui.m.scale;
    let hit = ui.interact(ui.id("settings back"), r, Sense::CLICK);
    let lit = active && hit.hovered;
    ui.draw.rect(r, if lit { Color::rgba(1.0, 1.0, 1.0, 0.18) } else { Color::rgba(1.0, 1.0, 1.0, 0.08) });
    ui.draw.stroke_rect(r, 2.0 * s, 0.0, if lit { style::BONE } else { style::DIM });
    ui.text_at("BACK", item, Vec2::new(r.min.x + 40.0 * s, r.min.y + 12.0 * s), r.width(), style::BONE);
    active && hit.clicked
}
