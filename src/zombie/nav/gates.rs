//! Doorways that can be shut: the grid is probed with them open, and a
//! gate is every link that crosses one. Shutting it cuts those links (the
//! dead go round, or can't get there at all); opening it puts them back.
//! Either way, which ground can get to which is worked out again.

use lntrn_math::{Vec2, Vec3};

use super::regions::Regions;
use super::{AROUND, NONE, NavGrid};

/// The links through a doorway: from which floor, which way, onto which.
#[derive(Clone, Debug, Default)]
pub struct Gate {
    links: Vec<(u32, u8, u32)>,
}

impl Gate {
    /// Whether any way at all runs through it.
    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.links.is_empty()
    }
}

impl NavGrid {
    /// The gate of the box `lo`–`hi`: every link whose step crosses it
    /// (flat), from a floor within its height.
    pub fn gate(&self, lo: Vec3, hi: Vec3) -> Gate {
        let mut links = Vec::new();
        for a in 0..self.height.len() as u32 {
            let h = self.floor(a);
            if h < lo.y - 0.5 || h > hi.y {
                continue;
            }
            let (x, z) = self.col_of(a);
            let from = Vec2::new(self.coord(x), self.coord(z));
            for (k, (dx, dz)) in AROUND.iter().enumerate() {
                let b = self.links[a as usize][k];
                if b == NONE {
                    continue;
                }
                let to = from + Vec2::new(*dx as f64, *dz as f64) * super::CELL;
                if crosses(from, to, Vec2::new(lo.x, lo.z), Vec2::new(hi.x, hi.z)) {
                    links.push((a, k as u8, b));
                }
            }
        }
        Gate { links }
    }

    /// Shut `gate` (its links cut) or open it (put back).
    pub fn shut(&mut self, gate: &Gate, shut: bool) {
        for &(a, k, b) in &gate.links {
            self.links[a as usize][k as usize] = if shut { NONE } else { b };
        }
        self.regions = Regions::build(self);
    }
}

/// Whether the segment `a`–`b` passes through the box `lo`–`hi` (flat).
fn crosses(a: Vec2, b: Vec2, lo: Vec2, hi: Vec2) -> bool {
    let d = b - a;
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for (p, v, l, h) in [(a.x, d.x, lo.x, hi.x), (a.y, d.y, lo.y, hi.y)] {
        if v.abs() < 1e-12 {
            if p < l || p > h {
                return false;
            }
            continue;
        }
        let (mut s0, mut s1) = ((l - p) / v, (h - p) / v);
        if s0 > s1 {
            std::mem::swap(&mut s0, &mut s1);
        }
        t0 = t0.max(s0);
        t1 = t1.min(s1);
        if t0 > t1 {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collide::{Solids, box_tris};
    use crate::player::capsule;

    /// Two rooms side by side, a doorway between (x 0, z from -0.6 to
    /// 0.6), and no other way between them.
    fn two_rooms() -> Solids {
        let mut s = Solids::new();
        s.add(&box_tris(Vec3::new(-12.0, -1.0, -12.0), Vec3::new(12.0, 0.0, 12.0)));
        let wall = |s: &mut Solids, lo: Vec3, hi: Vec3| s.add(&box_tris(lo, hi));
        // The outer walls.
        wall(&mut s, Vec3::new(-8.5, 0.0, -6.5), Vec3::new(8.5, 3.0, -6.3));
        wall(&mut s, Vec3::new(-8.5, 0.0, 6.3), Vec3::new(8.5, 3.0, 6.5));
        wall(&mut s, Vec3::new(-8.5, 0.0, -6.5), Vec3::new(-8.3, 3.0, 6.5));
        wall(&mut s, Vec3::new(8.3, 0.0, -6.5), Vec3::new(8.5, 3.0, 6.5));
        // The wall between, a doorway in it.
        wall(&mut s, Vec3::new(-0.1, 0.0, -6.5), Vec3::new(0.1, 3.0, -0.6));
        wall(&mut s, Vec3::new(-0.1, 0.0, 0.6), Vec3::new(0.1, 3.0, 6.5));
        s
    }

    #[test]
    fn a_shut_gate_parts_the_rooms_and_an_open_one_joins_them() {
        let solids = two_rooms();
        let mut nav = NavGrid::build(&solids, capsule(false), 12.0);
        let (a, b) = (Vec3::new(-4.0, 0.0, 3.0), Vec3::new(4.0, 0.0, 3.0));
        assert!(nav.connects(a, b), "the doorway isn't walked through");
        let gate = nav.gate(Vec3::new(-0.2, 0.0, -0.6), Vec3::new(0.2, 2.1, 0.6));
        assert!(!gate.is_empty());
        nav.shut(&gate, true);
        assert!(!nav.connects(a, b), "through a shut door");
        assert!(nav.connects(a, Vec3::new(-6.0, 0.0, -4.0)), "the room itself is cut up");
        nav.shut(&gate, false);
        assert!(nav.connects(a, b), "the door opened and the way didn't");
        assert!(nav.path(a, b).is_some());
    }

    #[test]
    fn a_segment_crosses_a_box_only_through_it() {
        let (lo, hi) = (Vec2::new(-0.2, -0.6), Vec2::new(0.2, 0.6));
        assert!(crosses(Vec2::new(-1.0, 0.0), Vec2::new(1.0, 0.0), lo, hi));
        assert!(!crosses(Vec2::new(-1.0, 1.0), Vec2::new(1.0, 1.0), lo, hi));
        assert!(crosses(Vec2::new(-0.5, 0.5), Vec2::new(0.5, -0.5), lo, hi));
        assert!(!crosses(Vec2::new(-2.0, 0.0), Vec2::new(-1.0, 0.0), lo, hi));
    }
}
