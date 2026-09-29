//! What's in the rooms: furniture against the walls (never in a doorway's
//! way, nothing tall across a window, the hall and its stairs left clear),
//! the things to search among it, a store's shelving in rows with aisles
//! the dead can walk, and a box of rounds or a kit left lying here and
//! there.

use lntrn_math::{Vec2, Vec3};

use super::Building;
use super::plan::{Kind, STOREY, Use};
use super::program::{Thing, needed, program};
pub use super::program::Furn;
use crate::loot::tables::Source;
use crate::loot::{Dice, Kind as ItemKind};
use crate::map::Spot;
use crate::map::outposts::Fixture;
use crate::map::scatter::{Piece, Scenery};

/// What a building holds: its furniture, its containers, and what's left
/// lying about in it.
#[derive(Default)]
pub struct Furnished {
    pub pieces: Vec<Piece>,
    pub containers: Vec<(Source, Spot)>,
    pub pickups: Vec<crate::items::Spot>,
}

/// A rectangle in the building's frame (x, z).
#[derive(Clone, Copy, Debug)]
struct Rect {
    lo: Vec2,
    hi: Vec2,
}

impl Rect {
    fn overlaps(&self, o: &Rect, gap: f64) -> bool {
        self.lo.x < o.hi.x + gap && o.lo.x < self.hi.x + gap && self.lo.y < o.hi.y + gap && o.lo.y < self.hi.y + gap
    }
}

/// A room as it's being furnished: where its floor is, the space inside
/// its walls, what's to be kept clear (in front of every doorway, the
/// stairs and their foot), its windows (nothing tall in front of them),
/// and what's in it so far.
struct Space {
    room: usize,
    floor: f64,
    inner: Rect,
    clear: Vec<Rect>,
    windows: Vec<Rect>,
    placed: Vec<Rect>,
}

impl Space {
    fn of(plan: &super::plan::Plan, ri: usize) -> Self {
        let room = &plan.rooms[ri];
        let floor = f64::from(room.storey) * STOREY;
        let inner = Rect { lo: Vec2::new(f64::from(room.x0) + 0.1, f64::from(room.z0) + 0.1), hi: Vec2::new(f64::from(room.x1) - 0.1, f64::from(room.z1) - 0.1) };
        let mut clear: Vec<Rect> = Vec::new();
        for o in plan.openings.iter().filter(|o| o.door) {
            let w = plan.walls[o.wall];
            if w.storey != room.storey || !w.sides.contains(&Some(ri)) {
                continue;
            }
            let at = f64::from(w.at);
            let (a, c) = (o.centre - 0.95, o.centre + 0.95);
            clear.push(if w.along_x { Rect { lo: Vec2::new(a, at - 1.8), hi: Vec2::new(c, at + 1.8) } } else { Rect { lo: Vec2::new(at - 1.8, a), hi: Vec2::new(at + 1.8, c) } });
        }
        if let Some(st) = plan.stair {
            let x = f64::from(st.x);
            clear.push(Rect { lo: Vec2::new(x - 1.2, 0.0), hi: Vec2::new(x + 2.2, st.z1() + 1.0) });
        }
        let windows: Vec<Rect> = plan
            .openings
            .iter()
            .filter(|o| !o.door && plan.walls[o.wall].storey == room.storey && plan.walls[o.wall].sides.contains(&Some(ri)))
            .map(|o| {
                let w = plan.walls[o.wall];
                let at = f64::from(w.at);
                let (a, c) = (o.centre - o.width * 0.5 - 0.1, o.centre + o.width * 0.5 + 0.1);
                if w.along_x { Rect { lo: Vec2::new(a, at - 1.0), hi: Vec2::new(c, at + 1.0) } } else { Rect { lo: Vec2::new(at - 1.0, a), hi: Vec2::new(at + 1.0, c) } }
            })
            .collect();
        Self { room: ri, floor, inner, clear, windows, placed: Vec::new() }
    }
}

/// How high a landmark's sign's foot stands on its front, and how far out
/// from the wall.
const SIGN_HIGH: f64 = 2.35;
const SIGN_OUT: f64 = 0.19;

/// The fire engine, across and long; how far in from its bay's doors it's
/// parked.
pub(super) const ENGINE: (f64, f64) = (2.5, 8.2);
const ENGINE_IN: f64 = 2.6;

