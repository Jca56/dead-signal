//! What is solid, and keeping a body out of it. The world's solids are
//! triangles in a grid over the ground plan; a body is an upright capsule
//! (feet, radius, height). `resolve` pushes a capsule out of whatever it
//! overlaps and says what it touched: floor it can stand on, wall (or a
//! slope too steep to climb), or ceiling. Floors push straight up, so a
//! body stands still on a slope instead of creeping down it; steep faces
//! push only sideways, so no amount of running climbs them.

use std::collections::HashMap;

use lntrn_math::Vec3;

use geometry::{closest_segment_triangle, ray_triangle};

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

/// What a solid is made of: what a bullet kicks up, what it sounds like.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Surface {
    #[default]
    Dirt,
    Wood,
    Stone,
    Metal,
    /// The dead.
    Flesh,
}

#[derive(Clone, Copy, Debug)]
struct Tri {
    a: Vec3,
    b: Vec3,
    c: Vec3,
    normal: Vec3,
    surface: Surface,
    /// Switched off: there, but nothing meets it.
    off: bool,
    /// Only bodies meet it: an empty window's frame, seen and shot and
    /// heard through, but not climbed through.
    ghost: bool,
}

/// Where a ray met a solid.
#[derive(Clone, Copy, Debug)]
pub struct RayHit {
    /// How far along the ray, metres.
    pub t: f64,
    pub point: Vec3,
    /// The surface's normal, turned to face where the ray came from.
    pub normal: Vec3,
    pub surface: Surface,
    /// Whether it met the face's front (the outside of a solid thing).
    pub front: bool,
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
    /// Make these triangles solid (stone, for want of saying).
    #[cfg(test)]
    pub fn add(&mut self, tris: &[[Vec3; 3]]) {
        self.add_as(tris, Surface::Stone);
    }

    /// Make these triangles solid, made of `surface`. Which they are (for
    /// switching them off and on).
    pub fn add_as(&mut self, tris: &[[Vec3; 3]], surface: Surface) -> std::ops::Range<u32> {
        self.add_tris(tris, surface, false)
    }

    /// Make these triangles solid to bodies only (rays pass through): what
    /// fills an empty window.
    pub fn add_barrier(&mut self, tris: &[[Vec3; 3]]) -> std::ops::Range<u32> {
        self.add_tris(tris, Surface::Stone, true)
    }

    fn add_tris(&mut self, tris: &[[Vec3; 3]], surface: Surface, ghost: bool) -> std::ops::Range<u32> {
        let first = self.tris.len() as u32;
        for &[a, b, c] in tris {
            let n = (b - a).cross(c - a);
            if n.length() < 1e-12 {
                continue; // a sliver
            }
            let i = self.tris.len() as u32;
            self.tris.push(Tri { a, b, c, normal: n.normalize(), surface, off: false, ghost });
            let (x0, x1) = (a.x.min(b.x).min(c.x), a.x.max(b.x).max(c.x));
            let (z0, z1) = (a.z.min(b.z).min(c.z), a.z.max(b.z).max(c.z));
            for gx in cell_of(x0)..=cell_of(x1) {
                for gz in cell_of(z0)..=cell_of(z1) {
                    self.cells.entry((gx, gz)).or_default().push(i);
                }
            }
        }
        first..self.tris.len() as u32
    }

    /// Switch the triangles `which` on (solid) or off (not there).
    pub fn switch(&mut self, which: std::ops::Range<u32>, on: bool) {
        for i in which {
            if let Some(t) = self.tris.get_mut(i as usize) {
                t.off = !on;
            }
        }
    }

    pub fn len(&self) -> usize {
        self.tris.len()
    }

    /// The first solid along the ray from `from` in unit direction `dir`
    /// within `max` metres (a window's barrier isn't met). Walks the grid cell by cell along the ground
    /// plan and stops at the first cell that holds a hit.
    /// (A triangle over more than one cell may be tried twice: that costs
    /// less than keeping count.)
    pub fn raycast(&self, from: Vec3, dir: Vec3, max: f64) -> Option<RayHit> {
        let mut best: Option<RayHit> = None;
        let (mut gx, mut gz) = (cell_of(from.x), cell_of(from.z));
        let step = |d: f64| if d > 0.0 { 1 } else { -1 };
        let next = |p: f64, g: i32, d: f64| {
            if d.abs() < 1e-12 {
                f64::INFINITY
            } else {
                let edge = if d > 0.0 { (g + 1) as f64 * CELL } else { g as f64 * CELL };
                (edge - p) / d
            }
        };
        let (mut tx, mut tz) = (next(from.x, gx, dir.x), next(from.z, gz, dir.z));
        let (dtx, dtz) = (if dir.x.abs() < 1e-12 { f64::INFINITY } else { CELL / dir.x.abs() }, if dir.z.abs() < 1e-12 { f64::INFINITY } else { CELL / dir.z.abs() });
        loop {
            if let Some(list) = self.cells.get(&(gx, gz)) {
                for &i in list {
                    let tri = &self.tris[i as usize];
                    if tri.off || tri.ghost {
                        continue;
                    }
                    if let Some(t) = ray_triangle(from, dir, tri)
                        && t <= max
                        && best.is_none_or(|b| t < b.t)
                    {
                        let front = tri.normal.dot(dir) < 0.0;
                        let normal = if front { tri.normal } else { -tri.normal };
                        best = Some(RayHit { t, point: from + dir * t, normal, surface: tri.surface, front });
                    }
                }
            }
            // Leaving this cell: a hit before its far side is the answer.
            let exit = tx.min(tz);
            if best.is_some_and(|b| b.t <= exit) || exit > max {
                return best;
            }
            if tx < tz {
                gx += step(dir.x);
                tx += dtx;
            } else {
                gz += step(dir.z);
                tz += dtz;
            }
        }
    }

    /// Every solid straight down from `from` (not a window's barrier), in
    /// order: where two faces meet at one height, one being left before
    /// one entered (a thing's underside before the floor it stands on);
    /// where the ray runs down an edge (between a face's two triangles, a
    /// roof's two slopes at its ridge), the faces there met once.
    pub fn hits_down(&self, from: Vec3) -> Vec<RayHit> {
        let down = Vec3::new(0.0, -1.0, 0.0);
        let mut hits: Vec<RayHit> = Vec::new();
        for &i in self.cells.get(&(cell_of(from.x), cell_of(from.z))).map_or(&[][..], |v| &v[..]) {
            let tri = &self.tris[i as usize];
            if tri.off || tri.ghost {
                continue;
            }
            if let Some(t) = ray_triangle(from, down, tri) {
                let front = tri.normal.y > 0.0;
                let normal = if front { tri.normal } else { -tri.normal };
                let hit = RayHit { t, point: from + down * t, normal, surface: tri.surface, front };
                if !hits.iter().any(|h| (h.t - t).abs() < 1e-6 && h.front == front) {
                    hits.push(hit);
                }
            }
        }
        hits.sort_by(|a, b| a.t.total_cmp(&b.t).then(a.front.cmp(&b.front)));
        hits
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
                if t.off {
                    continue;
                }
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
            let tri = &self.tris[i as usize];
            if tri.off {
                return true;
            }
            let (s, t) = closest_segment_triangle(low, high, tri);
            (s - t).length() >= r - 0.005
        })
    }
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

mod geometry;
#[cfg(test)]
mod tests;
