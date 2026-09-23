//! The player's body: an upright capsule that walks, sprints, crouches and
//! jumps through the world's solids at a fixed 60 steps a second. Snappy:
//! full speed in a tenth of a second, a quick pop of a jump, a little
//! steering in the air. It slides along walls, walks up steps as tall as
//! [`STEP_UP`] and down stairs without leaving them, climbs slopes up to
//! 45° and slides back off steeper ones, and will not stand up where there
//! is no room. The view that rides on it is in `head.rs`.

use bevy_ecs::prelude::*;
use lntrn_math::{Vec2, Vec3};

use crate::collide::{Capsule, Contacts, Solids};
use crate::head::View;
use crate::world::Solid;

pub const WALK: f64 = 6.0;
pub const SPRINT: f64 = 9.0;
pub const CROUCH: f64 = 3.0;
/// Metres per second², towards the speed wanted and back to rest.
const ACCEL: f64 = 80.0;
const FRICTION: f64 = 70.0;
/// How much of the ground's grip there is in the air.
const AIR_CONTROL: f64 = 0.3;
const GRAVITY: f64 = 20.0;
/// The fastest a body falls, m/s.
const MAX_FALL: f64 = 40.0;
pub const JUMP_HEIGHT: f64 = 1.2;
/// Grace, seconds: a jump pressed just before landing still happens, and
/// one pressed just after walking off an edge does too.
const JUMP_BUFFER: f64 = 0.1;
const COYOTE: f64 = 0.1;
/// The body: its width, and its height standing and crouched.
pub const RADIUS: f64 = 0.35;
pub const STAND_HEIGHT: f64 = 1.8;
pub const CROUCH_HEIGHT: f64 = 1.2;
/// The tallest step walked straight up; also how far the ground may drop
/// away underfoot and still be followed down (a stair, a hillside).
pub const STEP_UP: f64 = 0.4;
/// Drops under this are the ground's own shape, followed without the eye
/// gliding after them; bigger ones are stairs.
const STAIR: f64 = 0.15;
/// How far at a time a body is lowered looking for the floor.
const SNAP_STEP: f64 = 0.025;
/// The world ends this far from its middle, either way.
pub const BOUNDS: f64 = 115.0;

/// How fast a kind of body goes, m/s.
#[derive(Clone, Copy, Debug)]
pub struct Gait {
    pub walk: f64,
    pub sprint: f64,
    pub crouch: f64,
}

/// The player's.
pub const PLAYER_GAIT: Gait = Gait { walk: WALK, sprint: SPRINT, crouch: CROUCH };
/// How fast a shove fades, per second.
const PUSH_FADE: f64 = 7.0;

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
    /// Crouch, as asked for (C toggles it) and as it is (no room to stand
    /// keeps a body down).
    pub want_crouch: bool,
    pub crouched: bool,
    pub sprinting: bool,
    /// How hard it has landed since the view last looked (m/s down,
    /// summed): the view takes it, so each landing dips once.
    pub landed: f64,
    /// How far stairs have moved it up (or down) since the view last
    /// looked, metres: the view glides after.
    pub stepped: f64,
    /// A shove (a blow knocking it back), m/s, fading: added to how it
    /// moves on top of its steering.
    pub push: Vec3,
}

impl Body {
    pub fn at(pos: Vec3) -> Self {
        Self { pos, prev: pos, vel: Vec3::ZERO, grounded: true, airborne: 0.0, jump_wait: None, want_crouch: false, crouched: false, sprinting: false, landed: 0.0, stepped: 0.0, push: Vec3::ZERO }
    }

    pub fn speed_flat(&self) -> f64 {
        Vec2::new(self.vel.x, self.vel.z).length()
    }
}

/// The player, of whom there is one while a run is on.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct Player;

pub fn capsule(crouched: bool) -> Capsule {
    Capsule { radius: RADIUS, height: if crouched { CROUCH_HEIGHT } else { STAND_HEIGHT } }
}

/// `from` moved towards `to` by at most `max`.
fn towards(from: Vec2, to: Vec2, max: f64) -> Vec2 {
    let d = to - from;
    let len = d.length();
    if len <= max || len == 0.0 { to } else { from + d * (max / len) }
}

