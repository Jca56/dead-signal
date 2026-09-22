//! The player: a body that walks, sprints, crouches and jumps over the
//! ground at a fixed 60 steps a second, and the view riding on it (eye
//! height, head bob, the dip on landing, the sprint's wider view), which
//! moves every frame. Snappy: full speed in a tenth of a second, a quick
//! pop of a jump, a little steering in the air.

use bevy_ecs::prelude::*;
use lntrn_math::{Vec2, Vec3};

use crate::world::{Clock, Ground};

pub const WALK: f64 = 6.0;
pub const SPRINT: f64 = 9.0;
pub const CROUCH: f64 = 3.0;
/// Metres per second², towards the speed wanted and back to rest.
const ACCEL: f64 = 80.0;
const FRICTION: f64 = 70.0;
/// How much of the ground's grip there is in the air.
const AIR_CONTROL: f64 = 0.3;
const GRAVITY: f64 = 20.0;
pub const JUMP_HEIGHT: f64 = 1.2;
/// Grace, seconds: a jump pressed just before landing still happens, and
/// one pressed just after walking off an edge does too.
const JUMP_BUFFER: f64 = 0.1;
const COYOTE: f64 = 0.1;
/// Eye heights, metres, and how fast the eye moves between them.
pub const EYE_STAND: f64 = 1.7;
pub const EYE_CROUCH: f64 = 1.1;
const EYE_RATE: f64 = 20.0;
/// How far below the feet the ground may drop and still be walked down
/// onto rather than fallen off (a downhill stride).
const SNAP_DOWN: f64 = 0.35;
/// The world ends this far from its middle, either way.
pub const BOUNDS: f64 = 115.0;

/// Radians of turn per count of raw mouse motion.
pub const SENSITIVITY: f64 = 0.0015;
const PITCH_LIMIT: f64 = 1.55;
/// Vertical field of view, and how much wider it goes at a full sprint.
pub const FOV: f64 = 65.0;
const SPRINT_FOV: f64 = 4.0;

/// The one step every fixed update takes, seconds.
pub const STEP: f64 = 1.0 / 60.0;

/// What the player is doing with the keys, gathered each frame and read
/// by the fixed steps.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct Controls {
    /// x: right, y: forward; each -1, 0 or 1.
    pub walk: Vec2,
    pub sprint: bool,
    /// Pressed since the last step that looked (cleared once used).
    pub jump: bool,
    pub crouch_toggle: bool,
}

/// Where the body is and how it moves. `pos` is at the feet.
#[derive(Component, Clone, Copy, Debug)]
pub struct Body {
    pub pos: Vec3,
    /// Where the last step left it, to draw frames between steps smoothly.
    pub prev: Vec3,
    pub vel: Vec3,
    pub grounded: bool,
    /// Seconds since last on the ground, and since jump was pressed.
    pub airborne: f64,
    pub jump_wait: Option<f64>,
    pub crouched: bool,
    pub sprinting: bool,
    /// How hard it has landed since the view last looked (m/s down,
    /// summed): the view takes it, so each landing dips once.
    pub landed: f64,
}

impl Body {
    pub fn at(pos: Vec3) -> Self {
        Self { pos, prev: pos, vel: Vec3::ZERO, grounded: true, airborne: 0.0, jump_wait: None, crouched: false, sprinting: false, landed: 0.0 }
    }

    pub fn speed_flat(&self) -> f64 {
        Vec2::new(self.vel.x, self.vel.z).length()
    }
}

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
    /// 0–1: how much of the sprint's wider view is on.
    pub sprint_amount: f64,
}

