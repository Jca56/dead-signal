//! A mystery drop, as drawn: the question marks alight on its crate's
//! sides; and, the crate down, its gun over it: every gun it could be
//! flicking past as it rises out of the crate, then the one it is, turning
//! slowly, alight of itself against the night (one that's been amplified,
//! in the Amplifier's amber), a glow about it and a light out of the crate
//! under it.

use std::f64::consts::{FRAC_PI_2, PI};

use bevy_ecs::prelude::*;
use lntrn_math::{Mat4, Quat, Vec3};

use crate::assets::Prop;
use crate::loot::Kind;
use crate::render::{Draw, Light, MeshId, Renderer, Vertex};
use crate::support::mystery::{Prize, SHUFFLES};

/// A mystery's own colour; and its smoke's.
const VIOLET: [f32; 3] = [0.66, 0.30, 1.0];
pub(super) const SMOKE: [f32; 3] = [0.56, 0.24, 0.92];
/// How much of its own colour a gun shows whatever the light, and how
/// much of the mystery's (or the Amplifier's) with it.
const SEEN: (f32, f32) = (0.75, 0.16);
/// How far up out of the crate it rises as the guns flick past; how fast
/// it turns then, and once it's the one it is; how far it bobs; and how
/// much bigger than life it's drawn (it's to be made out from a way off).
const RISES: f64 = 0.6;
const TURNS: (f64, f64) = (7.0, 1.3);
const BOBS: f64 = 0.04;
const BIGGER: f64 = 1.2;
/// The glow about it: how big, and how thick.
const HALO: (f64, f32) = (1.1, 0.24);
/// A question mark: how tall, how far it stands off the crate's side, and
/// how high up the side its middle is.
const MARK: (f64, f64, f64) = (0.3, 0.036, 0.29);

/// The guns a mystery drop could hold, each alight of itself (plain, and
/// amplified); and the glow about one (a mystery's, an amplified one's).
#[derive(Resource, Clone)]
pub struct Guns {
    seen: Vec<(Kind, [MeshId; 2])>,
    halo: [MeshId; 2],
}

/// The Amplifier's first glow (what a mystery drop's amplified gun has).
fn amber() -> [f32; 3] {
    crate::weapon::amp::glow(1).unwrap_or([1.0, 0.62, 0.14])
}

/// Make them, for `world`'s runs: each of `kinds` from its model among
/// the things there are (`items`).
pub fn load(renderer: &mut Renderer, world: &mut World, items: &[Prop], kinds: &[Kind]) {
    let amber = amber();
    let mut alight = |p: &Prop, glow: [f32; 3]| {
        let seen: Vec<Vertex> = p.vertices.iter().map(|v| Vertex { emissive: [0, 1, 2].map(|i| v.color[i] * SEEN.0 + glow[i] * SEEN.1), ..*v }).collect();
        renderer.add_mesh(&seen)
    };
    let seen = kinds.iter().filter_map(|&kind| items.iter().find(|p| p.name == kind.def().model).map(|p| (kind, [alight(p, VIOLET), alight(p, amber)]))).collect();
    let mut ball = |glow: [f32; 3]| renderer.add_mesh(&crate::motes::ball().into_iter().map(|v| Vertex { emissive: glow, ..v }).collect::<Vec<_>>());
    let halo = [ball(VIOLET), ball(amber)];
    world.insert_resource(Guns { seen, halo });
}

/// A question mark, alight: `MARK.0` tall, flat, facing +Z, about its
/// middle.
pub(super) fn question() -> Vec<Vertex> {
    // (On a grid five across and seven up.)
    let cells: [(f64, f64, f64, f64); 6] = [(1.0, 6.0, 4.0, 7.0), (0.0, 5.0, 1.0, 6.0), (4.0, 4.0, 5.0, 6.0), (3.0, 3.0, 4.0, 4.0), (2.0, 2.0, 3.0, 3.0), (2.0, 0.0, 3.0, 1.0)];
    let u = MARK.0 / 7.0;
    let mut out = Vec::new();
    for (x0, y0, x1, y1) in cells {
        let (lo, hi) = (Vec3::new((x0 - 2.5) * u, (y0 - 3.5) * u, -0.008), Vec3::new((x1 - 2.5) * u, (y1 - 3.5) * u, 0.008));
        for [a, b, c] in crate::map::build::box_tris(lo, hi) {
            let n = (b - a).cross(c - a).normalize();
            for p in [a, b, c] {
                out.push(Vertex { pos: [p.x as f32, p.y as f32, p.z as f32], normal: [n.x as f32, n.y as f32, n.z as f32], color: [VIOLET[0], VIOLET[1], VIOLET[2], 1.0], emissive: VIOLET });
            }
        }
    }
    out
}

