//! The lie of the land. Three layers: the land as it lies (rolling hills,
//! rising into ridges at the map's edge but for the passes the highway
//! runs out through), then the plots levelled for the town and the places
//! out in the country, then the roads cut and filled into it. What comes
//! out is a grid of heights (a triangle mesh, its inner corners nudged off
//! true so the facets don't line up), and that's the ground.

use lntrn_math::{Vec2, Vec3};

use super::noise::Noise;
use super::{EDGE, EXTENT};

/// The mesh's cells, metres a side.
pub const STEP: f64 = 4.0;
/// How far a corner is nudged, as a share of a cell.
const NUDGE: f64 = 0.3;

/// A patch of land levelled (or only cleared: fields are left as they
/// lie): a rectangle turned `yaw`, `half` its size each way, levelled at
/// `height` and blended back into the land over `shoulder` metres.
#[derive(Clone, Debug)]
pub struct Plot {
    pub centre: Vec2,
    pub yaw: f64,
    pub half: Vec2,
    pub height: f64,
    pub shoulder: f64,
}

impl Plot {
    pub fn new(centre: Vec2, yaw: f64, half: Vec2, shoulder: f64) -> Self {
        Self { centre, yaw, half, height: 0.0, shoulder }
    }

    /// A point in the plot's own frame: x across it, y along (its front,
    /// facing its road, is at -y).
    pub fn local(&self, p: Vec2) -> Vec2 {
        let d = p - self.centre;
        let (s, c) = self.yaw.sin_cos();
        Vec2::new(d.x * c - d.y * s, d.x * s + d.y * c)
    }

    /// The plot's own frame back to the map's.
    pub fn world(&self, l: Vec2) -> Vec2 {
        let (s, c) = self.yaw.sin_cos();
        self.centre + Vec2::new(l.x * c + l.y * s, -l.x * s + l.y * c)
    }

    /// How far outside the rectangle a point is (0 within).
    pub fn outside(&self, p: Vec2) -> f64 {
        let l = self.local(p);
        let (dx, dy) = ((l.x.abs() - self.half.x).max(0.0), (l.y.abs() - self.half.y).max(0.0));
        dx.hypot(dy)
    }

    /// How much the point is the plot's: 1 within it, easing to 0 over its
    /// shoulder.
    pub fn weight(&self, p: Vec2) -> f64 {
        1.0 - smooth(self.outside(p) / self.shoulder.max(1e-6))
    }

    /// The farthest a corner is from the middle.
    pub fn reach(&self) -> f64 {
        self.half.length()
    }
}

/// 0 at 0, 1 at 1 and past it, eased at both ends.
pub fn smooth(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The land as it lies, before anything is levelled.
pub struct Natural {
    noise: Noise,
    /// Where the highway runs out through the ridges: from a point at the
    /// edge, heading in.
    passes: Vec<(Vec2, Vec2)>,
}

impl Natural {
    pub fn new(seed: u32) -> Self {
        Self { noise: Noise::new(seed), passes: Vec::new() }
    }

    /// Let the highway out through the ridge at `edge`, heading in along
    /// `inward` (a unit direction).
    pub fn pass(&mut self, edge: Vec2, inward: Vec2) {
        self.passes.push((edge, inward));
    }

    /// Wide hills with gentler folds on them and a little roughness, rising
    /// into ridges past the edge.
    pub fn height(&self, x: f64, z: f64) -> f64 {
        let n = &self.noise;
        let mut h = 16.0 * n.layered(x / 260.0, z / 260.0, 3);
        h += 3.5 * n.layered(x / 70.0 + 40.0, z / 70.0 - 40.0, 2);
        h += 0.6 * n.at(x / 14.0 - 90.0, z / 14.0 + 90.0);
        let out = (x.abs().max(z.abs()) - EDGE).max(0.0);
        if out > 0.0 {
            let ridge = ((out / 11.0).powi(2) * 1.5).min(45.0) * (0.8 + 0.4 * n.at(x / 40.0 + 7.0, z / 40.0 - 7.0));
            h += ridge * (1.0 - self.in_pass(x, z));
        }
        h
    }

    /// How far in a pass the point is: 1 on its line, 0 well off it.
    fn in_pass(&self, x: f64, z: f64) -> f64 {
        let p = Vec2::new(x, z);
        self.passes
            .iter()
            .map(|&(edge, inward)| {
                let d = p - edge;
                let along = d.dot(inward);
                let off = (d - inward * along).length();
                if !(-40.0..=130.0).contains(&along) { 0.0 } else { 1.0 - smooth((off - 14.0) / 30.0) }
            })
            .fold(0.0, f64::max)
    }
}

/// The land with its plots levelled.
pub struct Shaped<'a> {
    pub natural: &'a Natural,
    pub plots: &'a [Plot],
}

