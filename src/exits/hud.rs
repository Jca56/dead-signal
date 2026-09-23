//! The ways out on the HUD: the run's clock at the top, under it a compass
//! strip with every way out found on it (and how far), under that a bar of
//! how long is left while one is under way, and the radio's chatter.

use lntrn_math::{Color, Rect, Vec2};
use lntrn_text::TextStyle;
use lntrn_ui::Ui;

use crate::style;

/// The radio's green, for its words.
pub const SIGNAL_GREEN: Color = Color::rgb(0.42, 0.80, 0.46);
/// How far either side of straight ahead the strip shows, radians.
const SPAN: f64 = std::f64::consts::FRAC_PI_2;

/// A way out on the strip: its name, which way it lies (a yaw, as the
/// view's), how far, and whether it's open.
pub struct Mark {
    pub name: &'static str,
    pub yaw: f64,
    pub distance: f64,
    pub open: bool,
}

/// One under way: what it says, how far along (0–1), seconds left, and
/// whether it's slipping back (the player out of its zone).
pub struct Under {
    pub label: &'static str,
    pub done: f64,
    pub left: f64,
    pub slipping: bool,
}

pub struct Compass<'a> {
    /// Which way the view looks (its yaw).
    pub heading: f64,
    pub marks: &'a [Mark],
    pub clock: f64,
    pub surging: bool,
    pub under: Option<Under>,
    /// The radio's words, and how clearly (0–1).
    pub chatter: Option<(&'a str, f64)>,
    /// A shout under the strip ("THEY'RE SURGING"), and how clearly.
    pub shout: Option<(&'a str, f64)>,
}

/// `a` − `b` as an angle, −π to π.
fn off(a: f64, b: f64) -> f64 {
    (a - b + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU) - std::f64::consts::PI
}

fn faded(c: Color, a: f64) -> Color {
    Color::rgba(c.r, c.g, c.b, c.a * a)
}

pub fn draw(ui: &mut Ui, c: &Compass) {
    let s = ui.m.scale;
    let screen = ui.clip();
    let mid = screen.center().x;
    let small = TextStyle::new((22.0 * s) as f32).bold().family(style::FONT);
    let big = TextStyle::new((30.0 * s) as f32).bold().family(style::FONT);

    // The clock.
    let clock = format!("{}:{:02}", (c.clock / 60.0) as u32, (c.clock % 60.0) as u32);
    let w = ui.measure(&clock, &big);
    let top = screen.min.y + 18.0 * s;
    ui.text_at(&clock, &big, Vec2::new(mid - w * 0.5, top), w + 4.0, if c.surging { style::SIGNAL } else { style::BONE });

    // The strip: ticks every 15°, the four quarters lettered.
    let strip_w = 720.0 * s;
    let strip_y = top + f64::from(big.line_height()) + 12.0 * s;
    let strip = Rect::from_min_size(Vec2::new(mid - strip_w * 0.5, strip_y), Vec2::new(strip_w, 34.0 * s));
    ui.draw.rect(strip, Color::rgba(0.0, 0.0, 0.0, 0.4));
    let x_of = |yaw: f64| mid + off(c.heading, yaw) / SPAN * strip_w * 0.5;
    for k in 0..24 {
        let yaw = f64::from(k) * std::f64::consts::TAU / 24.0;
        if off(c.heading, yaw).abs() > SPAN {
            continue;
        }
        let x = x_of(yaw);
        let letter = match k {
            0 => Some("N"),
            6 => Some("W"),
            12 => Some("S"),
            18 => Some("E"),
            _ => None,
        };
        match letter {
            Some(l) => {
                let lw = ui.measure(l, &small);
                ui.text_at(l, &small, Vec2::new(x - lw * 0.5, strip.min.y + 3.0 * s), lw + 4.0, style::BONE);
            }
            None => {
                let h = if k % 3 == 0 { 12.0 } else { 7.0 } * s;
                ui.draw.rect(Rect::from_min_size(Vec2::new(x - 1.0 * s, strip.max.y - h - 4.0 * s), Vec2::new(2.0 * s, h)), style::DIM);
            }
        }
    }
    // The ways out found: a diamond on the strip (held at its end when
    // behind), the name and how far under it.
    for m in c.marks {
        let rel = off(c.heading, m.yaw);
        let x = mid + rel.clamp(-SPAN, SPAN) / SPAN * strip_w * 0.5;
        let colour = if m.open { SIGNAL_GREEN } else { style::DIM };
        let d = 9.0 * s;
        let centre = Vec2::new(x, strip.center().y);
        let pts = [centre + Vec2::new(0.0, -d), centre + Vec2::new(d, 0.0), centre + Vec2::new(0.0, d), centre + Vec2::new(-d, 0.0), centre + Vec2::new(0.0, -d)];
        ui.draw.polyline(&pts, 3.0 * s, colour, false);
        let label = if m.open { format!("{}  {:.0} m", m.name, m.distance) } else { format!("{}  BLOCKED", m.name) };
        let lw = ui.measure(&label, &small);
        let lx = (x - lw * 0.5).clamp(strip.min.x - 60.0 * s, strip.max.x + 60.0 * s - lw);
        ui.text_at(&label, &small, Vec2::new(lx, strip.max.y + 6.0 * s), lw + 4.0, colour);
    }

    // What's under way: its bar, and the seconds left.
    let mut y = strip.max.y + 6.0 * s + f64::from(small.line_height()) + 18.0 * s;
    if let Some(u) = &c.under {
        let text = if u.slipping { "GET BACK IN THE ZONE".to_string() } else { format!("{}  {}:{:02}", u.label, (u.left / 60.0) as u32, (u.left.ceil() % 60.0) as u32) };
        let colour = if u.slipping { style::SIGNAL } else { SIGNAL_GREEN };
        let tw = ui.measure(&text, &big);
        ui.text_at(&text, &big, Vec2::new(mid - tw * 0.5, y), tw + 4.0, colour);
        y += f64::from(big.line_height()) + 8.0 * s;
        let bar = Rect::from_min_size(Vec2::new(mid - 220.0 * s, y), Vec2::new(440.0 * s, 16.0 * s));
        ui.draw.rect(bar, Color::rgba(0.0, 0.0, 0.0, 0.55));
        ui.draw.rect(Rect::from_min_size(bar.min, Vec2::new(bar.width() * u.done.clamp(0.0, 1.0), bar.height())), colour);
        ui.draw.stroke_rect(bar, 2.0 * s, 0.0, Color::rgba(0.0, 0.0, 0.0, 0.8));
        y += 36.0 * s;
    }
    if let Some((shout, a)) = c.shout {
        let tw = ui.measure(shout, &big);
        ui.text_at(shout, &big, Vec2::new(mid - tw * 0.5, y), tw + 4.0, faded(style::SIGNAL, a));
        y += f64::from(big.line_height()) + 8.0 * s;
    }
    if let Some((words, a)) = c.chatter {
        let st = TextStyle::new((26.0 * s) as f32).family(style::FONT);
        let tw = ui.measure(words, &st);
        let r = Rect::from_min_size(Vec2::new(mid - tw * 0.5 - 16.0 * s, y - 6.0 * s), Vec2::new(tw + 32.0 * s, f64::from(st.line_height()) + 12.0 * s));
        ui.draw.rect(r, Color::rgba(0.0, 0.0, 0.0, 0.5 * a));
        ui.text_at(words, &st, Vec2::new(mid - tw * 0.5, y), tw + 4.0, faded(SIGNAL_GREEN, a));
    }
}
