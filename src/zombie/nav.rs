//! Where the dead can walk: a grid of one-metre cells over the map, each
//! either ground a body can stand on (and at what height) or not, probed
//! from the solids at load. Cells link to their eight neighbours when the
//! step between them is one a body can take and a diagonal cuts no corner.
//! Paths are found with A* and pulled straight wherever a straight walk
//! stays on linked ground.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use lntrn_math::Vec3;

use crate::collide::{Capsule, Solids, WALKABLE};
use crate::player::{BOUNDS, STEP_UP};

/// Cell size, metres.
const CELL: f64 = 1.0;
/// The most cells a search looks at before it gives up.
const SEARCH_LIMIT: usize = 40_000;
/// How high the probe starts looking down from.
const SKY: f64 = 80.0;

pub struct NavGrid {
    /// Cells a side.
    side: usize,
    /// The ground's height in each cell, or NaN where there is none a body
    /// can stand on.
    height: Vec<f32>,
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
    /// Probe `solids` for every cell a body of `body` could stand in.
    pub fn build(solids: &Solids, body: Capsule) -> Self {
        let side = (2.0 * BOUNDS / CELL) as usize + 1;
        let mut height = vec![f32::NAN; side * side];
        for z in 0..side {
            for x in 0..side {
                let (wx, wz) = (Self::coord(x), Self::coord(z));
                let from = Vec3::new(wx, SKY, wz);
                if let Some(hit) = solids.raycast(from, Vec3::new(0.0, -1.0, 0.0), SKY * 2.0)
                    && hit.normal.y >= WALKABLE
                    && solids.fits(body, hit.point + Vec3::new(0.0, 0.02, 0.0))
                {
                    height[z * side + x] = hit.point.y as f32;
                }
            }
        }
        Self { side, height }
    }

    fn coord(i: usize) -> f64 {
        -BOUNDS + i as f64 * CELL
    }

    fn cell_at(&self, p: Vec3) -> Option<(usize, usize)> {
        let x = ((p.x + BOUNDS) / CELL).round();
        let z = ((p.z + BOUNDS) / CELL).round();
        (x >= 0.0 && z >= 0.0 && (x as usize) < self.side && (z as usize) < self.side).then_some((x as usize, z as usize))
    }

    fn ground(&self, x: usize, z: usize) -> Option<f64> {
        let h = self.height[z * self.side + x];
        (!h.is_nan()).then_some(f64::from(h))
    }

    fn centre(&self, x: usize, z: usize) -> Vec3 {
        Vec3::new(Self::coord(x), self.ground(x, z).unwrap_or(0.0), Self::coord(z))
    }

    /// The ground a body stands on in the cell holding `p`, if any.
    pub fn height_at(&self, p: Vec3) -> Option<f64> {
        let (x, z) = self.cell_at(p)?;
        self.ground(x, z)
    }

    /// Whether a body can walk straight from `a` to `b` on linked ground.
    pub fn clear(&self, a: Vec3, b: Vec3) -> bool {
        match (self.cell_at(a), self.cell_at(b)) {
            (Some(ca), Some(cb)) => self.ground(ca.0, ca.1).is_some() && self.straight(ca, cb),
            _ => false,
        }
    }

    /// Whether a body can stand in the cell holding `p`.
    pub fn walkable(&self, p: Vec3) -> bool {
        self.cell_at(p).is_some_and(|(x, z)| self.ground(x, z).is_some())
    }

    /// How many cells a body can stand in.
    #[cfg(test)]
    pub fn open_cells(&self) -> usize {
        self.height.iter().filter(|h| !h.is_nan()).count()
    }

    /// Whether a body can step from cell `a` to its neighbour `b`.
    fn linked(&self, (ax, az): (usize, usize), (bx, bz): (usize, usize)) -> bool {
        let (Some(ha), Some(hb)) = (self.ground(ax, az), self.ground(bx, bz)) else { return false };
        if (ha - hb).abs() > STEP_UP {
            return false;
        }
        // A diagonal needs both cells beside it open: no cutting a corner.
        ax == bx || az == bz || (self.ground(bx, az).is_some() && self.ground(ax, bz).is_some())
    }

    /// The nearest open cell to `p`, looking a few cells out.
    fn nearest_open(&self, p: Vec3) -> Option<(usize, usize)> {
        let (cx, cz) = self.cell_at(p)?;
        if self.ground(cx, cz).is_some() {
            return Some((cx, cz));
        }
        let mut best: Option<((usize, usize), f64)> = None;
        for r in 1..=3i64 {
            for dz in -r..=r {
                for dx in -r..=r {
                    let (x, z) = (cx as i64 + dx, cz as i64 + dz);
                    if x < 0 || z < 0 || x as usize >= self.side || z as usize >= self.side || self.ground(x as usize, z as usize).is_none() {
                        continue;
                    }
                    let d = (dx * dx + dz * dz) as f64;
                    if best.is_none_or(|(_, b)| d < b) {
                        best = Some(((x as usize, z as usize), d));
                    }
                }
            }
            if best.is_some() {
                break;
            }
        }
        best.map(|(c, _)| c)
    }

