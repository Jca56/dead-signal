//! Getting in, in a holdout: each of the dead comes in from outside by a
//! barrier (a window, a gap in the fence) boarded up against them. It
//! walks up to the barrier's outside, tears the boards off one by one (and
//! swipes through at whoever's standing at it), then climbs through, and
//! from there hunts like any other.

use bevy_ecs::prelude::*;
use lntrn_math::Vec3;

use super::brain::{Intent, Senses, State, Zombie};
use super::special::Goal;
use super::steer::{flat_dist, level};
use crate::player::Body;
use crate::sound::Sfx;

/// Seconds to tear off a board, and to climb through.
pub const TEAR_EVERY: f64 = 1.4;
pub const VAULT_FOR: f64 = 0.9;
/// This near its outside, it's at the barrier.
const AT_BARRIER: f64 = 0.3;
/// How high a climb through lifts it at the top.
const HOP: f64 = 1.1;

/// A way in, as the dead know it: where to stand outside it, where it
/// lands inside, and how many boards are still across it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Barrier {
    pub outside: Vec3,
    pub inside: Vec3,
    pub boards: u8,
}

/// The holdout's ways in (none on an ordinary map).
#[derive(Resource, Default)]
pub struct Barriers(pub Vec<Barrier>);

/// Where one climbing through from `from` to `to` is, `t` seconds in.
pub fn vaulted(from: Vec3, to: Vec3, t: f64) -> Vec3 {
    let u = (t / VAULT_FOR).clamp(0.0, 1.0);
    from + (to - from) * u + Vec3::new(0.0, HOP * (u * std::f64::consts::PI).sin(), 0.0)
}

impl Zombie {
    /// At barrier `at`: on its way there; there, tearing a board off every
    /// so often (`t` towards the next), swiping through at the player if
    /// they're right there; the boards all gone, climbing through.
    pub(super) fn breaching(&mut self, at: u8, t: f64, body: &Body, s: &Senses, out: &mut Intent, dt: f64) -> Goal {
        let traits = self.kind.traits();
        let Some(b) = s.barriers.get(at as usize).copied() else {
            self.barrier = None;
            self.state = State::Hunt;
            return None;
        };
        if t <= 0.0 && flat_dist(body.pos, b.outside) > AT_BARRIER {
            return Some((b.outside, 1.0, false, traits.turn));
        }
        self.face(b.inside, body, traits.turn, dt);
        let swipe = traits.swipe;
        if let Some(p) = s.player
            && flat_dist(body.pos, p) <= swipe.reach
            && level(body.pos, p)
            && self.cooldown <= 0.0
        {
            out.sounds.push((traits.snarl, 0.8));
            self.set(State::Attack { t: 0.0, struck: false });
            return None;
        }
        if b.boards == 0 {
            self.set(State::Vault { t: 0.0, from: body.pos, to: b.inside });
            return None;
        }
        // (A Juggernaut makes short work of them.)
        let every = if self.kind == super::kind::Kind::Juggernaut { TEAR_EVERY * 0.4 } else { TEAR_EVERY };
        let t = t.max(1e-6) + dt;
        if t >= every {
            out.tore = Some(at);
            out.sounds.push((Sfx::HitWood, 1.0));
            self.set(State::Breach { at, t: 1e-6 });
        } else {
            self.state = State::Breach { at, t };
        }
        None
    }

    /// `t` into climbing through: once over, it's in, and hunts.
    pub(super) fn vaulting(&mut self, t: f64, from: Vec3, to: Vec3, dt: f64) {
        let t = t + dt;
        if t >= VAULT_FOR {
            self.barrier = None;
            self.set(State::Hunt);
        } else {
            self.state = State::Vault { t, from, to };
        }
    }

