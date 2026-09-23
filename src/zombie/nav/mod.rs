//! Where the dead can walk: a grid of one-metre columns over the map,
//! each holding every floor a body can stand on in it (the ground; in a
//! house, its floors, one over the other; a flat roof), probed from the
//! solids at load. A floor links to one in each of its eight neighbouring
//! columns when the way between them is one a body can take: nothing in
//! the way at knee height (a wall; a doorway is a gap in it), and no
//! climb but a run of small steps (a flight of stairs, probed along its
//! length; a ledge not too high can be dropped off, one way); a diagonal
//! cuts no corner.
//! Paths are found with A* and pulled straight wherever a straight walk
//! stays on linked ground.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use lntrn_math::Vec3;

mod build;
mod regions;

use crate::collide::{Solids, WALKABLE};
use regions::Regions;
use crate::player::STEP_UP;

/// Cell size, metres.
const CELL: f64 = 1.0;
/// The most cells a search looks at before it gives up.
const SEARCH_LIMIT: usize = 40_000;
/// How high the probe starts looking down from.
const SKY: f64 = 80.0;
/// A body is tried for fit this far above the ground found: on stairs the
/// next riser is within its reach, and it would step up onto it anyway.
const FIT_LIFT: f64 = 0.25;
/// Neighbours this much apart in height, or less, are probed between for a
/// way of small steps; more is a wall or a drop.
const CLIMB: f64 = 1.5;
/// The highest ledge a body will drop off, and what the drop costs a
/// route (a little over walking, so a stair close by is still taken).
const DROP: f64 = 3.5;
const DROP_COST: f64 = 2.0;
/// How far apart the probes between two cells are.
const PROBE: f64 = 0.1;
/// What stepping into a cell at an edge (one not linked all round: beside
/// a wall, a drop, a ledge) costs over its length, so routes keep to the
/// middle of a ramp or a pad where there is one.
const EDGE_COST: f64 = 1.5;
/// How many cells round an unreachable goal to look for the nearest one
/// that can be reached.
const CLOSEST: i64 = 6;
/// The most floors a column holds.
const FLOORS: usize = 6;
/// How far over the higher of two floors the way between them must be
/// clear: over a stair's nosing, under a doorway's head.
const KNEE: f64 = 0.45;
/// No link.
const NONE: u32 = u32::MAX;
/// The eight neighbours, in the order their links are kept.
const AROUND: [(i64, i64); 8] = [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (1, -1), (-1, 1), (1, 1)];

pub struct NavGrid {
    /// How far the grid reaches from the middle of the map, each way.
    half: f64,
    /// Columns a side.
    side: usize,
    /// Each column's floors are the nodes `first[c]..first[c + 1]`, lowest
    /// first.
    first: Vec<u32>,
    /// Each node's floor height, and its column.
    height: Vec<f32>,
    column: Vec<u32>,
    /// Which node in each neighbouring column (by [`AROUND`]) a node's
    /// floor leads onto, if any, and which of those ways are drops.
    links: Vec<[u32; 8]>,
    drops: Vec<u8>,
    /// The nodes at an edge: not walkable to every neighbour (beside a
    /// wall, a drop, a ledge; a way down doesn't count).
    edges: Vec<bool>,
    /// Which floor can get to which.
    regions: Regions,
    /// Where on each floor a body best stands, from its column's middle, in
    /// quarter metres: off the middle only where that is on a ledge's lip
    /// (a stair narrower than a cell, a ramp's side).
    spots: Vec<(i8, i8)>,
}

/// A search's working: each node's cost so far and where it was come to
/// from, kept between searches (a map's worth is megabytes, too much to
/// clear every time) and told apart by which search wrote them.
#[derive(Default)]
struct Scratch {
    search: u32,
    stamp: Vec<u32>,
    costs: Vec<f64>,
    from: Vec<u32>,
}

impl Scratch {
    /// A new search over `nodes` nodes: whatever was written before is
    /// forgotten.
    fn begin(&mut self, nodes: usize) {
        if self.stamp.len() != nodes {
            *self = Self { search: 0, stamp: vec![0; nodes], costs: vec![f64::INFINITY; nodes], from: vec![NONE; nodes] };
        }
        self.search = self.search.wrapping_add(1);
        if self.search == 0 {
            self.stamp.fill(0);
            self.search = 1;
        }
    }

    fn cost(&self, i: u32) -> f64 {
        if self.stamp[i as usize] == self.search { self.costs[i as usize] } else { f64::INFINITY }
    }

    fn came(&self, i: u32) -> u32 {
        self.from[i as usize]
    }

