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
/// A heap of junk's: its crates, a rusted drum, a sheet of tin.
const CRATE: [f32; 3] = [0.47, 0.37, 0.23];
const CRATE_DARK: [f32; 3] = [0.38, 0.29, 0.18];
const DRUM: [f32; 3] = [0.42, 0.16, 0.10];
const TIN: [f32; 3] = [0.44, 0.45, 0.43];
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

/// Something that comes and goes (a board, a door): its shadow's cast
/// afresh every frame.
#[derive(Component, Clone, Copy, Debug)]
pub struct Moves;

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
        mesh(&boxes(&[(Vec3::new(-half, -BOARD_TALL * 0.5, -BOARD_THICK * 0.5), Vec3::new(half, BOARD_TALL * 0.5, BOARD_THICK * 0.5), if dark { PLANK_DARK } else { PLANK }, Mat4::IDENTITY)]))
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
            world.spawn((Board { window: i, k, model }, Moves, Placed(model), Model(meshes[k as usize % 2]), Look::default(), OnMap));
        }
    }
    for (i, d) in arena.doors.iter().enumerate() {
        let mid = (d.lo + d.hi) * 0.5;
        let (lo, hi) = (d.lo - mid, d.hi - mid);
        let m = if d.heap {
            mesh(&heap(lo, hi))
        } else {
            // A slab, and a darker rail across it at either end.
            let rail = |y0: f64, y1: f64| (Vec3::new(lo.x - 0.01, y0, lo.z - 0.01), Vec3::new(hi.x + 0.01, y1, hi.z + 0.01), DOOR_FRAME, Mat4::IDENTITY);
            mesh(&boxes(&[(lo, hi, DOOR, Mat4::IDENTITY), rail(lo.y + 0.3, lo.y + 0.42), rail(hi.y - 0.45, hi.y - 0.33)]))
        };
        let model = Mat4::from_translation(mid);
        world.spawn((Door { door: i, model }, Moves, Placed(model), Model(m), Look::default(), OnMap));
    }
    let (ow, oh) = OUTLINE;
    let mut outline = boxes(&[
        (Vec3::new(-ow * 0.5, -oh * 0.5, 0.0), Vec3::new(ow * 0.5, -oh * 0.5 + LINE, 0.01), CHALK, Mat4::IDENTITY),
        (Vec3::new(-ow * 0.5, oh * 0.5 - LINE, 0.0), Vec3::new(ow * 0.5, oh * 0.5, 0.01), CHALK, Mat4::IDENTITY),
        (Vec3::new(-ow * 0.5, -oh * 0.5, 0.0), Vec3::new(-ow * 0.5 + LINE, oh * 0.5, 0.01), CHALK, Mat4::IDENTITY),
        (Vec3::new(ow * 0.5 - LINE, -oh * 0.5, 0.0), Vec3::new(ow * 0.5, oh * 0.5, 0.01), CHALK, Mat4::IDENTITY),
    ]);
    // (Chalk that shows in the dark: what's for sale is found by it.)
    for corner in &mut outline {
        corner.emissive = [corner.color[0] * 0.5, corner.color[1] * 0.5, corner.color[2] * 0.5];
    }
    let outline = mesh(&outline);
    let chalk = Look { emissive: 1.0, fog: 0.8, tint: [1.0; 3] };
    // (The Amplifier's no outline on a wall, nor a med station: each
    // stands there itself.)
    for b in arena.buys.iter().filter(|b| !matches!(b.wares, Wares::Amplifier | Wares::Stim(_))) {
        let frame = facing(b.at, b.facing);
        world.spawn((Placed(frame), Model(outline), chalk, OnMap));
        if let Some(&m) = b.wares.kind().and_then(|k| items.0.get(&k)) {
            // Stood up off the floor it lay on, the face that was up turned
            // out of the wall; a gun (made lying the other way up to a
            // kit) turned right way up.
            let flip = if matches!(b.wares, Wares::Weapon(_)) { std::f64::consts::PI } else { 0.0 };
            let hung = frame * Mat4::from_translation(Vec3::new(0.0, 0.0, 0.04)) * Mat4::from_quat(Quat::from_rotation_z(flip)) * Mat4::from_quat(Quat::from_rotation_x(std::f64::consts::FRAC_PI_2));
            world.spawn((Placed(hung), Model(m), Look::default(), OnMap));
        }
    }
    super::machine::spawn(world, arena, super::raise::buy_height(), &mut mesh);
    super::station::spawn(world, arena, super::raise::buy_height(), mesh);
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

