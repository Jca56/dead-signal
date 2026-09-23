//! The town: the highway down its middle as its main street, two or three
//! cross streets, and lots along them, each with a building set back from
//! its street, facing it. Stores stand near the middle of the main street,
//! houses round them (some of two storeys, some boarded up), and a few
//! lots lie empty. Cars are left parked along the kerbs.

use lntrn_math::Vec2;

use super::building::{Building, plan};
use super::roads::Kind as RoadKind;
use super::sites::Site;
use crate::loot::Dice;
use crate::loot::tables::Source;

/// How far the lots stand back from a street's edge.
const VERGE: f64 = 2.5;
/// How deep a lot runs back from its street.
const LOT_DEPTH: f64 = 21.0;

/// A lot: the middle of its front edge and which way is out to its street
/// (in the town's frame: x across the main street, y along it), how wide
/// along the street, and whether it's on the main street near the middle.
struct Lot {
    front: Vec2,
    out: Vec2,
    frontage: f64,
    central: bool,
}

/// The town laid out: its cross streets (each a line, flat, on the map),
/// its buildings, and cars parked (each a spot).
pub struct Town {
    pub streets: Vec<Vec<Vec2>>,
    pub buildings: Vec<Building>,
    pub cars: Vec<(Source, super::Spot)>,
}

fn between(dice: &mut Dice, lo: f64, hi: f64) -> f64 {
    lo + (hi - lo) * dice.unit()
}

