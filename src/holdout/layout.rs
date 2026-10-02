//! A holdout's arena as data (`raise.rs` makes it real): its buildings
//! (each a plan of rooms, as `map/building` builds them, every room in a
//! zone), the runs of wall between and round them (holes in them boarded
//! against the dead, a gate, a gap heaped with junk to be cleared), the
//! yards (zones out in the open), what's on the walls to buy, and what
//! stands about.
//!
//! Floors are told by their level: 0 the ground's, 1 the one over it, -1
//! a cellar's.
//!
//! It's all drawn on a grid of whole metres, walls standing on its lines:
//! the grid is the map's shifted half a metre, so the walls fall between
//! the dead's walking grid's points and every doorway's middle is on one.

use lntrn_math::{Vec2, Vec3};

use super::arena::{Wares, Way};
use crate::loot::tables::Source;
use crate::map::building::furnish::Furn;
use crate::map::building::plan::{Kind, STOREY, Use};
use crate::map::building::shape::RAISED;
use crate::map::outposts::Fixture;

/// The grid's (0, 0) on the map.
pub const OFFSET: f64 = 0.5;

/// Which way something faces (north is -z).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    N,
    S,
    E,
    W,
}

impl Face {
    pub fn dir(self) -> Vec2 {
        match self {
            Face::N => Vec2::new(0.0, -1.0),
            Face::S => Vec2::new(0.0, 1.0),
            Face::E => Vec2::new(1.0, 0.0),
            Face::W => Vec2::new(-1.0, 0.0),
        }
    }
}

/// A place on a wall's line: on the line running east–west at z `at`, `u`
/// along it (x); or north–south at x `at`, `u` along it (z).
#[derive(Clone, Copy, Debug)]
pub struct Line {
    pub along_x: bool,
    pub at: i32,
    pub u: f64,
}

/// On the east–west line at `z`, at `x`.
pub const fn ew(z: i32, x: f64) -> Line {
    Line { along_x: true, at: z, u: x }
}

/// On the north–south line at `x`, at `z`.
pub const fn ns(x: i32, z: f64) -> Line {
    Line { along_x: false, at: x, u: z }
}

impl Line {
    pub(super) fn point(self) -> Vec2 {
        if self.along_x { Vec2::new(self.u, f64::from(self.at)) } else { Vec2::new(f64::from(self.at), self.u) }
    }

    /// Along it, and square to it (towards +1 or -1).
    pub(super) fn along(self) -> Vec2 {
        if self.along_x { Vec2::new(1.0, 0.0) } else { Vec2::new(0.0, 1.0) }
    }

    pub(super) fn square(self, side: f64) -> Vec2 {
        if self.along_x { Vec2::new(0.0, side) } else { Vec2::new(side, 0.0) }
    }
}

/// A room: its level, its corners (in its building's metres), what it's
/// for (`Use::Void`: the air over a tall room under it), and its zone.
pub struct RoomAt(pub i8, pub i32, pub i32, pub i32, pub i32, pub Use, pub usize);

/// A doorway on a line (in its building's metres), on a level, how wide;
/// a door in it to buy open for so much (none: open).
pub struct DoorAt(pub Line, pub i8, pub f64, pub Option<u32>);

/// A window on an outside wall's line, on a level: one the dead come in
/// by (boarded), or only to look out of.
pub struct WindowAt(pub Line, pub i8, pub bool);

/// A rail on a wall's line, on a level, how long: the wall waist high and
/// open over it (a mezzanine's edge on the room it looks down into). Seen
/// and shot over, never climbed.
pub struct RailAt(pub Line, pub i8, pub f64);

/// A flight of stairs up from a level to the next: against the wall at x,
/// climbing from z towards +z.
pub struct StairAt(pub i8, pub i32, pub f64);

/// A building: where its corner is on the grid, what it's built as, how
/// big, how many storeys over the ground and how many cellars under it,
/// its roof flat or not, its rooms and doorways and windows, its rails and
/// stairs, and the luck its colours and furniture are drawn by.
pub struct House {
    pub name: &'static str,
    pub at: (i32, i32),
    pub kind: Kind,
    pub size: (i32, i32),
    pub storeys: u8,
    pub cellars: u8,
    pub flat_roof: bool,
    pub rooms: &'static [RoomAt],
    pub doors: &'static [DoorAt],
    pub windows: &'static [WindowAt],
    pub rails: &'static [RailAt],
    pub stairs: &'static [StairAt],
    pub seed: u32,
}

impl House {
    /// The storey of its plan (counted from its lowest cellar) a level is.
    pub(super) fn storey(&self, level: i8) -> u8 {
        u8::try_from(i16::from(level) + i16::from(self.cellars)).unwrap_or_else(|_| panic!("{}: no level {level}", self.name))
    }
}

/// What a run of wall is: the compound's own, tall and wired along its
/// top; or a lower one within it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Build {
    Perimeter,
    Inner,
}

impl Build {
    /// How thick, and how high.
    pub(super) fn size(self) -> (f64, f64) {
        match self {
            Build::Perimeter => (0.24, 3.0),
            Build::Inner => (0.2, 2.6),
        }
    }
}

