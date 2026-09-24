//! What the special dead do that the rest don't, as the brain (`brain.rs`)
//! asks: a Spitter keeping off and heaving up its globs; a Juggernaut
//! roaring, charging (into the player, or into a wall, and dazed), and its
//! plate.

use lntrn_math::{Vec2, Vec3};

use super::brain::{Blow, Intent, State, Zombie};
use super::kind::Kind;
use super::spit;
use crate::collide::Solids;
use crate::player::{Body, Controls};
use crate::sound::Sfx;

/// A Juggernaut charges from this near to this far, roaring this long
/// first, flat out for this long at most; hits what it meets this near,
/// this hard; then waits this long (about) to charge again. Run into
/// something, it's dazed this long.
const CHARGE_NEAREST: f64 = 6.0;
const CHARGE_FARTHEST: f64 = 25.0;
pub const ROAR_FOR: f64 = 1.0;
const CHARGE_FOR: f64 = 2.2;
const CHARGE_HITS: f64 = 1.7;
const CHARGE_DAMAGE: f64 = 50.0;
const CHARGE_EVERY: f64 = 7.0;
pub const DAZED_FOR: f64 = 2.5;
/// What its plate lets through (from the front, but for its arms and
/// legs), and what it takes dazed.
const PLATE: f64 = 0.25;
const DAZED_TAKES: f64 = 2.0;

/// Where it's heading: somewhere, at a pace (a share of its walk), lunging
/// or not, turning so fast.
pub(super) type Goal = Option<(Vec3, f64, bool, f64)>;

impl Zombie {
    /// A Spitter hunting the player at `p`: it spits from its distance (when
    /// it `sees` them), backs away from too near, closes from too far;
    /// turned away, it goes by where it saw them.
    pub(super) fn keep_off(&mut self, p: Vec3, sees: bool, body: &Body, out: &mut Intent, dt: f64) -> Goal {
        let turn = self.kind.traits().turn;
        let d = Vec2::new(p.x - body.pos.x, p.z - body.pos.z).length();
        if sees && self.spit_in <= 0.0 && (spit::SPIT_NEAREST..=spit::SPIT_FARTHEST).contains(&d) {
            out.sounds.push((self.kind.traits().snarl, 0.7));
            self.set(State::Spit { t: 0.0, thrown: false });
            None
        } else if d < spit::KEEP_NEAR {
            let away = Vec3::new(body.pos.x - p.x, 0.0, body.pos.z - p.z) * (1.0 / d.max(1e-6));
            Some((body.pos + away * 4.0, 1.0, false, turn))
        } else if d > spit::KEEP_FAR {
            Some((p, 1.0, false, turn))
        } else {
            self.face(p, body, turn, dt);
            None
        }
    }

    /// `t` into heaving up a glob at the player (at `player`): it's thrown
    /// partway, from the mouth; after, a while till the next.
    pub(super) fn spitting(&mut self, t: f64, thrown: bool, body: &Body, player: Option<Vec3>, out: &mut Intent, dt: f64) {
        let t = t + dt;
        let mut thrown = thrown;
        if let Some(p) = player {
            self.face(p, body, self.kind.traits().turn, dt);
            if !thrown && t >= spit::SPIT_AT {
                thrown = true;
                let facing = Vec3::new(-self.yaw.sin(), 0.0, -self.yaw.cos());
                out.spit = Some((body.pos + Vec3::new(0.0, spit::MOUTH, 0.0) + facing * 0.4, p));
                out.sounds.push((Sfx::Spit, 1.0));
            }
        }
        if t >= spit::SPIT_TIME {
            self.spit_in = spit::SPIT_EVERY * (0.8 + 0.4 * self.rand());
            self.set(State::Hunt);
        } else {
            self.state = State::Spit { t, thrown };
        }
    }

