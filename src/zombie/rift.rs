//! How a Hellhound comes: not by a window but out of the air, in a tear
//! of static. A rift crackles open where one's about to be, long enough to
//! be seen and backed away from; then it's there, and coming.

use bevy_ecs::prelude::*;
use lntrn_math::{Mat4, Quat, Vec3};

use super::brain::{State, Zombie};
use super::kind::Kind;
use super::looks::Theme;
use super::{Horde, Nav};
use crate::player::STEP;
use crate::render::{Draw, Renderer};
use crate::sound::Sfx;
use crate::throw::draw::Meshes;

/// How long a rift crackles before the hound's through it.
pub const OPENS_IN: f64 = 1.4;
/// A rift opens this near to this far from the one it's for, and never
/// nearer than `CLEAR` to anyone; where there's no room for that (a
/// small room, its door shut), nearer.
const RING: (f64, f64) = (7.0, 14.0);
const CLEAR: f64 = 5.0;
const CRAMPED: (f64, f64) = (3.0, 7.0);
const CLEAR_CRAMPED: f64 = 2.5;

/// A tear in the air at `at`, `t` seconds open; the hound that comes of
/// it is this tough, and faces this way.
#[derive(Component, Clone, Copy, Debug)]
pub struct Rift {
    pub at: Vec3,
    pub t: f64,
    hp: f64,
    yaw: f64,
}

/// Somewhere a rift can open for the player standing at `feet`: on a floor
/// near their level, a way from it to them, and not on top of anyone
/// (`others`: every player's feet). By luck `roll` (0–1, twice a try).
pub fn spot(world: &World, feet: Vec3, others: &[Vec3], mut roll: impl FnMut() -> f64) -> Option<Vec3> {
    let nav = world.resource::<Nav>().0.as_ref()?;
    (0..80).find_map(|i| {
        let ((near, far), clear) = if i < 40 { (RING, CLEAR) } else { (CRAMPED, CLEAR_CRAMPED) };
        let (a, r) = (roll() * std::f64::consts::TAU, near + (far - near) * roll());
        let p = Vec3::new(feet.x + a.cos() * r, feet.y, feet.z + a.sin() * r);
        let floor = nav.height_at(p).filter(|h| (h - feet.y).abs() < 1.5)?;
        let at = Vec3::new(p.x, floor, p.z);
        (nav.walkable(at) && nav.connects(at, feet) && others.iter().all(|o| (*o - at).length() > clear)).then_some(at)
    })
}

/// Open a rift at `at` for a hound with `hp`, to come out facing `toward`.
pub fn open(world: &mut World, at: Vec3, toward: Vec3, hp: f64) {
    let yaw = (-(toward.x - at.x)).atan2(-(toward.z - at.z));
    world.spawn(Rift { at, t: 0.0, hp, yaw });
    world.resource_mut::<Horde>().sounds.push((Sfx::Static, at + Vec3::new(0.0, 1.0, 0.0), 1.0));
}

/// How many rifts are still to open.
pub fn pending(world: &mut World) -> usize {
    world.query::<&Rift>().iter(world).count()
}

/// Every rift, gone (back to the title).
pub fn clear(world: &mut World) {
    let all: Vec<Entity> = world.query_filtered::<Entity, With<Rift>>().iter(world).collect();
    for e in all {
        world.despawn(e);
    }
}

/// A step of the rifts: each crackles on, and once it's open the hound's
/// there, relentless, and the rift's gone.
pub(super) fn step(world: &mut World) {
    let rifts: Vec<(Entity, Rift)> = world.query::<(Entity, &Rift)>().iter(world).map(|(e, r)| (e, *r)).collect();
    for (e, mut r) in rifts {
        r.t += STEP;
        if r.t < OPENS_IN {
            if let Some(mut now) = world.get_mut::<Rift>(e) {
                *now = r;
            }
            continue;
        }
        world.despawn(e);
        let hound = super::spawn_kind(world, r.at, r.yaw, Kind::Hound, Theme::Drifter);
        if let Some(mut z) = world.get_mut::<Zombie>(hound) {
            z.hp = r.hp;
            z.relentless = true;
            z.state = State::Hunt;
        }
        let mut horde = world.resource_mut::<Horde>();
        horde.sounds.push((Sfx::Zap, r.at + Vec3::new(0.0, 0.8, 0.0), 1.0));
        horde.sounds.push((Sfx::Bark, r.at + Vec3::new(0.0, 0.6, 0.0), 1.0));
    }
}