/// A gap in a run of wall, how far along: a hole the dead come in by; the
/// gate (how wide); a gap heaped with junk to clear (how wide, what it
/// costs).
pub enum Gap {
    Hole(f64),
    Gate(f64, f64),
    Heap(f64, f64, u32),
}

/// A run of wall on a line (east–west or not, at), from and to along it.
pub struct Run {
    pub along_x: bool,
    pub at: i32,
    pub from: i32,
    pub to: i32,
    pub build: Build,
    pub gaps: &'static [Gap],
}

/// A zone out in the open: a rectangle of the grid (the first that holds a
/// point has it, after the buildings).
pub struct Yard(pub i32, pub i32, pub i32, pub i32, pub usize);

/// Something for sale on a wall: where on its line, on which level,
/// facing which way (out of the wall), what.
pub struct BuyAt(pub Line, pub i8, pub Face, pub Wares);

/// A sign to the Amplifier on a wall: where on its line, on which level,
/// facing which way (out of the wall), pointing which way.
pub struct SignAt(pub Line, pub i8, pub Face, pub Way);

/// What stands about, on the ground or a building's ground floor: the
/// radio mast; one of the places' fixtures (where, its front facing a
/// bearing, degrees clockwise from north); something else set down (and
/// from how high over its floor it's dropped, to stack it on what's
/// under); a piece of furniture. Or one of those on another level of the
/// building it's in.
pub enum Prop {
    Tower(f64, f64),
    Fixture(Fixture, f64, f64, f64),
    Thing(Source, f64, f64, f64, f64),
    Furn(Furn, f64, f64, f64),
    On(i8, &'static Prop),
}

/// A flood out on a wall or a post: where (a grid point), and how high.
pub struct FloodAt(pub f64, pub f64, pub f64);

pub struct Layout {
    pub zones: &'static [&'static str],
    /// Where the player starts, facing a bearing.
    pub start: (f64, f64, f64),
    /// The compound's corners: outside them is outside every zone.
    pub bounds: (i32, i32, i32, i32),
    pub houses: &'static [House],
    pub runs: &'static [Run],
    pub yards: &'static [Yard],
    pub buys: &'static [BuyAt],
    pub signs: &'static [SignAt],
    pub floods: &'static [FloodAt],
    pub props: &'static [Prop],
}

/// A grid point on the map, `y` up.
pub(super) fn world(p: Vec2, y: f64) -> Vec3 {
    Vec3::new(p.x + OFFSET, y, p.y + OFFSET)
}

/// A bearing as a yaw (what turns a thing's front, -z, to face it).
pub(super) fn yaw(bearing: f64) -> f64 {
    -bearing.to_radians()
}

impl Layout {
    /// The zone a grid point on `level` is in; none outside. (Out in the
    /// open there's only the ground.)
    pub fn zone_at(&self, p: Vec2, level: i8) -> Option<usize> {
        let (x0, z0, x1, z1) = self.bounds;
        if p.x <= f64::from(x0) || p.x >= f64::from(x1) || p.y <= f64::from(z0) || p.y >= f64::from(z1) {
            return None;
        }
        for h in self.houses {
            let l = p - Vec2::new(f64::from(h.at.0), f64::from(h.at.1));
            if l.x > 0.0 && l.y > 0.0 && l.x < f64::from(h.size.0) && l.y < f64::from(h.size.1) {
                return h.rooms.iter().find(|r| r.0 == level && l.x > f64::from(r.1) && l.y > f64::from(r.2) && l.x < f64::from(r.3) && l.y < f64::from(r.4)).map(|r| r.6);
            }
        }
        if level != 0 {
            return None;
        }
        self.yards.iter().find(|y| p.x > f64::from(y.0) && p.y > f64::from(y.1) && p.x < f64::from(y.2) && p.y < f64::from(y.3)).map(|y| y.4)
    }

    /// The building a grid point's in, if it's in one.
    pub(super) fn house_at(&self, p: Vec2) -> Option<&House> {
        self.houses.iter().find(|h| {
            let l = p - Vec2::new(f64::from(h.at.0), f64::from(h.at.1));
            l.x > 0.0 && l.y > 0.0 && l.x < f64::from(h.size.0) && l.y < f64::from(h.size.1)
        })
    }

    /// Where the gate is (a grid point) and which way it opens out.
    pub fn gate(&self) -> Option<(Vec2, Vec2)> {
        self.runs.iter().find_map(|run| {
            run.gaps.iter().find_map(|gap| {
                let Gap::Gate(u, _) = *gap else { return None };
                let on = if run.along_x { ew(run.at, u) } else { ns(run.at, u) };
                let out = if self.zone_at(on.point() + on.square(0.5), 0).is_none() { on.square(1.0) } else { on.square(-1.0) };
                Some((on.point(), out))
            })
        })
    }

    /// The floor's height at a grid point on `level`.
    pub(super) fn floor_at(&self, p: Vec2, level: i8) -> f64 {
        self.house_at(p).map_or(0.0, |_| RAISED + f64::from(level) * STOREY)
    }
}
