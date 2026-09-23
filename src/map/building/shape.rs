//! A floor plan made into blocks, in the building's own frame (y up from
//! its ground floor): each wall two skins (the room's paint one side, the
//! next room's or the siding the other), cut round its doorways and
//! windows; a foundation and a step up to each door; each room's floor;
//! the floors between storeys with the stairwell left open and railed; the
//! stairs; a roof. Empty windows get a barrier only bodies meet; boarded
//! ones get planks. A barn is boarded up and down in red, a cabin laid in
//! logs (bare wood inside too) with a porch at its front.

use lntrn_math::Vec3;

use super::plan::{CEILING, Kind, Plan, STEPS, STOREY, TREAD, Use};
use crate::collide::Surface;
use crate::loot::Dice;

/// Colour as it looks (sRGB).
pub type Rgb = [f32; 3];

/// Each wall's skins, thick.
const SKIN: f64 = 0.1;
/// How high the floor stands off the ground, and how deep the foundation
/// goes into it.
pub const RAISED: f64 = 0.25;
const FOUNDATION: f64 = 0.8;
/// Roofs: how far they overhang.
const OVERHANG: f64 = 0.4;
/// A barn's boards, how wide; a cabin's logs, how high; a cabin's porch,
/// how deep and how high its roof.
const BOARD: f64 = 0.5;
const LOG: f64 = 0.28;
const PORCH: f64 = 1.8;
const PORCH_ROOF: f64 = 2.5;
/// The rail round the stairwell upstairs.
const RAIL: f64 = 1.0;
/// How far one block runs into the next where they meet: no two faces
/// ever lie on one another (looking down through a building, each face met
/// says in or out of something solid, and two at once would say both).
const TUCK: f64 = 0.01;

/// What a block is to what meets it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stuff {
    Solid(Surface),
    /// Only bodies meet it: an empty window.
    Ghost,
}

/// A box, lined up with the building: corners, colour, what it is.
#[derive(Clone, Copy, Debug)]
pub struct Block {
    pub lo: Vec3,
    pub hi: Vec3,
    pub colour: Rgb,
    pub stuff: Stuff,
}

/// A building's blocks, and its roof's slopes (triangles, with colours).
pub struct Shape {
    pub blocks: Vec<Block>,
    pub roof: Vec<([Vec3; 3], Rgb)>,
}

const SIDINGS: [Rgb; 7] = [[0.55, 0.58, 0.60], [0.66, 0.62, 0.52], [0.52, 0.30, 0.26], [0.72, 0.71, 0.66], [0.48, 0.55, 0.46], [0.46, 0.33, 0.27], [0.60, 0.52, 0.40]];
const PAINTS: [Rgb; 6] = [[0.74, 0.70, 0.60], [0.60, 0.66, 0.56], [0.58, 0.64, 0.68], [0.70, 0.58, 0.56], [0.62, 0.62, 0.60], [0.72, 0.66, 0.50]];
const ROOFS: [Rgb; 4] = [[0.25, 0.24, 0.23], [0.34, 0.24, 0.18], [0.36, 0.18, 0.15], [0.28, 0.31, 0.33]];
const STORE_WALLS: [Rgb; 3] = [[0.62, 0.58, 0.50], [0.50, 0.46, 0.42], [0.66, 0.66, 0.64]];
const CONCRETE: Rgb = [0.46, 0.45, 0.42];
const PLANKS: Rgb = [0.42, 0.33, 0.22];
const PLANKS_DARK: Rgb = [0.32, 0.25, 0.17];
const STAIRS: Rgb = [0.40, 0.30, 0.20];
const CEILING_WHITE: Rgb = [0.78, 0.77, 0.73];
const BARN_RED: [Rgb; 2] = [[0.50, 0.18, 0.14], [0.42, 0.15, 0.12]];
const LOGS: [Rgb; 2] = [[0.42, 0.29, 0.18], [0.33, 0.22, 0.13]];
const LOG_INSIDE: Rgb = [0.55, 0.41, 0.27];

