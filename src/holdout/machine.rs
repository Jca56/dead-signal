//! The Amplifier: the old station's valve amplifier, down in the bunker,
//! still warm. A weapon put through it comes out more than it was
//! (`weapon/amp.rs`), for a great many points. It stands against a wall
//! like something for sale there (`Wares::Amplifier`): a cabinet (solid,
//! part of the map), and over that its panel, its valves alight on top
//! and the cradle a weapon's laid in. And the signs that point the way
//! to it: the bars of a signal growing stronger, and an arrow.

use bevy_ecs::prelude::*;
use lntrn_math::{Mat4, Quat, Vec3};

use super::arena::{Arena, Sign, Wares, Way};
use super::lamps::Lamp;
use super::props::boxes;
use crate::collide::Surface;
use crate::map::building::shape::{Block, Rgb, Stuff};
use crate::render::{Light, MeshId, Vertex};
use crate::world::{Look, Model, OnMap, Placed};

/// The cabinet: how wide, how deep (out from the wall), how tall.
pub const WIDE: f64 = 1.3;
pub const DEEP: f64 = 0.5;
pub const TALL: f64 = 1.9;
/// Colours, as they look (sRGB): the cabinet's paint, its panel, bare
/// steel, and what's alight (its valves and dial, the cradle).
const CABINET: Rgb = [0.19, 0.23, 0.20];
const PANEL: [f32; 3] = [0.10, 0.11, 0.11];
const STEEL: [f32; 3] = [0.42, 0.43, 0.42];
const VALVE: [f32; 3] = [1.0, 0.62, 0.18];
const CRADLE: [f32; 3] = [0.62, 0.34, 0.95];
/// Its light: how far it reaches, and its colour (linear).
const GLOW: (f64, [f32; 3]) = (6.0, [0.95, 0.50, 0.14]);
/// A sign: how wide, how tall, and how high its middle is over a wall
/// buy's.
const SIGN: (f64, f64) = (0.95, 0.38);
pub const SIGN_UP: f64 = 0.4;

/// The cabinet standing against the wall at `wall` (on the wall's face,
/// `up` over the floor) facing `out`: a solid box.
pub fn cabinet(wall: Vec3, out: Vec3, up: f64) -> Block {
    let along = Vec3::new(-out.z, 0.0, out.x);
    let (p, q) = (wall - along * (WIDE * 0.5) - Vec3::new(0.0, up, 0.0), wall + along * (WIDE * 0.5) + out * DEEP + Vec3::new(0.0, TALL - up, 0.0));
    Block { lo: p.min(q), hi: p.max(q), colour: CABINET, stuff: Stuff::Solid(Surface::Metal) }
}

/// A frame whose z is `out` (flat), x along, y up, at `at`.
fn facing(at: Vec3, out: Vec3) -> Mat4 {
    Mat4::from_translation(at) * Mat4::from_quat(Quat::from_rotation_y(out.x.atan2(out.z)))
}

/// `list` as a mesh alight: each box its own colour's light.
fn alight(list: &[(Vec3, Vec3, [f32; 3], Mat4)], bright: f32) -> Vec<Vertex> {
    let mut out = boxes(list);
    for v in &mut out {
        v.emissive = [v.color[0] * bright, v.color[1] * bright, v.color[2] * bright];
    }
    out
}

/// What's on the cabinet, about the middle of its front (`up` over the
/// floor): what's dull, and what's alight.
fn fittings(up: f64) -> (Vec<Vertex>, Vec<Vertex>) {
    let b = |x0: f64, y0: f64, z0: f64, x1: f64, y1: f64, z1: f64, c: [f32; 3]| (Vec3::new(x0, y0, z0), Vec3::new(x1, y1, z1), c, Mat4::IDENTITY);
    let (top, foot) = (TALL - up, -up);
    let mut dull = vec![
        // The panel, and a rail under it.
        b(-0.58, -0.20, 0.0, 0.58, 0.38, 0.02, PANEL),
        b(-0.60, -0.26, 0.0, 0.60, -0.22, 0.04, STEEL),
        // The cradle: a shelf, and two forks a weapon's laid across.
        b(-0.48, -0.66, 0.0, 0.48, -0.62, 0.24, STEEL),
        b(-0.34, -0.62, 0.14, -0.30, -0.44, 0.18, STEEL),
        b(0.30, -0.62, 0.14, 0.34, -0.44, 0.18, STEEL),
        // The valves' caps.
        b(-0.43, top + 0.27, -0.33, -0.27, top + 0.30, -0.17, PANEL),
        b(-0.08, top + 0.27, -0.33, 0.08, top + 0.30, -0.17, PANEL),
        b(0.27, top + 0.27, -0.33, 0.43, top + 0.30, -0.17, PANEL),
        // The dial's needle.
        (Vec3::new(-0.01, 0.0, 0.0), Vec3::new(0.01, 0.11, 0.008), PANEL, Mat4::from_translation(Vec3::new(-0.28, 0.06, 0.036)) * Mat4::from_quat(Quat::from_rotation_z(-0.6))),
    ];
    // The grille over its speaker: slats.
    for k in 0..6 {
        let y = foot + 0.22 + 0.08 * f64::from(k);
        dull.push(b(-0.46, y, 0.0, 0.46, y + 0.035, 0.025, PANEL));
    }
    let mut lit = vec![
        // The dial, and the cradle's back.
        b(-0.44, 0.02, 0.02, -0.12, 0.30, 0.035, VALVE),
        b(-0.46, -0.60, 0.0, 0.46, -0.34, 0.012, CRADLE),
        // The valves.
        b(-0.41, top, -0.31, -0.29, top + 0.27, -0.19, VALVE),
        b(-0.06, top, -0.31, 0.06, top + 0.27, -0.19, VALVE),
        b(0.29, top, -0.31, 0.41, top + 0.27, -0.19, VALVE),
    ];
    // The meter: bars, each taller than the last.
    for k in 0..5 {
        let x = 0.02 + 0.105 * f64::from(k);
        lit.push(b(x, 0.03, 0.02, x + 0.07, 0.09 + 0.055 * f64::from(k), 0.035, if k < 3 { VALVE } else { CRADLE }));
    }
    (boxes(&dull), alight(&lit, 0.9))
}

