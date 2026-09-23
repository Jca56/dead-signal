//! Where the dead can walk: a grid of one-metre cells over the map, each
//! either ground a body can stand on (and at what height) or not, probed
//! from the solids at load. Cells link to their eight neighbours when the
//! way between them is one a body can take (a flight of stairs is many
//! small steps, probed along its length; a ledge not too high can be
//! dropped off, one way) and a diagonal cuts no corner.
//! Paths are found with A* and pulled straight wherever a straight walk
//! stays on linked ground.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use lntrn_math::Vec3;

mod regions;

use crate::collide::{Capsule, Solids, WALKABLE};
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
/// The eight neighbours, in the order their links are kept.
const AROUND: [(i64, i64); 8] = [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (1, -1), (-1, 1), (1, 1)];

pub struct NavGrid {
    /// How far the grid reaches from the middle of the map, each way.
    half: f64,
    /// Cells a side.
    side: usize,
    /// The ground's height in each cell, or NaN where there is none a body
    /// can stand on.
    height: Vec<f32>,
    /// Which of its neighbours (by [`AROUND`]) each cell's ground leads
    /// onto, a bit each, and which of those ways is a drop.
    links: Vec<u8>,
    drops: Vec<u8>,
    /// The cells at an edge: not walkable to every neighbour (beside a
    /// wall, a drop, a ledge; a way down doesn't count).
    edges: Vec<bool>,
    /// Which ground can get to which.
    regions: Regions,
    /// Where in each cell a body best stands, from its middle, in quarter
    /// metres: off the middle only where that is on a ledge's lip (a stair
    /// narrower than a cell, a ramp's side).
    spots: Vec<(i8, i8)>,
}

/// A search's working: each cell's cost so far and where it was come to
/// from, kept between searches (a map's worth is megabytes, too much to
/// clear every time) and told apart by which search wrote them.
#[derive(Default)]
struct Scratch {
    search: u32,
    stamp: Vec<u32>,
    costs: Vec<f64>,
    from: Vec<usize>,
}

impl Scratch {
    /// A new search over `cells` cells: whatever was written before is
    /// forgotten.
    fn begin(&mut self, cells: usize) {
        if self.stamp.len() != cells {
            *self = Self { search: 0, stamp: vec![0; cells], costs: vec![f64::INFINITY; cells], from: vec![usize::MAX; cells] };
        }
        self.search = self.search.wrapping_add(1);
        if self.search == 0 {
            self.stamp.fill(0);
            self.search = 1;
        }
    }

    fn cost(&self, i: usize) -> f64 {
        if self.stamp[i] == self.search { self.costs[i] } else { f64::INFINITY }
    }

    fn came(&self, i: usize) -> usize {
        self.from[i]
    }

    fn set(&mut self, i: usize, cost: f64, from: usize) {
        self.stamp[i] = self.search;
        self.costs[i] = cost;
        self.from[i] = from;
    }
}

thread_local! {
    static SCRATCH: std::cell::RefCell<Scratch> = std::cell::RefCell::new(Scratch::default());
}

#[derive(Clone, Copy, PartialEq)]
struct Open {
    cost: f64,
    cell: usize,
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
    /// Probe `solids` for every cell a body of `body` could stand in, over
    /// the square `half` metres each way from the middle. The rows are
    /// shared out over every core.
    pub fn build(solids: &Solids, body: Capsule, half: f64) -> Self {
        let side = (2.0 * half / CELL) as usize + 1;
        let mut grid = Self { half, side, height: vec![f32::NAN; side * side], links: vec![0; side * side], drops: vec![0; side * side], edges: vec![false; side * side], regions: Regions::default(), spots: vec![(0, 0); side * side] };
        let heights = crate::map::rows(side, |z| {
            (0..side)
                .map(|x| {
                    let from = Vec3::new(grid.coord(x), SKY, grid.coord(z));
                    match solids.raycast(from, Vec3::new(0.0, -1.0, 0.0), SKY * 2.0) {
                        Some(hit) if hit.normal.y >= WALKABLE && solids.fits(body, hit.point + Vec3::new(0.0, FIT_LIFT, 0.0)) => hit.point.y as f32,
                        _ => f32::NAN,
                    }
                })
                .collect()
        });
        grid.height = heights;
        let ways = crate::map::rows(side, |z| {
            (0..side)
                .map(|x| {
                    let Some(ha) = grid.ground(x, z) else { return (0u8, 0u8) };
                    let (mut walks, mut drops) = (0u8, 0u8);
                    for (k, (dx, dz)) in AROUND.iter().enumerate() {
                        let Some((nx, nz)) = grid.offset((x, z), *dx, *dz) else { continue };
                        let Some(hb) = grid.ground(nx, nz) else { continue };
                        let dh = (ha - hb).abs();
                        let a = Vec3::new(grid.coord(x), ha, grid.coord(z));
                        let b = Vec3::new(grid.coord(nx), hb, grid.coord(nz));
                        if dh <= STEP_UP || (dh <= CLIMB && steps_between(solids, a, b, STEP_UP)) {
                            walks |= 1 << k;
                        } else if ha - hb <= DROP && steps_between(solids, a, b, DROP) {
                            drops |= 1 << k;
                        }
                    }
                    (walks, drops)
                })
                .collect()
        });
        for (i, (walks, drops)) in ways.into_iter().enumerate() {
            grid.links[i] = walks | drops;
            grid.drops[i] = drops;
            grid.edges[i] = !grid.height[i].is_nan() && walks != 0xFF;
        }
        grid.regions = Regions::build(&grid);
        grid.spots = crate::map::rows(side, |z| (0..side).map(|x| if grid.edges[z * side + x] { grid.best_spot(solids, (x, z), body.radius) } else { (0, 0) }).collect());
        grid
    }

