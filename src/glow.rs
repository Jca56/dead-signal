//! What's alight, told to the renderer each frame of a night: the lamps
//! (as bright as each is just now), the fires and what's burning, the
//! hounds, a rift opening, a stream of flame, the mast's beacon when it
//! flashes, a flare thrown for a drop, and a shot's or a blast's flash.

use bevy_ecs::prelude::*;
use lntrn_math::Vec3;

use crate::fx::Fx;
use crate::holdout::lamps::Lamp;
use crate::player::Body;
use crate::render::{Light, Renderer};
use crate::throw::flame::Puff;
use crate::throw::{Burning, Fire};
use crate::world::{Blink, Bounds, Look};
use crate::zombie::brain::Zombie;
use crate::zombie::figure::Figure;
use crate::zombie::kind::Kind;
use crate::zombie::rift::{OPENS_IN, Rift};

/// Fire's colour, a rift's, the beacon's (linear).
const FIRE: [f32; 3] = [1.0, 0.42, 0.10];
const RIFT: [f32; 3] = [0.55, 0.75, 1.0];
const BEACON: [f32; 3] = [1.0, 0.05, 0.03];

/// `colour`, `k` times as bright.
fn times(colour: [f32; 3], k: f64) -> [f32; 3] {
    colour.map(|c| c * k as f32)
}

/// A flame's flutter at `time`, about 1 (each its own, by `own`).
fn flutter(time: f64, own: f64) -> f64 {
    0.85 + 0.1 * (time * 11.0 + own * 5.3).sin() + 0.05 * (time * 27.0 + own * 2.1).sin()
}

/// Every light there is at `time`, to `renderer`.
pub fn gather(world: &mut World, fx: &Fx, renderer: &mut Renderer, time: f64) {
    for lamp in world.query::<&Lamp>().iter(world) {
        if let Some(light) = lamp.light(time) {
            renderer.light(light);
        }
    }
    for f in world.query::<&Fire>().iter(world) {
        let size = f.size();
        if size > 0.05 {
            renderer.light(Light::open(f.at + Vec3::new(0.0, 0.6, 0.0), 4.0 + size * 2.5, times(FIRE, 1.3 * flutter(time, f.at.x + f.at.z))));
        }
    }
    for (body, burning) in world.query::<(&Body, &Burning)>().iter(world) {
        renderer.light(Light::open(body.pos + Vec3::new(0.0, 1.1, 0.0), 4.5, times(FIRE, 0.9 * burning.left.min(1.0) * flutter(time, body.pos.x))));
    }
    for (z, f) in world.query::<(&Zombie, &Figure)>().iter(world) {
        if z.kind == Kind::Hound
            && !z.dead()
            && let Some([hips, neck, _]) = f.spine()
        {
            renderer.light(Light::open((hips + neck) * 0.5 + Vec3::new(0.0, 0.45, 0.0), 4.0, times(FIRE, 0.75 * flutter(time, hips.x * 3.7))));
        }
    }
    for r in world.query::<&Rift>().iter(world) {
        let open = (r.t / OPENS_IN).clamp(0.0, 1.0);
        let crackle = 0.6 + 0.4 * (time * 47.0 + r.at.x).sin().abs();
        renderer.light(Light::open(r.at + Vec3::new(0.0, 1.0, 0.0), 3.5 + 6.0 * open, times(RIFT, (0.5 + 1.4 * open) * crackle)));
    }
    // A stream of flame: a light for every third puff of it.
    for (k, p) in world.query::<&Puff>().iter(world).enumerate() {
        if k % 3 == 0 {
            renderer.light(Light::open(p.pos, 4.0, times(FIRE, 0.8)));
        }
    }
    // (Where the beacon's lamp is: the middle of what's drawn of it.)
    for (_, look, bounds) in world.query::<(&Blink, &Look, &Bounds)>().iter(world) {
        if look.emissive > 0.5 {
            renderer.light(Light::open(bounds.centre, 22.0, times(BEACON, 1.4)));
        }
    }
    crate::support::draw::lights(world, renderer, time);
    for light in fx.flashes() {
        renderer.light(light);
    }
}
