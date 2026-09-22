//! The view riding on the player's body, every frame: which way it looks,
//! how high the eye is (standing or crouched), the head bob, the dip on
//! landing, the glide up and down stairs, and the sprint's wider view.

use bevy_ecs::prelude::*;
use lntrn_math::{Vec2, Vec3};

use crate::player::{Body, Player, WALK};
use crate::world::Clock;

/// Eye heights, metres, and how fast the eye moves between them.
pub const EYE_STAND: f64 = 1.7;
pub const EYE_CROUCH: f64 = 1.1;
const EYE_RATE: f64 = 20.0;
/// Radians of turn per count of raw mouse motion.
pub const SENSITIVITY: f64 = 0.0015;
const PITCH_LIMIT: f64 = 1.55;
/// Vertical field of view, and how much wider it goes at a full sprint.
pub const FOV: f64 = 65.0;
const SPRINT_FOV: f64 = 4.0;
/// How fast the eye catches up with the feet after a stair.
const STAIR_RATE: f64 = 14.0;

/// Which way the player faces, and how the view sits on the body.
#[derive(Component, Clone, Copy, Debug)]
pub struct View {
    pub yaw: f64,
    pub pitch: f64,
    pub eye: f64,
    /// Head bob: where in the stride, and how much of it shows (0–1).
    pub bob_phase: f64,
    pub bob_amount: f64,
    /// The landing dip, metres (negative is down), and its speed.
    pub dip: f64,
    pub dip_vel: f64,
    /// Where the eye still lags a stair the feet took at once, metres.
    pub stair: f64,
    /// 0–1: how much of the sprint's wider view is on.
    pub sprint_amount: f64,
}

impl View {
    pub fn facing(yaw: f64) -> Self {
        Self { yaw, pitch: 0.0, eye: EYE_STAND, bob_phase: 0.0, bob_amount: 0.0, dip: 0.0, dip_vel: 0.0, stair: 0.0, sprint_amount: 0.0 }
    }

    /// Turn by raw mouse counts.
    pub fn look(&mut self, counts: Vec2) {
        self.yaw -= counts.x * SENSITIVITY;
        self.pitch = (self.pitch - counts.y * SENSITIVITY).clamp(-PITCH_LIMIT, PITCH_LIMIT);
    }

    pub fn fov_y(&self) -> f64 {
        (FOV + SPRINT_FOV * self.sprint_amount).to_radians()
    }
}

/// The view's motion, every frame: eye height, bob, landing dip, stairs,
/// sprint. Takes the landings and stairs the body has made since.
pub fn settle_view(view: &mut View, body: &mut Body, dt: f64) {
    let eye = if body.crouched { EYE_CROUCH } else { EYE_STAND };
    view.eye += (eye - view.eye) * (1.0 - (-EYE_RATE * dt).exp());

    let speed = body.speed_flat();
    let striding = body.grounded && speed > 0.5;
    let target = if striding { (speed / WALK).min(1.5) } else { 0.0 };
    view.bob_amount += (target - view.bob_amount) * (1.0 - (-10.0 * dt).exp());
    // About a stride every 1.5 metres.
    view.bob_phase += dt * speed / 1.5 * std::f64::consts::PI;

    let hit = std::mem::take(&mut body.landed);
    if hit > 0.0 {
        view.dip_vel -= (hit * 0.25).min(3.0);
    }
    // A spring back to rest, damped just short of wobbling.
    let accel = -90.0 * view.dip - 16.0 * view.dip_vel;
    view.dip_vel += accel * dt;
    view.dip = (view.dip + view.dip_vel * dt).clamp(-0.3, 0.1);

    // A stair moves the feet at once; the eye starts where it was and
    // glides after them.
    view.stair -= std::mem::take(&mut body.stepped);
    view.stair *= (-STAIR_RATE * dt).exp();

    let sprint = if body.sprinting && speed > WALK { 1.0 } else { 0.0 };
    view.sprint_amount += (sprint - view.sprint_amount) * (1.0 - (-8.0 * dt).exp());
}

/// Where the eye is: on the body (between its last two steps by `alpha`),
/// raised to eye height, bobbing, dipping, gliding over stairs.
pub fn eye_position(view: &View, body: &Body, alpha: f64) -> Vec3 {
    let feet = body.prev + (body.pos - body.prev) * alpha;
    let bob_y = (view.bob_phase * 2.0).sin() * 0.035 * view.bob_amount;
    let bob_x = view.bob_phase.cos() * 0.025 * view.bob_amount;
    let (s, c) = view.yaw.sin_cos();
    let right = Vec3::new(c, 0.0, -s);
    feet + Vec3::new(0.0, view.eye + bob_y + view.dip + view.stair, 0.0) + right * bob_x
}

fn settle_views(clock: Res<Clock>, mut players: Query<(&mut View, &mut Body), With<Player>>) {
    for (mut view, mut body) in &mut players {
        settle_view(&mut view, &mut body, clock.dt);
    }
}

/// Add the view's system to the every-frame schedule.
pub fn install(frame: &mut Schedule) {
    frame.add_systems(settle_views);
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f64 = 1.0 / 60.0;

    #[test]
    fn a_landing_dips_the_view_and_it_springs_back() {
        let mut view = View::facing(0.0);
        let mut body = Body::at(Vec3::ZERO);
        body.landed = 8.0;
        settle_view(&mut view, &mut body, DT);
        assert_eq!(body.landed, 0.0, "taken once");
        let mut low: f64 = 0.0;
        for _ in 0..120 {
            settle_view(&mut view, &mut body, DT);
            low = low.min(view.dip);
        }
        assert!(low < -0.05 && low > -0.2, "dipped {low}");
        assert!(view.dip.abs() < 0.005, "back at rest, {}", view.dip);
    }

    #[test]
    fn the_eye_glides_up_a_stair() {
        let mut view = View::facing(0.0);
        let mut body = Body::at(Vec3::ZERO);
        body.stepped = 0.4;
        settle_view(&mut view, &mut body, DT);
        assert!(view.stair < -0.3, "starts below, {}", view.stair);
        for _ in 0..30 {
            settle_view(&mut view, &mut body, DT);
        }
        assert!(view.stair.abs() < 0.01, "caught up in half a second, {}", view.stair);
    }
}
