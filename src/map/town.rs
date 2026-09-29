//! The town: a grid of streets on its plot. The highway runs down its
//! middle as its main street; a back street runs each side of it, and
//! cross streets run the plot's width, out into the country. Lots are
//! packed along every street, each side, wherever one fits clear of the
//! streets and the lots already laid (the main street's first), each with
//! a building set back from its street, facing it. The landmarks
//! (`building/landmark.rs`) have their lots first: the gun store
//! downtown, the police station on the main street past it (its patrol
//! cars out front), the fire station and the school off the middle. Then
//! stores near the middle of the main street, houses round them (some of
//! two storeys, some boarded up), and a few lots lie empty. Cars are left
//! parked along the kerbs.
//!
//! The streets are plain data ([`Street`]), so a smaller place (a village
//! of one street and a crossing) can be laid out the same way.

use lntrn_math::Vec2;

use super::building::landmark::Landmark;
use super::building::{Building, plan};
use super::roads::Kind as RoadKind;
use super::sites::Site;
use crate::loot::Dice;
use crate::loot::tables::Source;

/// How far the lots stand back from a street's edge.
const VERGE: f64 = 2.5;
/// How deep a lot runs back from its street; how wide along it, fewest
/// and most metres; and the gap kept between two lots.
const LOT_DEPTH: f64 = 21.0;
const FRONTAGE: (f64, f64) = (13.0, 18.0);
const LOT_GAP: f64 = 1.0;
/// How far the back streets run from the main street's middle.
const BACK: f64 = 57.0;
/// Where the cross streets are along the main street, as shares of the
/// plot's half length.
const CROSSES: [f64; 4] = [-0.77, -0.26, 0.26, 0.77];
/// How much of the main street, out from the middle, has stores on it (a
/// share of the plot's half length).
const DOWNTOWN: f64 = 0.5;

/// A street, straight, in the town's frame (x across the main street, y
/// along it): along y (at x `at`) or along x (at y `at`), from `from` to
/// `to`; how wide, half; and whether it's the main street.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Street {
    pub along_y: bool,
    pub at: f64,
    pub from: f64,
    pub to: f64,
    pub half: f64,
    pub main: bool,
}

impl Street {
    /// The ground it covers and its verges, as a box (lo, hi).
    fn bounds(&self, verge: f64) -> (Vec2, Vec2) {
        let w = self.half + verge;
        if self.along_y { (Vec2::new(self.at - w, self.from), Vec2::new(self.at + w, self.to)) } else { (Vec2::new(self.from, self.at - w), Vec2::new(self.to, self.at + w)) }
    }

    /// How far `p` is from its middle line (as far as it runs).
    #[cfg(test)]
    pub fn distance(&self, p: Vec2) -> f64 {
        let (u, v) = if self.along_y { (p.y, p.x) } else { (p.x, p.y) };
        let along = (u - u.clamp(self.from, self.to)).abs();
        along.hypot(v - self.at)
    }
}

/// A lot: the middle of its front edge and which way is out to its street
/// (in the town's frame), how wide along the street and how deep, whether
/// it's on the main street (and near the middle), and the landmark it's
/// for, if it is.
struct Lot {
    front: Vec2,
    out: Vec2,
    frontage: f64,
    depth: f64,
    main: bool,
    downtown: bool,
    landmark: Option<Landmark>,
}

impl Lot {
    /// The ground it covers, as a box (lo, hi).
    fn bounds(&self) -> (Vec2, Vec2) {
        let along = Vec2::new(self.out.y.abs(), self.out.x.abs()) * (self.frontage * 0.5);
        let back = self.front - self.out * self.depth;
        let (a, b) = (self.front - along, back + along);
        (a.min(b), a.max(b))
    }
}

/// The town laid out: its streets but the main one (each a line, flat, on
/// the map), its buildings, cars parked (each a spot), and where each of
/// its landmarks stands (the middle of its floor, on the map).
pub struct Town {
    pub streets: Vec<Vec<Vec2>>,
    pub buildings: Vec<Building>,
    pub cars: Vec<(Source, super::Spot)>,
    pub landmarks: Vec<(Landmark, Vec2)>,
}