/// What a room's floor is.
fn floor_of(use_: Use, dice: &mut Dice) -> Rgb {
    match use_ {
        Use::Kitchen | Use::Bath => [0.64, 0.63, 0.60],
        Use::Bed => [[0.46, 0.38, 0.34], [0.36, 0.40, 0.46], [0.44, 0.44, 0.36]][dice.next() as usize % 3],
        Use::Shop => [0.56, 0.54, 0.48],
        Use::Back => [0.44, 0.43, 0.40],
        Use::Living | Use::Hall | Use::Den => [0.47, 0.35, 0.23],
        Use::Barn => [0.46, 0.40, 0.26],
    }
}

fn pick<const N: usize>(dice: &mut Dice, from: [Rgb; N]) -> Rgb {
    from[dice.next() as usize % N]
}

/// Build the blocks of `plan`.
pub fn shape(plan: &Plan, dice: &mut Dice) -> Shape {
    let store = plan.kind == Kind::Store;
    let siding = match plan.kind {
        Kind::Store => pick(dice, STORE_WALLS),
        Kind::Barn => BARN_RED[0],
        Kind::Cabin => LOGS[0],
        _ => pick(dice, SIDINGS),
    };
    // Outside, a barn's boards run up and down, a cabin's logs along: each
    // stretch of skin cut into strips of two shades.
    let banding: Option<(bool, f64, [Rgb; 2])> = match plan.kind {
        Kind::Barn => Some((false, BOARD, BARN_RED)),
        Kind::Cabin => Some((true, LOG, LOGS)),
        _ => None,
    };
    let roof_colour = pick(dice, ROOFS);
    let paints: Vec<Rgb> = plan
        .rooms
        .iter()
        .map(|r| match (plan.kind, r.use_) {
            (Kind::Cabin | Kind::Barn, _) => LOG_INSIDE,
            (_, Use::Bath) => [0.70, 0.74, 0.74],
            _ => pick(dice, PAINTS),
        })
        .collect();
    let floors: Vec<Rgb> = plan.rooms.iter().map(|r| floor_of(r.use_, dice)).collect();
    let wall_stuff = Stuff::Solid(if store { Surface::Stone } else { Surface::Wood });
    let mut blocks = Vec::new();
    let mut add = |lo: Vec3, hi: Vec3, colour: Rgb, stuff: Stuff| blocks.push(Block { lo: lo.min(hi), hi: lo.max(hi), colour, stuff });
    let (w, d) = (f64::from(plan.w), f64::from(plan.d));
    let top = f64::from(plan.storeys) * STOREY;

    // The foundation, each room's floor on it, and a step up to each door.
    add(Vec3::new(-SKIN, -RAISED - FOUNDATION, -SKIN), Vec3::new(w + SKIN, -0.02, d + SKIN), CONCRETE, Stuff::Solid(Surface::Stone));
    for (i, r) in plan.rooms.iter().enumerate().filter(|(_, r)| r.storey == 0) {
        add(Vec3::new(f64::from(r.x0), -0.02 - 2.0 * TUCK, f64::from(r.z0)), Vec3::new(f64::from(r.x1), 0.0, f64::from(r.z1)), floors[i], Stuff::Solid(Surface::Wood));
    }
    for o in plan.openings.iter().filter(|o| o.door && !o.boarded) {
        let wall = plan.walls[o.wall];
        if !wall.outside() || wall.storey != 0 {
            continue;
        }
        let at = f64::from(wall.at);
        let out = if wall.sides[0].is_none() { -1.0 } else { 1.0 };
        let (a, b) = (at + out * SKIN, at + out * (SKIN + 0.7));
        let half = o.width * 0.5 + 0.2;
        let (lo, hi) = if wall.along_x {
            (Vec3::new(o.centre - half, -RAISED - 0.3, a), Vec3::new(o.centre + half, -0.13, b))
        } else {
            (Vec3::new(a, -RAISED - 0.3, o.centre - half), Vec3::new(b, -0.13, o.centre + half))
        };
        add(lo, hi, CONCRETE, Stuff::Solid(Surface::Stone));
    }

    // The walls, skin by skin, cut round their gaps.
    for (wi, wall) in plan.walls.iter().enumerate() {
        let base = f64::from(wall.storey) * STOREY;
        let at = f64::from(wall.at);
        // Outside walls along x reach over the corners.
        let reach = if wall.outside() && wall.along_x { SKIN } else { 0.0 };
        let (from, to) = (f64::from(wall.from) - reach, f64::from(wall.to) + reach);
        let mut gaps: Vec<_> = plan.openings.iter().filter(|o| o.wall == wi).collect();
        gaps.sort_by(|a, b| a.centre.total_cmp(&b.centre));
        for (side, room) in wall.sides.iter().enumerate() {
            let colour = room.map_or(siding, |r| paints[r]);
            let (t0, t1) = if side == 0 { (at - SKIN, at) } else { (at, at + SKIN) };
            // A stretch of this skin, along the wall from `a` to `b`, from
            // `y0` to `y1` up.
            let mut piece = |a: f64, b: f64, y0: f64, y1: f64| {
                if b - a < 1e-6 || y1 - y0 < 1e-6 {
                    return;
                }
                // Tucked into the floor under it and the ceiling over it.
                let y0 = if y0 <= 0.0 { -TUCK } else { y0 };
                let y1 = if y1 >= CEILING { CEILING + TUCK } else { y1 };
                let mut strip = |a: f64, b: f64, y0: f64, y1: f64, colour: Rgb| {
                    let (lo, hi) = if wall.along_x { (Vec3::new(a, base + y0, t0), Vec3::new(b, base + y1, t1)) } else { (Vec3::new(t0, base + y0, a), Vec3::new(t1, base + y1, b)) };
                    add(lo, hi, colour, wall_stuff);
                };
                match banding.filter(|_| room.is_none()) {
                    // Logs: along the wall, one over another.
                    Some((true, pitch, shades)) => {
                        let mut y = y0;
                        while y < y1 - 1e-6 {
                            let k = ((y + TUCK) / pitch).floor();
                            let top = ((k + 1.0) * pitch).min(y1);
                            strip(a, b, y, top, shades[k.rem_euclid(2.0) as usize]);
                            y = top;
                        }
                    }
                    // Boards: up and down, side by side.
                    Some((false, pitch, shades)) => {
                        let mut u = a;
                        while u < b - 1e-6 {
                            let k = ((u + 1e-6) / pitch).floor();
                            let end = ((k + 1.0) * pitch).min(b);
                            strip(u, end, y0, y1, shades[k.rem_euclid(2.0) as usize]);
                            u = end;
                        }
                    }
                    None => strip(a, b, y0, y1, colour),
                }
            };
            let mut along = from;
            for g in &gaps {
                let (a, b) = (g.centre - g.width * 0.5, g.centre + g.width * 0.5);
                piece(along, a, 0.0, CEILING);
                piece(a, b, 0.0, g.sill);
                piece(a, b, g.head, CEILING);
                along = b;
            }
            piece(along, to, 0.0, CEILING);
        }
        for g in &gaps {
            let (a, b) = (g.centre - g.width * 0.5, g.centre + g.width * 0.5);
            let span = |t0: f64, t1: f64, y0: f64, y1: f64| {
                if wall.along_x { (Vec3::new(a, base + y0, t0), Vec3::new(b, base + y1, t1)) } else { (Vec3::new(t0, base + y0, a), Vec3::new(t1, base + y1, b)) }
            };
            if g.boarded {
                // Planks across it, and two more nailed over the outside.
                let (lo, hi) = span(at - 0.04, at + 0.04, g.sill, g.head);
                add(lo, hi, PLANKS, wall_stuff);
                let out = if wall.sides[0].is_none() { -1.0 } else { 1.0 };
                let face = at + out * (SKIN + 0.02);
                for k in [0.3, 0.7] {
                    let y = g.sill + (g.head - g.sill) * k;
                    let (lo, hi) = if wall.along_x {
                        (Vec3::new(a - 0.12, base + y - 0.1, face - 0.02), Vec3::new(b + 0.12, base + y + 0.1, face + 0.02))
                    } else {
                        (Vec3::new(face - 0.02, base + y - 0.1, a - 0.12), Vec3::new(face + 0.02, base + y + 0.1, b + 0.12))
                    };
                    add(lo, hi, PLANKS_DARK, wall_stuff);
                }
            } else if !g.door {
                let (lo, hi) = span(at - 0.03, at + 0.03, g.sill, g.head);
                add(lo, hi, [0.0; 3], Stuff::Ghost);
            }
        }
    }

    // The floors between storeys (the stairwell left open, a rail round
    // it), and the ceiling under the roof.
    let well = plan.stair.map(|st| {
        let x0 = if st.x == 0 { SKIN } else { f64::from(st.x) - SKIN };
        (x0, x0 + 1.0, st.z0, st.z1())
    });
    for (i, r) in plan.rooms.iter().enumerate().filter(|(_, r)| r.storey > 0) {
        let y = f64::from(r.storey) * STOREY;
        let (x0, z0, x1, z1) = (f64::from(r.x0), f64::from(r.z0), f64::from(r.x1), f64::from(r.z1));
        let mut rects = vec![(x0, z0, x1, z1)];
        if let Some((wx0, wx1, wz0, wz1)) = well
            && r.use_ == Use::Hall
        {
            rects = vec![(x0, z0, x1, wz0), (x0, wz1, x1, z1), (x0, wz0, wx0, wz1), (wx1, wz0, x1, wz1)];
            let open_side = if wx0 <= x0 + SKIN + 1e-9 { wx1 } else { wx0 };
            add(Vec3::new(open_side - 0.02, y, wz0), Vec3::new(open_side + 0.02, y + RAIL, wz1), STAIRS, Stuff::Solid(Surface::Wood));
            add(Vec3::new(wx0, y, wz0 - 0.04), Vec3::new(wx1, y + RAIL, wz0), STAIRS, Stuff::Solid(Surface::Wood));
        }
        for (a, b, c, e) in rects {
            if c - a > 1e-6 && e - b > 1e-6 {
                add(Vec3::new(a, y - (STOREY - CEILING), b), Vec3::new(c, y, e), floors[i], Stuff::Solid(Surface::Wood));
            }
        }
    }
    add(Vec3::new(-SKIN, top - (STOREY - CEILING), -SKIN), Vec3::new(w + SKIN, top, d + SKIN), CEILING_WHITE, Stuff::Solid(Surface::Wood));

    // The stairs: each step a block down to the floor.
    if let (Some(st), Some((wx0, wx1, _, _))) = (plan.stair, well) {
        let rise = STOREY / STEPS as f64;
        for k in 0..STEPS {
            add(Vec3::new(wx0, -TUCK, st.z0 + k as f64 * TREAD), Vec3::new(wx1, (k + 1) as f64 * rise, st.z1()), STAIRS, Stuff::Solid(Surface::Wood));
        }
    }

    // The roof: flat with a low wall round it, or a gable.
    let mut roof = Vec::new();
    if plan.flat_roof {
        let (y0, y1) = (top, top + 0.7);
        add(Vec3::new(-SKIN, y0, -SKIN), Vec3::new(w + SKIN, y1, 0.1), siding, wall_stuff);
        add(Vec3::new(-SKIN, y0, d - 0.1), Vec3::new(w + SKIN, y1, d + SKIN), siding, wall_stuff);
        add(Vec3::new(-SKIN, y0, 0.1), Vec3::new(0.1, y1, d - 0.1), siding, wall_stuff);
        add(Vec3::new(w - 0.1, y0, 0.1), Vec3::new(w + SKIN, y1, d - 0.1), siding, wall_stuff);
    } else {
        roof = gable(plan, top, roof_colour, siding);
    }
    // A cabin's porch: a deck along its front, two posts, a roof over.
    if plan.kind == Kind::Cabin {
        let deck = Vec3::new(-SKIN, -RAISED - 0.3, -SKIN - PORCH);
        add(deck, Vec3::new(w + SKIN, -0.02, -SKIN), PLANKS, Stuff::Solid(Surface::Wood));
        // A step up to it, all along its front.
        add(Vec3::new(-SKIN, -RAISED - 0.3, -SKIN - PORCH - 0.45), Vec3::new(w + SKIN, -0.16, -SKIN - PORCH + TUCK), PLANKS_DARK, Stuff::Solid(Surface::Wood));
        for x in [0.0, w - 0.2] {
            add(Vec3::new(x, -0.02, -SKIN - PORCH), Vec3::new(x + 0.2, PORCH_ROOF, -SKIN - PORCH + 0.2), LOGS[1], Stuff::Solid(Surface::Wood));
        }
        add(Vec3::new(-SKIN - 0.2, PORCH_ROOF, -SKIN - PORCH - 0.3), Vec3::new(w + SKIN + 0.2, PORCH_ROOF + 0.12, -SKIN), roof_colour, Stuff::Solid(Surface::Wood));
    }
    Shape { blocks, roof }
}