    fn set(&mut self, i: u32, cost: f64, from: u32) {
        self.stamp[i as usize] = self.search;
        self.costs[i as usize] = cost;
        self.from[i as usize] = from;
    }
}

thread_local! {
    static SCRATCH: std::cell::RefCell<Scratch> = std::cell::RefCell::new(Scratch::default());
}

#[derive(Clone, Copy, PartialEq)]
struct Open {
    cost: f64,
    node: u32,
}
impl Eq for Open {}
impl Ord for Open {
    fn cmp(&self, o: &Self) -> Ordering {
        o.cost.partial_cmp(&self.cost).unwrap_or(Ordering::Equal)
    }
}
impl PartialOrd for Open {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

impl NavGrid {
    /// Column `c`'s floors.
    fn nodes(&self, (x, z): (usize, usize)) -> std::ops::Range<u32> {
        let c = z * self.side + x;
        self.first[c]..self.first[c + 1]
    }

    fn floor(&self, a: u32) -> f64 {
        f64::from(self.height[a as usize])
    }

    fn col_of(&self, a: u32) -> (usize, usize) {
        let c = self.column[a as usize] as usize;
        (c % self.side, c / self.side)
    }

    /// Whether a body on floor `a` can get to floor `b` at all (by any way,
    /// drops and all).
    fn reaches(&self, a: u32, b: u32) -> bool {
        self.regions.reaches(a as usize, b as usize)
    }

    /// The floor nearest `to` (a few metres round it, at most) that a body
    /// on floor `from` can get to: for when `to` itself can't be reached
    /// (up on a car's roof, say), the dead gather as close as they can.
    fn nearest_reachable(&self, from: u32, to: Vec3) -> Option<u32> {
        let centre = self.column_at(to)?;
        let mut best: Option<(u32, f64)> = None;
        for dz in -CLOSEST..=CLOSEST {
            for dx in -CLOSEST..=CLOSEST {
                let Some(c) = self.offset(centre, dx, dz) else { continue };
                for b in self.nodes(c) {
                    if !self.reaches(from, b) {
                        continue;
                    }
                    let d = (self.coord(c.0) - to.x).powi(2) + (self.coord(c.1) - to.z).powi(2) + (self.floor(b) - to.y).powi(2);
                    if best.is_none_or(|(_, bd)| d < bd) {
                        best = Some((b, d));
                    }
                }
            }
        }
        best.map(|(b, _)| b)
    }

    /// The column `(dx, dz)` from `c`, if on the grid.
    fn offset(&self, c: (usize, usize), dx: i64, dz: i64) -> Option<(usize, usize)> {
        let (x, z) = (c.0 as i64 + dx, c.1 as i64 + dz);
        (x >= 0 && z >= 0 && (x as usize) < self.side && (z as usize) < self.side).then_some((x as usize, z as usize))
    }

    fn coord(&self, i: usize) -> f64 {
        -self.half + i as f64 * CELL
    }

    fn column_at(&self, p: Vec3) -> Option<(usize, usize)> {
        let x = ((p.x + self.half) / CELL).round();
        let z = ((p.z + self.half) / CELL).round();
        (x >= 0.0 && z >= 0.0 && (x as usize) < self.side && (z as usize) < self.side).then_some((x as usize, z as usize))
    }

    /// The floor in `p`'s column nearest its height.
    fn node_at(&self, p: Vec3) -> Option<u32> {
        let c = self.column_at(p)?;
        self.nodes(c).min_by(|&a, &b| (self.floor(a) - p.y).abs().total_cmp(&(self.floor(b) - p.y).abs()))
    }

    fn edge(&self, a: u32) -> bool {
        self.edges[a as usize]
    }

    /// The floor node `a` leads onto in direction `k`, if any.
    fn link(&self, a: u32, k: usize) -> Option<u32> {
        let b = self.links[a as usize][k];
        (b != NONE).then_some(b)
    }

    /// The direction (by [`AROUND`]) from column of `a` to that of `b`, if
    /// they're neighbours.
    fn direction(&self, a: u32, b: u32) -> Option<usize> {
        let ((ax, az), (bx, bz)) = (self.col_of(a), self.col_of(b));
        let d = (bx as i64 - ax as i64, bz as i64 - az as i64);
        AROUND.iter().position(|&o| o == d)
    }

    /// Whether the way from floor `a` to its neighbour `b` is a drop.
    fn drop_to(&self, a: u32, b: u32) -> bool {
        self.direction(a, b).is_some_and(|k| self.drops[a as usize] & (1 << k) != 0)
    }

