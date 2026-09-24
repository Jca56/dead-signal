//! What's thrown and what it's done, as drawn: the thing turning end over
//! end in the air (a pipe bomb's light blinking as it beeps), the fires
//! (a ring of flickering tongues, dying down at the end), the dead on fire,
//! and, while a throw's being aimed, its arc dot by dot and a ring where
//! it'll land.

use bevy_ecs::prelude::*;
use lntrn_core::log_error;
use lntrn_math::{Mat4, Quat, Vec3};

use super::{Burning, Fire, Thrown};
use crate::player::Body;
use crate::render::{Draw, MeshId, Renderer};

/// What flames, the arc's dots and the landing ring look like
/// (`effects.glb`).
#[derive(Resource, Clone, Copy)]
pub struct Meshes {
    flame: MeshId,
    dot: MeshId,
    ring: MeshId,
}

/// Load what they look like, for `world`'s runs.
pub fn load(renderer: &mut Renderer, world: &mut World) {
    match crate::assets::load(renderer, "effects") {
        Ok(props) => {
            let find = |n: &str| props.iter().find(|p| p.name == n).and_then(|p| p.mesh);
            match (find("Flame"), find("Dot"), find("Ring")) {
                (Some(flame), Some(dot), Some(ring)) => {
                    world.insert_resource(Meshes { flame, dot, ring });
                }
                _ => log_error!("effects: no Flame, Dot or Ring"),
            }
        }
        Err(e) => log_error!("effects: {e}"),
    }
}

/// A flame at `at`, `size` tall, flickering by `time` (and `seed`, so no
/// two flicker together).
fn flame(renderer: &mut Renderer, m: MeshId, at: Vec3, size: f64, time: f64, seed: f64) {
    let flick = 1.0 + 0.25 * (time * 9.0 + seed * 7.1).sin() + 0.15 * (time * 23.0 + seed * 3.3).sin();
    let model = Mat4::from_translation(at) * Mat4::from_quat(Quat::from_rotation_y(seed * 2.4 + time * 1.5)) * Mat4::from_scale(Vec3::new(size, size * flick, size));
    renderer.draw(Draw { mesh: m, model, emissive: 0.85, fog: 1.0, tint: [1.0; 3] });
}

/// Queue everything thrown and burning for drawing, `alpha` between the
/// last two steps, at `time`.
pub fn draw(world: &mut World, renderer: &mut Renderer, alpha: f64, time: f64) {
    let Some(m) = world.get_resource::<Meshes>().copied() else { return };
    let items = world.get_resource::<crate::items::Meshes>().map(|i| i.0.clone()).unwrap_or_default();
    for t in world.query::<&Thrown>().iter(world) {
        let at = t.prev + (t.pos - t.prev) * alpha;
        if let Some(&mesh) = items.get(&t.what.kind()) {
            let turn = Quat::from_rotation_y(t.age * 2.0) * Quat::from_rotation_x(t.spin);
            renderer.draw(Draw { mesh, model: Mat4::from_translation(at) * Mat4::from_quat(turn), emissive: 0.0, fog: 1.0, tint: [1.0; 3] });
        }
        if t.lit > 0.0 {
            renderer.draw(Draw { mesh: m.dot, model: Mat4::from_translation(at + Vec3::new(0.0, 0.08, 0.0)) * Mat4::from_scale(Vec3::splat(1.6)), emissive: 1.0, fog: 1.0, tint: [1.0, 0.15, 0.1] });
        }
    }
    for f in world.query::<&Fire>().iter(world) {
        let size = f.size();
        if size < 0.05 {
            continue;
        }
        // A ring of flames and a few in the middle, as many as it's wide.
        let n = (size * 4.0).ceil() as usize;
        for k in 0..n {
            let a = k as f64 / n as f64 * std::f64::consts::TAU;
            let r = size * (0.55 + 0.35 * ((k * 7) % 5) as f64 / 4.0);
            flame(renderer, m.flame, f.at + Vec3::new(a.cos() * r, 0.0, a.sin() * r), 0.9 + 0.5 * ((k * 3) % 4) as f64 / 3.0, time, k as f64);
        }
        for k in 0..3 {
            let a = k as f64 * 2.1;
            flame(renderer, m.flame, f.at + Vec3::new(a.cos(), 0.0, a.sin()) * (size * 0.25), 1.5, time, 10.0 + k as f64);
        }
    }
    for (b, burning) in world.query::<(&Body, &Burning)>().iter(world) {
        let fade = burning.0.min(1.0);
        for k in 0..3 {
            let a = k as f64 * 2.1 + time * 0.7;
            flame(renderer, m.flame, b.pos + Vec3::new(a.cos() * 0.15, 0.4 + 0.45 * k as f64, a.sin() * 0.15), 1.1 * fade, time, 20.0 + k as f64);
        }
    }
}

/// The arc a throw would take (its `dots`), and a ring where it'd come
/// down.
pub fn aim(world: &World, renderer: &mut Renderer, dots: &[Vec3], lands: Option<Vec3>) {
    let Some(m) = world.get_resource::<Meshes>().copied() else { return };
    for &p in dots {
        renderer.draw(Draw { mesh: m.dot, model: Mat4::from_translation(p), emissive: 0.9, fog: 0.3, tint: [1.0; 3] });
    }
    if let Some(p) = lands {
        renderer.draw(Draw { mesh: m.ring, model: Mat4::from_translation(p), emissive: 0.9, fog: 0.3, tint: [1.0, 0.8, 0.4] });
    }
}
