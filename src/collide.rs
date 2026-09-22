//! What is solid, and keeping a body out of it. The world's solids are
//! triangles in a grid over the ground plan; a body is an upright capsule
//! (feet, radius, height). `resolve` pushes a capsule out of whatever it
//! overlaps and says what it touched: floor it can stand on, wall (or a
//! slope too steep to climb), or ceiling. Floors push straight up, so a
//! body stands still on a slope instead of creeping down it; steep faces
//! push only sideways, so no amount of running climbs them.

use std::collections::HashMap;

use lntrn_math::Vec3;

/// A surface this close to level or closer is floor: 46°, so a 45° ramp
/// is (just) walkable.
pub const WALKABLE: f64 = 0.6947;
/// Faces turned up more than this (and less than [`WALKABLE`]) are steep
/// slopes; anything closer to upright is a wall.
const STEEP_FROM: f64 = 0.1;
/// Normals pointing further down than this are ceilings.
const CEILING: f64 = -0.3;
/// Grid cells, metres a side.
const CELL: f64 = 4.0;
/// Passes over the nearby triangles per resolve: pushing out of one can
/// push into another (a corner).
const PASSES: usize = 4;
/// Left between a body and what it touches, so the next resolve still
/// finds the floor it stands on.
const SKIN: f64 = 0.001;
/// How far off a floor still counts as standing on it.
const TOUCH: f64 = 0.01;

#[derive(Clone, Copy, Debug)]
struct Tri {
    a: Vec3,
    b: Vec3,
    c: Vec3,
    normal: Vec3,
}

/// An upright capsule standing at `feet`.
#[derive(Clone, Copy, Debug)]
pub struct Capsule {
    pub radius: f64,
    pub height: f64,
}

impl Capsule {
    /// The segment its spheres run along, standing at `feet`.
    fn segment(&self, feet: Vec3) -> (Vec3, Vec3) {
        let low = feet + Vec3::new(0.0, self.radius, 0.0);
        let high = feet + Vec3::new(0.0, (self.height - self.radius).max(self.radius), 0.0);
        (low, high)
    }
}

/// What a resolve touched.
#[derive(Clone, Debug, Default)]
pub struct Contacts {
    /// The most level floor under the body, if any.
    pub floor: Option<Vec3>,
    /// The highest point of floor touched: how tall a step is, measured
    /// where it is stood on (a capsule rests on an edge below its top).
    pub floor_top: Option<f64>,
    /// Pushed sideways by a wall or a too-steep slope.
    pub wall: bool,
    pub ceiling: bool,
    /// Every surface normal touched, for clipping velocity.
    pub normals: Vec<Vec3>,
}

impl Contacts {
    pub fn merge(&mut self, o: Contacts) {
        if let Some(f) = o.floor
            && self.floor.is_none_or(|g| f.y > g.y)
        {
            self.floor = Some(f);
        }
        if let Some(t) = o.floor_top {
            self.floor_top = Some(self.floor_top.map_or(t, |u| u.max(t)));
        }
        self.wall |= o.wall;
        self.ceiling |= o.ceiling;
        self.normals.extend(o.normals);
    }
}

/// The world's solid triangles.
#[derive(Clone, Debug, Default)]
pub struct Solids {
    tris: Vec<Tri>,
    cells: HashMap<(i32, i32), Vec<u32>>,
}

fn cell_of(v: f64) -> i32 {
    (v / CELL).floor() as i32
}

impl Solids {
    #[cfg(test)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Make these triangles solid.
    pub fn add(&mut self, tris: &[[Vec3; 3]]) {
        for &[a, b, c] in tris {
            let n = (b - a).cross(c - a);
            if n.length() < 1e-12 {
                continue; // a sliver
            }
            let i = self.tris.len() as u32;
            self.tris.push(Tri { a, b, c, normal: n.normalize() });
            let (x0, x1) = (a.x.min(b.x).min(c.x), a.x.max(b.x).max(c.x));
            let (z0, z1) = (a.z.min(b.z).min(c.z), a.z.max(b.z).max(c.z));
            for gx in cell_of(x0)..=cell_of(x1) {
                for gz in cell_of(z0)..=cell_of(z1) {
                    self.cells.entry((gx, gz)).or_default().push(i);
                }
            }
        }
    }