/// A number from `k` and `time`, 0–1, the same within a twelfth of a
/// second: what static flickers by.
fn flicker(k: u32, time: f64) -> f64 {
    let mut h = k.wrapping_mul(0x9E37_79B9) ^ ((time * 12.0) as u32).wrapping_mul(0x85EB_CA6B);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    f64::from(h & 0xFFFF) / 65535.0
}

/// Queue the rifts for drawing at `time`: a ring on the ground drawing in,
/// and shards of white static jumping about a column over it, more and
/// higher as it opens.
pub fn draw(world: &mut World, renderer: &mut Renderer, time: f64) {
    let Some(m) = world.get_resource::<Meshes>().copied() else { return };
    const PALE: [f32; 3] = [0.75, 0.88, 1.0];
    for r in world.query::<&Rift>().iter(world) {
        let open = (r.t / OPENS_IN).clamp(0.0, 1.0);
        let seed = (r.at.x * 7.3 + r.at.z * 3.1).abs() as u32;
        let ring = Mat4::from_translation(r.at + Vec3::new(0.0, 0.03, 0.0)) * Mat4::from_scale(Vec3::splat(2.2 - 1.4 * open));
        renderer.draw(Draw { mesh: m.ring, model: ring, emissive: 1.0, fog: 0.4, tint: PALE });
        let shards = 5 + (9.0 * open) as u32;
        for k in 0..shards {
            let f = |n: u32| flicker(seed.wrapping_add(k * 16 + n), time);
            let (a, out, up) = (f(0) * std::f64::consts::TAU, 0.15 + 0.55 * f(1), 0.1 + (0.5 + 1.3 * open) * f(2));
            let at = r.at + Vec3::new(a.cos() * out, up, a.sin() * out);
            let turn = Quat::from_rotation_y(f(3) * std::f64::consts::TAU) * Quat::from_rotation_z((f(4) - 0.5) * 3.0);
            let size = 1.2 + 2.2 * f(5) * (0.5 + open);
            renderer.draw(Draw { mesh: m.flash, model: Mat4::from_translation(at) * Mat4::from_quat(turn) * Mat4::from_scale(Vec3::splat(size)), emissive: 1.0, fog: 0.4, tint: PALE });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::Body;

    #[test]
    fn a_rift_crackles_open_and_a_hound_comes_of_it_hunting() {
        let mut world = World::new();
        world.insert_resource(Horde::default());
        open(&mut world, Vec3::new(4.0, 0.0, 0.0), Vec3::ZERO, 140.0);
        assert_eq!((pending(&mut world), super::super::alive(&mut world)), (1, 0));
        let steps = (OPENS_IN / STEP) as usize;
        for _ in 0..steps - 2 {
            step(&mut world);
        }
        assert_eq!((pending(&mut world), super::super::alive(&mut world)), (1, 0), "not yet");
        for _ in 0..4 {
            step(&mut world);
        }
        assert_eq!(pending(&mut world), 0);
        let (z, body) = world.query::<(&Zombie, &Body)>().single(&world).expect("the hound");
        assert_eq!((z.kind, z.hp, z.relentless, z.barrier), (Kind::Hound, 140.0, true, None));
        assert!(matches!(z.state, State::Hunt) && body.pos == Vec3::new(4.0, 0.0, 0.0));
        // Facing the one it came for (off along -x: a yaw of a quarter turn).
        assert!((z.yaw - std::f64::consts::FRAC_PI_2).abs() < 1e-9, "{}", z.yaw);
        let said: Vec<Sfx> = world.resource::<Horde>().sounds.iter().map(|s| s.0).collect();
        assert_eq!(said, vec![Sfx::Static, Sfx::Zap, Sfx::Bark]);
    }
}