impl Shaped<'_> {
    pub fn height(&self, x: f64, z: f64) -> f64 {
        let mut h = self.natural.height(x, z);
        let p = Vec2::new(x, z);
        for plot in self.plots {
            let w = plot.weight(p);
            if w > 0.0 {
                h += (plot.height - h) * w;
            }
        }
        h
    }
}

/// Level a plot at the land's average height over it.
pub fn level(plot: &mut Plot, natural: &Natural) {
    let mut sum = 0.0;
    let mut n = 0.0;
    for i in -3..=3 {
        for j in -3..=3 {
            let l = Vec2::new(plot.half.x * f64::from(i) / 3.0, plot.half.y * f64::from(j) / 3.0);
            let w = plot.world(l);
            sum += natural.height(w.x, w.y);
            n += 1.0;
        }
    }
    plot.height = sum / n;
}

/// The ground: a grid of corners over the whole land, each where it ended
/// up (nudged, and at its final height).
#[derive(Clone, Debug)]
pub struct Field {
    /// Corners a side.
    pub n: usize,
    pub corners: Vec<Vec3>,
}

impl Field {
    /// Lay the grid over the land, each corner at `height`.
    pub fn new(seed: u32, height: impl Fn(f64, f64) -> f64 + Sync) -> Self {
        let n = (2.0 * EXTENT / STEP) as usize + 1;
        let corners = super::rows(n, |j| {
            (0..n)
                .map(|i| {
                    let (mut x, mut z) = (-EXTENT + i as f64 * STEP, -EXTENT + j as f64 * STEP);
                    if i > 0 && j > 0 && i < n - 1 && j < n - 1 {
                        x += (hash(seed, i, j, 1) - 0.5) * 2.0 * NUDGE * STEP;
                        z += (hash(seed, i, j, 2) - 0.5) * 2.0 * NUDGE * STEP;
                    }
                    Vec3::new(x, height(x, z), z)
                })
                .collect()
        });
        Self { n, corners }
    }

    pub fn corner(&self, i: usize, j: usize) -> Vec3 {
        self.corners[j * self.n + i]
    }

    /// Cell `(i, j)`'s two triangles, wound to face up; the diagonal
    /// alternates, so the facets don't run in stripes.
    pub fn cell(&self, i: usize, j: usize) -> [[Vec3; 3]; 2] {
        let (a, b, c, d) = (self.corner(i, j), self.corner(i + 1, j), self.corner(i + 1, j + 1), self.corner(i, j + 1));
        // Seen from above, x right and z down the page: a b / d c, wound
        // a d c (anticlockwise from above, with +z towards the viewer).
        if (i + j) % 2 == 1 { [[a, d, c], [a, c, b]] } else { [[a, d, b], [b, d, c]] }
    }

    /// The ground's height at `(x, z)`, if it's on the land.
    pub fn height_at(&self, x: f64, z: f64) -> Option<f64> {
        let fi = ((x + EXTENT) / STEP).floor() as i64;
        let fj = ((z + EXTENT) / STEP).floor() as i64;
        let cells = self.n as i64 - 1;
        // The corners are nudged: the point can be in a cell beside the one
        // it rounds to.
        for dj in [0, -1, 1] {
            for di in [0, -1, 1] {
                let (i, j) = (fi + di, fj + dj);
                if i < 0 || j < 0 || i >= cells || j >= cells {
                    continue;
                }
                for t in self.cell(i as usize, j as usize) {
                    if let Some(y) = on_triangle(t, x, z) {
                        return Some(y);
                    }
                }
            }
        }
        None
    }
}