    /// A walkable route from `from` to `to`: its turning points, the last
    /// being `to`'s cell. `None` when there is no way (or it is too far to
    /// find).
    pub fn path(&self, from: Vec3, to: Vec3) -> Option<Vec<Vec3>> {
        let start = self.nearest_open(from)?;
        let goal = self.nearest_open(to)?;
        let idx = |(x, z): (usize, usize)| z * self.side + x;
        let h = |(x, z): (usize, usize)| {
            let (dx, dz) = ((x as f64 - goal.0 as f64).abs(), (z as f64 - goal.1 as f64).abs());
            dx.max(dz) + (std::f64::consts::SQRT_2 - 1.0) * dx.min(dz)
        };
        let mut cost = vec![f64::INFINITY; self.side * self.side];
        let mut came = vec![usize::MAX; self.side * self.side];
        let mut open = BinaryHeap::new();
        cost[idx(start)] = 0.0;
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
            for (dx, dz) in [(-1i64, 0i64), (1, 0), (0, -1), (0, 1), (-1, -1), (1, -1), (-1, 1), (1, 1)] {
                let (nx, nz) = (here.0 as i64 + dx, here.1 as i64 + dz);
                if nx < 0 || nz < 0 || nx as usize >= self.side || nz as usize >= self.side {
                    continue;
                }
                let next = (nx as usize, nz as usize);
                if !self.linked(here, next) {
                    continue;
                }
                let step = if dx != 0 && dz != 0 { std::f64::consts::SQRT_2 } else { 1.0 };
                let c = cost[cell] + step;
                if c < cost[idx(next)] {
                    cost[idx(next)] = c;
                    came[idx(next)] = cell;
                    open.push(Open { cost: c + h(next), cell: idx(next) });
                }
            }
        }
        if !cost[idx(goal)].is_finite() {
            return None;
        }
        let mut cells = vec![goal];
        let mut at = idx(goal);
        while at != idx(start) {
            at = came[at];
            cells.push((at % self.side, at / self.side));
        }
        cells.reverse();
        Some(self.pull(&cells))
    }

    /// Whether a straight walk from cell `a` to cell `b` stays on linked
    /// ground, checked cell by cell along it.
    fn straight(&self, a: (usize, usize), b: (usize, usize)) -> bool {
        let (dx, dz) = (b.0 as f64 - a.0 as f64, b.1 as f64 - a.1 as f64);
        let steps = (dx.abs().max(dz.abs()) * 2.0).ceil().max(1.0) as usize;
        let mut prev = a;
        for i in 1..=steps {
            let t = i as f64 / steps as f64;
            let c = ((a.0 as f64 + dx * t).round() as usize, (a.1 as f64 + dz * t).round() as usize);
            if c != prev && !self.linked(prev, c) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::capsule;

    #[test]
    fn the_real_map_has_ground_and_routes_round_the_shack() {
        let solids = crate::testing::real_world();
        let started = std::time::Instant::now();
        let nav = NavGrid::build(&solids, capsule(false));
        eprintln!("nav: {} open cells in {:.0} ms", nav.open_cells(), started.elapsed().as_secs_f64() * 1000.0);
        assert!(nav.open_cells() > 30_000, "{} open cells", nav.open_cells());
        // The spawn is ground. The shack (game x 4, z -40) stands on its
        // cells: what is there is its roof, a step too tall to reach.
        let ground = nav.height_at(Vec3::new(0.0, 0.0, 6.0)).expect("the spawn is ground");
        assert!(nav.height_at(Vec3::new(4.0, 0.0, -40.0)).is_none_or(|h| h > ground + 2.0), "the shack's floor is open");
        // Round the shack: the route stays down on the ground, and comes out
        // longer than a straight line through its walls.
        let (a, b) = (Vec3::new(4.0, 0.0, -35.0), Vec3::new(4.0, 0.0, -45.0));
        let route = nav.path(a, b).expect("a way round");
        let mut length = 0.0;
        let mut prev = a;
        for p in &route {
            let mut t = 0.0;
            while t <= 1.0 {
                let q = prev + (*p - prev) * t;
                let h = nav.height_at(q).unwrap_or(f64::NAN);
                assert!(h < 2.5, "the route goes over the shack near {q:?} at {h}");
                t += 0.1;
            }
            length += (*p - prev).length();
            prev = *p;
        }
        assert!(length > 11.0, "round, not through: {length:.1} m");
        // Up onto the proving ground by its ramp: the pad is reachable.
        assert!(nav.path(Vec3::new(0.0, 0.0, 6.0), Vec3::new(0.0, 0.0, 38.0)).is_some(), "no way onto the pad");
    }
}
