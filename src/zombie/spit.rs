//! What a Spitter does to the world: globs of bile lobbed in an arc (a
//! little off, and slow enough to step out of the way of), the poison
//! puddles they leave where they land, and, dead, the burst it swells up
//! to. The globs fly and the puddles fester on the fixed step; what they
//! do to the player goes out through the [`Horde`] (a glob that hits is a
//! blow; standing in a puddle, poison), and a burst is left for the combat
//! side (`combat::bursts`) to hurt what's near.

use bevy_ecs::prelude::*;
use lntrn_core::log_error;
use lntrn_math::{Mat4, Quat, Vec2, Vec3};

use super::Horde;
use super::brain::Blow;
use crate::player::{Body, Player, STEP};
use crate::render::{Draw, MeshId, Renderer};
use crate::sound::Sfx;
use crate::vitals::Affliction;
use crate::world::Solid;

/// The heave: how long it takes, and when in it the glob's thrown; how
/// long before the next (about); where the mouth is, above the feet.
pub const SPIT_TIME: f64 = 0.9;
pub const SPIT_AT: f64 = 0.55;
pub const SPIT_EVERY: f64 = 3.2;
pub const MOUTH: f64 = 1.55;
/// It spits from this near to this far, and keeps between these.
pub const SPIT_NEAREST: f64 = 4.0;
pub const SPIT_FARTHEST: f64 = 22.0;
pub const KEEP_NEAR: f64 = 8.0;
pub const KEEP_FAR: f64 = 15.0;
/// A glob: how fast it's thrown, how far off it lands (at most, metres),
/// what a hit takes off, and how near the player's middle counts as one.
const SPEED: f64 = 15.0;
const GRAVITY: f64 = 9.8;
const OFF: f64 = 1.2;
const GLOB_DAMAGE: f64 = 8.0;
const GLOB_HITS: f64 = 0.6;
/// A puddle: how wide, how long it lasts, and the last of that it spends
/// shrinking away.
pub const PUDDLE: f64 = 1.4;
const PUDDLE_FOR: f64 = 7.0;
const DRYING: f64 = 1.5;
/// Dead, it bursts this long after it falls; the burst reaches this far,
/// and leaves a puddle this wide for this long.
pub const BURST_AT: f64 = 2.0;
pub const BURST_REACH: f64 = 4.5;
const BURST_PUDDLE: f64 = 3.0;
const BURST_PUDDLE_FOR: f64 = 10.0;

/// A glob in the air.
#[derive(Component, Clone, Copy, Debug)]
pub struct Glob {
    pub pos: Vec3,
    pub prev: Vec3,
    vel: Vec3,
    age: f64,
}

/// A pool of bile on the ground.
#[derive(Component, Clone, Copy, Debug)]
pub struct Puddle {
    pub at: Vec3,
    pub radius: f64,
    left: f64,
}

impl Puddle {
    pub fn new(at: Vec3, radius: f64, lasts: f64) -> Self {
        Self { at, radius, left: lasts }
    }

    /// How wide it is now (drying up at the end).
    pub fn size(&self) -> f64 {
        self.radius * (self.left / DRYING).clamp(0.0, 1.0).sqrt()
    }
}

/// The throw that lands a glob from `from` on `to` (the lower of the two
/// arcs; as far as it goes at 45° if it can't reach).
pub fn throw(from: Vec3, to: Vec3) -> Vec3 {
    let flat = Vec2::new(to.x - from.x, to.z - from.z);
    let x = flat.length().max(0.01);
    let y = to.y - from.y;
    let v2 = SPEED * SPEED;
    let disc = v2 * v2 - GRAVITY * (GRAVITY * x * x + 2.0 * y * v2);
    let angle = if disc >= 0.0 { ((v2 - disc.sqrt()) / (GRAVITY * x)).atan() } else { std::f64::consts::FRAC_PI_4 };
    let dir = flat * (1.0 / x);
    Vec3::new(dir.x * angle.cos(), angle.sin(), dir.y * angle.cos()) * SPEED
}

/// Launch what the Spitters threw this step, a little off where they
/// aimed.
fn launch(commands: &mut Commands, horde: &mut Horde) {
    for (from, at) in std::mem::take(&mut horde.spits) {
        let (a, r) = (horde.roll() * std::f64::consts::TAU, horde.roll().sqrt() * OFF);
        let at = at + Vec3::new(a.cos() * r, 0.0, a.sin() * r);
        commands.spawn(Glob { pos: from, prev: from, vel: throw(from, at), age: 0.0 });
    }
    for at in std::mem::take(&mut horde.bursting) {
        commands.spawn(Puddle::new(at, BURST_PUDDLE, BURST_PUDDLE_FOR));
        horde.bursts.push(at);
    }
}

/// The globs fly: into the player (a blow, and poison), or onto whatever
/// they meet (a puddle, if it's the ground).
pub fn fly(mut commands: Commands, mut globs: Query<(Entity, &mut Glob)>, players: Query<&Body, With<Player>>, solid: Res<Solid>, mut horde: ResMut<Horde>) {
    launch(&mut commands, &mut horde);
    let chest = players.iter().next().map(|b| b.pos + Vec3::new(0.0, 1.1, 0.0));
    for (e, mut g) in &mut globs {
        g.prev = g.pos;
        g.vel.y -= GRAVITY * STEP;
        g.age += STEP;
        let step = g.vel * STEP;
        let len = step.length();
        let dir = step * (1.0 / len.max(1e-9));
        let wall = solid.0.raycast(g.pos, dir, len);
        let reach = wall.map_or(len, |h| h.t);
        if let Some(c) = chest
            && near_segment(g.pos, dir, reach, c) < GLOB_HITS
        {
            let push = Vec3::new(dir.x, 0.0, dir.z);
            let push = if push.length() > 1e-6 { push.normalize() } else { Vec3::ZERO };
            horde.blows.push(Blow { push: push * 0.4, damage: GLOB_DAMAGE, leaves: Some(Affliction::Poison) });
            horde.sounds.push((Sfx::Splat, c, 1.0));
            commands.entity(e).despawn();
            continue;
        }
        if let Some(h) = wall {
            if h.normal.y > 0.5 {
                commands.spawn(Puddle::new(h.point, PUDDLE, PUDDLE_FOR));
            }
            horde.sounds.push((Sfx::Splat, h.point, 0.9));
            commands.entity(e).despawn();
            continue;
        }
        g.pos += step;
        if g.age > 6.0 {
            commands.entity(e).despawn();
        }
    }
}

