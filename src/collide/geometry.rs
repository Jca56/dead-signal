//! The geometry under the collision: where a ray meets a triangle, and
//! the closest points of a point, a segment and a triangle to each other
//! (after Ericson, Real-Time Collision Detection).

use lntrn_math::Vec3;

use super::Tri;

/// How far along a ray (unit `dir`) it meets triangle `t`, from either
/// side (Möller–Trumbore).
pub(super) fn ray_triangle(from: Vec3, dir: Vec3, t: &Tri) -> Option<f64> {
    let e1 = t.b - t.a;
    let e2 = t.c - t.a;
    let p = dir.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-12 {
        return None;
    }
    let inv = 1.0 / det;
    let s = from - t.a;
    let u = s.dot(p) * inv;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = s.cross(e1);
    let v = dir.dot(q) * inv;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let dist = e2.dot(q) * inv;
    (dist > 1e-6).then_some(dist)
}

/// The point of triangle `t` closest to `p`.
pub(super) fn closest_point_triangle(p: Vec3, t: &Tri) -> Vec3 {
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
pub(super) fn closest_segments(p1: Vec3, q1: Vec3, p2: Vec3, q2: Vec3) -> (Vec3, Vec3) {
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
pub(super) fn closest_segment_triangle(p: Vec3, q: Vec3, t: &Tri) -> (Vec3, Vec3) {
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