/// A gable roof over the plan: a closed prism, the ridge along its longer
/// way, its slopes overhanging, the siding carried up its ends.
fn gable(plan: &Plan, top: f64, colour: Rgb, ends: Rgb) -> Vec<([Vec3; 3], Rgb)> {
    let (w, d) = (f64::from(plan.w), f64::from(plan.d));
    // In the ridge's own frame: `u` along it, `v` across; mapped back.
    let (len, span) = if plan.ridge_along_x { (w, d) } else { (d, w) };
    let at = |u: f64, v: f64, y: f64| if plan.ridge_along_x { Vec3::new(u, y, v) } else { Vec3::new(v, y, u) };
    let (u0, u1) = (-OVERHANG, len + OVERHANG);
    let (v0, v1) = (-OVERHANG, span + OVERHANG);
    let eave = top - 0.25;
    let peak = top + plan.ridge;
    let vm = span * 0.5;
    let (a0, b0, r0) = (at(u0, v0, eave), at(u0, v1, eave), at(u0, vm, peak));
    let (a1, b1, r1) = (at(u1, v0, eave), at(u1, v1, eave), at(u1, vm, peak));
    let mut out = vec![
        ([a0, r0, r1], colour),
        ([a0, r1, a1], colour),
        ([b0, b1, r1], colour),
        ([b0, r1, r0], colour),
        ([a0, b0, r0], ends),
        ([a1, r1, b1], ends),
        ([a0, a1, b1], colour),
        ([a0, b1, b0], colour),
    ];
    // Every face outwards, from the prism's middle.
    let mid = at(len * 0.5, vm, (eave + peak) * 0.5);
    for (t, _) in &mut out {
        let n = (t[1] - t[0]).cross(t[2] - t[0]);
        let c = (t[0] + t[1] + t[2]) * (1.0 / 3.0);
        if n.dot(c - mid) < 0.0 {
            t.swap(1, 2);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::plan;
    use super::*;

    #[test]
    fn a_house_is_closed_and_its_gaps_are_open() {
        let mut dice = Dice(42);
        let p = plan::house(&mut dice, 10, 9, true);
        let s = shape(&p, &mut dice);
        assert!(s.blocks.len() > 40 && s.roof.len() == 8);
        // No block is inside out or flat.
        assert!(s.blocks.iter().all(|b| b.hi.x > b.lo.x && b.hi.y > b.lo.y && b.hi.z > b.lo.z));
        // The roof faces out.
        for (t, _) in &s.roof {
            let n = (t[1] - t[0]).cross(t[2] - t[0]);
            assert!(n.length() > 1e-6);
        }
        // Through every open doorway's middle at waist height, nothing
        // solid.
        for o in p.openings.iter().filter(|o| o.door && !o.boarded) {
            let wall = p.walls[o.wall];
            let base = f64::from(wall.storey) * STOREY;
            let at = f64::from(wall.at);
            let point = if wall.along_x { Vec3::new(o.centre, base + 1.0, at) } else { Vec3::new(at, base + 1.0, o.centre) };
            let blocked = s.blocks.iter().filter(|b| b.stuff != Stuff::Ghost).any(|b| (0..3).all(|k| {
                let (p, lo, hi) = ([point.x, point.y, point.z][k], [b.lo.x, b.lo.y, b.lo.z][k], [b.hi.x, b.hi.y, b.hi.z][k]);
                p > lo + 1e-9 && p < hi - 1e-9
            }));
            assert!(!blocked, "the doorway at {point:?} is blocked");
        }
    }
}
