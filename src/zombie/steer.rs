//! How one of the dead gets where it's going: turning to face it, and
//! walking straight at it when the way is clear, else along a route found
//! on the nav grid, leg by leg.

use lntrn_math::{Vec2, Vec3};

use super::brain::{REPATH, NO_WAY, GATHER, ARRIVE, LOOK_AHEAD, REACH_DOWN, REACH_UP, Zombie};
use super::nav::NavGrid;
use crate::player::{Body, Controls};

impl Zombie {
    /// Turn toward `p` at up to `rate` rad/s. How far off it still is.
    pub(super) fn face(&mut self, p: Vec3, body: &Body, rate: f64, dt: f64) -> f64 {
        let (dx, dz) = (p.x - body.pos.x, p.z - body.pos.z);
        if dx.abs() + dz.abs() < 1e-6 {
            return 0.0;
        }
        let want = (-dx).atan2(-dz);
        let mut off = want - self.yaw;
        off = (off + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU) - std::f64::consts::PI;
        let step = off.clamp(-rate * dt, rate * dt);
        self.yaw += step;
        off - step
    }

    /// The keys that take it toward `target`: along the nav grid when the
    /// way isn't straight, slowed while it turns.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn steer(&mut self, body: &Body, target: Vec3, pace: f64, lunge: bool, turn: f64, nav: Option<&NavGrid>, searches: &std::cell::Cell<u32>, dt: f64) -> Controls {
        // Close and on its level it just goes; anywhere else the grid
        // decides (up on a roof over it means going round by the stairs).
        let straight = (flat_dist(body.pos, target) < 3.0 && level(body.pos, target)) || nav.is_none_or(|n| n.clear(body.pos, target));
        if !straight && self.cut_off && flat_dist(body.pos, target) < GATHER {
            // As near as it gets: it stands and glares up at them.
            self.face(target, body, turn, dt);
            return Controls::default();
        }
        let waypoint = if straight {
            self.cut_off = false;
            self.path.clear();
            target
        } else {
            // (No way found is remembered until the next repath too: a search
            // that fails looks at the whole grid.)
            let wants = self.repath <= 0.0 || flat_dist(self.path_goal, target) > 2.0;
            if wants && searches.get() > 0 {
                searches.set(searches.get() - 1);
                let found = nav.and_then(|n| n.path(body.pos, target));
                let beat = 0.75 + 0.5 * self.rand();
                self.repath = if found.is_some() { REPATH } else { NO_WAY } * beat;
                self.cut_off = found.as_ref().and_then(|p| p.last()).is_some_and(|end| flat_dist(*end, target) > 1.5 || !level(*end, target));
                self.path = found.unwrap_or_default();
                self.path_goal = target;
                self.leg_from = body.pos;
            }
            // Each leg is walked along its line, not cut across: a turning
            // point counts once it's truly reached (or passed), and it aims
            // a little ahead on the leg, so knocked off it, it steers back
            // on, and it comes to the foot of a flight of stairs lined up.
            while self.path.len() > 1 && (flat_dist(body.pos, self.path[0]) < ARRIVE || along(self.leg_from, self.path[0], body.pos) >= 1.0) {
                self.leg_from = self.path.remove(0);
            }
            match self.path.first() {
                Some(&to) => {
                    let len = flat_dist(self.leg_from, to);
                    let t = if len > 1e-6 { (along(self.leg_from, to, body.pos) + LOOK_AHEAD / len).clamp(0.0, 1.0) } else { 1.0 };
                    self.leg_from + (to - self.leg_from) * t
                }
                None => target,
            }
        };
        let off = self.face(waypoint, body, turn, dt);
        let go = pace * off.cos().max(0.0);
        Controls { walk: Vec2::new(0.0, go), sprint: lunge && off.abs() < 0.5, jump: false, crouch_toggle: false }
    }
}

/// How far along the way from `a` to `b` the point `p` is, flat: 0 at `a`,
/// 1 at `b`.
pub(super) fn along(a: Vec3, b: Vec3, p: Vec3) -> f64 {
    let ab = Vec2::new(b.x - a.x, b.z - a.z);
    let len2 = ab.dot(ab);
    if len2 < 1e-9 {
        return 1.0;
    }
    Vec2::new(p.x - a.x, p.z - a.z).dot(ab) / len2
}

/// Whether feet at `b` are near enough the level of feet at `a` to strike.
pub(super) fn level(a: Vec3, b: Vec3) -> bool {
    (-REACH_DOWN..=REACH_UP).contains(&(b.y - a.y))
}

pub(super) fn flat_dist(a: Vec3, b: Vec3) -> f64 {
    Vec2::new(a.x - b.x, a.z - b.z).length()
}
