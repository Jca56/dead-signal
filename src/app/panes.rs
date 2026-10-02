//! The window cut into panes, a player each: where each goes (one above
//! the other, or side by side), each player's camera as their pane sees
//! it, and the rule between them. Off a run, one pane: the title's scene.

use lntrn_math::{Color, Rect, Vec2, Vec3};
use lntrn_ui::Ui;

use super::{DeadSignal, Screen, TITLE_EYE, TITLE_FOV, TITLE_LOOK};
use crate::camera::{Camera, pane_fov};
use crate::head;
use crate::mates::Mark;

/// The rule between two panes, logical pixels.
const RULE: f64 = 6.0;

/// Where `n` panes go, each as its share of the window (left, top, right,
/// bottom: 0 to 1): the whole of it for one; more, one above the other,
/// or `side_by_side`.
pub(super) fn layout(n: usize, side_by_side: bool) -> Vec<[f64; 4]> {
    let n = n.max(1);
    let step = 1.0 / n as f64;
    (0..n)
        .map(|i| {
            let (from, to) = (i as f64 * step, (i + 1) as f64 * step);
            if side_by_side { [from, 0.0, to, 1.0] } else { [0.0, from, 1.0, to] }
        })
        .collect()
}

/// A share of `area`, as a rect of it.
pub(super) fn in_rect(share: [f64; 4], area: Rect) -> Rect {
    let at = |x: f64, y: f64| Vec2::new(area.min.x + area.width() * x, area.min.y + area.height() * y);
    Rect::new(at(share[0], share[1]), at(share[2], share[3]))
}

/// A share of a window `size` pixels, as whole pixels: left, top, width,
/// height.
pub(super) fn in_pixels(share: [f64; 4], size: [u32; 2]) -> [u32; 4] {
    let (w, h) = (f64::from(size[0]), f64::from(size[1]));
    let (x0, y0) = ((share[0] * w).round(), (share[1] * h).round());
    let (x1, y1) = ((share[2] * w).round(), (share[3] * h).round());
    [x0 as u32, y0 as u32, (x1 - x0).max(1.0) as u32, (y1 - y0).max(1.0) as u32]
}

impl DeadSignal {
    /// How the window's cut this frame: a pane each for the players of a
    /// run (the end is the first player's, whole), else the one.
    pub(super) fn lay_out(&mut self) {
        let players = if self.screen == Screen::Run && self.run.ending.is_none() { self.run.seats.len() } else { 1 };
        let side_by_side = self.game.world.resource::<crate::settings::Settings>().side_by_side;
        self.shares = layout(players, side_by_side);
    }

    /// The rules between the panes, over the world.
    pub(super) fn draw_rules(&self, ui: &mut Ui) {
        let area = ui.clip();
        let half = RULE * ui.m.scale * 0.5;
        for share in self.shares.iter().skip(1) {
            let r = in_rect(*share, area);
            // Along its top edge, or its left, whichever it shares.
            let rule = if share[1] > 0.0 { Rect::new(Vec2::new(r.min.x, r.min.y - half), Vec2::new(r.max.x, r.min.y + half)) } else { Rect::new(Vec2::new(r.min.x - half, r.min.y), Vec2::new(r.min.x + half, r.max.y)) };
            ui.draw.rect(rule, Color::rgb(0.02, 0.02, 0.02));
        }
    }

    /// Over each player's pane, the others' tags (playing together), as
    /// that pane's camera saw them (`panes`: each one's rect).
    pub(super) fn draw_marks(&mut self, ui: &mut Ui, panes: &[Rect]) {
        if self.run.seats.len() < 2 || self.run.ending.is_some() {
            return;
        }
        let time = self.game.clock().time;
        let marks: Vec<Mark> = self
            .run
            .seats
            .iter()
            .filter(|s| !s.out)
            .filter_map(|s| {
                let (body, _) = self.game.player(s.n)?;
                let down = s.down.map(|d| d.left);
                // Over their head, or (down) over them on the ground.
                let at = body.pos + Vec3::new(0.0, if down.is_some() { 0.9 } else { 2.1 }, 0.0);
                Some(Mark { seat: s.n, at, down })
            })
            .collect();
        for (i, (pane, camera)) in panes.iter().zip(&self.cameras).enumerate() {
            let Some(seat) = self.run.seats.get(i).filter(|s| !s.out) else { continue };
            let others: Vec<Mark> = marks.iter().copied().filter(|m| m.seat != seat.n).collect();
            ui.draw.push_clip(*pane);
            crate::mates::draw(ui, *pane, camera, &others, time);
            ui.draw.pop_clip();
        }
    }

