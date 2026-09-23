//! Probing the solids for the grid: every floor in every column, then
//! the ways between neighbouring floors, then where on each floor at an
//! edge a body best stands. Each pass is shared out over every core.

use lntrn_math::Vec3;

use super::regions::Regions;
use super::{AROUND, CELL, CLIMB, DROP, FIT_LIFT, FLOORS, NONE, NavGrid, SKY, passage, steps_between};
use crate::collide::{Capsule, Solids, WALKABLE};
use crate::player::STEP_UP;

impl NavGrid {
    /// Probe `solids` for every floor a body of `body` could stand on, over
    /// the square `half` metres each way from the middle. The rows are
    /// shared out over every core.
    pub fn build(solids: &Solids, body: Capsule, half: f64) -> Self {
        let side = (2.0 * half / CELL) as usize + 1;
        let coord = |i: usize| -half + i as f64 * CELL;
        // Every floor in every column: every face straight down from the
        // sky, in order. Solid things are closed, their faces outwards: a
        // top met is into something, an underside out of it; a floor is
        // only a floor met out of everything (not the ground in a sealed
        // box or under a foundation).
        let floors: Vec<Vec<f32>> = crate::map::rows(side, |z| {
            (0..side)
                .map(|x| {
                    let mut found = Vec::new();
                    let mut inside = 0u32;
                    for hit in solids.hits_down(Vec3::new(coord(x), SKY, coord(z))) {
                        if hit.front {
                            if inside == 0 && hit.normal.y >= WALKABLE && found.len() < FLOORS && solids.fits(body, hit.point + Vec3::new(0.0, FIT_LIFT, 0.0)) {
                                found.push(hit.point.y as f32);
                            }
                            inside += 1;
                        } else {
                            inside = inside.saturating_sub(1);
                        }
                    }
                    found.reverse();
                    found
                })
                .collect()
        });
        let mut first = Vec::with_capacity(side * side + 1);
        let (mut height, mut column) = (Vec::new(), Vec::new());
        for (c, fl) in floors.iter().enumerate() {
            first.push(height.len() as u32);
            for &h in fl {
                height.push(h);
                column.push(c as u32);
            }
        }
        first.push(height.len() as u32);
        let n = height.len();
        let mut grid = Self { half, side, first, height, column, links: vec![[NONE; 8]; n], drops: vec![0; n], edges: vec![false; n], regions: Regions::default(), spots: vec![(0, 0); n] };
        let ways: Vec<([u32; 8], u8, bool)> = crate::map::rows(side, |z| {
            let mut out = Vec::new();
            for x in 0..side {
                for a in grid.nodes((x, z)) {
                    out.push(grid.probe(solids, a));
                }
            }
            out
        });
        for (i, (links, drops, edge)) in ways.into_iter().enumerate() {
            grid.links[i] = links;
            grid.drops[i] = drops;
            grid.edges[i] = edge;
        }
        grid.regions = Regions::build(&grid);
        grid.spots = crate::map::rows(side, |z| {
            let mut out = Vec::new();
            for x in 0..side {
                for a in grid.nodes((x, z)) {
                    out.push(if grid.edges[a as usize] { grid.best_spot(solids, a, body.radius) } else { (0, 0) });
                }
            }
            out
        });
        grid
    }

    /// The ways on from node `a`: which floor in each neighbouring column
    /// it leads onto (walking in preference to dropping; of those, the
    /// nearest in height), which of those are drops, and whether it's at an
    /// edge.
    fn probe(&self, solids: &Solids, a: u32) -> ([u32; 8], u8, bool) {
        let (mut links, mut drops, mut walks) = ([NONE; 8], 0u8, 0u8);
        let ha = self.floor(a);
        let (x, z) = self.col_of(a);
        let pa = Vec3::new(self.coord(x), ha, self.coord(z));
        for (k, (dx, dz)) in AROUND.iter().enumerate() {
            let Some(c) = self.offset((x, z), *dx, *dz) else { continue };
            let mut walk: Option<(u32, f64)> = None;
            let mut drop: Option<(u32, f64)> = None;
            for b in self.nodes(c) {
                let hb = self.floor(b);
                let pb = Vec3::new(self.coord(c.0), hb, self.coord(c.1));
                let dh = (ha - hb).abs();
                if dh > DROP.max(CLIMB) || !passage(solids, pa, pb) {
                    continue;
                }
                if dh <= STEP_UP || (dh <= CLIMB && steps_between(solids, pa, pb, STEP_UP)) {
                    if walk.is_none_or(|(_, d)| dh < d) {
                        walk = Some((b, dh));
                    }
                } else if ha - hb <= DROP && ha > hb && steps_between(solids, pa, pb, DROP) && drop.is_none_or(|(_, d)| dh < d) {
                    drop = Some((b, dh));
                }
            }
            if let Some((b, _)) = walk {
                links[k] = b;
                walks |= 1 << k;
            } else if let Some((b, _)) = drop {
                links[k] = b;
                drops |= 1 << k;
            }
        }
        (links, drops, walks != 0xFF)
    }

    /// Where on floor `a` a body of `radius` stands on even ground: its
    /// column's middle if that will do, else the nearest point (up to half
    /// a metre off) where the ground all round under it is at one height.
    fn best_spot(&self, solids: &Solids, a: u32, radius: f64) -> (i8, i8) {
        let h = self.floor(a);
        let (x, z) = self.col_of(a);
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

}
