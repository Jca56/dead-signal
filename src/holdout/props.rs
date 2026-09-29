//! What of the arena moves or goes: each window's boards (up or torn
//! off), the doors (there till bought open), and on the walls, what's for
//! sale: the thing itself (a gun, a kit) hung in a chalk outline. The meshes are made once the arena's in; each frame,
//! what's gone is put away.

use bevy_ecs::prelude::*;
use lntrn_math::{Mat4, Quat, Vec3};

use super::arena::{Arena, Wares};
use super::{BOARDS, Holdout};
use crate::render::{MeshId, Vertex};
use crate::world::{Look, Model, OnMap, Placed};
use crate::zombie::breach::Barriers;

/// Colours, as they look (sRGB).
const PLANK: [f32; 3] = [0.45, 0.34, 0.22];
const PLANK_DARK: [f32; 3] = [0.34, 0.25, 0.16];
const DOOR: [f32; 3] = [0.30, 0.24, 0.18];
const DOOR_FRAME: [f32; 3] = [0.20, 0.16, 0.12];
const CHALK: [f32; 3] = [0.86, 0.85, 0.80];
/// A board: how long past its window's sides, how tall, how thick; how
/// far out from the window's middle it's nailed, and how far it tips.
const BOARD_LAP: f64 = 0.14;
const BOARD_TALL: f64 = 0.15;
const BOARD_THICK: f64 = 0.04;
const BOARD_OUT: f64 = 0.16;
const BOARD_TIP: f64 = 0.07;
/// A wall buy's outline: how wide, how tall, how thick its lines.
const OUTLINE: (f64, f64) = (1.3, 0.6);
const LINE: f64 = 0.035;

/// One of a window's boards, and where it's nailed.
#[derive(Component, Clone, Copy, Debug)]
pub struct Board {
    window: usize,
    k: u8,
    model: Mat4,
}

/// A door, and where it stands.
#[derive(Component, Clone, Copy, Debug)]
pub struct Door {
    door: usize,
    model: Mat4,
}

/// Nothing to see: shrunk to a point.
fn gone() -> Mat4 {
    Mat4::from_scale(Vec3::splat(0.0))
}

/// A frame whose z is `out` (flat), x along, y up, at `at`.
fn facing(at: Vec3, out: Vec3) -> Mat4 {
    Mat4::from_translation(at) * Mat4::from_quat(Quat::from_rotation_y(out.x.atan2(out.z)))
}

/// Set the arena's props down, their meshes made with `mesh`: the boards
/// on every window, the doors, the wall buys (the guns' own meshes from
/// `items`).
pub fn spawn(world: &mut World, arena: &Arena, items: &crate::items::Meshes, mut mesh: impl FnMut(&[Vertex]) -> MeshId) {
    let mut board_mesh = |dark: bool, width: f64| {
        let half = width * 0.5 + BOARD_LAP;
        mesh(&boxes(&[(Vec3::new(-half, -BOARD_TALL * 0.5, -BOARD_THICK * 0.5), Vec3::new(half, BOARD_TALL * 0.5, BOARD_THICK * 0.5), if dark { PLANK_DARK } else { PLANK })]))
    };
    for (i, w) in arena.windows.iter().enumerate() {
        let meshes = [board_mesh(false, w.width), board_mesh(true, w.width)];
        let frame = facing(w.centre, w.inward);
        for k in 0..BOARDS {
            // Up the window from the bottom, each tipped a little, one way
            // then the other.
            let y = (w.head - w.sill) * (f64::from(k) + 0.5) / f64::from(BOARDS);
            let tip = if k % 2 == 0 { BOARD_TIP } else { -BOARD_TIP * 0.7 };
            let model = frame * Mat4::from_translation(Vec3::new(0.0, y, -BOARD_OUT - f64::from(k % 2) * BOARD_THICK)) * Mat4::from_quat(Quat::from_rotation_z(tip));
            world.spawn((Board { window: i, k, model }, Placed(model), Model(meshes[k as usize % 2]), Look::default(), OnMap));
        }
    }
    for (i, d) in arena.doors.iter().enumerate() {
        let mid = (d.lo + d.hi) * 0.5;
        let (lo, hi) = (d.lo - mid, d.hi - mid);
        // A slab, and a darker rail across it at either end.
        let rail = |y0: f64, y1: f64| (Vec3::new(lo.x - 0.01, y0, lo.z - 0.01), Vec3::new(hi.x + 0.01, y1, hi.z + 0.01), DOOR_FRAME);
        let m = mesh(&boxes(&[(lo, hi, DOOR), rail(lo.y + 0.3, lo.y + 0.42), rail(hi.y - 0.45, hi.y - 0.33)]));
        let model = Mat4::from_translation(mid);
        world.spawn((Door { door: i, model }, Placed(model), Model(m), Look::default(), OnMap));
    }
    let (ow, oh) = OUTLINE;
    let outline = mesh(&boxes(&[
        (Vec3::new(-ow * 0.5, -oh * 0.5, 0.0), Vec3::new(ow * 0.5, -oh * 0.5 + LINE, 0.01), CHALK),
        (Vec3::new(-ow * 0.5, oh * 0.5 - LINE, 0.0), Vec3::new(ow * 0.5, oh * 0.5, 0.01), CHALK),
        (Vec3::new(-ow * 0.5, -oh * 0.5, 0.0), Vec3::new(-ow * 0.5 + LINE, oh * 0.5, 0.01), CHALK),
        (Vec3::new(ow * 0.5 - LINE, -oh * 0.5, 0.0), Vec3::new(ow * 0.5, oh * 0.5, 0.01), CHALK),
    ]));
    let chalk = Look { emissive: 1.0, fog: 0.8, tint: [1.0; 3] };
    for b in &arena.buys {
        let frame = facing(b.at, b.facing);
        world.spawn((Placed(frame), Model(outline), chalk, OnMap));
        let (Wares::Weapon(kind) | Wares::Kit(kind)) = b.wares;
        if let Some(&m) = items.0.get(&kind) {
            // Stood up off the floor it lay on, the face that was up turned
            // out of the wall; a gun (made lying the other way up to a
            // kit) turned right way up.
            let flip = if matches!(b.wares, Wares::Weapon(_)) { std::f64::consts::PI } else { 0.0 };
            let hung = frame * Mat4::from_translation(Vec3::new(0.0, 0.0, 0.04)) * Mat4::from_quat(Quat::from_rotation_z(flip)) * Mat4::from_quat(Quat::from_rotation_x(std::f64::consts::FRAC_PI_2));
            world.spawn((Placed(hung), Model(m), Look::default(), OnMap));
        }
    }
}