fn between(dice: &mut Dice, lo: f64, hi: f64) -> f64 {
    lo + (hi - lo) * dice.unit()
}

fn overlaps((alo, ahi): (Vec2, Vec2), (blo, bhi): (Vec2, Vec2), gap: f64) -> bool {
    alo.x < bhi.x + gap && blo.x < ahi.x + gap && alo.y < bhi.y + gap && blo.y < ahi.y + gap
}

/// The town's streets on a plot `half` across and along: the main street
/// its length, a back street each side between the outermost cross
/// streets, the cross streets its width.
pub fn grid(half: Vec2) -> Vec<Street> {
    let (hx, hy) = (half.x, half.y);
    let (main, paved) = (RoadKind::Highway.half_width(), RoadKind::Paved.half_width());
    let ys: Vec<f64> = CROSSES.iter().map(|k| (k * hy).round()).collect();
    let (first, last) = (ys[0], ys[ys.len() - 1]);
    let mut out = vec![Street { along_y: true, at: 0.0, from: -hy, to: hy, half: main, main: true }];
    for x in [-BACK, BACK] {
        out.push(Street { along_y: true, at: x, from: first, to: last, half: paved, main: false });
    }
    for &y in &ys {
        out.push(Street { along_y: false, at: y, from: -hx + 2.0, to: hx - 2.0, half: paved, main: false });
    }
    out
}

/// Where a landmark may stand: on which streets (by their place in the
/// grid), and how far from the middle of the town along them, least and
/// most (shares of the plot's half length).
fn wanted(landmark: Landmark) -> (&'static [usize], (f64, f64)) {
    match landmark {
        // Downtown on the main street.
        Landmark::GunStore => (&[0], (0.0, DOWNTOWN)),
        // On the main street past downtown, where it's houses.
        Landmark::Police => (&[0], (DOWNTOWN, 0.95)),
        // Off the middle: on a back street, or out on a cross street.
        Landmark::FireStation => (&[1, 2, 3, 6], (0.0, 0.8)),
        Landmark::School => (&[1, 2], (0.0, 0.6)),
    }
}

/// How far a landmark stands back from its street.
const LANDMARK_SETBACK: f64 = 2.0;

/// Whether `lot` fits on the plot (`half`), clear of every street and of
/// every lot in `laid`.
fn fits(lot: &Lot, streets: &[Street], half: Vec2, laid: &[Lot]) -> bool {
    let (lo, hi) = lot.bounds();
    let on_plot = lo.x > -half.x + 2.0 && hi.x < half.x - 2.0 && lo.y > -half.y + 2.0 && hi.y < half.y - 2.0;
    on_plot && streets.iter().all(|t| !overlaps((lo, hi), t.bounds(VERGE - 0.01), 0.0)) && laid.iter().all(|o| !overlaps((lo, hi), o.bounds(), LOT_GAP))
}

/// Lots: first each landmark's (somewhere it may stand, as it fits), then
/// packed along each of `streets`, each side, in turn, wherever one fits.
fn lots(dice: &mut Dice, streets: &[Street], half: Vec2) -> Vec<Lot> {
    let hy = half.y;
    let mut out: Vec<Lot> = Vec::new();
    let side_of = |s: &Street, side: f64| (if s.along_y { Vec2::new(-side, 0.0) } else { Vec2::new(0.0, -side) }, s.at + side * (s.half + VERGE));
    for landmark in Landmark::ALL {
        let (on, (near, far)) = wanted(landmark);
        let (w, d) = landmark.size();
        let (frontage, depth) = (f64::from(w) + 2.0, f64::from(d) + LANDMARK_SETBACK + 1.0);
        for _ in 0..400 {
            let s = &streets[on[dice.next() as usize % on.len()]];
            let side = if dice.unit() < 0.5 { -1.0 } else { 1.0 };
            let (out_dir, edge) = side_of(s, side);
            let off = between(dice, near * hy, far * hy) * if dice.unit() < 0.5 { -1.0 } else { 1.0 };
            let mid = off.clamp(s.from, s.to).round();
            let front = if s.along_y { Vec2::new(edge, mid) } else { Vec2::new(mid, edge) };
            let lot = Lot { front, out: out_dir, frontage, depth, main: s.main, downtown: false, landmark: Some(landmark) };
            if fits(&lot, streets, half, &out) {
                out.push(lot);
                break;
            }
        }
    }
    for s in streets {
        for side in [-1.0, 1.0] {
            let (out_dir, edge) = side_of(s, side);
            let mut u = s.from;
            while u < s.to {
                let frontage = between(dice, FRONTAGE.0, FRONTAGE.1).round();
                let mid = u + frontage * 0.5;
                let front = if s.along_y { Vec2::new(edge, mid) } else { Vec2::new(mid, edge) };
                let lot = Lot { front, out: out_dir, frontage, depth: LOT_DEPTH, main: s.main, downtown: s.main && mid.abs() < DOWNTOWN * hy, landmark: None };
                if fits(&lot, streets, half, &out) {
                    out.push(lot);
                    u += frontage;
                } else {
                    u += 1.0;
                }
            }
        }
    }
    out
}