    pub fn len(&self) -> usize {
        self.tris.len()
    }

    /// The triangles whose cells a box over the ground plan touches.
    fn near(&self, x0: f64, x1: f64, z0: f64, z1: f64) -> Vec<u32> {
        let mut out = Vec::new();
        for gx in cell_of(x0)..=cell_of(x1) {
            for gz in cell_of(z0)..=cell_of(z1) {
                if let Some(list) = self.cells.get(&(gx, gz)) {
                    out.extend_from_slice(list);
                }
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Push a capsule standing at `feet` out of everything it overlaps.
    pub fn resolve(&self, capsule: Capsule, feet: &mut Vec3) -> Contacts {
        let mut contacts = Contacts::default();
        let r = capsule.radius;
        let near = self.near(feet.x - r - 0.1, feet.x + r + 0.1, feet.z - r - 0.1, feet.z + r + 0.1);
        for _ in 0..PASSES {
            let mut moved = false;
            for &i in &near {
                let t = &self.tris[i as usize];
                let (low, high) = capsule.segment(*feet);
                let (on_seg, on_tri) = closest_segment_triangle(low, high, t);
                let gap = on_seg - on_tri;
                let d = gap.length();
                // Floors count as touched a little way off (a body rests a
                // skin above them); anything else only when overlapped.
                if d >= r + TOUCH {
                    continue;
                }
                // Which way out: from the triangle to the body, or (through
                // it) the side of the face the body's middle is on.
                let n = if d > 1e-9 {
                    gap * (1.0 / d)
                } else {
                    let mid = (low + high) * 0.5;
                    if (mid - t.a).dot(t.normal) >= 0.0 { t.normal } else { -t.normal }
                };
                let depth = r - d + SKIN;
                // Floor when the way out is walkable, unless the surface is a
                // steep slope: resting on one, the seam it shares with its
                // neighbour can lie in a walkable direction, and that is no
                // ground. The top edge of a wall or a stair's riser is.
                let face = if t.normal.dot(n) >= 0.0 { t.normal } else { -t.normal };
                let steep = face.y > STEEP_FROM && face.y < WALKABLE;
                if n.y >= WALKABLE && !steep {
                    contacts.merge(Contacts { floor: Some(n), floor_top: Some(on_tri.y), ..Default::default() });
                    if d >= r - 1e-9 {
                        continue;
                    }
                    feet.y += depth / n.y;
                } else if d >= r - 1e-9 {
                    continue;
                } else if n.y > CEILING {
                    let side = Vec3::new(n.x, 0.0, n.z);
                    let len = side.length();
                    if len < 1e-9 {
                        continue;
                    }
                    let side = side * (1.0 / len);
                    *feet += side * (depth / n.dot(side)).min(r);
                    contacts.merge(Contacts { wall: true, normals: vec![side], ..Default::default() });
                } else {
                    *feet += n * depth;
                    contacts.merge(Contacts { ceiling: true, normals: vec![n], ..Default::default() });
                }
                moved = true;
            }
            if !moved {
                break;
            }
        }
        contacts
    }

    /// Whether a capsule standing at `feet` overlaps nothing (touching the
    /// floor it stands on is allowed).
    pub fn fits(&self, capsule: Capsule, feet: Vec3) -> bool {
        let r = capsule.radius;
        // Lifted a hair, so the floor underfoot does not count.
        let (low, high) = capsule.segment(feet + Vec3::new(0.0, 0.01, 0.0));
        self.near(feet.x - r, feet.x + r, feet.z - r, feet.z + r).into_iter().all(|i| {
            let (s, t) = closest_segment_triangle(low, high, &self.tris[i as usize]);
            (s - t).length() >= r - 0.005
        })
    }
}

/// The point of triangle `t` closest to `p`.
fn closest_point_triangle(p: Vec3, t: &Tri) -> Vec3 {
    // Ericson, Real-Time Collision Detection 5.1.5.
    let (a, b, c) = (t.a, t.b, t.c);
    let ab = b - a;
    let ac = c - a;
    let ap = p - a;
    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }
    let bp = p - b;
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return a + ab * (d1 / (d1 - d3));
    }
    let cp = p - c;
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return a + ac * (d2 / (d2 - d6));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        return b + (c - b) * ((d4 - d3) / ((d4 - d3) + (d5 - d6)));
    }
    let denom = 1.0 / (va + vb + vc);
    a + ab * (vb * denom) + ac * (vc * denom)
}

