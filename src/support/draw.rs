//! What's called down, as drawn: a flare turning end over end in the air,
//! then lying where it stopped with its red flame and the smoke going up
//! from it; a crate swinging down under its parachute; and the crate
//! down, its lid off, the parachute fallen in a heap beside it. And what
//! they light of a night.

use bevy_ecs::prelude::*;
use lntrn_core::log_error;
use lntrn_math::{Mat4, Quat, Vec3};

use super::{Drop, Flare, SETTLES, STANDS};
use crate::render::{Draw, Light, MeshId, Renderer};

/// A flare's flame: how big, in the air and burning on the ground, and
/// what it's tinted (the flame's own is a fire's: this makes it a
/// flare's red).
const FLAME: (f64, f64) = (0.45, 0.8);
const RED: [f32; 3] = [1.0, 0.22, 0.42];
/// Its light, and how far it reaches.
const GLOW: [f32; 3] = [1.0, 0.10, 0.12];
const REACH: f64 = 11.0;
/// Its smoke: how many puffs there are in the column at once, how high
/// it goes, how long a puff takes to get there, and how thick it is.
const PUFFS: usize = 14;
const COLUMN: f64 = 12.0;
const RISES: f64 = 7.0;
const SMOKE: ([f32; 3], f32) = ([0.85, 0.16, 0.14], 0.3);
/// How high the crate's top is (where its cords meet).
const CRATE_TOP: f64 = 0.6;

/// What they look like (`airdrop.glb`), and a ball for the smoke.
#[derive(Resource, Clone, Copy)]
pub struct Meshes {
    flare: MeshId,
    shut: MeshId,
    open: MeshId,
    chute: MeshId,
    puff: MeshId,
}

/// Load what they look like, for `world`'s runs.
pub fn load(renderer: &mut Renderer, world: &mut World) {
    match crate::assets::load(renderer, "airdrop") {
        Ok(props) => {
            let find = |n: &str| props.iter().find(|p| p.name == n).and_then(|p| p.mesh);
            match (find("Flare"), find("Crate"), find("Open"), find("Chute")) {
                (Some(flare), Some(shut), Some(open), Some(chute)) => {
                    let puff = renderer.add_mesh(&crate::motes::ball());
                    world.insert_resource(Meshes { flare, shut, open, chute, puff });
                }
                _ => log_error!("airdrop: no Flare, Crate, Open or Chute"),
            }
        }
        Err(e) => log_error!("airdrop: {e}"),
    }
}

/// A flame's flutter at `time`, about 1.
fn flutter(time: f64, own: f64) -> f64 {
    1.0 + 0.25 * (time * 31.0 + own * 5.3).sin() + 0.15 * (time * 57.0 + own * 2.1).sin()
}

/// How a crate swings under its parachute, `age` into its fall: a turn
/// about where its cords meet the canopy.
fn swing(age: f64) -> Quat {
    Quat::from_rotation_z(0.11 * (age * 1.4).sin()) * Quat::from_rotation_x(0.07 * (age * 1.1 + 1.0).sin())
}

