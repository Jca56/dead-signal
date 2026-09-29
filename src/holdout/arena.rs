//! The holdout's arena: a building laid out by hand (as data: its rooms,
//! the doorways between them, the windows the dead come in by, what's on
//! its walls to buy), set on flat ground. Each room belongs to a zone, and
//! the zones are opened up one by one, a door at a time.
//!
//! For now it's a test box: two rooms, a door between them.

use std::ops::Range;

use lntrn_math::Vec3;

use crate::loot::Kind;
use crate::map::building::Building;
use crate::map::building::plan::{self, Opening, Plan, Room, Use};
use crate::map::building::shape::RAISED;
use crate::map::{Map, scatter, terrain};
use crate::zombie::nav::Gate;

/// How far the dead's walking grid reaches from the middle, each way.
const REACH: f64 = 40.0;
/// A window the dead come in by: how wide, from how high to how high.
const WINDOW: (f64, f64, f64) = (1.2, 0.9, 2.1);
/// Where the dead stand outside a window, and land inside it; how far out
/// they come from.
const OUTSIDE: f64 = 0.7;
const INSIDE: f64 = 0.8;
const COME_FROM: f64 = 9.0;
/// How thick a door is, and how far past its doorway it reaches.
const DOOR_THICK: f64 = 0.16;
const DOOR_LAP: f64 = 0.08;
/// How high a wall buy's middle hangs.
const BUY_HEIGHT: f64 = 1.45;

/// Something for sale on a wall: a weapon (with its rounds), or a kit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wares {
    Weapon(Kind),
    Kit(Kind),
}

/// A window the dead come in by: its middle (on its sill), which way is
/// in, how wide and from how high to how high; where they stand outside
/// it, where they land inside, and where they come from; the zone it
/// lets into.
#[derive(Clone, Debug)]
pub struct Window {
    pub zone: usize,
    pub centre: Vec3,
    pub inward: Vec3,
    pub width: f64,
    pub sill: f64,
    pub head: f64,
    pub outside: Vec3,
    pub inside: Vec3,
    pub from: Vec3,
}

impl Window {
    /// Along it, flat (the way its boards run).
    pub fn across(&self) -> Vec3 {
        Vec3::new(-self.inward.z, 0.0, self.inward.x)
    }
}

/// A door to buy open (or debris to clear): the box it fills, what it
/// costs, the zones either side; what's solid of it, and the dead's ways
/// through it (both filled in once the arena's built).
#[derive(Clone, Debug)]
pub struct Door {
    pub lo: Vec3,
    pub hi: Vec3,
    pub cost: u32,
    pub zones: (usize, usize),
    pub solid: Range<u32>,
    pub gate: Gate,
}

/// Something on a wall to buy: where (on the wall's face, at eye level),
/// the way out of the wall, what, in which zone.
#[derive(Clone, Copy, Debug)]
pub struct Buy {
    pub at: Vec3,
    pub facing: Vec3,
    pub wares: Wares,
    pub zone: usize,
}

pub struct Arena {
    /// How far the walking grid need reach.
    pub reach: f64,
    pub zones: Vec<&'static str>,
    /// The zone the player starts in.
    pub start: usize,
    pub windows: Vec<Window>,
    pub doors: Vec<Door>,
    pub buys: Vec<Buy>,
}

// ---- the layout, as data ---------------------------------------------------
//
// In the building's frame: whole metres, x across its front, z back from
// it. A wall is a line: along x (at a z) or along z (at an x), and a place
// on it is how far along.

/// A room: its corners, and its zone.
struct RoomAt(i32, i32, i32, i32, usize);
/// A doorway on a line, how far along; to buy open for so much (none: open).
struct DoorAt(bool, i32, f64, Option<u32>);
/// A window on an outside wall's line, how far along.
struct WindowAt(bool, i32, f64);
/// A wall buy on a line, how far along, facing into +1 or -1 of it.
struct BuyAt(bool, i32, f64, f64, Wares);

struct Layout {
    w: i32,
    d: i32,
    zones: &'static [&'static str],
    rooms: &'static [RoomAt],
    doors: &'static [DoorAt],
    windows: &'static [WindowAt],
    buys: &'static [BuyAt],
    /// Where the player starts (and which way they face, a yaw).
    start: (f64, f64, f64),
}

const TEST_BOX: Layout = Layout {
    w: 16,
    d: 10,
    zones: &["START ROOM", "BACK ROOM"],
    rooms: &[RoomAt(0, 0, 8, 10, 0), RoomAt(8, 0, 16, 10, 1)],
    doors: &[DoorAt(false, 8, 5.5, Some(750))],
    windows: &[WindowAt(true, 0, 3.5), WindowAt(false, 0, 5.5), WindowAt(true, 10, 4.5), WindowAt(true, 0, 12.5), WindowAt(false, 16, 5.5)],
    buys: &[
        BuyAt(true, 10, 1.5, -1.0, Wares::Weapon(Kind::Rifle)),
        BuyAt(true, 0, 6.5, 1.0, Wares::Kit(Kind::Bandage)),
        BuyAt(true, 10, 12.5, -1.0, Wares::Weapon(Kind::Smg)),
        BuyAt(true, 0, 14.5, 1.0, Wares::Kit(Kind::Medkit)),
        BuyAt(false, 8, 2.5, 1.0, Wares::Weapon(Kind::Shotgun)),
    ],
    start: (4.0, 5.0, std::f64::consts::PI),
};

