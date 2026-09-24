//! What one of the dead takes in: whether it sees the player (ahead of it,
//! near enough, nothing between), and which noise it heeds (near, surely
//! and just where; far off, maybe, and only roughly where).

use lntrn_math::{Vec2, Vec3};

use super::brain::*;
use super::steer::flat_dist;
use crate::collide::Solids;
use crate::player::Body;

impl Zombie {
    /// Whether it sees a player standing at `player`, as far as `reach`
    /// of its sight.
    pub(super) fn sees(&self, body: &Body, player: Vec3, solids: &Solids, reach: f64) -> bool {
        let to = Vec2::new(player.x - body.pos.x, player.z - body.pos.z);
        let d = to.length();
        if d > SIGHT * reach * self.kind.traits().sight {
            return false;
        }
        if d > CLOSE {
            let facing = Vec2::new(-self.yaw.sin(), -self.yaw.cos());
            if facing.dot(to * (1.0 / d)) < SIGHT_HALF.to_radians().cos() {
                return false;
            }
        }
        let eye = body.pos + Vec3::new(0.0, EYE, 0.0);
        let look = player + Vec3::new(0.0, PLAYER_EYE, 0.0) - eye;
        let len = look.length();
        solids.raycast(eye, look * (1.0 / len), (len - 0.3).max(0.0)).is_none()
    }

    /// The noises it hasn't listened to yet, standing at `at`: where it
    /// thinks the one it heeds came from, if it heeds one. Near, it's
    /// always heard, and just where; far off, fewer heed it, and only
    /// know roughly where it was.
    pub(super) fn listen(&mut self, at: Vec3, noises: &[(u32, Vec3, f64)]) -> Option<Vec3> {
        let mut heeded = None;
        for &(id, from, range) in noises {
            if id <= self.heard {
                continue;
            }
            self.heard = id;
            let d = flat_dist(at, from);
            let far = d / range.max(1e-6);
            if far > 1.0 || heeded.is_some() {
                continue;
            }
            let chance = if far <= HEARD_SURELY { 1.0 } else { 1.0 - (far - HEARD_SURELY) / (1.0 - HEARD_SURELY) * (1.0 - HEARD_AT_EDGE) };
            if self.rand() < chance {
                let (a, r) = (self.rand() * std::f64::consts::TAU, self.rand().sqrt() * d * HEARD_ROUGHLY);
                heeded = Some(from + Vec3::new(a.cos() * r, 0.0, a.sin() * r));
            }
        }
        heeded
    }
}
