//! Playing together, on screen: the others' tags over them in each pane
//! (by their colour, and how far off), a downed one's always, pinned to
//! the pane's edge when they're out of sight (REVIVE, and how long they've
//! left); a downed player's own pane (DOWN, bleeding out, being picked
//! up); and a bled out one's (watching the one still standing).

use lntrn_math::{Color, Rect, Vec2, Vec3};
use lntrn_text::TextStyle;
use lntrn_ui::Ui;

use crate::camera::Camera;
use crate::style;

/// Another player as a pane shows them: whose seat, the point over them
/// the tag goes, and (down) how long they've left.
#[derive(Clone, Copy, Debug)]
pub struct Mark {
    pub seat: usize,
    pub at: Vec3,
    pub down: Option<f64>,
}

/// How far in from a pane's edge a tag pinned to it sits.
const INSET: f64 = 70.0;

/// Where `p` shows over `screen` from `camera` (whose view fills it), and
/// whether it's ahead of the eye.
pub fn onto(camera: &Camera, screen: Rect, p: Vec3) -> (Vec2, bool) {
    let (right, up, forward) = camera.basis();
    let to = p - camera.position;
    let z = to.dot(forward);
    let half_h = (camera.fov_y * 0.5).tan();
    let half_w = half_h * screen.width() / screen.height().max(1.0);
    let depth = z.abs().max(1e-3);
    let (x, y) = (to.dot(right) / (depth * half_w), to.dot(up) / (depth * half_h));
    let c = screen.center();
    (Vec2::new(c.x + x * screen.width() * 0.5, c.y - y * screen.height() * 0.5), z > 0.0)
}

/// `p`, seen from the middle of `screen`, brought in to within `inset` of
/// its edge (always, if it's `behind`: there's no seeing it; right behind,
/// at the bottom).
fn pinned(screen: Rect, p: Vec2, behind: bool, inset: f64) -> Vec2 {
    let c = screen.center();
    let d = if (p - c).length() < 1e-6 { Vec2::new(0.0, 1.0) } else { p - c };
    let (hw, hh) = ((screen.width() * 0.5 - inset).max(1.0), (screen.height() * 0.5 - inset).max(1.0));
    let k = (d.x.abs() / hw).max(d.y.abs() / hh);
    if k <= 1.0 && !behind { p } else { c + d * (1.0 / k.max(1e-9)) }
}

/// The other players over `screen`, as `camera` sees them.
pub fn draw(ui: &mut Ui, screen: Rect, camera: &Camera, marks: &[Mark], time: f64) {
    let s = ui.m.scale;
    let name = TextStyle::new((32.0 * s) as f32).bold().family(style::FONT);
    let small = TextStyle::new((24.0 * s) as f32).bold().family(style::FONT);
    for m in marks {
        let (at, ahead) = onto(camera, screen, m.at);
        let inside = ahead && screen.shrink(INSET * s).contains(at);
        // A standing one is only tagged in sight; a downed one always.
        if m.down.is_none() && !inside {
            continue;
        }
        let at = pinned(screen, at, !ahead, INSET * s);
        let far = (m.at - camera.position).length();
        let (words, colour) = match m.down {
            Some(left) => (format!("REVIVE P{}  {}", m.seat + 1, left.ceil() as u32), style::SIGNAL),
            None => (format!("P{}", m.seat + 1), style::player(m.seat)),
        };
        let under = format!("{far:.0} M");
        let (w, h) = (ui.measure(&words, &name), f64::from(name.line_height()));
        let (uw, uh) = (ui.measure(&under, &small), f64::from(small.line_height()));
        let back = Rect::from_center_size(at, Vec2::new(w.max(uw) + 24.0 * s, h + uh + 16.0 * s));
        ui.draw.rect(back, Color::rgba(0.0, 0.0, 0.0, 0.55));
        if m.down.is_some() {
            // A downed one's tag throbs.
            let pulse = 0.5 + 0.5 * (time * 6.0).sin();
            ui.draw.stroke_rect(back, 3.0 * s, 0.0, Color::rgba(colour.r, colour.g, colour.b, 0.4 + 0.6 * pulse));
        } else {
            ui.draw.rect(Rect::from_min_size(back.min, Vec2::new(back.width(), 4.0 * s)), colour);
        }
        ui.text_at(&words, &name, Vec2::new(at.x - w * 0.5, back.min.y + 8.0 * s), w + 8.0, colour);
        ui.text_at(&under, &small, Vec2::new(at.x - uw * 0.5, back.min.y + 8.0 * s + h), uw + 8.0, style::DIM);
    }
}