/// The height of triangle `t` over `(x, z)`, if it's over it.
fn on_triangle([a, b, c]: [Vec3; 3], x: f64, z: f64) -> Option<f64> {
    let d = (b.z - c.z) * (a.x - c.x) + (c.x - b.x) * (a.z - c.z);
    if d.abs() < 1e-12 {
        return None;
    }
    let u = ((b.z - c.z) * (x - c.x) + (c.x - b.x) * (z - c.z)) / d;
    let v = ((c.z - a.z) * (x - c.x) + (a.x - c.x) * (z - c.z)) / d;
    let w = 1.0 - u - v;
    let eps = -1e-9;
    (u >= eps && v >= eps && w >= eps).then_some(u * a.y + v * b.y + w * c.y)
}

/// A number 0–1 for grid point `(i, j)`, the same every time.
pub fn hash(seed: u32, i: usize, j: usize, salt: u32) -> f64 {
    let mut h = seed ^ (i as u32).wrapping_mul(0x9E37_79B1) ^ (j as u32).wrapping_mul(0x85EB_CA77) ^ salt.wrapping_mul(0xC2B2_AE3D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A_2D39);
    h ^= h >> 15;
    f64::from(h) / f64::from(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plot_is_level_within_and_eases_out() {
        let natural = Natural::new(3);
        let mut plot = Plot::new(Vec2::new(40.0, -20.0), 0.6, Vec2::new(20.0, 10.0), 12.0);
        level(&mut plot, &natural);
        let plots = [plot.clone()];
        let shaped = Shaped { natural: &natural, plots: &plots };
        for (lx, ly) in [(0.0, 0.0), (19.0, 9.0), (-19.0, -9.0)] {
            let w = plot.world(Vec2::new(lx, ly));
            assert!((shaped.height(w.x, w.y) - plot.height).abs() < 1e-9);
        }
        let far = plot.world(Vec2::new(0.0, 10.0 + 12.5));
        assert_eq!(shaped.height(far.x, far.y), natural.height(far.x, far.y));
        // Its frame goes there and back.
        let p = Vec2::new(3.0, 7.0);
        assert!((plot.world(plot.local(p)) - p).length() < 1e-9);
    }

    #[test]
    fn the_field_answers_heights_between_its_corners() {
        let field = Field::new(9, |x, z| 0.1 * x + 0.05 * z);
        for (x, z) in [(0.0, 0.0), (13.3, -71.9), (-200.5, 150.25), (EXTENT - 1.0, -EXTENT + 1.0)] {
            let h = field.height_at(x, z).expect("on the land");
            assert!((h - (0.1 * x + 0.05 * z)).abs() < 1e-6, "{h} at {x} {z}");
        }
        assert!(field.height_at(EXTENT + 10.0, 0.0).is_none());
        // Every triangle faces up.
        for [a, b, c] in field.cell(10, 10).into_iter().chain(field.cell(11, 10)) {
            assert!((b - a).cross(c - a).y > 0.0);
        }
    }

    #[test]
    fn ridges_rise_past_the_edge_but_not_in_a_pass() {
        let mut natural = Natural::new(5);
        let flat_avg = |n: &Natural, x: f64| (0..20).map(|k| n.height(x, -100.0 + f64::from(k) * 10.0)).sum::<f64>() / 20.0;
        let inside = flat_avg(&natural, 200.0);
        let beyond = flat_avg(&natural, EDGE + 60.0);
        assert!(beyond > inside + 15.0, "{inside} then {beyond}");
        natural.pass(Vec2::new(EXTENT, 0.0), Vec2::new(-1.0, 0.0));
        let open = natural.height(EDGE + 60.0, 0.0);
        let shut = Natural::new(5).height(EDGE + 60.0, 0.0);
        assert!(shut - open > 10.0, "the pass lowers the ridge: {shut} vs {open}");
    }
}