    /// Where in the cell a body of `radius` stands on even ground: its
    /// middle if that will do, else the nearest point (up to half a metre
    /// off) where the ground all round under it is at one height.
    fn best_spot(&self, solids: &Solids, (x, z): (usize, usize), radius: f64) -> (i8, i8) {
        let Some(h) = self.ground(x, z) else { return (0, 0) };
        let floor = |px: f64, pz: f64| solids.raycast(Vec3::new(px, h + 1.0, pz), Vec3::new(0.0, -1.0, 0.0), 2.0).map(|hit| hit.point.y);
        let even = |ox: i8, oz: i8| {
            let (px, pz) = (self.coord(x) + f64::from(ox) * 0.25, self.coord(z) + f64::from(oz) * 0.25);
            let at = floor(px, pz).filter(|y| (y - h).abs() < 0.3);
            at.is_some_and(|y| [(radius, 0.0), (-radius, 0.0), (0.0, radius), (0.0, -radius)].iter().all(|(dx, dz)| floor(px + dx, pz + dz).is_some_and(|f| (f - y).abs() < STEP_UP * 0.5)))
        };
        if even(0, 0) {
            return (0, 0);
        }
        let mut offs: Vec<(i8, i8)> = (-2..=2).flat_map(|ox| (-2..=2).map(move |oz| (ox, oz))).filter(|&o| o != (0, 0)).collect();
        offs.sort_by_key(|&(ox, oz)| ox * ox + oz * oz);
        offs.into_iter().find(|&(ox, oz)| even(ox, oz)).unwrap_or((0, 0))
    }

    /// Whether a body in cell `a` can get to cell `b` at all (by any way,
    /// drops and all).
    fn reaches(&self, (ax, az): (usize, usize), (bx, bz): (usize, usize)) -> bool {
        self.regions.reaches(az * self.side + ax, bz * self.side + bx)
    }

    /// The cell nearest `to` (a few metres round it, at most) that a body in
    /// cell `from` can get to: for when `to` itself can't be reached (up on
    /// a car's roof, say), the dead gather as close as they can.
    fn nearest_reachable(&self, from: (usize, usize), to: Vec3) -> Option<(usize, usize)> {
        let centre = self.cell_at(to)?;
        let mut best: Option<((usize, usize), f64)> = None;
        for dz in -CLOSEST..=CLOSEST {
            for dx in -CLOSEST..=CLOSEST {
                let Some(c) = self.offset(centre, dx, dz) else { continue };
                if self.ground(c.0, c.1).is_none() || !self.reaches(from, c) {
                    continue;
                }
                let d = (self.coord(c.0) - to.x).powi(2) + (self.coord(c.1) - to.z).powi(2);
                if best.is_none_or(|(_, b)| d < b) {
                    best = Some((c, d));
                }
            }
        }
        best.map(|(c, _)| c)
    }

    /// The cell `(dx, dz)` from `c`, if on the grid.
    fn offset(&self, c: (usize, usize), dx: i64, dz: i64) -> Option<(usize, usize)> {
        let (x, z) = (c.0 as i64 + dx, c.1 as i64 + dz);
        (x >= 0 && z >= 0 && (x as usize) < self.side && (z as usize) < self.side).then_some((x as usize, z as usize))
    }