/// A downed player's own pane: red all round, DOWN, how long they've left
/// bleeding out (`left` seconds of `of`), and someone picking them up (how
/// far along, `revive`, 0–1).
pub fn down(ui: &mut Ui, screen: Rect, left: f64, of: f64, revive: f64) {
    let s = ui.m.scale;
    crate::hud::vignette(ui, screen, 1.0);
    let big = TextStyle::new((80.0 * s) as f32).bold().family(style::SERIF);
    let line = TextStyle::new((32.0 * s) as f32).bold().family(style::FONT);
    let c = screen.center();
    let mut y = screen.min.y + screen.height() * 0.2;
    let w = ui.measure("DOWN", &big);
    ui.text_at("DOWN", &big, Vec2::new(c.x - w * 0.5, y), w + 8.0, style::SIGNAL);
    y += f64::from(big.line_height()) + 12.0 * s;
    let (words, share, colour) = if revive > 0.0 {
        ("BEING PICKED UP".to_string(), revive, style::BONE)
    } else {
        (format!("BLEEDING OUT  {}", left.ceil() as u32), (left / of).clamp(0.0, 1.0), style::SIGNAL)
    };
    let ww = ui.measure(&words, &line);
    ui.text_at(&words, &line, Vec2::new(c.x - ww * 0.5, y), ww + 8.0, style::BONE);
    y += f64::from(line.line_height()) + 12.0 * s;
    let bar = Rect::from_min_size(Vec2::new(c.x - 260.0 * s, y), Vec2::new(520.0 * s, 18.0 * s));
    ui.draw.rect(bar, Color::rgba(0.0, 0.0, 0.0, 0.6));
    ui.draw.rect(Rect::from_min_size(bar.min, Vec2::new(bar.width() * share, bar.height())), colour);
}

/// A bled out player's pane: out till the next round, and watching
/// `watching` (a seat), if anyone's standing.
pub fn out(ui: &mut Ui, screen: Rect, watching: Option<usize>) {
    let s = ui.m.scale;
    let big = TextStyle::new((60.0 * s) as f32).bold().family(style::SERIF);
    let line = TextStyle::new((32.0 * s) as f32).bold().family(style::FONT);
    let band = Rect::from_min_size(Vec2::new(screen.min.x, screen.min.y + screen.height() * 0.08), Vec2::new(screen.width(), f64::from(big.line_height() + line.line_height()) + 40.0 * s));
    ui.draw.rect(band, Color::rgba(0.0, 0.0, 0.0, 0.6));
    let c = screen.center().x;
    let w = ui.measure("BLED OUT", &big);
    ui.text_at("BLED OUT", &big, Vec2::new(c - w * 0.5, band.min.y + 12.0 * s), w + 8.0, style::SIGNAL);
    if let Some(seat) = watching {
        let words = format!("BACK NEXT ROUND  ·  WATCHING P{}", seat + 1);
        let ww = ui.measure(&words, &line);
        ui.text_at(&words, &line, Vec2::new(c - ww * 0.5, band.min.y + 20.0 * s + f64::from(big.line_height())), ww + 8.0, style::player(seat));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mark_ahead_lands_where_it_is_and_one_behind_is_pinned_to_the_edge() {
        let screen = Rect::new(Vec2::ZERO, Vec2::new(1600.0, 900.0));
        let camera = Camera::new(Vec3::ZERO);
        let (dead_ahead, ahead) = onto(&camera, screen, Vec3::new(0.0, 0.0, -10.0));
        assert!(ahead && (dead_ahead - screen.center()).length() < 1e-9);
        let (off_right, _) = onto(&camera, screen, Vec3::new(3.0, 0.0, -10.0));
        assert!(off_right.x > 800.0 && (off_right.y - 450.0).abs() < 1e-9);
        // Behind and to the right: pinned to the right edge, not the left.
        let (p, ahead) = onto(&camera, screen, Vec3::new(3.0, 0.0, 10.0));
        let at = pinned(screen, p, !ahead, 70.0);
        assert!(!ahead && (at.x - (1600.0 - 70.0)).abs() < 1e-6, "{at:?}");
    }
}