/// A glass display case, across and deep; and how far in from the gun
/// store's front the row of them stands.
pub(super) const DISPLAY: (f64, f64) = (1.3, 0.65);
const COUNTER_IN: f64 = 4.5;

/// How deep the floor kept free in front of something to search.
const ACCESS: f64 = 1.2;

/// `thing` set down in `b` at `centre` (in its frame, on `floor`), facing
/// `facing`.
fn put(b: &Building, thing: Thing, floor: f64, centre: Vec2, facing: Vec2, out: &mut Furnished) {
    let at = b.world(Vec3::new(centre.x, floor, centre.y));
    let inward = b.turn(Vec3::new(facing.x, 0.0, facing.y));
    let yaw = (-inward.x).atan2(-inward.z);
    match thing {
        Thing::Furn(f) => out.pieces.push(Piece::new(Scenery::Furn(f), at, yaw, 1.0)),
        Thing::Box(s) => out.containers.push((s, (at.x, at.z, yaw, at.y + 1.5))),
    }
}

/// Stand `thing` against one of `space`'s walls, facing into the room: a
/// few tries (more for what's needed, which at the last may stand across a
/// window). Whether it went in.
fn against_wall(b: &Building, space: &mut Space, thing: Thing, dice: &mut Dice, out: &mut Furnished) -> bool {
    let (w, d, tall) = thing.size();
    let inner = space.inner;
    let tries = if needed(thing) { 64 } else { 24 };
    for k in 0..tries {
        let tall = tall && k < 40;
        let side = dice.next() % 4;
        let (len_lo, len_hi) = if side < 2 { (inner.lo.x, inner.hi.x) } else { (inner.lo.y, inner.hi.y) };
        if len_hi - len_lo < w + 0.05 {
            continue;
        }
        let u = len_lo + w * 0.5 + dice.unit() * (len_hi - len_lo - w);
        let (rect, centre, facing) = match side {
            0 => (Rect { lo: Vec2::new(u - w * 0.5, inner.lo.y), hi: Vec2::new(u + w * 0.5, inner.lo.y + d) }, Vec2::new(u, inner.lo.y + d * 0.5), Vec2::new(0.0, 1.0)),
            1 => (Rect { lo: Vec2::new(u - w * 0.5, inner.hi.y - d), hi: Vec2::new(u + w * 0.5, inner.hi.y) }, Vec2::new(u, inner.hi.y - d * 0.5), Vec2::new(0.0, -1.0)),
            2 => (Rect { lo: Vec2::new(inner.lo.x, u - w * 0.5), hi: Vec2::new(inner.lo.x + d, u + w * 0.5) }, Vec2::new(inner.lo.x + d * 0.5, u), Vec2::new(1.0, 0.0)),
            _ => (Rect { lo: Vec2::new(inner.hi.x - d, u - w * 0.5), hi: Vec2::new(inner.hi.x, u + w * 0.5) }, Vec2::new(inner.hi.x - d * 0.5, u), Vec2::new(-1.0, 0.0)),
        };
        // The room left walkable: nothing reaching past its middle.
        let room_w = inner.hi - inner.lo;
        let deep = if side < 2 { d > room_w.y * 0.5 - 0.5 } else { d > room_w.x * 0.5 - 0.5 };
        // Something to search has the floor in front of it kept free, to
        // stand at it.
        let access = matches!(thing, Thing::Box(_)).then(|| match side {
            0 => Rect { lo: Vec2::new(rect.lo.x, rect.hi.y), hi: Vec2::new(rect.hi.x, rect.hi.y + ACCESS) },
            1 => Rect { lo: Vec2::new(rect.lo.x, rect.lo.y - ACCESS), hi: Vec2::new(rect.hi.x, rect.lo.y) },
            2 => Rect { lo: Vec2::new(rect.hi.x, rect.lo.y), hi: Vec2::new(rect.hi.x + ACCESS, rect.hi.y) },
            _ => Rect { lo: Vec2::new(rect.lo.x - ACCESS, rect.lo.y), hi: Vec2::new(rect.lo.x, rect.hi.y) },
        });
        let blocked = |r: &Rect| space.placed.iter().any(|p| p.overlaps(r, 0.05));
        if deep || blocked(&rect) || access.as_ref().is_some_and(blocked) || space.clear.iter().any(|c| c.overlaps(&rect, 0.0)) || (tall && space.windows.iter().any(|wn| wn.overlaps(&rect, 0.0))) {
            continue;
        }
        space.placed.push(rect);
        space.placed.extend(access);
        put(b, thing, space.floor, centre, facing, out);
        return true;
    }
    false
}