    /// Over each player's pane, the dead's hurts as that pane's camera
    /// sees them: their bars, and the numbers of that player's own hits
    /// (each as the settings have it).
    pub(super) fn draw_hurts(&mut self, ui: &mut Ui, panes: &[Rect]) {
        let (bars, numbers) = {
            let s = self.game.world.resource::<crate::settings::Settings>();
            (s.health_bars, s.damage_numbers)
        };
        if !bars && !numbers {
            return;
        }
        for ((pane, camera), &seat) in panes.iter().zip(&self.cameras).zip(&self.eyes) {
            ui.draw.push_clip(*pane);
            if bars {
                self.combat.hurts.bars(ui, *pane, camera, &mut self.game.world, seat);
            }
            if numbers {
                self.combat.hurts.numbers(ui, *pane, camera, &self.game.world, seat);
            }
            ui.draw.pop_clip();
        }
    }

    /// Where each pane's camera is this frame, the window `aspect` wide
    /// (the window cut again: the screen may have changed since).
    pub(super) fn place_cameras(&mut self, time: f64, aspect: f64) {
        self.lay_out();
        self.cameras.clear();
        self.eyes.clear();
        match self.screen {
            Screen::Title | Screen::Hideout | Screen::Loading => {
                // A slow drift, never quite still.
                let drift = Vec3::new((time * 0.05).sin() * 4.0, (time * 0.07).sin() * 0.5, (time * 0.04).cos() * 2.5);
                let mut eye = TITLE_EYE + drift;
                if let Some(h) = self.game.ground().height_at(eye.x, eye.z) {
                    eye.y = eye.y.max(h + 2.0);
                }
                let mut camera = Camera::new(eye);
                camera.fov_y = TITLE_FOV.to_radians();
                camera.look_at(TITLE_LOOK + Vec3::new((time * 0.09).sin() * 1.5, 0.0, 0.0));
                self.cameras.push(camera);
            }
            Screen::Run => {
                let alpha = self.game.alpha();
                for (i, share) in self.shares.clone().into_iter().enumerate() {
                    let Some(seat) = self.run.seats.get(i) else { continue };
                    // Bled out, they watch whoever's still standing.
                    let watched = if seat.out { self.run.seats.iter().find(|s| s.standing()).map_or(seat.n, |s| s.n) } else { seat.n };
                    let Some((body, view)) = self.game.player(watched) else { continue };
                    let mut camera = Camera::new(head::eye_position(&view, &body, alpha));
                    (camera.yaw, camera.pitch) = view.aim();
                    let pane = aspect * (share[2] - share[0]) / (share[3] - share[1]).max(1e-6);
                    camera.fov_y = pane_fov(view.fov_y(), aspect, pane);
                    if let Some(ending) = &self.run.ending {
                        ending.fall(&mut camera);
                    }
                    self.cameras.push(camera);
                    self.eyes.push(watched);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_share_the_window_without_a_gap_either_way() {
        assert_eq!(layout(1, true), vec![[0.0, 0.0, 1.0, 1.0]]);
        let above = layout(2, false);
        assert_eq!(above, vec![[0.0, 0.0, 1.0, 0.5], [0.0, 0.5, 1.0, 1.0]]);
        let beside = layout(2, true);
        assert_eq!(beside, vec![[0.0, 0.0, 0.5, 1.0], [0.5, 0.0, 1.0, 1.0]]);
        // In whole pixels, an odd window still meets in the middle.
        let [top, bottom] = [in_pixels(above[0], [1921, 1081]), in_pixels(above[1], [1921, 1081])];
        assert_eq!(top[1] + top[3], bottom[1]);
        assert_eq!(bottom[1] + bottom[3], 1081);
        let r = in_rect(beside[1], Rect::new(Vec2::ZERO, Vec2::new(1600.0, 900.0)));
        assert_eq!((r.min.x, r.max.x, r.height()), (800.0, 1600.0, 900.0));
    }
}