/// The puddles dry up; standing in one poisons.
pub fn fester(mut commands: Commands, mut puddles: Query<(Entity, &mut Puddle)>, players: Query<&Body, With<Player>>, mut horde: ResMut<Horde>) {
    let feet = players.iter().next().map(|b| b.pos);
    for (e, mut p) in &mut puddles {
        p.left -= STEP;
        if p.left <= 0.0 {
            commands.entity(e).despawn();
            continue;
        }
        if let Some(f) = feet
            && Vec2::new(f.x - p.at.x, f.z - p.at.z).length() < p.size()
            && (f.y - p.at.y).abs() < 0.8
        {
            horde.poisoned = true;
        }
    }
}

/// How near `p` comes to the segment from `a`, `len` along `dir`.
fn near_segment(a: Vec3, dir: Vec3, len: f64, p: Vec3) -> f64 {
    let t = (p - a).dot(dir).clamp(0.0, len);
    (a + dir * t - p).length()
}

/// How much a burst does at `d` metres, of `most`: all of it close, none
/// at its reach.
pub fn burst_share(d: f64) -> f64 {
    (1.0 - d / BURST_REACH).clamp(0.0, 1.0)
}

/// What globs and puddles look like (`spit.glb`).
#[derive(Resource, Clone, Copy)]
pub struct Meshes {
    glob: MeshId,
    puddle: MeshId,
}

/// Load what they look like, for `world`'s runs.
pub fn load(renderer: &mut Renderer, world: &mut World) {
    match crate::assets::load(renderer, "spit") {
        Ok(props) => {
            let find = |n: &str| props.iter().find(|p| p.name == n).and_then(|p| p.mesh);
            match (find("Glob"), find("Puddle")) {
                (Some(glob), Some(puddle)) => {
                    world.insert_resource(Meshes { glob, puddle });
                }
                _ => log_error!("spit: no Glob or Puddle"),
            }
        }
        Err(e) => log_error!("spit: {e}"),
    }
}

/// Queue every glob (where it is between its last two steps, by `alpha`,
/// turned along its flight) and puddle for drawing, a little aglow.
pub fn draw(world: &mut World, renderer: &mut Renderer, alpha: f64) {
    let Some(m) = world.get_resource::<Meshes>().copied() else { return };
    for g in world.query::<&Glob>().iter(world) {
        let at = g.prev + (g.pos - g.prev) * alpha;
        let spin = Quat::from_rotation_y((-g.vel.x).atan2(-g.vel.z));
        renderer.draw(Draw { mesh: m.glob, model: Mat4::from_translation(at) * Mat4::from_quat(spin), emissive: 0.35, fog: 1.0, tint: [1.0; 3] });
    }
    for p in world.query::<&Puddle>().iter(world) {
        let size = p.size();
        if size > 0.01 {
            let model = Mat4::from_translation(p.at) * Mat4::from_scale(Vec3::new(size, 1.0, size));
            renderer.draw(Draw { mesh: m.puddle, model, emissive: 0.2, fog: 1.0, tint: [1.0; 3] });
        }
    }
}

/// Every glob and puddle, gone (back to the title).
pub fn clear(world: &mut World) {
    let all: Vec<Entity> = world.query_filtered::<Entity, Or<(With<Glob>, With<Puddle>)>>().iter(world).collect();
    for e in all {
        world.despawn(e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Where a glob thrown `vel` from `from` comes down to `y`.
    fn lands(from: Vec3, vel: Vec3, y: f64) -> Vec3 {
        let (mut p, mut v) = (from, vel);
        while v.y > 0.0 || p.y > y {
            v.y -= GRAVITY * STEP;
            p += v * STEP;
        }
        p
    }

    #[test]
    fn a_throw_comes_down_where_it_was_aimed() {
        let from = Vec3::new(0.0, 1.55, 0.0);
        for to in [Vec3::new(10.0, 0.0, 0.0), Vec3::new(-6.0, 0.5, 12.0), Vec3::new(3.0, 1.0, -18.0)] {
            let at = lands(from, throw(from, to), to.y);
            assert!(Vec2::new(at.x - to.x, at.z - to.z).length() < 0.6, "aimed at {to:?}, came down at {at:?}");
        }
        // Too far to reach: as far as it goes, the right way.
        let v = throw(from, Vec3::new(0.0, 0.0, -80.0));
        assert!(v.z < 0.0 && v.y > 0.0);
    }

    #[test]
    fn a_puddle_dries_up_at_the_end() {
        let mut p = Puddle::new(Vec3::ZERO, 2.0, 5.0);
        assert_eq!(p.size(), 2.0);
        p.left = 0.0;
        assert_eq!(p.size(), 0.0);
        assert!(burst_share(0.0) == 1.0 && burst_share(BURST_REACH) == 0.0);
    }
}