/// Queue everything called down for drawing, `alpha` between the last two
/// steps, at `time`.
pub fn draw(world: &mut World, renderer: &mut Renderer, alpha: f64, time: f64) {
    let (Some(m), Some(flames)) = (world.get_resource::<Meshes>().copied(), world.get_resource::<crate::throw::draw::Meshes>().copied()) else { return };
    let lit = |mesh, model| Draw { mesh, model, emissive: 0.0, fog: 1.0, tint: [1.0; 3] };
    for f in world.query::<&Flare>().iter(world) {
        let at = f.prev + (f.pos - f.prev) * alpha;
        let own = f.pos.x * 1.7 + f.pos.z;
        // In the air it tumbles; at rest it lies flat, its tip burning.
        let (turn, tip, size) = match f.rest {
            None => {
                let turn = Quat::from_rotation_y(f.age * 3.0) * Quat::from_rotation_x(f.spin);
                (turn, at + turn * Vec3::new(0.0, 0.22, 0.0), FLAME.0)
            }
            Some(_) => {
                let turn = Quat::from_rotation_y(own) * Quat::from_rotation_x(std::f64::consts::FRAC_PI_2 * 0.93);
                (turn, at + turn * Vec3::new(0.0, 0.22, 0.0), FLAME.1)
            }
        };
        renderer.draw(lit(m.flare, Mat4::from_translation(at) * Mat4::from_quat(turn)));
        let high = size * flutter(time, own);
        let flame = Mat4::from_translation(tip - Vec3::new(0.0, 0.03, 0.0)) * Mat4::from_quat(Quat::from_rotation_y(time * 5.0 + own)) * Mat4::from_scale(Vec3::new(size * 0.8, high, size * 0.8));
        renderer.draw(Draw { mesh: flames.flame, model: flame, emissive: 1.0, fog: 1.0, tint: RED });
        // The smoke, once it's settled: puffs going up one after another,
        // each swelling, leaning with the air, and thinning as it climbs.
        if let Some(rest) = f.rest {
            for k in 0..PUFFS {
                let share = (time / RISES + k as f64 / PUFFS as f64).fract();
                // (None older than the flare's been burning.)
                if share * RISES > rest {
                    continue;
                }
                // (Each a little off the last, and its own size: no stack
                // of beads.)
                let off = Vec3::new((own + k as f64 * 2.4).sin(), 0.0, (own * 1.3 + k as f64 * 1.7).cos()) * (0.12 + 0.5 * share);
                let lean = Vec3::new(0.9, 0.0, 0.3) * (share * share * 2.2) + off;
                let wide = (0.45 + 2.3 * share) * (0.8 + 0.4 * ((k * 5) % 7) as f64 / 6.0);
                let model = Mat4::from_translation(tip + Vec3::new(0.0, 0.3 + COLUMN * share, 0.0) + lean) * Mat4::from_scale(Vec3::new(wide, wide * 1.15, wide));
                let thin = (1.0 - share).powf(1.3) * (share * 8.0).min(1.0);
                renderer.draw_soft(Draw { mesh: m.puff, model, emissive: 0.25, fog: 1.0, tint: SMOKE.0 }, SMOKE.1 * thin as f32);
            }
        }
    }
    for d in world.query::<&Drop>().iter(world) {
        match d.down {
            None => {
                let height = d.prev + (d.height - d.prev) * alpha;
                // Swung about where the cords meet the canopy, well over it.
                let pivot = Vec3::new(0.0, CRATE_TOP + 3.4, 0.0);
                let hung = Mat4::from_translation(d.at + Vec3::new(0.0, height, 0.0) + pivot) * Mat4::from_quat(swing(d.age)) * Mat4::from_translation(-pivot);
                let turned = hung * Mat4::from_quat(Quat::from_rotation_y(d.at.x + d.age * 0.15));
                renderer.draw(lit(m.shut, turned));
                renderer.draw(lit(m.chute, turned * Mat4::from_translation(Vec3::new(0.0, CRATE_TOP, 0.0))));
            }
            Some(t) => {
                // Down: open, and at the last sinking away; its parachute
                // falling in a heap to one side, then gone.
                let sunk = ((t - (STANDS - 1.5)) / 1.5).clamp(0.0, 1.0);
                let stood = Mat4::from_translation(d.at - Vec3::new(0.0, sunk * 0.7, 0.0)) * Mat4::from_quat(Quat::from_rotation_y(d.at.x + d.age * 0.15));
                renderer.draw(lit(m.open, stood));
                // (It comes down beside the crate, flat and gathered in,
                // and after a while sinks away.)
                let fall = (t / SETTLES).clamp(0.0, 1.0);
                if t < SETTLES + 6.0 {
                    let (heap, drawn) = (1.0 - 0.95 * fall * fall, 1.0 - 0.35 * fall);
                    let gone = ((t - SETTLES - 4.0) / 2.0).clamp(0.0, 1.0);
                    let model = Mat4::from_translation(d.at + Vec3::new(1.9 * fall, CRATE_TOP * (1.0 - fall) - 0.12 * fall - gone * 0.5, 1.1 * fall)) * Mat4::from_scale(Vec3::new(drawn, heap, drawn));
                    renderer.draw(lit(m.chute, model));
                }
            }
        }
    }
}

/// What they light at `time`, to `renderer`: a flare's red.
pub fn lights(world: &mut World, renderer: &mut Renderer, time: f64) {
    for f in world.query::<&Flare>().iter(world) {
        let k = 1.5 * (0.8 + 0.2 * flutter(time, f.pos.x + f.pos.z));
        renderer.light(Light::open(f.pos + Vec3::new(0.0, 0.35, 0.0), REACH, GLOW.map(|c| c * k as f32)));
    }
}