/// Put away what's gone: boards torn off, doors bought open.
pub fn sync(world: &mut World, holdout: &Holdout) {
    let boards: Vec<u8> = world.get_resource::<Barriers>().map(|b| b.0.iter().map(|w| w.boards).collect()).unwrap_or_default();
    for (board, mut placed) in world.query::<(&Board, &mut Placed)>().iter_mut(world) {
        let up = boards.get(board.window).is_some_and(|&n| board.k < n);
        placed.0 = if up { board.model } else { gone() };
    }
    for (door, mut placed) in world.query::<(&Door, &mut Placed)>().iter_mut(world) {
        placed.0 = if holdout.door_open(door.door) { gone() } else { door.model };
    }
}

/// Boxes' triangles, each face outwards, from their corners and colours.
fn boxes(list: &[(Vec3, Vec3, [f32; 3])]) -> Vec<Vertex> {
    let mut out = Vec::new();
    for &(lo, hi, colour) in list {
        let color = colour.map(linear);
        let color = [color[0], color[1], color[2], 1.0];
        let p = |x: bool, y: bool, z: bool| Vec3::new(if x { hi.x } else { lo.x }, if y { hi.y } else { lo.y }, if z { hi.z } else { lo.z });
        let quads = [
            ([p(false, false, false), p(true, false, false), p(true, false, true), p(false, false, true)], [0.0, -1.0, 0.0]),
            ([p(false, true, false), p(false, true, true), p(true, true, true), p(true, true, false)], [0.0, 1.0, 0.0]),
            ([p(false, false, false), p(false, true, false), p(true, true, false), p(true, false, false)], [0.0, 0.0, -1.0]),
            ([p(false, false, true), p(true, false, true), p(true, true, true), p(false, true, true)], [0.0, 0.0, 1.0]),
            ([p(false, false, false), p(false, false, true), p(false, true, true), p(false, true, false)], [-1.0, 0.0, 0.0]),
            ([p(true, false, false), p(true, true, false), p(true, true, true), p(true, false, true)], [1.0, 0.0, 0.0]),
        ];
        for (q, normal) in quads {
            let v = |c: Vec3| Vertex { pos: [c.x as f32, c.y as f32, c.z as f32], normal, color, emissive: [0.0; 3] };
            out.extend([v(q[0]), v(q[1]), v(q[2]), v(q[0]), v(q[2]), v(q[3])]);
        }
    }
    out
}

/// An sRGB channel as light (what the renderer works in).
fn linear(c: f32) -> f32 {
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}
