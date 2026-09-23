//! The roads: found over the land the way a road builder would (the
//! short way, round the steep ground, never through a plot), smoothed
//! into curves, and given a profile (the land under them, evened out) the
//! land is then cut and filled to. They're drawn as strips of their own,
//! a hair over the ground.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use lntrn_math::{Vec2, Vec3};

use super::EXTENT;
use super::terrain::smooth;

/// What a road is: the highway (two lanes, lines painted), a lesser
/// paved road, or a dirt track.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Dirt,
    Paved,
    Highway,
}

impl Kind {
    pub fn half_width(self) -> f64 {
        match self {
            Kind::Highway => 4.5,
            Kind::Paved => 3.0,
            Kind::Dirt => 2.0,
        }
    }

    /// How far over the ground it's drawn (the highway on top where they
    /// meet).
    pub fn lift(self) -> f64 {
        match self {
            Kind::Highway => 0.14,
            Kind::Paved => 0.11,
            Kind::Dirt => 0.09,
        }
    }

    /// The steepest it's let climb, rise over run.
    pub fn steepest(self) -> f64 {
        match self {
            Kind::Dirt => 0.16,
            _ => 0.12,
        }
    }
}

/// Points along a road, metres apart.
pub const SPACING: f64 = 4.0;
/// Either side of a road, the ground this far past its edge is at the
/// road's height (a cell's width and more, so no facet of the ground under
/// the road tilts up through it); then it eases back into the land. Under
/// it, it's a little lower still, for the facets that can't quite follow
/// its twists on a steep bend.
pub const FLAT: f64 = 6.0;
pub const SINK: f64 = 0.12;
pub const SHOULDER: f64 = 10.0;
/// The routing grid, metres a cell.
const GRID: f64 = 8.0;
/// What a grade costs a route: this times its square, per metre.
const GRADE_COST: f64 = 60.0;
/// What a metre through somewhere a road mustn't go costs.
const BLOCKED: f64 = 200.0;
/// How far either way the land's height is averaged for a road's profile,
/// in points.
const EVEN: i64 = 5;

#[derive(Clone, Debug)]
pub struct Road {
    pub kind: Kind,
    /// Its middle line, every [`SPACING`] metres, at its height.
    pub points: Vec<Vec3>,
}

impl Road {
    /// Which way it runs through point `i` (flat, unit): between its
    /// neighbours, so a strip's cross-cuts bisect its bends.
    pub fn tangent(&self, i: usize) -> Vec2 {
        let (a, b) = (self.points[i.saturating_sub(1)], self.points[(i + 1).min(self.points.len() - 1)]);
        let d = Vec2::new(b.x - a.x, b.z - a.z);
        d * (1.0 / d.length().max(1e-9))
    }

    /// Which way it runs at point `i` (flat, unit).
    pub fn heading(&self, i: usize) -> Vec2 {
        let (a, b) = if i + 1 < self.points.len() { (self.points[i], self.points[i + 1]) } else { (self.points[i - 1], self.points[i]) };
        let d = Vec2::new(b.x - a.x, b.z - a.z);
        d * (1.0 / d.length().max(1e-9))
    }
}

/// How much a point at `d` metres from a road's middle is the road's: 1
/// on it and its flat, easing to 0 over the shoulder.
pub fn weight(kind: Kind, d: f64) -> f64 {
    1.0 - smooth((d - kind.half_width() - FLAT) / SHOULDER)
}

/// Every road, and a way to find the ones near a point quickly.
pub struct Network {
    pub roads: Vec<Road>,
    /// Buckets of (road, segment) by where they lie, 32 m a side.
    buckets: std::collections::HashMap<(i32, i32), Vec<(u32, u32)>>,
}

const BUCKET: f64 = 32.0;

impl Network {
    pub fn new(roads: Vec<Road>) -> Self {
        let mut buckets: std::collections::HashMap<(i32, i32), Vec<(u32, u32)>> = std::collections::HashMap::new();
        let reach = Kind::Highway.half_width() + FLAT + SHOULDER;
        for (r, road) in roads.iter().enumerate() {
            for (s, w) in road.points.windows(2).enumerate() {
                let (x0, x1) = (w[0].x.min(w[1].x) - reach, w[0].x.max(w[1].x) + reach);
                let (z0, z1) = (w[0].z.min(w[1].z) - reach, w[0].z.max(w[1].z) + reach);
                for bx in (x0 / BUCKET).floor() as i32..=(x1 / BUCKET).floor() as i32 {
                    for bz in (z0 / BUCKET).floor() as i32..=(z1 / BUCKET).floor() as i32 {
                        buckets.entry((bx, bz)).or_default().push((r as u32, s as u32));
                    }
                }
            }
        }
        Self { roads, buckets }
    }

