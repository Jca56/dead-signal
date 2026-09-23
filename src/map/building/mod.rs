//! Buildings: a floor plan (`plan.rs`, and out in the country
//! `country.rs`), the blocks it's built of (`shape.rs`), and what's put in
//! its rooms (`furnish.rs`); each set down on the map square to the
//! walking grid, turned a whole quarter.

pub mod country;
pub mod furnish;
pub mod plan;
pub mod shape;

use lntrn_math::{Vec2, Vec3};

use plan::Plan;

use super::terrain::Plot;

/// A building on the map: its plan, where its front left corner stands
/// (on its ground floor, on a half metre each way so its walls fall
/// between the walking grid's points), which quarter it's turned, and the
/// luck its colours and furnishings are drawn by.
#[derive(Clone, Debug)]
pub struct Building {
    pub plan: Plan,
    pub origin: Vec3,
    pub quarter: u8,
    pub seed: u32,
}

impl Building {
    /// `plan` set down on `plot`, the middle of its front at `front` (in
    /// the plot's frame: x across, y along) and facing `out` (square to
    /// that frame); its corner on a half metre, its floor raised off the
    /// plot's level.
    pub fn facing(plan: Plan, plot: &Plot, front: Vec2, out: Vec2, seed: u32) -> Self {
        let theta = (-out.x).atan2(-out.y);
        let quarter = ((plot.yaw + theta) / std::f64::consts::FRAC_PI_2).round().rem_euclid(4.0) as u8;
        let across = Vec2::new(theta.cos(), -theta.sin());
        let corner = plot.world(front - across * (f64::from(plan.w) * 0.5));
        let origin = Vec3::new(corner.x.floor() + 0.5, plot.height + shape::RAISED, corner.y.floor() + 0.5);
        Self { plan, origin, quarter, seed }
    }

    /// Which way it's turned, as a yaw.
    #[cfg(test)]
    pub fn yaw(&self) -> f64 {
        f64::from(self.quarter) * std::f64::consts::FRAC_PI_2
    }

    /// A direction in its frame, turned to the map's (exactly: a whole
    /// quarter).
    pub fn turn(&self, v: Vec3) -> Vec3 {
        match self.quarter % 4 {
            0 => v,
            1 => Vec3::new(v.z, v.y, -v.x),
            2 => Vec3::new(-v.x, v.y, -v.z),
            _ => Vec3::new(-v.z, v.y, v.x),
        }
    }

    /// A point in its frame, on the map.
    pub fn world(&self, p: Vec3) -> Vec3 {
        self.origin + self.turn(p)
    }

    /// Its corners on the map, flat.
    pub fn footprint(&self) -> [Vec2; 4] {
        let (w, d) = (f64::from(self.plan.w), f64::from(self.plan.d));
        [(0.0, 0.0), (w, 0.0), (w, d), (0.0, d)].map(|(x, z)| {
            let p = self.world(Vec3::new(x, 0.0, z));
            Vec2::new(p.x, p.z)
        })
    }

    /// Whether `p` (flat, on the map) is within its walls, and `margin`
    /// more round them.
    #[cfg(test)]
    pub fn covers(&self, p: Vec2, margin: f64) -> bool {
        let f = self.footprint();
        let (lo, hi) = f.iter().fold((Vec2::splat(f64::INFINITY), Vec2::splat(f64::NEG_INFINITY)), |(lo, hi), c| (lo.min(*c), hi.max(*c)));
        p.x >= lo.x - margin && p.x <= hi.x + margin && p.y >= lo.y - margin && p.y <= hi.y + margin
    }
}

#[cfg(test)]
mod plan_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loot::Dice;

    #[test]
    fn its_frame_turns_by_whole_quarters_as_a_yaw_does() {
        let plan = plan::house(&mut Dice(3), 9, 8, false);
        for quarter in 0..4u8 {
            let b = Building { plan: plan.clone(), origin: Vec3::new(10.5, 2.0, -4.5), quarter, seed: 1 };
            let turned = lntrn_math::Mat4::from_quat(lntrn_math::Quat::from_rotation_y(b.yaw()));
            for v in [Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 1.0), Vec3::new(2.0, 1.0, -3.0)] {
                assert!((b.turn(v) - turned.transform_vector(v)).length() < 1e-9, "quarter {quarter}");
            }
            // Its corners land on half metres.
            for c in b.footprint() {
                assert_eq!((c.x.fract().abs(), c.y.fract().abs()), (0.5, 0.5), "quarter {quarter}: {c:?}");
            }
        }
    }
}