impl View {
    pub fn facing(yaw: f64) -> Self {
        Self { yaw, pitch: 0.0, eye: EYE_STAND, bob_phase: 0.0, bob_amount: 0.0, dip: 0.0, dip_vel: 0.0, sprint_amount: 0.0 }
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

/// The player, of whom there is one while a run is on.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct Player;

/// `from` moved towards `to` by at most `max`.
fn towards(from: Vec2, to: Vec2, max: f64) -> Vec2 {
    let d = to - from;
    let len = d.length();
    if len <= max || len == 0.0 { to } else { from + d * (max / len) }
}

/// One fixed step of the body: steer, fall, jump, land.
pub fn step_body(body: &mut Body, yaw: f64, controls: &mut Controls, ground: &Ground, dt: f64) {
    body.prev = body.pos;
    if std::mem::take(&mut controls.crouch_toggle) {
        body.crouched = !body.crouched;
    }
    if std::mem::take(&mut controls.jump) {
        body.jump_wait = Some(0.0);
    }
    // Sprinting is forward only, and stands you up.
    body.sprinting = controls.sprint && controls.walk.y > 0.0;
    if body.sprinting {
        body.crouched = false;
    }
    let speed = if body.crouched { CROUCH } else if body.sprinting { SPRINT } else { WALK };

    // The keys, turned to face where the player looks, on the flat.
    let (s, c) = yaw.sin_cos();
    let forward = Vec2::new(-s, -c);
    let right = Vec2::new(c, -s);
    let mut wish = right * controls.walk.x + forward * controls.walk.y;
    if wish.length() > 1.0 {
        wish = wish.normalize();
    }
    let flat = Vec2::new(body.vel.x, body.vel.z);
    let moving = wish != Vec2::ZERO;
    let flat = if body.grounded {
        towards(flat, wish * speed, if moving { ACCEL } else { FRICTION } * dt)
    } else if moving {
        towards(flat, wish * speed.max(flat.length()), ACCEL * AIR_CONTROL * dt)
    } else {
        flat
    };
    body.vel.x = flat.x;
    body.vel.z = flat.y;

    // Jump, with a little grace either side of the ground.
    if let Some(waited) = body.jump_wait {
        if body.airborne <= COYOTE && body.vel.y <= 0.0 {
            body.vel.y = (2.0 * GRAVITY * JUMP_HEIGHT).sqrt();
            body.grounded = false;
            body.airborne = COYOTE + dt;
            body.jump_wait = None;
        } else if waited + dt > JUMP_BUFFER {
            body.jump_wait = None;
        } else {
            body.jump_wait = Some(waited + dt);
        }
    }
    if !body.grounded {
        body.vel.y -= GRAVITY * dt;
    }
    body.pos += body.vel * dt;
    body.pos.x = body.pos.x.clamp(-BOUNDS, BOUNDS);
    body.pos.z = body.pos.z.clamp(-BOUNDS, BOUNDS);

    // Stand on the ground: land on it, or follow it down a slope.
    let floor = ground.height_at(body.pos.x, body.pos.z).unwrap_or(0.0);
    let was_grounded = body.grounded;
    if body.pos.y <= floor {
        if !was_grounded {
            body.landed += -body.vel.y;
        }
        body.pos.y = floor;
        body.vel.y = 0.0;
        body.grounded = true;
    } else if was_grounded && body.vel.y <= 0.0 && body.pos.y - floor <= SNAP_DOWN {
        body.pos.y = floor;
        body.vel.y = 0.0;
    } else {
        body.grounded = false;
    }
    body.airborne = if body.grounded { 0.0 } else { body.airborne + dt };
}

fn step_players(mut controls: ResMut<Controls>, ground: Res<Ground>, mut players: Query<(&mut Body, &View), With<Player>>) {
    for (mut body, view) in &mut players {
        step_body(&mut body, view.yaw, &mut controls, &ground, STEP);
    }
}

/// The view's motion, every frame: eye height, bob, landing dip, sprint.
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

    let sprint = if body.sprinting && speed > WALK { 1.0 } else { 0.0 };
    view.sprint_amount += (sprint - view.sprint_amount) * (1.0 - (-8.0 * dt).exp());
}

/// Where the eye is: on the body (between its last two steps by `alpha`),
/// raised to eye height, bobbing and dipping.
pub fn eye_position(view: &View, body: &Body, alpha: f64) -> Vec3 {
    let feet = body.prev + (body.pos - body.prev) * alpha;
    let bob_y = (view.bob_phase * 2.0).sin() * 0.035 * view.bob_amount;
    let bob_x = view.bob_phase.cos() * 0.025 * view.bob_amount;
    let (s, c) = view.yaw.sin_cos();
    let right = Vec3::new(c, 0.0, -s);
    feet + Vec3::new(0.0, view.eye + bob_y + view.dip, 0.0) + right * bob_x
}

fn settle_views(clock: Res<Clock>, mut players: Query<(&mut View, &mut Body), With<Player>>) {
    for (mut view, mut body) in &mut players {
        settle_view(&mut view, &mut body, clock.dt);
    }
}

/// Add the player's systems: the body to the fixed steps, the view to
/// every frame.
pub fn install(fixed: &mut Schedule, frame: &mut Schedule) {
    fixed.add_systems(step_players);
    frame.add_systems(settle_views);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat_ground() -> Ground {
        let a = Vec3::new(-200.0, 0.0, -200.0);
        let b = Vec3::new(200.0, 0.0, -200.0);
        let c = Vec3::new(200.0, 0.0, 200.0);
        let d = Vec3::new(-200.0, 0.0, 200.0);
        Ground(vec![[a, c, b], [a, d, c]])
    }

    fn steps(body: &mut Body, controls: &mut Controls, ground: &Ground, n: usize) {
        for _ in 0..n {
            step_body(body, 0.0, controls, ground, STEP);
        }
    }

    #[test]
    fn walks_up_to_speed_in_a_tenth_of_a_second() {
        let g = flat_ground();
        let mut body = Body::at(Vec3::ZERO);
        let mut c = Controls { walk: Vec2::new(0.0, 1.0), ..Default::default() };
        steps(&mut body, &mut c, &g, 6);
        assert!((body.speed_flat() - WALK).abs() < 1e-9, "full speed after 0.1 s, got {}", body.speed_flat());
        assert!(body.vel.z < 0.0, "yaw 0 walks towards -Z");
        c.walk = Vec2::ZERO;
        steps(&mut body, &mut c, &g, 6);
        assert_eq!(body.speed_flat(), 0.0, "and stops as quick");
        assert!(body.grounded && body.pos.y == 0.0);
    }

    #[test]
    fn a_jump_rises_its_height_and_lands() {
        let g = flat_ground();
        let mut body = Body::at(Vec3::ZERO);
        let mut c = Controls { jump: true, ..Default::default() };
        let mut top: f64 = 0.0;
        for _ in 0..120 {
            step_body(&mut body, 0.0, &mut c, &g, STEP);
            top = top.max(body.pos.y);
        }
        assert!((top - JUMP_HEIGHT).abs() < 0.08, "peak {top}");
        assert!(body.grounded && body.landed > 5.0, "landed at {}", body.landed);
    }

    #[test]
    fn sprint_is_forward_only_and_stands_you_up() {
        let g = flat_ground();
        let mut body = Body::at(Vec3::ZERO);
        let mut c = Controls { walk: Vec2::new(0.0, -1.0), sprint: true, crouch_toggle: true, ..Default::default() };
        steps(&mut body, &mut c, &g, 30);
        assert!(body.crouched && (body.speed_flat() - CROUCH).abs() < 1e-9, "backwards: no sprint, still crouched");
        c.walk = Vec2::new(0.0, 1.0);
        steps(&mut body, &mut c, &g, 30);
        assert!(!body.crouched && (body.speed_flat() - SPRINT).abs() < 1e-9);
    }

    #[test]
    fn a_landing_dips_the_view_and_it_springs_back() {
        let mut view = View::facing(0.0);
        let mut body = Body::at(Vec3::ZERO);
        body.landed = 8.0;
        settle_view(&mut view, &mut body, STEP);
        assert_eq!(body.landed, 0.0, "taken once");
        let mut low: f64 = 0.0;
        for _ in 0..120 {
            settle_view(&mut view, &mut body, STEP);
            low = low.min(view.dip);
        }
        assert!(low < -0.05 && low > -0.2, "dipped {low}");
        assert!(view.dip.abs() < 0.005, "back at rest, {}", view.dip);
    }
}