/// The question marks (`mesh`) on a mystery drop's crate, placed by
/// `model`, at `time`: one on each side (its lid off, none on the side
/// the lid leans against).
pub(super) fn marks(renderer: &mut Renderer, mesh: MeshId, model: Mat4, open: bool, time: f64) {
    // (The crate's 0.9 m long and 0.6 deep.)
    let (long, deep) = (0.45 + MARK.1, 0.3 + MARK.1);
    let sides = [(Vec3::new(0.0, MARK.2, -deep), PI), (Vec3::new(long, MARK.2, 0.0), FRAC_PI_2), (Vec3::new(-long, MARK.2, 0.0), -FRAC_PI_2), (Vec3::new(0.0, MARK.2, deep), 0.0)];
    let glows = 0.7 + 0.3 * (time * 3.0).sin();
    for (at, yaw) in sides.into_iter().take(if open { 3 } else { 4 }) {
        renderer.draw(Draw { mesh, model: model * Mat4::from_translation(at) * Mat4::from_quat(Quat::from_rotation_y(yaw)), emissive: glows as f32, fog: 0.5, tint: [1.0; 3] });
    }
}

/// How far up out of its crate `p`'s got, 0–1.
fn risen(p: &Prize) -> f64 {
    let t = (p.age / SHUFFLES).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Where `p` is at `time`.
fn place(p: &Prize, time: f64) -> Vec3 {
    p.at - Vec3::new(0.0, RISES * (1.0 - risen(p)) - BOBS * (time * 2.1 + p.at.x).sin(), 0.0)
}

/// Every mystery drop's gun over its crate, at `time`.
pub fn draw(world: &mut World, renderer: &mut Renderer, time: f64) {
    let Some(guns) = world.get_resource::<Guns>().cloned() else { return };
    for p in world.query::<&Prize>().iter(world).filter(|p| p.shows()) {
        let amplified = usize::from(p.gun.tier > 0 && p.settled());
        // The gun it is; or, till then, one of the others, the next with
        // every flick.
        let others: Vec<&(Kind, [MeshId; 2])> = guns.seen.iter().filter(|(kind, _)| *kind != p.gun.kind).collect();
        let shown = match p.settled() || others.is_empty() {
            true => guns.seen.iter().find(|(kind, _)| *kind == p.gun.kind),
            false => Some(others[(p.flick() as usize + (p.at.x.abs() * 7.0) as usize) % others.len()]),
        };
        let Some((_, meshes)) = shown else { continue };
        let at = place(p, time);
        // (Its top's its own +Z: stood up, its muzzle a little raised.)
        let turn = Quat::from_rotation_y(time * if p.settled() { TURNS.1 } else { TURNS.0 }) * Quat::from_rotation_z(0.3) * Quat::from_rotation_x(-FRAC_PI_2);
        renderer.draw(Draw { mesh: meshes[amplified], model: Mat4::from_translation(at) * Mat4::from_quat(turn) * Mat4::from_scale(Vec3::splat(BIGGER)), emissive: 1.0, fog: 0.5, tint: [1.0; 3] });
        let wide = HALO.0 * (0.9 + 0.1 * (time * 4.0).sin());
        renderer.draw_soft(Draw { mesh: guns.halo[amplified], model: Mat4::from_translation(at) * Mat4::from_scale(Vec3::splat(wide)), emissive: 1.0, fog: 0.5, tint: [1.0; 3] }, HALO.1);
    }
}

/// What they light at `time`, to `renderer`: the gun and its crate, from
/// just over it (flickering as the guns flick past).
pub fn lights(world: &mut World, renderer: &mut Renderer, time: f64) {
    for p in world.query::<&Prize>().iter(world).filter(|p| p.shows()) {
        let colour = if p.gun.tier > 0 && p.settled() { amber() } else { VIOLET };
        let k = if p.settled() { 1.5 + 0.2 * (time * 4.0).sin() } else { 1.0 + 0.9 * f64::from(p.flick() % 2) };
        renderer.light(Light::open(place(p, time) + Vec3::new(0.0, 0.5, 0.0), 8.0, colour.map(|c| c * k as f32)));
    }
}