/// The closest points of segments `p1q1` and `p2q2`.
fn closest_segments(p1: Vec3, q1: Vec3, p2: Vec3, q2: Vec3) -> (Vec3, Vec3) {
    // Ericson 5.1.9.
    let d1 = q1 - p1;
    let d2 = q2 - p2;
    let r = p1 - p2;
    let a = d1.dot(d1);
    let e = d2.dot(d2);
    let f = d2.dot(r);
    let (s, t);
    if a <= 1e-12 && e <= 1e-12 {
        return (p1, p2);
    }
    if a <= 1e-12 {
        s = 0.0;
        t = (f / e).clamp(0.0, 1.0);
    } else {
        let c = d1.dot(r);
        if e <= 1e-12 {
            t = 0.0;
            s = (-c / a).clamp(0.0, 1.0);
        } else {
            let b = d1.dot(d2);
            let denom = a * e - b * b;
            let mut s0 = if denom.abs() > 1e-12 { ((b * f - c * e) / denom).clamp(0.0, 1.0) } else { 0.0 };
            let mut t0 = (b * s0 + f) / e;
            if t0 < 0.0 {
                t0 = 0.0;
                s0 = (-c / a).clamp(0.0, 1.0);
            } else if t0 > 1.0 {
                t0 = 1.0;
                s0 = ((b - c) / a).clamp(0.0, 1.0);
            }
            s = s0;
            t = t0;
        }
    }
    (p1 + d1 * s, p2 + d2 * t)
}

/// The closest points of a segment and a triangle: (on the segment, on the
/// triangle). The same point twice when the segment passes through it.
fn closest_segment_triangle(p: Vec3, q: Vec3, t: &Tri) -> (Vec3, Vec3) {
    // Through the face?
    let d = q - p;
    let denom = t.normal.dot(d);
    if denom.abs() > 1e-12 {
        let s = t.normal.dot(t.a - p) / denom;
        if (0.0..=1.0).contains(&s) {
            let x = p + d * s;
            if (closest_point_triangle(x, t) - x).length() < 1e-9 {
                return (x, x);
            }
        }
    }
    let mut best = (p, closest_point_triangle(p, t));
    let mut best_d = (best.0 - best.1).length();
    let mut consider = |pair: (Vec3, Vec3)| {
        let dd = (pair.0 - pair.1).length();
        if dd < best_d {
            best_d = dd;
            best = pair;
        }
    };
    consider((q, closest_point_triangle(q, t)));
    consider(closest_segments(p, q, t.a, t.b));
    consider(closest_segments(p, q, t.b, t.c));
    consider(closest_segments(p, q, t.c, t.a));
    best
}