    /// Where a body stands on floor `a`: its column's middle, or off it
    /// (see `spots`).
    fn centre(&self, a: u32) -> Vec3 {
        let (x, z) = self.col_of(a);
        let (ox, oz) = self.spots[a as usize];
        Vec3::new(self.coord(x) + f64::from(ox) * 0.25, self.floor(a), self.coord(z) + f64::from(oz) * 0.25)
    }

    /// The floor a body stands on in the column holding `p` (the one
    /// nearest its height), if any.
    pub fn height_at(&self, p: Vec3) -> Option<f64> {
        self.node_at(p).map(|a| self.floor(a))
    }

    /// Whether a body can walk straight from `a` to `b` on linked floors.
    pub fn clear(&self, a: Vec3, b: Vec3) -> bool {
        match (self.nearest_open(a), self.nearest_open(b)) {
            (Some(na), Some(nb)) => self.straight(na, nb),
            _ => false,
        }
    }

    /// Whether a body can stand in the column holding `p`, near its height.
    pub fn walkable(&self, p: Vec3) -> bool {
        self.node_at(p).is_some_and(|a| (self.floor(a) - p.y).abs() < 2.5)
    }

    /// How many floors, all told, a body can stand on.
    #[cfg(test)]
    pub fn open_cells(&self) -> usize {
        self.height.len()
    }

    /// Whether a body with its feet at `a` can get to `b` at all (by any
    /// way, however far).
    pub fn connects(&self, a: Vec3, b: Vec3) -> bool {
        match (self.nearest_open(a), self.nearest_open(b)) {
            (Some(na), Some(nb)) => self.reaches(na, nb),
            _ => false,
        }
    }

    /// Whether a body can step from floor `a` to floor `b` in the next
    /// column: a diagonal only where it could as well go round either
    /// corner, so none is cut past a wall's end or a ramp's side.
    fn linked(&self, a: u32, b: u32) -> bool {
        let Some(k) = self.direction(a, b) else { return false };
        if self.link(a, k) != Some(b) {
            return false;
        }
        let (dx, dz) = AROUND[k];
        if dx == 0 || dz == 0 {
            return true;
        }
        let dir = |d: (i64, i64)| AROUND.iter().position(|&o| o == d).expect("one of the eight");
        let (along_x, along_z) = (dir((dx, 0)), dir((0, dz)));
        let via_x = self.link(a, along_x).is_some_and(|c| self.link(c, along_z) == Some(b));
        let via_z = self.link(a, along_z).is_some_and(|c| self.link(c, along_x) == Some(b));
        via_x && via_z
    }

    /// The floor a body with its feet at `p` is on: the nearest, but one at
    /// its feet's height before one that isn't (right under a ledge, the
    /// column it rounds to may hold only the ledge's top).
    fn nearest_open(&self, p: Vec3) -> Option<u32> {
        let (cx, cz) = self.column_at(p)?;
        let mut best: Option<(u32, f64)> = None;
        for dz in -3..=3i64 {
            for dx in -3..=3i64 {
                let Some((x, z)) = self.offset((cx, cz), dx, dz) else { continue };
                let flat = ((self.coord(x) - p.x).powi(2) + (self.coord(z) - p.z).powi(2)).sqrt();
                for a in self.nodes((x, z)) {
                    let score = flat + 4.0 * ((self.floor(a) - p.y).abs() - 0.5).max(0.0);
                    if best.is_none_or(|(_, b)| score < b) {
                        best = Some((a, score));
                    }
                }
            }
        }
        best.map(|(a, _)| a)
    }

    /// A walkable route from `from` to `to`: its turning points, the last
    /// being `to`'s floor. `None` when there is no way (or it is too far to
    /// find).
    pub fn path(&self, from: Vec3, to: Vec3) -> Option<Vec<Vec3>> {
        let start = self.nearest_open(from)?;
        let mut goal = self.nearest_open(to)?;
        // Known unreachable costs nothing to find out: head as near as can
        // be got instead of searching the whole grid in vain.
        if !self.reaches(start, goal) {
            goal = self.nearest_reachable(start, to)?;
        }
        let (gx, gz) = self.col_of(goal);
        let h = |a: u32| {
            let (x, z) = self.col_of(a);
            let (dx, dz) = ((x as f64 - gx as f64).abs(), (z as f64 - gz as f64).abs());
            dx.max(dz) + (std::f64::consts::SQRT_2 - 1.0) * dx.min(dz)
        };
        SCRATCH.with_borrow_mut(|scratch| {
            scratch.begin(self.height.len());
            let mut open = BinaryHeap::new();
            scratch.set(start, 0.0, NONE);
            open.push(Open { cost: h(start), node: start });
            let mut seen = 0;
            while let Some(Open { node, .. }) = open.pop() {
                if node == goal {
                    break;
                }
                seen += 1;
                if seen > SEARCH_LIMIT {
                    return None;
                }
                for (k, (dx, dz)) in AROUND.iter().enumerate() {
                    let Some(next) = self.link(node, k) else { continue };
                    if !self.linked(node, next) {
                        continue;
                    }
                    let step = if *dx != 0 && *dz != 0 { std::f64::consts::SQRT_2 } else { 1.0 };
                    let c = scratch.cost(node) + step + if self.edge(next) { EDGE_COST } else { 0.0 } + if self.drops[node as usize] & (1 << k) != 0 { DROP_COST } else { 0.0 };
                    if c < scratch.cost(next) {
                        scratch.set(next, c, node);
                        open.push(Open { cost: c + h(next), node: next });
                    }
                }
            }
            if !scratch.cost(goal).is_finite() {
                return None;
            }
            let mut nodes = vec![goal];
            let mut at = goal;
            while at != start {
                at = scratch.came(at);
                nodes.push(at);
            }
            nodes.reverse();
            Some(self.pull(&nodes))
        })
    }