/// Furnish `b`.
pub fn furnish(b: &Building, dice: &mut Dice) -> Furnished {
    let mut out = Furnished::default();
    if b.plan.kind == Kind::Shell {
        return out;
    }
    let plan = &b.plan;
    // A landmark's sign on its front, over its door.
    // (The fire station's over its bay doors, between them.)
    let sign = match plan.kind {
        Kind::GunStore => Some((Fixture::SignGuns, f64::from(plan.w) * 0.5, SIGN_HIGH)),
        Kind::Police => Some((Fixture::SignPolice, f64::from(plan.w) * 0.5, SIGN_HIGH)),
        Kind::FireStation => Some((Fixture::SignFire, 5.0, super::landmark::BAY_DOOR.1 + 0.15)),
        Kind::School => Some((Fixture::SignSchool, f64::from(plan.w) * 0.5, SIGN_HIGH)),
        _ => None,
    };
    if let Some((sign, x, y)) = sign {
        let at = b.world(Vec3::new(x, y, -SIGN_OUT));
        let facing = b.turn(Vec3::new(0.0, 0.0, -1.0));
        out.pieces.push(Piece::new(Scenery::Fixture(sign), at, (-facing.x).atan2(-facing.z), 1.0));
    }
    let mut spaces: Vec<Space> = (0..plan.rooms.len()).map(|ri| Space::of(plan, ri)).collect();
    let mut homeless: Vec<Thing> = Vec::new();
    for space in &mut spaces {
        let room = plan.rooms[space.room];
        let inner = space.inner;
        if room.use_ == Use::Shop {
            // Rows of shelving down the shop, an aisle between each.
            let mut x = room.x0 + 3;
            while f64::from(x) + 2.5 <= f64::from(room.x1) {
                let mut z = f64::from(room.z0) + 3.0;
                while z + 1.8 <= f64::from(room.z1) - 1.5 {
                    let r = Rect { lo: Vec2::new(f64::from(x) - 0.3, z), hi: Vec2::new(f64::from(x) + 0.3, z + 1.8) };
                    if !space.clear.iter().any(|c| c.overlaps(&r, 0.0)) {
                        space.placed.push(r);
                        put(b, Thing::Box(Source::Shelf), space.floor, Vec2::new(f64::from(x), z + 0.9), Vec2::new(-1.0, 0.0), &mut out);
                    }
                    z += 1.9;
                }
                x += 3;
            }
        }
        if room.use_ == Use::Bay {
            // The engine, parked facing out of one of the bay's doors.
            let doors: Vec<f64> = plan.openings.iter().filter(|o| o.door && o.width >= super::landmark::BAY_DOOR.0 && plan.walls[o.wall].sides.contains(&Some(space.room))).map(|o| o.centre).collect();
            if let Some(&x) = doors.get(dice.next() as usize % doors.len().max(1)) {
                let z = f64::from(room.z0) + ENGINE_IN + ENGINE.1 * 0.5;
                let r = Rect { lo: Vec2::new(x - ENGINE.0 * 0.5, z - ENGINE.1 * 0.5), hi: Vec2::new(x + ENGINE.0 * 0.5, z + ENGINE.1 * 0.5) };
                space.placed.push(r);
                put(b, Thing::Box(Source::FireEngine), space.floor, Vec2::new(x, z), Vec2::new(0.0, -1.0), &mut out);
            }
        }
        if room.use_ == Use::GunShop {
            // A row of glass cases across the floor, facing the way in, a
            // gap at one end to get behind them.
            let z = f64::from(room.z0) + COUNTER_IN;
            let gap_left = dice.unit() < 0.5;
            let (x0, x1) = if gap_left { (f64::from(room.x0) + 2.0, f64::from(room.x1) - 0.6) } else { (f64::from(room.x0) + 0.6, f64::from(room.x1) - 2.0) };
            let n = ((x1 - x0) / DISPLAY.0).floor() as i32;
            let start = if gap_left { x1 - f64::from(n) * DISPLAY.0 } else { x0 };
            for k in 0..n {
                let x = start + (f64::from(k) + 0.5) * DISPLAY.0;
                let r = Rect { lo: Vec2::new(x - DISPLAY.0 * 0.5, z - DISPLAY.1 * 0.5), hi: Vec2::new(x + DISPLAY.0 * 0.5, z + DISPLAY.1 * 0.5) };
                if !space.clear.iter().any(|c| c.overlaps(&r, 0.0)) {
                    space.placed.push(r);
                    put(b, Thing::Box(Source::DisplayCase), space.floor, Vec2::new(x, z), Vec2::new(0.0, -1.0), &mut out);
                }
            }
            // Behind them kept free to stand at the racks on the far wall.
            space.placed.push(Rect { lo: Vec2::new(inner.lo.x, z + DISPLAY.1 * 0.5), hi: Vec2::new(inner.hi.x, z + DISPLAY.1 * 0.5 + 1.2) });
        }
        for (thing, chance) in program(room.use_, &room, b.plan.kind) {
            if dice.unit() > chance {
                continue;
            }
            if !against_wall(b, space, thing, dice, &mut out) && needed(thing) {
                homeless.push(thing);
            }
        }
        // Now and then something left lying about.
        if dice.unit() < 0.35 && room.use_ != Use::Hall {
            let p = Vec2::new(inner.lo.x + 0.8 + dice.unit() * (inner.hi.x - inner.lo.x - 1.6).max(0.0), inner.lo.y + 0.8 + dice.unit() * (inner.hi.y - inner.lo.y - 1.6).max(0.0));
            let at = b.world(Vec3::new(p.x, space.floor, p.y));
            let roll = dice.unit();
            let (kind, count) = if roll < 0.55 { (ItemKind::Rounds, crate::items::ROUNDS) } else if roll < 0.85 { (ItemKind::Bandage, (1, 1)) } else { (ItemKind::Medkit, (1, 1)) };
            out.pickups.push((kind, count, at.x, at.y + 1.2, at.z, dice.unit() * std::f64::consts::TAU));
        }
    }
    // What's needed and found no wall in its own room: in the biggest
    // other room it fits (never a hall or a bathroom).
    let mut by_size: Vec<usize> = (0..plan.rooms.len()).filter(|&i| !matches!(plan.rooms[i].use_, Use::Hall | Use::Bath)).collect();
    by_size.sort_by_key(|&i| (-plan.rooms[i].area(), i));
    for thing in homeless {
        by_size.iter().any(|&i| against_wall(b, &mut spaces[i], thing, dice, &mut out));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::plan;
    use super::*;

    #[test]
    fn rooms_are_furnished_for_what_they_are_and_doorways_left_clear() {
        let mut kitchens = 0;
        let mut fridges = 0;
        for seed in 1..60u32 {
            let mut dice = Dice(seed * 7_919);
            let p = plan::house(&mut dice, 10, 10, seed % 2 == 0);
            kitchens += p.rooms.iter().filter(|r| r.use_ == Use::Kitchen).count();
            let b = Building { plan: p, origin: Vec3::new(0.5, 0.25, 0.5), quarter: (seed % 4) as u8, seed };
            let f = furnish(&b, &mut dice);
            fridges += f.containers.iter().filter(|(s, _)| *s == Source::Fridge).count();
            assert!(!f.pieces.is_empty() && !f.containers.is_empty(), "seed {seed}: an empty house");
            // Everything within the house.
            for piece in &f.pieces {
                assert!(b.covers(Vec2::new(piece.at.x, piece.at.z), 0.0), "seed {seed}: {:?} outside", piece.what);
            }
        }
        assert!(fridges * 10 >= kitchens * 8, "{fridges} fridges in {kitchens} kitchens");
        let mut dice = Dice(9);
        let s = plan::store(&mut dice, 14, 15);
        let b = Building { plan: s, origin: Vec3::new(0.5, 0.25, 0.5), quarter: 0, seed: 1 };
        let f = furnish(&b, &mut dice);
        assert!(f.containers.iter().filter(|(s, _)| *s == Source::Shelf).count() >= 4, "{:?}", f.containers);
        assert_eq!(f.containers.iter().filter(|(s, _)| *s == Source::Register).count(), 1);
    }
}