/// A box's twelve triangles, from its lower and upper corners.
#[cfg(test)]
pub fn box_tris(min: Vec3, max: Vec3) -> Vec<[Vec3; 3]> {
    let p = |x: bool, y: bool, z: bool| Vec3::new(if x { max.x } else { min.x }, if y { max.y } else { min.y }, if z { max.z } else { min.z });
    let quads = [
        [p(false, false, false), p(true, false, false), p(true, false, true), p(false, false, true)],
        [p(false, true, false), p(false, true, true), p(true, true, true), p(true, true, false)],
        [p(false, false, false), p(false, true, false), p(true, true, false), p(true, false, false)],
        [p(false, false, true), p(true, false, true), p(true, true, true), p(false, true, true)],
        [p(false, false, false), p(false, false, true), p(false, true, true), p(false, true, false)],
        [p(true, false, false), p(true, true, false), p(true, true, true), p(true, false, true)],
    ];
    quads.iter().flat_map(|q| [[q[0], q[1], q[2]], [q[0], q[2], q[3]]]).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const BODY: Capsule = Capsule { radius: 0.35, height: 1.8 };

    fn floor() -> Solids {
        let mut s = Solids::new();
        s.add(&box_tris(Vec3::new(-50.0, -1.0, -50.0), Vec3::new(50.0, 0.0, 50.0)));
        s
    }

    #[test]
    fn standing_on_a_floor_is_pushed_up_onto_it() {
        let s = floor();
        let mut feet = Vec3::new(1.0, -0.2, 2.0);
        let c = s.resolve(BODY, &mut feet);
        assert!(c.floor.is_some() && !c.wall);
        assert!((feet.y - SKIN).abs() < 0.005, "at {}", feet.y);
        assert!(feet.x == 1.0 && feet.z == 2.0, "straight up, not sideways");
        assert!(s.fits(BODY, feet));
        // Resting there, the floor is still underfoot and nothing moves.
        let rest = feet;
        assert!(s.resolve(BODY, &mut feet).floor.is_some());
        assert_eq!(feet, rest);
    }

    #[test]
    fn a_wall_pushes_out_sideways_and_says_so() {
        let mut s = floor();
        s.add(&box_tris(Vec3::new(2.0, 0.0, -5.0), Vec3::new(3.0, 3.0, 5.0)));
        let mut feet = Vec3::new(1.8, 0.0, 0.0);
        let c = s.resolve(BODY, &mut feet);
        assert!(c.wall);
        assert!((feet.x - (2.0 - 0.35)).abs() < 0.01, "against the wall at {}", feet.x);
        assert!(feet.y.abs() < 0.01, "not lifted");
    }

    #[test]
    fn a_steep_slope_never_lifts() {
        // A 60° face: walking into it pushes back, not up it.
        let mut s = Solids::new();
        let (a, b, c) = (Vec3::new(0.0, 0.0, -5.0), Vec3::new(0.0, 0.0, 5.0), Vec3::new(-2.0, 2.0 * 3f64.sqrt(), 0.0));
        s.add(&[[a, b, c]]);
        let mut feet = Vec3::new(-0.3, 0.2, 0.0);
        let before = feet.y;
        let c = s.resolve(BODY, &mut feet);
        assert!(c.wall && c.floor.is_none());
        assert_eq!(feet.y, before);
    }

    #[test]
    fn a_ceiling_pushes_down() {
        let mut s = floor();
        s.add(&box_tris(Vec3::new(-2.0, 1.3, -2.0), Vec3::new(2.0, 1.6, 2.0)));
        assert!(!s.fits(BODY, Vec3::ZERO), "no room to stand");
        assert!(s.fits(Capsule { radius: 0.35, height: 1.2 }, Vec3::ZERO), "room to crouch");
    }

    #[test]
    fn closest_points_agree_with_brute_force() {
        let t = Tri { a: Vec3::new(0.0, 0.0, 0.0), b: Vec3::new(2.0, 0.0, 0.0), c: Vec3::new(0.0, 0.0, 2.0), normal: Vec3::Y };
        // Straight above the middle, off past a corner, and crossing.
        for (p, q, want) in [
            (Vec3::new(0.5, 1.0, 0.5), Vec3::new(0.5, 3.0, 0.5), 1.0),
            (Vec3::new(3.0, 0.5, 3.0), Vec3::new(4.0, 0.5, 4.0), (2.0f64 * 2.0 + 2.0 * 2.0 + 0.25).sqrt()),
            (Vec3::new(0.5, -1.0, 0.5), Vec3::new(0.5, 1.0, 0.5), 0.0),
        ] {
            let (a, b) = closest_segment_triangle(p, q, &t);
            assert!(((a - b).length() - want).abs() < 1e-6, "{p:?}-{q:?}: {} not {want}", (a - b).length());
        }
    }
}