    /// Whether a straight walk from floor `a` to floor `b` stays on linked
    /// floors, checked column by column along it, and keeps off edges on
    /// the way (a walk along a ledge's line hangs half off it).
    fn straight(&self, a: u32, b: u32) -> bool {
        let ((ax, az), (bx, bz)) = (self.col_of(a), self.col_of(b));
        let (dx, dz) = (bx as f64 - ax as f64, bz as f64 - az as f64);
        let steps = (dx.abs().max(dz.abs()) * 2.0).ceil().max(1.0) as usize;
        let mut at = a;
        let mut prev = (ax, az);
        for i in 1..=steps {
            let t = i as f64 / steps as f64;
            let c = ((ax as f64 + dx * t).round() as usize, (az as f64 + dz * t).round() as usize);
            if c == prev {
                continue;
            }
            let d = (c.0 as i64 - prev.0 as i64, c.1 as i64 - prev.1 as i64);
            let Some(k) = AROUND.iter().position(|&o| o == d) else { return false };
            let Some(next) = self.link(at, k).filter(|&n| self.linked(at, n)) else { return false };
            if next != b && self.edge(next) {
                return false;
            }
            at = next;
            prev = c;
        }
        at == b
    }

    /// The turning points of a route over floors: each kept point is as far
    /// along as a straight walk from the last one reaches.
    fn pull(&self, nodes: &[u32]) -> Vec<Vec3> {
        let mut out = Vec::new();
        let mut from = 0;
        while from + 1 < nodes.len() {
            let mut to = from + 1;
            while to + 1 < nodes.len() && self.straight(nodes[from], nodes[to + 1]) {
                to += 1;
            }
            out.push(self.centre(nodes[to]));
            from = to;
        }
        if out.is_empty() {
            out.push(self.centre(nodes[0]));
        }
        out
    }
}

/// Whether nothing stands between floors at `a` and `b` (neighbours) at
/// knee height over the higher of them: a wall, a railing, a table; a
/// doorway is a gap, and so is a window's frame (but its sill isn't).
fn passage(solids: &Solids, a: Vec3, b: Vec3) -> bool {
    let y = a.y.max(b.y) + KNEE;
    let (from, to) = (Vec3::new(a.x, y, a.z), Vec3::new(b.x, y, b.z));
    let d = to - from;
    let len = d.length();
    len < 1e-9 || solids.raycast(from, d * (1.0 / len), len).is_none()
}

/// Whether the ground from `a` to `b` (too far apart in height for one
/// step) is a run of small ones (a stair, a steep ramp, not a ledge),
/// allowing on the way down drops of up to `drop`.
fn steps_between(solids: &Solids, a: Vec3, b: Vec3, drop: f64) -> bool {
    let top = a.y.max(b.y) + STEP_UP;
    let n = ((b - a).length() / PROBE).ceil() as usize;
    let mut last = a.y;
    for i in 1..=n {
        let p = a + (b - a) * (i as f64 / n as f64);
        let Some(hit) = solids.raycast(Vec3::new(p.x, top, p.z), Vec3::new(0.0, -1.0, 0.0), top - a.y.min(b.y) + 0.5) else { return false };
        let rise = hit.point.y - last;
        if rise > STEP_UP || -rise > drop || (hit.normal.y < WALKABLE && rise.abs() > 0.02) {
            return false;
        }
        last = hit.point.y;
    }
    (last - b.y).abs() <= STEP_UP
}

#[cfg(test)]
mod tests;