    /// Whether a Juggernaut might charge at the player at `p` now: rested,
    /// near enough (but not too), on their level, and nothing in the way.
    pub(super) fn may_charge(&self, p: Vec3, body: &Body, solids: &Solids) -> bool {
        let to = Vec3::new(p.x - body.pos.x, 0.0, p.z - body.pos.z);
        let d = to.length();
        if self.charge_in > 0.0 || !(CHARGE_NEAREST..=CHARGE_FARTHEST).contains(&d) || (p.y - body.pos.y).abs() > 1.0 {
            return false;
        }
        let chest = body.pos + Vec3::new(0.0, 1.2, 0.0);
        solids.raycast(chest, to * (1.0 / d), d).is_none()
    }

    /// `t` into its roar: it turns to the player (at `player`), and once
    /// it's roared, charges the way it faces.
    pub(super) fn roaring(&mut self, t: f64, body: &Body, player: Option<Vec3>, dt: f64) {
        if let Some(p) = player {
            self.face(p, body, 4.0, dt);
        }
        let t = t + dt;
        if t >= ROAR_FOR {
            let dir = Vec3::new(-self.yaw.sin(), 0.0, -self.yaw.cos());
            self.set(State::Charge { t: 0.0, dir, last: body.pos });
        } else {
            self.state = State::Roar { t };
        }
    }

    /// `t` into its charge along `dir` (it was at `last` the step before):
    /// flat out and never turning; into the player (at `player`) it hits
    /// them hard and it's over; stopped short by something, it's dazed.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn charging(&mut self, t: f64, dir: Vec3, last: Vec3, body: &Body, player: Option<Vec3>, out: &mut Intent, dt: f64) {
        let t = t + dt;
        self.yaw = (-dir.x).atan2(-dir.z);
        out.controls = Controls { walk: Vec2::new(0.0, 1.0), sprint: true, jump: false, crouch_toggle: false };
        // A footfall every so often, felt as much as heard.
        if (t * 3.2).floor() != ((t - dt) * 3.2).floor() {
            out.sounds.push((Sfx::Stomp, 1.0));
        }
        if let Some(p) = player
            && Vec2::new(p.x - body.pos.x, p.z - body.pos.z).length() < CHARGE_HITS
            && (-1.2..1.2).contains(&(p.y - body.pos.y))
        {
            out.hit = Some(Blow { push: dir * 3.0, damage: CHARGE_DAMAGE, leaves: None });
            self.charge_in = CHARGE_EVERY * (0.8 + 0.4 * self.rand());
            self.set(State::Hunt);
            return;
        }
        let went = Vec2::new(body.pos.x - last.x, body.pos.z - last.z).length();
        let full = self.kind.traits().pace.2 * dt;
        if t > 0.25 && went < full * 0.35 {
            out.sounds.push((Sfx::Slam, 1.0));
            self.charge_in = CHARGE_EVERY * (0.8 + 0.4 * self.rand());
            self.set(State::Dazed { t: 0.0 });
        } else if t >= CHARGE_FOR {
            self.charge_in = CHARGE_EVERY * (0.8 + 0.4 * self.rand());
            self.set(State::Hunt);
        } else {
            self.state = State::Charge { t, dir, last: body.pos };
        }
    }

    /// What of a hit along `dir` (on a limb, or not) gets through: a
    /// Juggernaut's plate turns most of what meets it from the front, and
    /// dazed it takes double.
    pub fn plating(&self, dir: Vec3, limb: bool) -> f64 {
        if self.kind != Kind::Juggernaut {
            return 1.0;
        }
        if matches!(self.state, State::Dazed { .. }) {
            return DAZED_TAKES;
        }
        let facing = Vec3::new(-self.yaw.sin(), 0.0, -self.yaw.cos());
        let from = Vec3::new(-dir.x, 0.0, -dir.z);
        let from = if from.length() > 1e-6 { from.normalize() } else { facing };
        if !limb && facing.dot(from) > 0.3 { PLATE } else { 1.0 }
    }
}