/// Lay out the town on `site`'s plot.
pub fn lay_out(dice: &mut Dice, site: &Site) -> Town {
    let plot = &site.plot;
    let (hx, hy) = (plot.half.x, plot.half.y);
    let main = RoadKind::Highway.half_width();
    let cross = RoadKind::Paved.half_width();
    // The cross streets: two or three, spread along the main street.
    let count = if dice.unit() < 0.5 { 2 } else { 3 };
    let ys: Vec<f64> = match count {
        2 => {
            let y = between(dice, 28.0, 38.0).round();
            vec![-y, y]
        }
        _ => vec![-(hy * 0.62).round(), 0.0, (hy * 0.62).round()],
    };
    let streets: Vec<Vec<Vec2>> = ys.iter().map(|&y| super::roads::resample(&[plot.world(Vec2::new(-hx + 4.0, y)), plot.world(Vec2::new(hx - 4.0, y))], super::roads::SPACING)).collect();

    // The lots down both sides of the main street, between the cross
    // streets.
    let mut lots = Vec::new();
    let mut edges: Vec<f64> = vec![-hy + 4.0];
    for &y in &ys {
        edges.push(y - cross - VERGE);
        edges.push(y + cross + VERGE);
    }
    edges.push(hy - 4.0);
    for side in [-1.0, 1.0] {
        for pair in edges.chunks(2) {
            let (y0, y1) = (pair[0], pair[1]);
            let mut y = y0;
            while y1 - y >= 12.0 {
                let frontage = between(dice, 13.0, 18.0).round().min(y1 - y);
                let front = Vec2::new(side * (main + VERGE), y + frontage * 0.5);
                lots.push(Lot { front, out: Vec2::new(-side, 0.0), frontage, central: (y + frontage * 0.5).abs() < 50.0 });
                y += frontage;
            }
        }
    }
    // Beyond them, a lot each side of each cross street on each side of
    // the main street.
    let inner = main + VERGE + LOT_DEPTH + 3.0;
    for &y in &ys {
        for side in [-1.0, 1.0] {
            for up in [-1.0, 1.0] {
                let frontage = (hx - 3.0 - inner).min(20.0);
                if frontage < 12.0 {
                    continue;
                }
                let x = side * (inner + frontage * 0.5);
                let front_y = y + up * (cross + VERGE);
                // Clear of the plot's ends and the next cross street's lots.
                let back_y = front_y + up * LOT_DEPTH;
                if back_y.abs() > hy - 2.0 || ys.iter().any(|&o| o != y && (o - back_y).abs() < cross + VERGE) {
                    continue;
                }
                lots.push(Lot { front: Vec2::new(x, front_y), out: Vec2::new(0.0, -up), frontage, central: false });
            }
        }
    }

    // A building on most lots.
    let mut buildings = Vec::new();
    for lot in &lots {
        if dice.unit() < 0.12 {
            continue;
        }
        let room = lot.frontage - 2.0;
        let store = lot.central && dice.unit() < 0.65 && room >= 10.0;
        let (w, d, setback) = if store { (room.min(16.0) as i32, between(dice, 12.0, 15.0) as i32, 1.0) } else { (between(dice, 8.0, room.min(12.0)) as i32, between(dice, 8.0, 12.0) as i32, between(dice, 3.0, 5.0)) };
        let seed = dice.next();
        let mut own = Dice(seed | 1);
        let plan = if store {
            plan::store(&mut own, w, d)
        } else if dice.unit() < 0.14 {
            plan::shell(&mut own, w, d)
        } else {
            plan::house(&mut own, w, d, dice.unit() < 0.38)
        };
        // Its front faces the street, set back from it.
        buildings.push(Building::facing(plan, plot, lot.front - lot.out * setback, lot.out, seed));
    }

    // Cars left at the kerbs.
    let mut cars = Vec::new();
    for &y in &ys {
        for _ in 0..2 {
            let x = between(dice, main + 8.0, hx - 8.0) * if dice.unit() < 0.5 { -1.0 } else { 1.0 };
            let kerb = y + (cross - 1.0) * if dice.unit() < 0.5 { -1.0 } else { 1.0 };
            let at = plot.world(Vec2::new(x, kerb));
            let along = plot.world(Vec2::new(1.0, 0.0)) - plot.centre;
            let yaw = (-along.y).atan2(along.x) + (dice.unit() - 0.5) * 0.3;
            cars.push((Source::Car, (at.x, at.y, yaw, plot.height + 3.0)));
        }
    }
    Town { streets, buildings, cars }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lntrn_math::Vec3;
    use crate::map::sites::Kind;
    use crate::map::terrain::Plot;

    #[test]
    fn the_town_is_built_up_and_no_two_buildings_meet() {
        for seed in 1..30u32 {
            for quarter in 0..4 {
                let mut dice = Dice(seed * 31 + quarter);
                let mut plot = Plot::new(Vec2::new(40.0, -12.0), f64::from(quarter) * std::f64::consts::FRAC_PI_2, Vec2::new(55.0, 85.0), 26.0);
                plot.height = 3.0;
                let site = Site { kind: Kind::Town, name: "TEST", plot };
                let town = lay_out(&mut dice, &site);
                assert!(town.buildings.len() >= 14, "seed {seed}: {} buildings", town.buildings.len());
                assert!(town.buildings.iter().any(|b| b.plan.kind == plan::Kind::Store), "seed {seed}: no stores");
                for (i, a) in town.buildings.iter().enumerate() {
                    // Within the plot, off the streets.
                    for c in a.footprint() {
                        assert!(site.plot.outside(c) < 0.01, "seed {seed}: a building off the plot at {c:?}");
                        let l = site.plot.local(c);
                        assert!(l.x.abs() > RoadKind::Highway.half_width() + 0.5, "seed {seed}: on the main street at {l:?}");
                    }
                    for b in &town.buildings[i + 1..] {
                        let (fa, fb) = (a.footprint(), b.footprint());
                        let bounds = |f: [Vec2; 4]| f.iter().fold((Vec2::splat(f64::INFINITY), Vec2::splat(f64::NEG_INFINITY)), |(lo, hi), c| (lo.min(*c), hi.max(*c)));
                        let ((alo, ahi), (blo, bhi)) = (bounds(fa), bounds(fb));
                        let apart = ahi.x <= blo.x + 0.01 || bhi.x <= alo.x + 0.01 || ahi.y <= blo.y + 0.01 || bhi.y <= alo.y + 0.01;
                        assert!(apart, "seed {seed}: two buildings meet: {fa:?} {fb:?}");
                    }
                    // Its front faces the nearest street.
                    let front = a.world(Vec3::new(f64::from(a.plan.w) * 0.5, 0.0, -1.0));
                    let back = a.world(Vec3::new(f64::from(a.plan.w) * 0.5, 0.0, f64::from(a.plan.d) + 1.0));
                    let to_street = |p: Vec3| {
                        let l = site.plot.local(Vec2::new(p.x, p.z));
                        let cross = town.streets.iter().map(|s| (site.plot.local(s[0]).y - l.y).abs()).fold(f64::INFINITY, f64::min);
                        l.x.abs().min(cross)
                    };
                    assert!(to_street(front) < to_street(back), "seed {seed}: a building turned its back on the street");
                }
            }
        }
    }
}