    fn coord(&self, i: usize) -> f64 {
        -self.half + i as f64 * CELL
    }

    fn cell_at(&self, p: Vec3) -> Option<(usize, usize)> {
        let x = ((p.x + self.half) / CELL).round();
        let z = ((p.z + self.half) / CELL).round();
        (x >= 0.0 && z >= 0.0 && (x as usize) < self.side && (z as usize) < self.side).then_some((x as usize, z as usize))
    }

    /// Whether the cell is at an edge: not every neighbour a body could be
    /// in is one it can step to.
    fn edge(&self, (x, z): (usize, usize)) -> bool {
        self.edges[z * self.side + x]
    }

    /// Whether the way from cell `a` to its neighbour `b` is a drop.
    fn drop_to(&self, (ax, az): (usize, usize), (bx, bz): (usize, usize)) -> bool {
        let (dx, dz) = (bx as i64 - ax as i64, bz as i64 - az as i64);
        AROUND.iter().position(|&d| d == (dx, dz)).is_some_and(|k| self.drops[az * self.side + ax] & (1 << k) != 0)
    }

    fn ground(&self, x: usize, z: usize) -> Option<f64> {
        let h = self.height[z * self.side + x];
        (!h.is_nan()).then_some(f64::from(h))
    }

    /// Where a body stands in the cell: its middle, or off it (see
    /// `spots`).
    fn centre(&self, x: usize, z: usize) -> Vec3 {
        let (ox, oz) = self.spots[z * self.side + x];
        Vec3::new(self.coord(x) + f64::from(ox) * 0.25, self.ground(x, z).unwrap_or(0.0), self.coord(z) + f64::from(oz) * 0.25)
    }

    /// The ground a body stands on in the cell holding `p`, if any.
    pub fn height_at(&self, p: Vec3) -> Option<f64> {
        let (x, z) = self.cell_at(p)?;
        self.ground(x, z)
    }

    /// Whether a body can walk straight from `a` to `b` on linked ground.
    pub fn clear(&self, a: Vec3, b: Vec3) -> bool {
        match (self.nearest_open(a), self.nearest_open(b)) {
            (Some(ca), Some(cb)) => self.straight(ca, cb),
            _ => false,
        }
    }

    /// Whether a body can stand in the cell holding `p`.
    pub fn walkable(&self, p: Vec3) -> bool {
        self.cell_at(p).is_some_and(|(x, z)| self.ground(x, z).is_some())
    }

    /// Whether a body with its feet at `a` can get to `b` at all (by any
    /// way, however far).
    #[cfg(test)]
    pub fn connects(&self, a: Vec3, b: Vec3) -> bool {
        match (self.nearest_open(a), self.nearest_open(b)) {
            (Some(ca), Some(cb)) => self.reaches(ca, cb),
            _ => false,
        }
    }

    /// How many cells a body can stand in.
    #[cfg(test)]
    pub fn open_cells(&self) -> usize {
        self.height.iter().filter(|h| !h.is_nan()).count()
    }

    /// Whether cell `a`'s ground leads onto its neighbour `b`'s.
    fn leads(&self, (ax, az): (usize, usize), (bx, bz): (usize, usize)) -> bool {
        let (dx, dz) = (bx as i64 - ax as i64, bz as i64 - az as i64);
        AROUND.iter().position(|&d| d == (dx, dz)).is_some_and(|k| self.links[az * self.side + ax] & (1 << k) != 0)
    }

    /// Whether a body can step from cell `a` to its neighbour `b`: a
    /// diagonal only where it could as well go round either corner, so
    /// none is cut past a wall's end or a ramp's side.
    fn linked(&self, a: (usize, usize), b: (usize, usize)) -> bool {
        if !self.leads(a, b) {
            return false;
        }
        a.0 == b.0 || a.1 == b.1 || [(b.0, a.1), (a.0, b.1)].iter().all(|&c| self.leads(a, c) && self.leads(c, b))
    }