/// A sign's mesh, pointing `way`: its plate, and (alight) a signal's bars
/// and an arrow.
fn sign(way: Way) -> (Vec<Vertex>, Vec<Vertex>) {
    let (w, h) = SIGN;
    let plate = boxes(&[(Vec3::new(-w * 0.5, -h * 0.5, 0.0), Vec3::new(w * 0.5, h * 0.5, 0.02), PANEL, Mat4::IDENTITY)]);
    let mut lit = Vec::new();
    for k in 0..4 {
        let x = -w * 0.5 + 0.07 + 0.075 * f64::from(k);
        lit.push((Vec3::new(x, -0.11, 0.02), Vec3::new(x + 0.05, -0.05 + 0.055 * f64::from(k), 0.03), VALVE, Mat4::IDENTITY));
    }
    // The arrow, made pointing right about its own middle, then turned.
    let turn = Mat4::from_translation(Vec3::new(0.17, 0.0, 0.0))
        * Mat4::from_quat(Quat::from_rotation_z(match way {
            Way::Right => 0.0,
            Way::Left => std::f64::consts::PI,
            Way::Down => -std::f64::consts::FRAC_PI_2,
        }));
    lit.push((Vec3::new(-0.16, -0.022, 0.02), Vec3::new(0.13, 0.022, 0.03), VALVE, turn));
    for side in [1.0, -1.0] {
        let barb = turn * Mat4::from_translation(Vec3::new(0.16, 0.0, 0.0)) * Mat4::from_quat(Quat::from_rotation_z(side * 2.45));
        lit.push((Vec3::new(0.0, -0.022, 0.02), Vec3::new(0.15, 0.022, 0.03), VALVE, barb));
    }
    (plate, alight(&lit, 0.8))
}

/// Set the Amplifier's fittings down (wherever the arena has one), its
/// light with them, and every sign; their meshes made with `mesh`.
pub fn spawn(world: &mut World, arena: &Arena, up: f64, mut mesh: impl FnMut(&[Vertex]) -> MeshId) {
    let bright = Look { emissive: 1.0, fog: 0.8, tint: [1.0; 3] };
    for b in arena.buys.iter().filter(|b| b.wares == Wares::Amplifier) {
        let (dull, lit) = fittings(up);
        let frame = facing(b.at, b.facing);
        world.spawn((Placed(frame), Model(mesh(&dull)), Look::default(), OnMap));
        // Its glow lights the room it's in, and no further.
        let at = b.at + b.facing * 0.5 + Vec3::new(0.0, 0.3, 0.0);
        let room = arena.lamps.iter().filter_map(|l| l.room).find(|(lo, hi)| at.x > lo.x && at.x < hi.x && at.y > lo.y && at.y < hi.y && at.z > lo.z && at.z < hi.z);
        let light = Light { at, radius: GLOW.0, color: GLOW.1, within: room };
        world.spawn((Lamp::steady(light), Placed(frame), Model(mesh(&lit)), bright, OnMap));
    }
    let mut made: Vec<(Way, MeshId, MeshId)> = Vec::new();
    for s in &arena.signs {
        let (plate, lit) = match made.iter().find(|m| m.0 == s.way) {
            Some(&(_, plate, lit)) => (plate, lit),
            None => {
                let (plate, lit) = sign(s.way);
                made.push((s.way, mesh(&plate), mesh(&lit)));
                (made[made.len() - 1].1, made[made.len() - 1].2)
            }
        };
        let frame = facing(s.at, s.facing);
        world.spawn((Placed(frame), Model(plate), Look::default(), OnMap));
        world.spawn((Placed(frame), Model(lit), bright, OnMap));
    }
}

/// A sign as the arena has it.
pub fn placed(at: Vec3, facing: Vec3, way: Way) -> Sign {
    Sign { at, facing, way }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cabinet_stands_on_the_floor_out_from_its_wall() {
        // Against a wall on z = 5, facing north (out is -z), at a buy's
        // height over a floor at y = -3.
        let c = cabinet(Vec3::new(10.0, -3.0 + 1.45, 5.0), Vec3::new(0.0, 0.0, -1.0), 1.45);
        assert!((c.lo - Vec3::new(10.0 - WIDE * 0.5, -3.0, 5.0 - DEEP)).length() < 1e-9, "{:?}", c.lo);
        assert!((c.hi - Vec3::new(10.0 + WIDE * 0.5, -3.0 + TALL, 5.0)).length() < 1e-9, "{:?}", c.hi);
        // Its fittings: something dull, something alight, none of it
        // behind the cabinet's front.
        let (dull, lit) = fittings(1.45);
        assert!(!dull.is_empty() && lit.iter().all(|v| v.emissive.iter().any(|&e| e > 0.0)));
        assert!(dull.iter().chain(&lit).filter(|v| v.pos[1] < 0.4).all(|v| v.pos[2] >= 0.0));
        // A sign of each kind.
        for way in [Way::Left, Way::Right, Way::Down] {
            let (plate, lit) = sign(way);
            assert!(!plate.is_empty() && !lit.is_empty());
        }
    }
}