/// A heap of junk filling the box `lo`–`hi` (about its middle, on its
/// floor): crates piled up the middle, a rusted drum, planks and a sheet
/// of tin leant against it either side.
fn heap(lo: Vec3, hi: Vec3) -> Vec<Vertex> {
    // In its own frame: `u` along the wall it fills a gap in, `v` across.
    let along_x = hi.x - lo.x >= hi.z - lo.z;
    let (len, deep) = if along_x { (hi.x - lo.x, hi.z - lo.z) } else { (hi.z - lo.z, hi.x - lo.x) };
    let frame = if along_x { Mat4::IDENTITY } else { Mat4::from_quat(Quat::from_rotation_y(std::f64::consts::FRAC_PI_2)) };
    let y0 = lo.y;
    let mut list = Vec::new();
    let turn = |yaw: f64, tip: f64| frame * Mat4::from_quat(Quat::from_rotation_y(yaw)) * Mat4::from_quat(Quat::from_rotation_z(tip));
    // Crates: three along the bottom, two on them, one on top.
    let c = 0.78;
    let rows: [(f64, &[f64]); 3] = [(0.0, &[-0.82, 0.0, 0.82]), (c, &[-0.4, 0.42]), (2.0 * c, &[0.05])];
    for (k, &(y, us)) in rows.iter().enumerate() {
        for (j, &u) in us.iter().enumerate() {
            let yaw = [0.06, -0.1, 0.14, -0.05, 0.09, -0.12][(k * 3 + j) % 6];
            let m = frame * Mat4::from_translation(Vec3::new(u * (len / 2.4), y0 + y + c * 0.5, (j as f64 - 0.5) * 0.08)) * Mat4::from_quat(Quat::from_rotation_y(yaw));
            let shade = if (k + j) % 2 == 0 { CRATE } else { CRATE_DARK };
            list.push((Vec3::splat(-c * 0.5), Vec3::splat(c * 0.5), shade, m));
        }
    }
    // A drum, knocked over along the foot of it; planks leant against it.
    list.push((Vec3::new(-0.45, -0.3, -0.3), Vec3::new(0.45, 0.3, 0.3), DRUM, frame * Mat4::from_translation(Vec3::new(len * 0.3, y0 + 0.3, -deep * 0.32))));
    for (k, u) in [-0.9, -0.35, 0.5].iter().enumerate() {
        let side = if k % 2 == 0 { 1.0 } else { -1.0 };
        let m = Mat4::from_translation(Vec3::new(0.0, y0 + 1.0, side * deep * 0.36)) * turn(0.2 * side, 0.0) * Mat4::from_translation(Vec3::new(*u, 0.0, 0.0)) * Mat4::from_quat(Quat::from_rotation_x(side * 0.35));
        list.push((Vec3::new(-0.12, -1.05, -0.03), Vec3::new(0.12, 1.05, 0.03), PLANK_DARK, m));
    }
    list.push((Vec3::new(-0.6, -0.9, -0.02), Vec3::new(0.6, 0.9, 0.02), TIN, Mat4::from_translation(Vec3::new(0.0, y0 + 0.85, -deep * 0.4)) * turn(-0.15, 0.1) * Mat4::from_quat(Quat::from_rotation_x(-0.3))));
    // A beam across the top of the gap.
    list.push((Vec3::new(-len * 0.5, -0.1, -0.1), Vec3::new(len * 0.5, 0.1, 0.1), PLANK, frame * Mat4::from_translation(Vec3::new(0.0, y0 + 2.0 * c + 0.2, 0.25)) * Mat4::from_quat(Quat::from_rotation_z(0.12))));
    boxes(&list)
}

/// Boxes' triangles, each face outwards, from their corners (about their
/// own middles, then turned and moved), and their colours.
pub(super) fn boxes(list: &[(Vec3, Vec3, [f32; 3], Mat4)]) -> Vec<Vertex> {
    let mut out = Vec::new();
    for &(lo, hi, colour, turn) in list {
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
            let n = turn.transform_vector(Vec3::new(normal[0], normal[1], normal[2]));
            let normal = [n.x as f32, n.y as f32, n.z as f32];
            let v = |c: Vec3| {
                let c = turn.transform_point(c);
                Vertex { pos: [c.x as f32, c.y as f32, c.z as f32], normal, color, emissive: [0.0; 3] }
            };
            out.extend([v(q[0]), v(q[1]), v(q[2]), v(q[0]), v(q[2]), v(q[3])]);
        }
    }
    out
}

/// An sRGB channel as light (what the renderer works in).
fn linear(c: f32) -> f32 {
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}