    /// The open cell a body with its feet at `p` is in: the nearest, but
    /// one on the ground at its feet's height before one that isn't (right
    /// under a ledge, the cell it rounds to may be the ledge's top).
    fn nearest_open(&self, p: Vec3) -> Option<(usize, usize)> {
        let (cx, cz) = self.cell_at(p)?;
        let mut best: Option<((usize, usize), f64)> = None;
        for dz in -3..=3i64 {
            for dx in -3..=3i64 {
                let Some((x, z)) = self.offset((cx, cz), dx, dz) else { continue };
                let Some(h) = self.ground(x, z) else { continue };
                let flat = ((self.coord(x) - p.x).powi(2) + (self.coord(z) - p.z).powi(2)).sqrt();
                let score = flat + 4.0 * ((h - p.y).abs() - 0.5).max(0.0);
                if best.is_none_or(|(_, b)| score < b) {
                    best = Some(((x, z), score));
                }
            }
        }
        best.map(|(c, _)| c)
    }

    /// A walkable route from `from` to `to`: its turning points, the last
    /// being `to`'s cell. `None` when there is no way (or it is too far to
    /// find).
    pub fn path(&self, from: Vec3, to: Vec3) -> Option<Vec<Vec3>> {
        let start = self.nearest_open(from)?;
        let mut goal = self.nearest_open(to)?;
        // Known unreachable costs nothing to find out: head as near as can
        // be got instead of searching the whole grid in vain.
        if !self.reaches(start, goal) {
            goal = self.nearest_reachable(start, to)?;
        }
        let idx = |(x, z): (usize, usize)| z * self.side + x;
        let h = |(x, z): (usize, usize)| {
            let (dx, dz) = ((x as f64 - goal.0 as f64).abs(), (z as f64 - goal.1 as f64).abs());
            dx.max(dz) + (std::f64::consts::SQRT_2 - 1.0) * dx.min(dz)
        };
        SCRATCH.with_borrow_mut(|scratch| {
            scratch.begin(self.side * self.side);
            let mut open = BinaryHeap::new();
            scratch.set(idx(start), 0.0, usize::MAX);
            open.push(Open { cost: h(start), cell: idx(start) });
            let mut seen = 0;
            while let Some(Open { cell, .. }) = open.pop() {
                if cell == idx(goal) {
                    break;
                }
                seen += 1;
                if seen > SEARCH_LIMIT {
                    return None;
                }
                let here = (cell % self.side, cell / self.side);
                for (dx, dz) in AROUND {
                    let Some(next) = self.offset(here, dx, dz) else { continue };
                    if !self.linked(here, next) {
                        continue;
                    }
                    let step = if dx != 0 && dz != 0 { std::f64::consts::SQRT_2 } else { 1.0 };
                    let c = scratch.cost(cell) + step + if self.edge(next) { EDGE_COST } else { 0.0 } + if self.drop_to(here, next) { DROP_COST } else { 0.0 };
                    if c < scratch.cost(idx(next)) {
                        scratch.set(idx(next), c, cell);
                        open.push(Open { cost: c + h(next), cell: idx(next) });
                    }
                }
            }
            if !scratch.cost(idx(goal)).is_finite() {
                return None;
            }
            let mut cells = vec![goal];
            let mut at = idx(goal);
            while at != idx(start) {
                at = scratch.came(at);
                cells.push((at % self.side, at / self.side));
            }
            cells.reverse();
            Some(self.pull(&cells))
        })
    }

    /// Whether a straight walk from cell `a` to cell `b` stays on linked
    /// ground, checked cell by cell along it, and keeps off edges on the way
    /// (a walk along a ledge's line hangs half off it).
    fn straight(&self, a: (usize, usize), b: (usize, usize)) -> bool {
        let (dx, dz) = (b.0 as f64 - a.0 as f64, b.1 as f64 - a.1 as f64);
        let steps = (dx.abs().max(dz.abs()) * 2.0).ceil().max(1.0) as usize;
        let mut prev = a;
        for i in 1..=steps {
            let t = i as f64 / steps as f64;
            let c = ((a.0 as f64 + dx * t).round() as usize, (a.1 as f64 + dz * t).round() as usize);
            if c != prev && (!self.linked(prev, c) || (c != b && self.edge(c))) {
                return false;
            }
            prev = c;
        }
        true
    }

    /// The turning points of a cell route: each kept point is as far along
    /// as a straight walk from the last one reaches.
    fn pull(&self, cells: &[(usize, usize)]) -> Vec<Vec3> {
        let mut out = Vec::new();
        let mut from = 0;
        while from + 1 < cells.len() {
            let mut to = from + 1;
            while to + 1 < cells.len() && self.straight(cells[from], cells[to + 1]) {
                to += 1;
            }
            out.push(self.centre(cells[to].0, cells[to].1));
            from = to;
        }
        if out.is_empty() {
            out.push(self.centre(cells[0].0, cells[0].1));
        }
        out
    }
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
