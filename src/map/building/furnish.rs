//! What's in the rooms: furniture against the walls (never in a doorway's
//! way, nothing tall across a window, the hall and its stairs left clear),
//! the things to search among it, a store's shelving in rows with aisles
//! the dead can walk, and a box of rounds or a kit left lying here and
//! there.

use lntrn_math::{Vec2, Vec3};

use super::Building;
use super::plan::{Kind, Room, STOREY, Use};
use crate::loot::tables::Source;
use crate::loot::{Dice, Kind as ItemKind};
use crate::map::Spot;
use crate::map::scatter::{Piece, Scenery};

/// A piece of furniture: its object in `furniture.glb`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Furn {
    Bed,
    Sofa,
    Armchair,
    Table,
    Counter,
    Stove,
    Bathtub,
    Toilet,
    Basin,
    Bookcase,
    Tv,
    HayBale,
    HayStack,
    WoodStove,
}

impl Furn {
    pub const ALL: [Furn; 14] = [Furn::Bed, Furn::Sofa, Furn::Armchair, Furn::Table, Furn::Counter, Furn::Stove, Furn::Bathtub, Furn::Toilet, Furn::Basin, Furn::Bookcase, Furn::Tv, Furn::HayBale, Furn::HayStack, Furn::WoodStove];

    pub fn name(self) -> &'static str {
        match self {
            Furn::Bed => "FURN_Bed",
            Furn::Sofa => "FURN_Sofa",
            Furn::Armchair => "FURN_Armchair",
            Furn::Table => "FURN_Table",
            Furn::Counter => "FURN_Counter",
            Furn::Stove => "FURN_Stove",
            Furn::Bathtub => "FURN_Bathtub",
            Furn::Toilet => "FURN_Toilet",
            Furn::Basin => "FURN_Basin",
            Furn::Bookcase => "FURN_Bookcase",
            Furn::Tv => "FURN_Tv",
            Furn::HayBale => "FURN_HayBale",
            Furn::HayStack => "FURN_HayStack",
            Furn::WoodStove => "FURN_WoodStove",
        }
    }
}

/// Something put in a room: furniture, or a thing to search.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Thing {
    Furn(Furn),
    Box(Source),
}

impl Thing {
    /// Its footprint (along the wall it backs onto, out from it), and
    /// whether it stands tall enough to cover a window.
    fn size(self) -> (f64, f64, bool) {
        match self {
            Thing::Furn(f) => match f {
                Furn::Bed => (1.6, 2.1, false),
                Furn::Sofa => (2.0, 0.9, false),
                Furn::Armchair => (0.9, 0.9, false),
                Furn::Table => (2.1, 0.8, false),
                Furn::Counter => (1.8, 0.65, false),
                Furn::Stove => (0.7, 0.65, false),
                Furn::Bathtub => (1.7, 0.75, false),
                Furn::Toilet => (0.45, 0.7, false),
                Furn::Basin => (0.6, 0.5, false),
                Furn::Bookcase => (1.0, 0.35, true),
                Furn::Tv => (1.2, 0.45, false),
                Furn::HayBale => (1.1, 0.5, false),
                Furn::HayStack => (1.1, 1.0, true),
                Furn::WoodStove => (0.6, 0.55, true),
            },
            Thing::Box(s) => match s {
                Source::Fridge => (0.75, 0.7, true),
                Source::Cabinet => (1.04, 0.52, false),
                Source::Desk => (1.3, 0.65, false),
                Source::Wardrobe => (1.24, 0.6, true),
                Source::Shelf => (1.8, 0.6, true),
                Source::Register => (2.06, 0.86, false),
                Source::Locker | Source::ToolLocker => (0.6, 0.55, true),
                Source::GunCabinet | Source::HunterCabinet => (0.84, 0.5, true),
                _ => (1.0, 0.7, false),
            },
        }
    }
}

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

/// What goes in a room of each use: what must, then what might (and how
/// likely).
fn program(use_: Use, room: &Room, kind: Kind) -> Vec<(Thing, f64)> {
    use Furn::*;
    let f = |x: Furn, p: f64| (Thing::Furn(x), p);
    let b = |s: Source, p: f64| (Thing::Box(s), p);
    let big = (room.x1 - room.x0).min(room.z1 - room.z0) >= 4;
    match use_ {
        Use::Living => vec![f(Sofa, 1.0), f(Tv, 0.9), f(Armchair, 0.6), b(Source::Desk, 0.45), b(Source::Cabinet, 0.4), f(Bookcase, 0.5)],
        Use::Kitchen => vec![f(Counter, 1.0), b(Source::Fridge, 1.0), f(Stove, 1.0), b(Source::Cabinet, 0.7), f(Table, if big { 0.9 } else { 0.4 })],
        Use::Bed => vec![f(Bed, 1.0), b(Source::Wardrobe, 0.9), b(Source::Cabinet, 0.6), b(Source::Desk, 0.3)],
        Use::Bath => vec![f(Bathtub, 0.9), f(Toilet, 1.0), f(Basin, 1.0), b(Source::Cabinet, 0.25)],
        Use::Back => vec![b(Source::Crate, 1.0), b(Source::Crate, 0.6), b(Source::Locker, 0.35), b(Source::Shelf, 0.5)],
        Use::Shop => vec![b(Source::Register, 1.0)],
        Use::Hall => Vec::new(),
        // The gun cabinet first: it has the pick of the walls.
        Use::Den => vec![b(if kind == Kind::Cabin { Source::HunterCabinet } else { Source::GunCabinet }, 1.0), f(WoodStove, 0.9), f(Armchair, 0.8), f(Table, 0.6), f(Bookcase, 0.5), b(Source::Cabinet, 0.5), f(Sofa, 0.35)],
        Use::Barn => vec![f(HayStack, 1.0), b(Source::Crate, 1.0), f(HayStack, 0.8), f(HayBale, 1.0), b(Source::ToolLocker, 1.0), b(Source::Crate, 0.6), f(HayBale, 0.7), f(HayBale, 0.5)],
    }
}