/// Lay out the town on `site`'s plot.
pub fn lay_out(dice: &mut Dice, site: &Site) -> Town {
    let plot = &site.plot;
    let grid = grid(plot.half);
    let streets: Vec<Vec<Vec2>> = grid
        .iter()
        .filter(|s| !s.main)
        .map(|s| {
            let (a, b) = if s.along_y { (Vec2::new(s.at, s.from), Vec2::new(s.at, s.to)) } else { (Vec2::new(s.from, s.at), Vec2::new(s.to, s.at)) };
            super::roads::resample(&[plot.world(a), plot.world(b)], super::roads::SPACING)
        })
        .collect();

    // A building on most lots: fewer off the main street, and more of
    // those boarded up.
    let mut buildings = Vec::new();
    let mut cars = Vec::new();
    let mut landmarks = Vec::new();
    for lot in lots(dice, &grid, plot.half) {
        if let Some(landmark) = lot.landmark {
            let seed = dice.next();
            let plan = landmark.plan(&mut Dice(seed | 1));
            let (w, d) = (f64::from(plan.w), f64::from(plan.d));
            let b = Building::facing(plan, plot, lot.front - lot.out * LANDMARK_SETBACK, lot.out, seed);
            let middle = b.world(lntrn_math::Vec3::new(w * 0.5, 0.0, d * 0.5));
            landmarks.push((landmark, Vec2::new(middle.x, middle.z)));
            buildings.push(b);
            // The patrol cars out front of the station, at the kerb.
            if landmark == Landmark::Police {
                let along = Vec2::new(lot.out.y.abs(), lot.out.x.abs());
                let kerb = lot.front + lot.out * (VERGE + 1.0);
                for k in [-1.0, 1.0] {
                    if k > 0.0 && dice.unit() < 0.4 {
                        continue;
                    }
                    let at = plot.world(kerb + along * (k * (3.0 + dice.unit() * 2.0)));
                    let dir = plot.world(along) - plot.centre;
                    let yaw = (-dir.y).atan2(dir.x) + (dice.unit() - 0.5) * 0.2;
                    cars.push((Source::CopCar, (at.x, at.y, yaw, plot.height + 3.0)));
                }
            }
            continue;
        }
        let (empty, shells) = if lot.main { (0.08, 0.1) } else { (0.15, 0.18) };
        if dice.unit() < empty {
            continue;
        }
        let room = lot.frontage - 2.0;
        let store = lot.downtown && dice.unit() < 0.85 && room >= 10.0;
        let (w, d, setback) = if store { (room.min(16.0) as i32, between(dice, 12.0, 15.0) as i32, 1.0) } else { (between(dice, 8.0, room.min(12.0)) as i32, between(dice, 8.0, 12.0) as i32, between(dice, 3.0, 5.0)) };
        let seed = dice.next();
        let mut own = Dice(seed | 1);
        let plan = if store {
            plan::store(&mut own, w, d)
        } else if dice.unit() < shells {
            plan::shell(&mut own, w, d)
        } else {
            plan::house(&mut own, w, d, dice.unit() < 0.38)
        };
        // Its front faces the street, set back from it.
        buildings.push(Building::facing(plan, plot, lot.front - lot.out * setback, lot.out, seed));
    }

    // Cars left at the kerbs, a couple down each street but the main one.
    for s in grid.iter().filter(|s| !s.main) {
        for _ in 0..2 {
            let u = between(dice, s.from + 8.0, s.to - 8.0);
            let kerb = s.at + (s.half - 1.0) * if dice.unit() < 0.5 { -1.0 } else { 1.0 };
            // Clear of the crossings.
            if grid.iter().any(|o| o.along_y != s.along_y && (o.at - u).abs() < o.half + 6.0) {
                continue;
            }
            let (l, dir) = if s.along_y { (Vec2::new(kerb, u), Vec2::new(0.0, 1.0)) } else { (Vec2::new(u, kerb), Vec2::new(1.0, 0.0)) };
            let at = plot.world(l);
            let along = plot.world(dir) - plot.centre;
            let yaw = (-along.y).atan2(along.x) + (dice.unit() - 0.5) * 0.3;
            cars.push((Source::Car, (at.x, at.y, yaw, plot.height + 3.0)));
        }
    }
    Town { streets, buildings, cars, landmarks }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::sites::Kind;
    use crate::map::terrain::Plot;
    use lntrn_math::Vec3;

    #[test]
    fn the_town_is_built_up_and_no_two_buildings_meet() {
        let (half, shoulder) = Kind::Town.size();
        for seed in 1..20u32 {
            for quarter in 0..4 {
                let mut dice = Dice(seed * 31 + quarter);
                let mut plot = Plot::new(Vec2::new(40.0, -12.0), f64::from(quarter) * std::f64::consts::FRAC_PI_2, half, shoulder);
                plot.height = 3.0;
                let site = Site { kind: Kind::Town, name: "TEST", plot };
                let town = lay_out(&mut dice, &site);
                let streets = grid(half);
                assert!(town.buildings.len() >= 45, "seed {seed}: {} buildings", town.buildings.len());
                assert!(town.buildings.iter().filter(|b| matches!(b.plan.kind, plan::Kind::Store | plan::Kind::GunStore)).count() >= 4, "seed {seed}: too few stores");
                for landmark in Landmark::ALL {
                    assert_eq!(town.landmarks.iter().filter(|(l, _)| *l == landmark).count(), 1, "seed {seed}: no {landmark:?}");
                }
                assert!(town.cars.iter().any(|(s, _)| *s == Source::CopCar), "seed {seed}: no patrol car");
                assert_eq!(town.streets.len(), streets.len() - 1, "every street but the main one laid as a road");
                for (i, a) in town.buildings.iter().enumerate() {
                    // Within the plot, off every street.
                    for c in a.footprint() {
                        assert!(site.plot.outside(c) < 0.01, "seed {seed}: a building off the plot at {c:?}");
                        let l = site.plot.local(c);
                        for s in &streets {
                            let (lo, hi) = s.bounds(0.5);
                            assert!(!(l.x > lo.x && l.x < hi.x && l.y > lo.y && l.y < hi.y), "seed {seed}: a building on a street at {l:?}");
                        }
                    }
                    for b in &town.buildings[i + 1..] {
                        let (fa, fb) = (a.footprint(), b.footprint());
                        let bounds = |f: [Vec2; 4]| f.iter().fold((Vec2::splat(f64::INFINITY), Vec2::splat(f64::NEG_INFINITY)), |(lo, hi), c| (lo.min(*c), hi.max(*c)));
                        assert!(!overlaps(bounds(fa), bounds(fb), -0.01), "seed {seed}: two buildings meet: {fa:?} {fb:?}");
                    }
                    // Its front faces the nearest street.
                    let front = a.world(Vec3::new(f64::from(a.plan.w) * 0.5, 0.0, -1.0));
                    let back = a.world(Vec3::new(f64::from(a.plan.w) * 0.5, 0.0, f64::from(a.plan.d) + 1.0));
                    let to_street = |p: Vec3| {
                        let l = site.plot.local(Vec2::new(p.x, p.z));
                        streets.iter().map(|s| s.distance(l) - s.half).fold(f64::INFINITY, f64::min)
                    };
                    assert!(to_street(front) < to_street(back), "seed {seed}: a building turned its back on the street");
                }
            }
        }
    }
}