    /// The road nearest `p` that lays claim to it (within its shoulder):
    /// which, how far off its middle, its height there, and how much the
    /// point is the road's. Where two claim it, the more its; on a tie,
    /// the greater road, then the nearer.
    pub fn claim(&self, p: Vec2) -> Option<(usize, f64, f64, f64)> {
        let key = ((p.x / BUCKET).floor() as i32, (p.y / BUCKET).floor() as i32);
        let mut best: Option<(usize, f64, f64, f64)> = None;
        for &(r, s) in self.buckets.get(&key)? {
            let road = &self.roads[r as usize];
            let (a, b) = (road.points[s as usize], road.points[s as usize + 1]);
            let (a2, b2) = (Vec2::new(a.x, a.z), Vec2::new(b.x, b.z));
            let ab = b2 - a2;
            let t = ((p - a2).dot(ab) / ab.dot(ab).max(1e-9)).clamp(0.0, 1.0);
            let d = (a2 + ab * t - p).length();
            let w = weight(road.kind, d);
            if w <= 0.0 {
                continue;
            }
            // (Within a road's flat every point is wholly its: the nearest
            // stretch of it is the one whose height counts.)
            let better = best.is_none_or(|(br, bd, _, bw)| {
                let bk = self.roads[br].kind;
                w > bw + 1e-9 || ((w - bw).abs() <= 1e-9 && (road.kind > bk || (road.kind == bk && d < bd)))
            });
            if better {
                best = Some((r as usize, d, a.y + (b.y - a.y) * t, w));
            }
        }
        best
    }