fn flat(v: Vec3) -> Vec2 {
    Vec2::new(v.x, v.z)
}

/// Take away the part of `vel` going into what was touched.
fn clip(vel: &mut Vec3, contacts: &Contacts) {
    for n in &contacts.normals {
        let into = vel.dot(*n);
        if into < 0.0 {
            *vel -= *n * into;
        }
    }
    if contacts.floor.is_some() && vel.y < 0.0 {
        vel.y = 0.0;
    }
}

/// Move a capsule from `start` at `vel` for `dt`, in steps no longer than
/// half its radius (so nothing is passed through), sliding along what it
/// meets. Where it ends, what it touched, and what is left of `vel`.
fn slide(solids: &Solids, cap: Capsule, start: Vec3, vel: Vec3, dt: f64) -> (Vec3, Contacts, Vec3) {
    let steps = ((vel * dt).length() / (cap.radius * 0.5)).ceil().clamp(1.0, 16.0) as usize;
    let h = dt / steps as f64;
    let (mut pos, mut vel) = (start, vel);
    let mut contacts = Contacts::default();
    for _ in 0..steps {
        pos += vel * h;
        let c = solids.resolve(cap, &mut pos);
        clip(&mut vel, &c);
        contacts.merge(c);
    }
    (pos, contacts, vel)
}

/// Lower a capsule by up to `max` onto a floor: where it stands, or `None`
/// with no floor that close below.
fn snap_down(solids: &Solids, cap: Capsule, from: Vec3, max: f64) -> Option<(Vec3, Contacts)> {
    // Fine steps: lowered too far at once onto an edge, the first touch
    // (walkable) is passed for a lower one (steep), and the edge is lost.
    let steps = (max / SNAP_STEP).ceil().max(1.0) as usize;
    let mut p = from;
    for _ in 0..steps {
        p.y -= max / steps as f64;
        let mut q = p;
        let c = solids.resolve(cap, &mut q);
        if c.floor.is_some() {
            return Some((q, c));
        }
        p = q;
    }
    None
}

/// Try the move again from a step higher, then back down onto what is
/// there: how a body walks up a stair instead of into it. It reaches at
/// least half its radius forward, or a slow walk would never get far
/// enough over a step's edge to stand on it.
fn step_up(solids: &Solids, cap: Capsule, start: Vec3, vel: Vec3, dt: f64) -> Option<(Vec3, Contacts, Vec3)> {
    let up = start + Vec3::new(0.0, STEP_UP, 0.0);
    if !solids.fits(cap, up) {
        return None; // no headroom
    }
    let flat_vel = Vec3::new(vel.x, 0.0, vel.z);
    let reach = flat_vel.length() * dt;
    let min_reach = cap.radius * 0.5;
    let push = if reach > 1e-9 && reach < min_reach { min_reach / reach } else { 1.0 };
    let (over, _, _) = slide(solids, cap, up, flat_vel * push, dt);
    let (landed, contacts) = snap_down(solids, cap, over, STEP_UP + 0.05)?;
    // As tall as the point stood on, not as high as the capsule rests.
    let rise = contacts.floor_top.unwrap_or(landed.y) - start.y;
    (0.02..=STEP_UP + 0.01).contains(&rise).then_some((landed, contacts, flat_vel))
}