    /// Where it is, climbing through (its feet), if it is.
    pub fn climbing(&self) -> Option<Vec3> {
        match self.state {
            State::Vault { t, from, to } => Some(vaulted(from, to, t)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collide::{Solids, box_tris};
    use crate::player::{self, STEP};

    /// A wall along z 0 (the outside at -z), a window in it from x -0.6 to
    /// 0.6, its sill 0.9 high: the frame only bodies meet.
    fn wall_with_window() -> Solids {
        let mut s = super::super::tests::floor();
        s.add(&box_tris(Vec3::new(-10.0, 0.0, -0.1), Vec3::new(-0.6, 3.0, 0.1)));
        s.add(&box_tris(Vec3::new(0.6, 0.0, -0.1), Vec3::new(10.0, 3.0, 0.1)));
        s.add(&box_tris(Vec3::new(-0.6, 0.0, -0.1), Vec3::new(0.6, 0.9, 0.1)));
        s.add(&box_tris(Vec3::new(-0.6, 2.1, -0.1), Vec3::new(0.6, 3.0, 0.1)));
        s.add_barrier(&box_tris(Vec3::new(-0.6, 0.9, -0.03), Vec3::new(0.6, 2.1, 0.03)));
        s
    }

    /// Step one of the dead as the world does (boards torn off, carried
    /// through the window) for `seconds`; the blows it landed.
    fn run(z: &mut Zombie, body: &mut Body, solids: &Solids, barriers: &mut [Barrier], player: Vec3, seconds: f64) -> usize {
        let mut blows = 0;
        for _ in 0..(seconds / STEP) as usize {
            let ways = barriers.to_vec();
            let senses = Senses { barriers: &ways, lures: &[], solids, nav: None, player: Some(player), noises: &[], alerts: &[], searches: &std::cell::Cell::new(u32::MAX), sight: 1.0 };
            let mut intent = z.think(body, &senses, STEP);
            blows += usize::from(intent.hit.is_some());
            if let Some(at) = intent.tore {
                barriers[at as usize].boards -= 1;
            }
            if let Some(at) = z.climbing() {
                body.pos = at;
                continue;
            }
            let gait = z.gait;
            player::step_body(body, z.yaw, &mut intent.controls, solids, &gait, STEP);
        }
        blows
    }

    fn breacher() -> (Zombie, Body) {
        let mut z = Zombie::of(super::super::kind::Kind::Shambler, 0.0, 11);
        z.relentless = true;
        z.barrier = Some(0);
        z.state = State::Breach { at: 0, t: 0.0 };
        (z, Body::at(Vec3::new(3.0, 0.0, -8.0)))
    }

    #[test]
    fn it_tears_the_boards_off_one_by_one_then_climbs_in() {
        let solids = wall_with_window();
        let mut barriers = [Barrier { outside: Vec3::new(0.0, 0.0, -0.7), inside: Vec3::new(0.0, 0.0, 0.8), boards: 3 }];
        let (mut z, mut body) = breacher();
        // The player well inside, out of its reach.
        let player = Vec3::new(0.0, 0.0, 8.0);
        run(&mut z, &mut body, &solids, &mut barriers, player, 6.0);
        assert!(flat_dist(body.pos, barriers[0].outside) < 0.8, "not at the window: {:?}", body.pos);
        assert!(barriers[0].boards < 3, "no board torn off");
        assert!(body.pos.z < 0.0, "through the boards");
        run(&mut z, &mut body, &solids, &mut barriers, player, 3.0 * TEAR_EVERY + VAULT_FOR + 0.5);
        assert_eq!(barriers[0].boards, 0);
        assert!(body.pos.z > 0.3, "never climbed in: {:?} ({:?})", body.pos, z.state);
        assert_eq!(z.barrier, None);
        // In, it comes for the player.
        run(&mut z, &mut body, &solids, &mut barriers, player, 2.0);
        assert!(body.pos.z > 2.0, "didn't come on: {:?}", body.pos);
    }

    #[test]
    fn it_swipes_through_the_window_at_whoever_stands_at_it_and_a_hit_leaves_it_there() {
        let solids = wall_with_window();
        let mut barriers = [Barrier { outside: Vec3::new(0.0, 0.0, -0.7), inside: Vec3::new(0.0, 0.0, 0.8), boards: 6 }];
        let (mut z, mut body) = breacher();
        let player = Vec3::new(0.0, 0.0, 0.5);
        let blows = run(&mut z, &mut body, &solids, &mut barriers, player, 8.0);
        assert!(blows > 0, "never swiped through");
        assert!(body.pos.z < 0.0, "got through the boards");
        // Shot, it staggers, and goes back to the boards.
        z.hurt(10.0, false, true, player);
        run(&mut z, &mut body, &solids, &mut barriers, Vec3::new(0.0, 0.0, 8.0), 1.0);
        assert!(matches!(z.state, State::Breach { .. }), "{:?}", z.state);
    }
}
