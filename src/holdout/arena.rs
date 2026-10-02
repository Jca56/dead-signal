//! The holdout's arena: the Relay Station (`relay.rs`, data as `layout.rs`
//! has it, made real by `raise.rs`), on its hill (`land.rs`). Its zones
//! are opened up one by one, a door at a time; the dead come in by its
//! windows and the holes in its walls, every one of them on the woods.

use std::ops::Range;

use lntrn_math::{Vec2, Vec3};

use super::layout::{Layout, OFFSET};
use super::{land, raise, relay};
use crate::loot::Kind;
use crate::map::Map;
use crate::zombie::nav::Gate;

/// How far the dead's walking grid reaches past the compound's farthest
/// wall (from the map's middle, each way): out to where they come from.
const BEYOND: f64 = 16.0;
/// The luck the land and its woods are made by (the same every time: the
/// arena's made by hand).
const SEED: u32 = 0x5E1A_7;
/// No tree this near where the dead come from.
const WOODS_BACK: f64 = 6.0;

/// Something for sale on a wall: a weapon (with its rounds), a kit, or
/// something to wear (armor: put on as it's bought).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wares {
    Weapon(Kind),
    Kit(Kind),
    Gear(Kind),
}

impl Wares {
    /// The thing itself.
    pub fn kind(self) -> Kind {
        let (Wares::Weapon(kind) | Wares::Kit(kind) | Wares::Gear(kind)) = self;
        kind
    }
}

/// A way in for the dead (a window, a hole in a wall, a breach in a
/// cellar's from a tunnel): its middle (on its sill), which way is in, how
/// wide and from how high to how high; where they stand outside it, where
/// they land inside, where they come from and how far about that they
/// start; the zone it lets into.
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
    pub spread: f64,
}

impl Window {
    /// Along it, flat (the way its boards run).
    pub fn across(&self) -> Vec3 {
        Vec3::new(-self.inward.z, 0.0, self.inward.x)
    }
}

/// A door to buy open (or a heap of junk to clear): the box it fills,
/// what it costs, the zones either side, whether it's a heap; what's solid
/// of it, and the dead's ways through it (both filled in once the arena's
/// built).
#[derive(Clone, Debug)]
pub struct Door {
    pub lo: Vec3,
    pub hi: Vec3,
    pub cost: u32,
    pub zones: (usize, usize),
    pub heap: bool,
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

/// The arena, as a map (its hill, its buildings and walls, what stands
/// about) and what it holds.
pub fn generate() -> (Map, Arena) {
    of(&relay::RELAY_STATION)
}

/// An arena laid out as `plan` has it.
pub(super) fn of(plan: &Layout) -> (Map, Arena) {
    let raised = raise::raise(plan);
    let (x0, z0, x1, z1) = plan.bounds;
    let corner = |x: i32, z: i32| Vec2::new(f64::from(x) + OFFSET, f64::from(z) + OFFSET);
    let (gate, out) = plan.gate().expect("a gate");
    let keep_out: Vec<(Vec2, f64)> = raised.windows.iter().map(|w| (Vec2::new(w.from.x, w.from.z), WOODS_BACK)).collect();
    let land = land::lay(SEED, corner(x0, z0), corner(x1, z1), gate + Vec2::splat(OFFSET), out, &keep_out);
    let mut scenery = raised.pieces;
    scenery.extend(land.scenery);
    let map = Map {
        seed: SEED,
        field: land.field,
        roads: land.roads,
        sites: vec![land.site],
        fields: Vec::new(),
        spawn: raised.start,
        exits: Vec::new(),
        containers: raised.containers,
        pickups: Vec::new(),
        scenery,
        buildings: raised.buildings,
        targets: Vec::new(),
        forest: land.forest,
        landmarks: Vec::new(),
        blocks: raised.blocks,
        digs: raised.digs,
    };
    let reach = [x0, z0, x1, z1].iter().map(|v| f64::from(v.abs())).fold(0.0, f64::max) + BEYOND;
    let arena = Arena { reach, zones: plan.zones.to_vec(), start: raised.start_zone, windows: raised.windows, doors: raised.doors, buys: raised.buys };
    (map, arena)
}