/// One fixed step of the body: steer, jump, fall, and move through the
/// solids.
pub fn step_body(body: &mut Body, yaw: f64, controls: &mut Controls, solids: &Solids, gait: &Gait, dt: f64) {
    body.prev = body.pos;
    if std::mem::take(&mut controls.crouch_toggle) {
        body.want_crouch = !body.want_crouch;
    }
    // Sprinting is forward only, and stands you up (if there is room).
    if controls.sprint && controls.walk.y > 0.0 {
        body.want_crouch = false;
    }
    body.crouched = body.want_crouch || (body.crouched && !solids.fits(capsule(false), body.pos));
    body.sprinting = controls.sprint && controls.walk.y > 0.0 && !body.crouched;
    let speed = if body.crouched { gait.crouch } else if body.sprinting { gait.sprint } else { gait.walk };

    // The keys, turned to face where the player looks, on the flat.
    let (s, c) = yaw.sin_cos();
    let forward = Vec2::new(-s, -c);
    let right = Vec2::new(c, -s);
    let mut wish = right * controls.walk.x + forward * controls.walk.y;
    if wish.length() > 1.0 {
        wish = wish.normalize();
    }
    let now = flat(body.vel);
    let moving = wish != Vec2::ZERO;
    let steer = if body.grounded {
        towards(now, wish * speed, if moving { ACCEL } else { FRICTION } * dt)
    } else if moving {
        towards(now, wish * speed.max(now.length()), ACCEL * AIR_CONTROL * dt)
    } else {
        now
    };
    body.vel.x = steer.x;
    body.vel.z = steer.y;

    // Jump, with a little grace either side of the ground, and room above.
    if std::mem::take(&mut controls.jump) {
        body.jump_wait = Some(0.0);
    }
    let mut jumped = false;
    if let Some(waited) = body.jump_wait {
        if body.airborne <= COYOTE && body.vel.y <= 0.0 && solids.fits(capsule(body.crouched), body.pos + Vec3::new(0.0, 0.05, 0.0)) {
            body.vel.y = (2.0 * GRAVITY * JUMP_HEIGHT).sqrt();
            body.grounded = false;
            body.airborne = COYOTE + dt;
            body.jump_wait = None;
            jumped = true;
        } else if waited + dt > JUMP_BUFFER {
            body.jump_wait = None;
        } else {
            body.jump_wait = Some(waited + dt);
        }
    }
    if !body.grounded {
        body.vel.y = (body.vel.y - GRAVITY * dt).max(-MAX_FALL);
    }
    let falling = -body.vel.y;

    let cap = capsule(body.crouched);
    let start = body.pos;
    let (mut pos, mut contacts, _) = slide(solids, cap, start, body.vel + body.push, dt);
    // Its own motion and the shove each lose what went into what it touched.
    let mut vel = body.vel;
    clip(&mut vel, &contacts);
    clip(&mut body.push, &contacts);
    body.push *= (-PUSH_FADE * dt).exp();

    // Blocked while walking: perhaps it is a step. Tried at the speed
    // asked for, not what the wall left of it (which is next to nothing
    // after a frame against the step).
    let asked = Vec3::new(wish.x * speed, 0.0, wish.y * speed);
    let wanted = asked.length() * dt;
    if body.grounded && !jumped && contacts.wall && wanted > 1e-6 {
        // Progress the way the keys point: pushed back counts as none, not
        // as the distance it went backwards.
        let dir = flat(asked) * (1.0 / flat(asked).length());
        let went = flat(pos - start).dot(dir);
        if went < wanted * 0.8
            && let Some((p, c, v)) = step_up(solids, cap, start, asked, dt)
            && flat(p - start).dot(dir) > went + 1e-4
        {
            body.stepped += p.y - start.y;
            (pos, contacts, vel) = (p, c, v);
        }
    }

    // On the ground, or following it down a stair or a slope.
    let mut grounded = contacts.floor.is_some() && vel.y <= 0.0;
    if !grounded
        && body.grounded
        && !jumped
        && vel.y <= 0.0
        && let Some((p, _)) = snap_down(solids, cap, pos, STEP_UP)
    {
        let drop = p.y - pos.y;
        if -drop > STAIR {
            body.stepped += drop;
        }
        pos = p;
        grounded = true;
    }
    if grounded {
        if !body.grounded {
            body.landed += falling.max(0.0);
        }
        vel.y = 0.0;
    }
    pos.x = pos.x.clamp(-BOUNDS, BOUNDS);
    pos.z = pos.z.clamp(-BOUNDS, BOUNDS);
    body.pos = pos;
    body.vel = vel;
    body.grounded = grounded;
    body.airborne = if grounded { 0.0 } else { body.airborne + dt };
}

fn step_players(mut controls: ResMut<Controls>, solid: Res<Solid>, mut players: Query<(&mut Body, &View), With<Player>>) {
    for (mut body, view) in &mut players {
        step_body(&mut body, view.yaw, &mut controls, &solid.0, &PLAYER_GAIT, STEP);
    }
}

/// Add the body's system to the fixed steps.
pub fn install(fixed: &mut Schedule) {
    fixed.add_systems(step_players);
}

#[cfg(test)]
mod tests;
