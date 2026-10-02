//! What's thrown and what it's done, as drawn: the thing turning end over
//! end in the air (a pipe bomb's light blinking as it beeps), the fires
//! (a ring of flickering tongues, dying down at the end), the dead on fire
//! (and a hound, which always is: flames down its back),
//! a flamethrower's stream (a puff of flame swelling as it flies),
//! and, while a throw's being aimed, its arc dot by dot and a ring where
//! it'll land.

use bevy_ecs::prelude::*;
use lntrn_core::log_error;
use lntrn_math::{Mat4, Quat, Vec3};

use super::flame::Puff;
use super::{Burning, Fire, Thrown};
use crate::player::Body;
use crate::render::{Draw, MeshId, Renderer, Vertex};
use crate::zombie::brain::Zombie;
use crate::zombie::figure::Figure;
use crate::zombie::kind::Kind;

/// What flames, the arc's dots and the landing ring look like, and a
/// shot's muzzle flash (a buddy's gun, as the others see it)
/// (`effects.glb`).
#[derive(Resource, Clone, Copy)]
pub struct Meshes {
    pub flame: MeshId,
    dot: MeshId,
    pub ring: MeshId,
    pub flash: MeshId,
}

/// Load what they look like, for `world`'s runs.
pub fn load(renderer: &mut Renderer, world: &mut World) {
    match crate::assets::load(renderer, "effects") {
        Ok(props) => {
            // (Every one of them is a light of its own: its colours are
            // what it glows, whatever's on it.)
            let mut find = |n: &str| {
                props.iter().find(|p| p.name == n).map(|p| {
                    let glowing: Vec<Vertex> = p.vertices.iter().map(|v| Vertex { emissive: [v.color[0], v.color[1], v.color[2]], ..*v }).collect();
                    renderer.add_mesh(&glowing)
                })
            };
            match (find("Flame"), find("Dot"), find("Ring"), find("Flash")) {
                (Some(flame), Some(dot), Some(ring), Some(flash)) => {
                    world.insert_resource(Meshes { flame, dot, ring, flash });
                }
                _ => log_error!("effects: no Flame, Dot, Ring or Flash"),
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
    for p in world.query::<&Puff>().iter(world) {
        // Small at the muzzle, swelling as it goes, guttering at the end.
        let (at, share) = (p.prev + (p.pos - p.prev) * alpha, (p.age / p.life).clamp(0.0, 1.0));
        let size = (0.22 + 1.0 * share) * ((1.0 - share) / 0.2).min(1.0);
        flame(renderer, m.flame, at - Vec3::new(0.0, size * 0.45, 0.0), size, time, p.seed * 40.0);
    }
    for (b, burning) in world.query::<(&Body, &Burning)>().iter(world) {
        let fade = burning.left.min(1.0);
        for k in 0..3 {
            let a = k as f64 * 2.1 + time * 0.7;
            flame(renderer, m.flame, b.pos + Vec3::new(a.cos() * 0.15, 0.4 + 0.45 * k as f64, a.sin() * 0.15), 1.1 * fade, time, 20.0 + k as f64);
        }
    }
    // A hound burns as it goes: flames from its rump to its shoulders,
    // the biggest between them.
    for (z, f) in world.query::<(&Zombie, &Figure)>().iter(world) {
        if z.kind != Kind::Hound || z.dead() {
            continue;
        }
        let Some([hips, neck, _]) = f.spine() else { continue };
        let own = hips.x * 3.7 + hips.z * 1.3;
        for (k, (along, size)) in [(0.1, 0.55), (0.45, 0.8), (0.8, 0.65)].into_iter().enumerate() {
            flame(renderer, m.flame, hips + (neck - hips) * along + Vec3::new(0.0, 0.12, 0.0), size, time, own + k as f64);
        }
    }
}

/// The arc a throw would take (its `dots`), and a ring where it'd come
/// down: in the thrower's pane only.
pub fn aim(world: &World, renderer: &mut Renderer, pane: usize, dots: &[Vec3], lands: Option<Vec3>) {
    let Some(m) = world.get_resource::<Meshes>().copied() else { return };
    for &p in dots {
        renderer.draw_in(pane, Draw { mesh: m.dot, model: Mat4::from_translation(p), emissive: 0.9, fog: 0.3, tint: [1.0; 3] });
    }
    if let Some(p) = lands {
        renderer.draw_in(pane, Draw { mesh: m.ring, model: Mat4::from_translation(p), emissive: 0.9, fog: 0.3, tint: [1.0, 0.8, 0.4] });
    }
}