    /// How far `p` is from the nearest road's edge (negative on it), within
    /// the reach of any, else infinity.
    pub fn off_road(&self, p: Vec2) -> f64 {
        self.claim(p).map_or(f64::INFINITY, |(r, d, _, _)| d - self.roads[r].kind.half_width())
    }
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

/// A way over the land from `from` to `to` (both kept), the short way but
/// round steep ground (`height` says how the land lies) and never through
/// where `blocked` says: smoothed into curves, a point every
/// [`SPACING`] metres, flat (heights come later).
pub fn route(height: &(impl Fn(f64, f64) -> f64 + ?Sized), blocked: &(impl Fn(Vec2) -> bool + ?Sized), from: Vec2, to: Vec2) -> Vec<Vec2> {
    let side = (2.0 * EXTENT / GRID) as usize + 1;
    let at = |i: usize| Vec2::new(-EXTENT + (i % side) as f64 * GRID, -EXTENT + (i / side) as f64 * GRID);
    let cell = |p: Vec2| {
        let i = (((p.x + EXTENT) / GRID).round() as usize).min(side - 1);
        let j = (((p.y + EXTENT) / GRID).round() as usize).min(side - 1);
        j * side + i
    };
    let heights: Vec<f64> = (0..side * side).map(|i| height(at(i).x, at(i).y)).collect();
    let stuck: Vec<bool> = (0..side * side).map(|i| blocked(at(i))).collect();
    let (start, goal) = (cell(from), cell(to));
    let mut cost = vec![f64::INFINITY; side * side];
    let mut came = vec![usize::MAX; side * side];
    let mut open = BinaryHeap::new();
    let guess = |i: usize| (at(i) - at(goal)).length();
    cost[start] = 0.0;
    open.push(Open { cost: guess(start), cell: start });
    while let Some(Open { cell: c, .. }) = open.pop() {
        if c == goal {
            break;
        }
        let (ci, cj) = ((c % side) as i64, (c / side) as i64);
        for (di, dj) in [(-1i64, 0i64), (1, 0), (0, -1), (0, 1), (-1, -1), (1, -1), (-1, 1), (1, 1)] {
            let (ni, nj) = (ci + di, cj + dj);
            if ni < 0 || nj < 0 || ni >= side as i64 || nj >= side as i64 {
                continue;
            }
            let n = nj as usize * side + ni as usize;
            let len = if di != 0 && dj != 0 { GRID * std::f64::consts::SQRT_2 } else { GRID };
            let grade = (heights[n] - heights[c]).abs() / len;
            let step = len * (1.0 + GRADE_COST * grade * grade) + if stuck[n] && n != goal { BLOCKED * len } else { 0.0 };
            let total = cost[c] + step;
            if total < cost[n] {
                cost[n] = total;
                came[n] = c;
                open.push(Open { cost: total + guess(n), cell: n });
            }
        }
    }
    let mut cells = vec![goal];
    while let Some(&last) = cells.last() {
        if last == start || came[last] == usize::MAX {
            break;
        }
        cells.push(came[last]);
    }
    cells.reverse();
    let mut line: Vec<Vec2> = Vec::with_capacity(cells.len() + 2);
    line.push(from);
    line.extend(cells.iter().skip(1).take(cells.len().saturating_sub(2)).map(|&c| at(c)));
    line.push(to);
    resample(&curve(&line, 3), SPACING)
}

/// Corners cut, `passes` times over (Chaikin): the ends stay put.
pub fn curve(line: &[Vec2], passes: usize) -> Vec<Vec2> {
    let mut line = line.to_vec();
    for _ in 0..passes {
        if line.len() < 3 {
            return line;
        }
        let mut out = vec![line[0]];
        for w in line.windows(2) {
            out.push(w[0] * 0.75 + w[1] * 0.25);
            out.push(w[0] * 0.25 + w[1] * 0.75);
        }
        out.push(*line.last().expect("not empty"));
        line = out;
    }
    line
}

/// Points every `every` metres along `line`, its ends included.
pub fn resample(line: &[Vec2], every: f64) -> Vec<Vec2> {
    let total: f64 = line.windows(2).map(|w| (w[1] - w[0]).length()).sum();
    let n = (total / every).round().max(1.0) as usize;
    let step = total / n as f64;
    let mut out = vec![line[0]];
    let (mut seg, mut into) = (0, 0.0);
    for k in 1..n {
        let mut want = step;
        loop {
            let len = (line[seg + 1] - line[seg]).length();
            if into + want <= len || seg + 2 >= line.len() {
                into += want;
                let t = (into / len.max(1e-9)).min(1.0);
                out.push(line[seg] + (line[seg + 1] - line[seg]) * t);
                break;
            }
            want -= len - into;
            into = 0.0;
            seg += 1;
        }
        let _ = k;
    }
    out.push(*line.last().expect("not empty"));
    out
}

/// A road's heights: the land along it, evened out over a stretch each
/// way, so it rolls gently over the bumps; level with a plot where it runs
/// through one (`plot` says how much a point is a plot's, and its height);
/// and where it runs over ground a road already `laid` has wholly (where
/// it starts off it, where it crosses one), at that road's height, so the
/// ground (that road's) is never over this one. Then no steeper than its
/// kind climbs, but where it's held to those.
pub fn profile(kind: Kind, line: &[Vec2], height: impl Fn(f64, f64) -> f64, plot: impl Fn(Vec2) -> Option<(f64, f64)>, laid: Option<&Network>) -> Vec<Vec3> {
    let raw: Vec<f64> = line.iter().map(|p| height(p.x, p.y)).collect();
    let n = raw.len() as i64;
    let mut ys: Vec<f64> = (0..n)
        .map(|i| {
            let (a, b) = ((i - EVEN).max(0), (i + EVEN).min(n - 1));
            (a..=b).map(|k| raw[k as usize]).sum::<f64>() / (b - a + 1) as f64
        })
        .collect();
    let mut pinned = vec![false; ys.len()];
    for ((y, p), pin) in ys.iter_mut().zip(line).zip(&mut pinned) {
        if let Some((w, h)) = plot(*p) {
            *y += (h - *y) * w;
            *pin = w > 0.99;
        }
        if let Some((_, _, h, _)) = laid.and_then(|l| l.claim(*p)).filter(|c| c.3 >= 1.0 - 1e-9) {
            *y = h;
            *pin = true;
        }
    }
    limit(&mut ys, &pinned, kind.steepest() * SPACING);
    // Kinks rounded off (where it goes from level to climbing), so the
    // ground's facets, spanning one, don't bulge up through it.
    for _ in 0..3 {
        let before = ys.clone();
        for i in 1..ys.len().saturating_sub(1) {
            if !pinned[i] {
                ys[i] = 0.25 * before[i - 1] + 0.5 * before[i] + 0.25 * before[i + 1];
            }
        }
    }
    line.iter().zip(ys).map(|(p, y)| Vec3::new(p.x, y, p.y)).collect()
}

/// Heights evened till none rises more than `most` from the one before,
/// the `pinned` ones (level with a plot, or a road) left be: back and forth, each
/// pulled within reach of its neighbour.
fn limit(ys: &mut [f64], pinned: &[bool], most: f64) {
    for _ in 0..12 {
        for i in 1..ys.len() {
            if !pinned[i] {
                ys[i] = ys[i].clamp(ys[i - 1] - most, ys[i - 1] + most);
            }
        }
        for i in (0..ys.len().saturating_sub(1)).rev() {
            if !pinned[i] {
                ys[i] = ys[i].clamp(ys[i + 1] - most, ys[i + 1] + most);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_route_goes_round_a_hill_and_what_it_must_not_cross() {
        // A steep hill in the way, and a wall of blocked ground.
        let hill = |x: f64, z: f64| 40.0 * (-(x * x + z * z) / 3000.0).exp();
        let line = route(&hill, &|p: Vec2| p.x.abs() < 10.0 && p.y > 60.0, Vec2::new(-150.0, 0.0), Vec2::new(150.0, 0.0));
        assert_eq!(line[0], Vec2::new(-150.0, 0.0));
        assert_eq!(*line.last().unwrap(), Vec2::new(150.0, 0.0));
        let peak = line.iter().map(|p| hill(p.x, p.y)).fold(0.0, f64::max);
        assert!(peak < 25.0, "over the top: {peak}");
        assert!(line.iter().all(|p| !(p.x.abs() < 6.0 && p.y > 64.0)), "through the wall");
        for w in line.windows(2) {
            let d = (w[1] - w[0]).length();
            assert!(d > SPACING * 0.5 && d < SPACING * 1.5, "{d}");
        }
    }

    #[test]
    fn a_profile_rolls_gently_and_sits_level_in_a_plot() {
        let line: Vec<Vec2> = (0..60).map(|i| Vec2::new(f64::from(i) * SPACING, 0.0)).collect();
        let bumpy = |x: f64, _z: f64| 2.0 * (x * 0.5).sin();
        let in_plot = |p: Vec2| (p.x > 150.0).then_some((1.0, 7.0));
        let prof = profile(Kind::Dirt, &line, bumpy, in_plot, None);
        assert!(prof[10..20].iter().all(|p| p.y.abs() < 0.8), "evened out");
        assert!(prof.iter().filter(|p| p.x > 150.0).all(|p| (p.y - 7.0).abs() < 1e-9));
        // Up to the plot no steeper than a dirt road climbs.
        assert!(prof.windows(2).all(|w| (w[1].y - w[0].y).abs() <= Kind::Dirt.steepest() * SPACING + 1e-9));
        // Starting off a road at 3 m: level with it on its ground, easing
        // off after as steeply as it may.
        let parent = Road { kind: Kind::Highway, points: (0..20).map(|i| Vec3::new(0.0, 3.0, f64::from(i) * SPACING - 40.0)).collect() };
        let prof = profile(Kind::Dirt, &line, bumpy, in_plot, Some(&Network::new(vec![parent])));
        assert!((0..3).all(|i| (prof[i].y - 3.0).abs() < 1e-9), "{:?}", &prof[..4]);
        assert!(prof.windows(2).all(|w| (w[1].y - w[0].y).abs() <= Kind::Dirt.steepest() * SPACING + 1e-9));
        assert!(prof[12..20].iter().all(|p| p.y.abs() < 0.8), "its own again");
    }

    #[test]
    fn a_network_claims_the_ground_by_its_roads() {
        let road = |kind, z: f64, y: f64| Road { kind, points: (0..30).map(|i| Vec3::new(f64::from(i) * SPACING - 60.0, y, z)).collect() };
        let net = Network::new(vec![road(Kind::Dirt, 0.0, 1.0), road(Kind::Highway, 20.0, 5.0)]);
        let (r, d, h, w) = net.claim(Vec2::new(0.0, 1.0)).unwrap();
        assert_eq!((r, h, w), (0, 1.0, 1.0));
        assert!((d - 1.0).abs() < 1e-9);
        // Halfway between: the highway's, being greater and as near.
        assert_eq!(net.claim(Vec2::new(0.0, 10.0)).map(|c| c.0), Some(1));
        assert!(net.claim(Vec2::new(0.0, 80.0)).is_none());
        assert!(net.off_road(Vec2::new(0.0, 20.0)) < 0.0);
    }
}
