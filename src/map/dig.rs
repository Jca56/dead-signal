//! Digs: where the ground's dug out for what's built down into it (a
//! cellar under a building, a tunnel). The land is a sheet of triangles
//! with nothing under it, and everything under a sheet is in the ground.
//! An open dig cuts its rectangle out of the sheet: what's built down
//! there stands in a hole, under a lid of its own (the building over it).
//! A dig with the ground left over it as its lid gives the ground an
//! underside there instead (down in the roof of what's built under it),
//! so the ground is a crust with room beneath.

use lntrn_math::{Vec2, Vec3};

/// A rectangle of the map dug out (flat, square to the map): open to the
/// sky, or with the ground left over it as its lid, the ground's underside
/// at that height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dig {
    pub lo: Vec2,
    pub hi: Vec2,
    pub lid: Option<f64>,
}

/// What's left of a corner-list (convex, flat or not) on one side of a
/// line square to the map: where `x` (or `z`) is at most `at`, or at
/// least. Its corners stay in order, so it faces the way it did.
fn keep(poly: &[Vec3], along_x: bool, at: f64, under: bool) -> Vec<Vec3> {
    let side = |p: Vec3| {
        let v = if along_x { p.x } else { p.z };
        if under { at - v } else { v - at }
    };
    let mut out = Vec::with_capacity(poly.len() + 1);
    for (i, &a) in poly.iter().enumerate() {
        let b = poly[(i + 1) % poly.len()];
        let (da, db) = (side(a), side(b));
        if da >= 0.0 {
            out.push(a);
        }
        if (da > 0.0 && db < 0.0) || (da < 0.0 && db > 0.0) {
            out.push(a + (b - a) * (da / (da - db)));
        }
    }
    out
}

/// A corner-list's triangles (fanned from its first corner), but for any
/// with no area.
fn fan(part: &[Vec3], out: &mut Vec<[Vec3; 3]>) {
    for k in 1..part.len().saturating_sub(1) {
        let t = [part[0], part[k], part[k + 1]];
        if (t[1] - t[0]).cross(t[2] - t[0]).length() > 1e-9 {
            out.push(t);
        }
    }
}

impl Dig {
    /// Whether a triangle of ground is clear of the dig.
    fn clear_of(&self, tri: &[Vec3; 3]) -> bool {
        let (lo, hi) = tri.iter().fold((Vec3::splat(f64::INFINITY), Vec3::splat(f64::NEG_INFINITY)), |(lo, hi), p| (lo.min(*p), hi.max(*p)));
        hi.x <= self.lo.x || lo.x >= self.hi.x || hi.z <= self.lo.y || lo.z >= self.hi.y
    }

    /// The ground's underside over the dig, if the ground's its lid: its
    /// rectangle, facing down.
    pub fn underside(&self) -> Option<[[Vec3; 3]; 2]> {
        let y = self.lid?;
        let p = |x: f64, z: f64| Vec3::new(x, y, z);
        let (a, b, c, d) = (p(self.lo.x, self.lo.y), p(self.hi.x, self.lo.y), p(self.hi.x, self.hi.y), p(self.lo.x, self.hi.y));
        Some([[a, b, c], [a, c, d]])
    }

    /// What's left of a triangle of ground round the dig: all of it, if
    /// it's clear of it; nothing, if it's all dug away.
    pub fn leaves(&self, tri: [Vec3; 3]) -> Vec<[Vec3; 3]> {
        if self.clear_of(&tri) {
            return vec![tri];
        }
        // To either side of it; and of the strip between, before it and
        // past it.
        let between = keep(&keep(&tri, true, self.lo.x, false), true, self.hi.x, true);
        let parts = [keep(&tri, true, self.lo.x, true), keep(&tri, true, self.hi.x, false), keep(&between, false, self.lo.y, true), keep(&between, false, self.hi.y, false)];
        let mut out = Vec::new();
        for part in parts {
            fan(&part, &mut out);
        }
        out
    }
}