/// The arena, as a map (flat ground, its building) and what it holds.
pub fn generate() -> (Map, Arena) {
    let layout = &TEST_BOX;
    let seed = 1;
    let rooms: Vec<Room> = layout.rooms.iter().map(|r| Room { storey: 0, x0: r.0, z0: r.1, x1: r.2, z1: r.3, use_: Use::Office }).collect();
    let walls = plan::walls_of(&rooms, 0);
    let mut plan = Plan { kind: plan::Kind::Store, w: layout.w, d: layout.d, storeys: 1, rooms, walls, openings: Vec::new(), stair: None, flat_roof: true, ridge_along_x: true, ridge: plan::RIDGE, bars: Vec::new() };
    let wall = |plan: &Plan, along_x: bool, at: i32, centre: f64| plan::wall_at(&plan.walls, 0, along_x, at, centre).unwrap_or_else(|| panic!("no wall on ({along_x}, {at}) at {centre}"));
    for d in layout.doors {
        let w = wall(&plan, d.0, d.1, d.2);
        plan.openings.push(Opening { wall: w, centre: d.2, width: plan::DOOR_WIDTH, sill: 0.0, head: plan::DOOR_HEAD, door: true, boarded: false });
    }
    for win in layout.windows {
        let w = wall(&plan, win.0, win.1, win.2);
        plan.openings.push(Opening { wall: w, centre: win.2, width: WINDOW.0, sill: WINDOW.1, head: WINDOW.2, door: false, boarded: false });
    }
    // Set down with its middle on the map's, its corner on a half metre.
    let origin = Vec3::new(-(f64::from(layout.w) * 0.5).floor() + 0.5, RAISED, -(f64::from(layout.d) * 0.5).floor() + 0.5);
    let building = Building { plan: plan.clone(), origin, quarter: 0, seed };
    // A point on a line, how far along, `y` up.
    let on = |along_x: bool, at: i32, u: f64, y: f64| if along_x { Vec3::new(u, y, f64::from(at)) } else { Vec3::new(f64::from(at), y, u) };
    // Square to a line, towards +1 or -1 of it.
    let square = |along_x: bool, side: f64| if along_x { Vec3::new(0.0, 0.0, side) } else { Vec3::new(side, 0.0, 0.0) };
    let room_of = |p: Vec3| layout.rooms.iter().find(|r| p.x > f64::from(r.0) && p.x < f64::from(r.2) && p.z > f64::from(r.1) && p.z < f64::from(r.3)).map(|r| r.4);

    let windows = layout
        .windows
        .iter()
        .map(|win| {
            let w = plan.walls[wall(&plan, win.0, win.1, win.2)];
            // In is the side a room's on.
            let side = if w.sides[1].is_some() { 1.0 } else { -1.0 };
            let inward = square(win.0, side);
            let mid = on(win.0, win.1, win.2, 0.0);
            let zone = room_of(mid + inward * 0.5).expect("a window into a room");
            let ground = -RAISED;
            Window {
                zone,
                centre: building.world(mid + Vec3::new(0.0, WINDOW.1, 0.0)),
                inward: building.turn(inward),
                width: WINDOW.0,
                sill: WINDOW.1,
                head: WINDOW.2,
                outside: building.world(mid - inward * OUTSIDE + Vec3::new(0.0, ground, 0.0)),
                inside: building.world(mid + inward * INSIDE),
                from: building.world(mid - inward * COME_FROM + Vec3::new(0.0, ground, 0.0)),
            }
        })
        .collect();
    let doors = layout
        .doors
        .iter()
        .filter_map(|d| {
            let cost = d.3?;
            let half = plan::DOOR_WIDTH * 0.5 + DOOR_LAP;
            let (a, b) = (on(d.0, d.1, d.2 - half, 0.0) - square(d.0, DOOR_THICK * 0.5), on(d.0, d.1, d.2 + half, plan::DOOR_HEAD) + square(d.0, DOOR_THICK * 0.5));
            let (a, b) = (building.world(a), building.world(b));
            let mid = on(d.0, d.1, d.2, 0.0);
            let zones = (room_of(mid - square(d.0, 0.5)).expect("a room behind the door"), room_of(mid + square(d.0, 0.5)).expect("a room before the door"));
            Some(Door { lo: a.min(b), hi: a.max(b), cost, zones, solid: 0..0, gate: Gate::default() })
        })
        .collect();
    let buys = layout
        .buys
        .iter()
        .map(|b| {
            let facing = square(b.0, b.3);
            let at = on(b.0, b.1, b.2, BUY_HEIGHT) + facing * 0.1;
            let zone = room_of(at + facing * 0.5).expect("a buy in a room");
            Buy { at: building.world(at), facing: building.turn(facing), wares: b.4, zone }
        })
        .collect();
    let start_at = building.world(Vec3::new(layout.start.0, 0.0, layout.start.1));
    let start = room_of(Vec3::new(layout.start.0, 0.0, layout.start.1)).expect("a start in a room");

    let field = terrain::Field::new(seed, |_, _| 0.0);
    let map = Map {
        seed,
        field,
        roads: Vec::new(),
        sites: Vec::new(),
        fields: Vec::new(),
        spawn: (start_at, layout.start.2),
        exits: Vec::new(),
        containers: Vec::new(),
        pickups: Vec::new(),
        scenery: Vec::new(),
        buildings: vec![building],
        targets: Vec::new(),
        forest: scatter::Forest::new(seed),
        landmarks: Vec::new(),
    };
    let arena = Arena { reach: REACH, zones: layout.zones.to_vec(), start, windows, doors, buys };
    (map, arena)
}