/// Furnish `b`.
pub fn furnish(b: &Building, dice: &mut Dice) -> Furnished {
    let mut out = Furnished::default();
    if b.plan.kind == Kind::Shell {
        return out;
    }
    let plan = &b.plan;
    for (ri, room) in plan.rooms.iter().enumerate() {
        let floor = f64::from(room.storey) * STOREY;
        let inner = Rect { lo: Vec2::new(f64::from(room.x0) + 0.1, f64::from(room.z0) + 0.1), hi: Vec2::new(f64::from(room.x1) - 0.1, f64::from(room.z1) - 0.1) };
        // Kept clear: in front of every doorway, the stairs and their foot.
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
        // Windows in this room's walls: nothing tall in front of them.
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
        let mut placed: Vec<Rect> = Vec::new();
        let put = |thing: Thing, centre: Vec2, facing: Vec2, out: &mut Furnished| {
            let at = b.world(Vec3::new(centre.x, floor, centre.y));
            let inward = b.turn(Vec3::new(facing.x, 0.0, facing.y));
            let yaw = (-inward.x).atan2(-inward.z);
            match thing {
                Thing::Furn(f) => out.pieces.push(Piece::new(Scenery::Furn(f), at, yaw, 1.0)),
                Thing::Box(s) => out.containers.push((s, (at.x, at.z, yaw, at.y + 1.5))),
            }
        };
        if room.use_ == Use::Shop {
            // Rows of shelving down the shop, an aisle between each.
            let mut x = room.x0 + 3;
            while f64::from(x) + 2.5 <= f64::from(room.x1) {
                let mut z = f64::from(room.z0) + 3.0;
                while z + 1.8 <= f64::from(room.z1) - 1.5 {
                    let r = Rect { lo: Vec2::new(f64::from(x) - 0.3, z), hi: Vec2::new(f64::from(x) + 0.3, z + 1.8) };
                    if !clear.iter().any(|c| c.overlaps(&r, 0.0)) {
                        placed.push(r);
                        put(Thing::Box(Source::Shelf), Vec2::new(f64::from(x), z + 0.9), Vec2::new(-1.0, 0.0), &mut out);
                    }
                    z += 1.9;
                }
                x += 3;
            }
        }
        for (thing, chance) in program(room.use_, room, b.plan.kind) {
            if dice.unit() > chance {
                continue;
            }
            let (w, d, tall) = thing.size();
            // Against a wall, facing into the room: a few tries. The gun
            // cabinet (a den's reason to be looked in) gets more, and at the
            // last may stand across a window.
            let tries = if matches!(thing, Thing::Box(Source::GunCabinet | Source::HunterCabinet)) { 64 } else { 24 };
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
                if deep || placed.iter().any(|p| p.overlaps(&rect, 0.05)) || clear.iter().any(|c| c.overlaps(&rect, 0.0)) || (tall && windows.iter().any(|wn| wn.overlaps(&rect, 0.0))) {
                    continue;
                }
                placed.push(rect);
                put(thing, centre, facing, &mut out);
                break;
            }
        }
        // Now and then something left lying about.
        if dice.unit() < 0.35 && room.use_ != Use::Hall {
            let p = Vec2::new(inner.lo.x + 0.8 + dice.unit() * (inner.hi.x - inner.lo.x - 1.6).max(0.0), inner.lo.y + 0.8 + dice.unit() * (inner.hi.y - inner.lo.y - 1.6).max(0.0));
            let at = b.world(Vec3::new(p.x, floor, p.y));
            let roll = dice.unit();
            let (kind, count) = if roll < 0.55 { (ItemKind::Rounds, crate::items::ROUNDS) } else if roll < 0.85 { (ItemKind::Bandage, (1, 1)) } else { (ItemKind::Medkit, (1, 1)) };
            out.pickups.push((kind, count, at.x, at.y + 1.2, at.z, dice.unit() * std::f64::consts::TAU));
        }
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