/// A triangle of ground with every open dig cut out of it.
pub fn cut(digs: &[Dig], tri: [Vec3; 3]) -> Vec<[Vec3; 3]> {
    let mut left = vec![tri];
    for dig in digs.iter().filter(|d| d.lid.is_none()) {
        left = left.into_iter().flat_map(|t| dig.leaves(t)).collect();
    }
    left
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(t: &[Vec3; 3]) -> f64 {
        let (a, b) = (t[1] - t[0], t[2] - t[0]);
        (a.x * b.z - a.z * b.x).abs() * 0.5
    }

    #[test]
    fn a_dig_takes_its_rectangle_out_of_the_ground_and_no_more() {
        let dig = Dig { lo: Vec2::new(2.0, -1.0), hi: Vec2::new(5.0, 3.0), lid: None };
        // Ground on a slope, in two triangles facing up, far wider than
        // the dig.
        let y = |x: f64, z: f64| 0.1 * x - 0.05 * z;
        let p = |x: f64, z: f64| Vec3::new(x, y(x, z), z);
        let ground = [[p(-4.0, -6.0), p(-4.0, 8.0), p(9.0, 8.0)], [p(-4.0, -6.0), p(9.0, 8.0), p(9.0, -6.0)]];
        let left: Vec<[Vec3; 3]> = ground.iter().flat_map(|t| dig.leaves(*t)).collect();
        let whole: f64 = ground.iter().map(area).sum();
        let kept: f64 = left.iter().map(area).sum();
        assert!((whole - kept - 12.0).abs() < 1e-9, "{whole} less {kept} isn't the dig's 12");
        for t in &left {
            // Still on the slope, still facing up, and none of it in the
            // dig.
            assert!(t.iter().all(|c| (c.y - y(c.x, c.z)).abs() < 1e-9));
            assert!((t[1] - t[0]).cross(t[2] - t[0]).y > 0.0, "{t:?} faces down");
            let mid = (t[0] + t[1] + t[2]) * (1.0 / 3.0);
            assert!(!(mid.x > 2.0 && mid.x < 5.0 && mid.z > -1.0 && mid.z < 3.0), "{t:?} is in the dig");
        }
    }

    #[test]
    fn ground_clear_of_a_dig_is_left_be_and_ground_in_it_is_gone() {
        let dig = Dig { lo: Vec2::new(0.0, 0.0), hi: Vec2::new(10.0, 10.0), lid: None };
        let far = [Vec3::new(20.0, 1.0, 0.0), Vec3::new(20.0, 1.0, 4.0), Vec3::new(24.0, 1.0, 4.0)];
        assert_eq!(dig.leaves(far), vec![far]);
        let within = [Vec3::new(2.0, 0.0, 2.0), Vec3::new(2.0, 0.0, 6.0), Vec3::new(6.0, 0.0, 6.0)];
        assert!(dig.leaves(within).is_empty());
        // Two digs, side by side: both gone.
        let digs = [dig, Dig { lo: Vec2::new(10.0, 0.0), hi: Vec2::new(14.0, 10.0), lid: None }];
        let across = [Vec3::new(1.0, 0.0, 1.0), Vec3::new(1.0, 0.0, 9.0), Vec3::new(13.0, 0.0, 9.0)];
        assert!(cut(&digs, across).is_empty());
        assert_eq!(cut(&[], across), vec![across]);
    }

    #[test]
    fn a_dig_with_a_lid_leaves_the_ground_and_gives_it_an_underside() {
        let dig = Dig { lo: Vec2::new(2.0, -1.0), hi: Vec2::new(5.0, 3.0), lid: Some(-0.4) };
        let ground = [Vec3::new(-4.0, 1.0, -6.0), Vec3::new(-4.0, 1.0, 8.0), Vec3::new(9.0, 1.0, 8.0)];
        assert_eq!(cut(&[dig], ground), vec![ground], "the ground's cut");
        // Its rectangle, at its height, facing down.
        let under = dig.underside().expect("an underside");
        for t in &under {
            assert!((t[1] - t[0]).cross(t[2] - t[0]).y < 0.0, "{t:?} faces up");
            assert!(t.iter().all(|c| c.y == -0.4));
        }
        assert!((under.iter().map(area).sum::<f64>() - 12.0).abs() < 1e-9);
        assert!(Dig { lid: None, ..dig }.underside().is_none());
    }
}
