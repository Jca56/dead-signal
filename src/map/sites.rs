//! Where things are: the highway's two ends on opposite edges, the town
//! on it near the middle, and the places out in the country (a gas
//! station on the highway, farms with their fields, a military camp, a
//! hunter's cabin, a crash in the woods, the old proving ground, and the
//! radio mast on the highest hill), each on a plot of its own, well apart.

use lntrn_math::Vec2;

use super::terrain::{Natural, Plot};
use super::{EDGE, EXTENT};
use crate::loot::Dice;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Town,
    Gas,
    Farm,
    Military,
    Cabin,
    Crash,
    Pad,
    Radio,
}

impl Kind {
    /// What the map calls it.
    pub fn label(self) -> &'static str {
        match self {
            Kind::Town => "TOWN",
            Kind::Gas => "GAS STATION",
            Kind::Farm => "FARM",
            Kind::Military => "MILITARY CAMP",
            Kind::Cabin => "HUNTER'S CABIN",
            Kind::Crash => "CRASH SITE",
            Kind::Pad => "PROVING GROUND",
            Kind::Radio => "RADIO TOWER",
        }
    }

    /// Its plot's size, half each way (across, along), and how far it
    /// eases back into the land.
    pub(crate) fn size(self) -> (Vec2, f64) {
        match self {
            Kind::Town => (Vec2::new(55.0, 85.0), 26.0),
            Kind::Gas => (Vec2::new(22.0, 16.0), 10.0),
            Kind::Farm => (Vec2::new(32.0, 28.0), 12.0),
            Kind::Military => (Vec2::new(30.0, 26.0), 12.0),
            Kind::Cabin => (Vec2::new(10.0, 9.0), 8.0),
            Kind::Crash => (Vec2::new(24.0, 18.0), 8.0),
            Kind::Pad => (Vec2::new(22.0, 26.0), 10.0),
            Kind::Radio => (Vec2::new(14.0, 14.0), 10.0),
        }
    }
}

/// The names a town can have.
const TOWNS: [&str; 8] = ["HOLLOW CREEK", "MILLBROOK", "ASHFORD", "PINE FALLS", "GREYWATER", "DUNMORE", "COLD SPRINGS", "HARLOW"];

#[derive(Clone, Debug)]
pub struct Site {
    pub kind: Kind,
    pub name: &'static str,
    pub plot: Plot,
}

impl Site {
    /// A site's plot, squared to the walking grid (so what's built on it
    /// lines up with where the dead can walk): turned a whole quarter,
    /// its middle on a whole metre.
    fn new(kind: Kind, centre: Vec2, yaw: f64) -> Self {
        let (half, shoulder) = kind.size();
        let quarter = (yaw / std::f64::consts::FRAC_PI_2).round();
        let centre = Vec2::new(centre.x.round(), centre.y.round());
        Self { kind, name: kind.label(), plot: Plot::new(centre, quarter * std::f64::consts::FRAC_PI_2, half, shoulder) }
    }

    /// Where its road comes to it: the middle of its front edge, and a
    /// little out from it.
    pub fn door(&self) -> Vec2 {
        self.plot.world(Vec2::new(0.0, -self.plot.half.y - 4.0))
    }
}

/// The plan before any road is laid: the highway's ends (where it leaves
/// at the edge, heading in), the town's main street (its ends), the
/// sites, which end is the way out, and a field or two by each farm.
pub struct Plan {
    pub ends: [(Vec2, Vec2); 2],
    pub street: (Vec2, Vec2),
    pub sites: Vec<Site>,
    pub fields: Vec<Plot>,
    /// Which of `ends` is the road out (the other is jammed with wrecks).
    pub out: usize,
}

/// A point on the edge of the land, `along` from the middle of side
/// `side` (0 +x, 1 -x, 2 +z, 3 -z), and which way is in.
fn edge_point(side: u32, along: f64) -> (Vec2, Vec2) {
    match side % 4 {
        0 => (Vec2::new(EXTENT, along), Vec2::new(-1.0, 0.0)),
        1 => (Vec2::new(-EXTENT, along), Vec2::new(1.0, 0.0)),
        2 => (Vec2::new(along, EXTENT), Vec2::new(0.0, -1.0)),
        _ => (Vec2::new(along, -EXTENT), Vec2::new(0.0, 1.0)),
    }
}

fn between(dice: &mut Dice, lo: f64, hi: f64) -> f64 {
    lo + (hi - lo) * dice.unit()
}

/// Lay the map out. `natural` is the land as it lies (the passes for the
/// highway are cut into it here); `highway` later says how far a point is
/// from the highway once it's laid: the places that keep off it are placed
/// by [`place_country`], after.
pub fn plan(dice: &mut Dice, natural: &mut Natural) -> Plan {
    let side = dice.next() % 4;
    let a = edge_point(side, between(dice, -170.0, 170.0));
    let b = edge_point(side ^ 1, between(dice, -170.0, 170.0));
    natural.pass(a.0, a.1);
    natural.pass(b.0, b.1);
    // The town straddles the highway near the middle, its main street
    // along it.
    let t = between(dice, 0.4, 0.6);
    let axis = b.0 - a.0;
    let dir = axis * (1.0 / axis.length());
    let across = Vec2::new(-dir.y, dir.x);
    let centre = a.0 + axis * t + across * between(dice, -40.0, 40.0);
    // The plot's frame: its "along" (local y) is the street.
    let yaw = dir.x.atan2(dir.y);
    let mut town = Site::new(Kind::Town, centre, yaw);
    town.name = TOWNS[dice.next() as usize % TOWNS.len()];
    let street = (town.plot.world(Vec2::new(0.0, -town.plot.half.y - 30.0)), town.plot.world(Vec2::new(0.0, town.plot.half.y + 30.0)));
    // Which way along the street is towards `a`: the street runs from its
    // end nearer `a`.
    let street = if (street.0 - a.0).length() < (street.1 - a.0).length() { street } else { (street.1, street.0) };
    Plan { ends: [a, b], street, sites: vec![town], fields: Vec::new(), out: (dice.next() % 2) as usize }
}

/// The gas station: beside the highway (`line`, its points), well out of
/// town, facing the road (where it runs near enough square to the grid
/// for a plot squared to it to face it).
pub fn place_gas(dice: &mut Dice, plan: &mut Plan, line: &[Vec2]) {
    let town = plan.sites[0].plot.centre;
    let square = |i: usize| {
        let d = line[i + 1] - line[i - 1];
        d.x.abs().min(d.y.abs()) < 0.3 * d.x.abs().max(d.y.abs())
    };
    let spots: Vec<usize> = (2..line.len().saturating_sub(2))
        .filter(|&i| {
            let d = (line[i] - town).length();
            d > 150.0 && d < 230.0 && line[i].x.abs().max(line[i].y.abs()) < EDGE - 50.0 && square(i)
        })
        .collect();
    if spots.is_empty() {
        return;
    }
    let i = spots[dice.next() as usize % spots.len()];
    let dir = (line[i + 1] - line[i - 1]) * (1.0 / (line[i + 1] - line[i - 1]).length().max(1e-9));
    let side = if dice.unit() < 0.5 { 1.0 } else { -1.0 };
    // Square to the grid: straight out from the road's nearest axis.
    let dir = if dir.x.abs() > dir.y.abs() { Vec2::new(dir.x.signum(), 0.0) } else { Vec2::new(0.0, dir.y.signum()) };
    let out = Vec2::new(-dir.y, dir.x) * side;
    let (half, _) = Kind::Gas.size();
    let centre = line[i] + out * (super::roads::Kind::Highway.half_width() + 3.0 + half.y);
    // Its front (-y in its frame) towards the road.
    let yaw = out.x.atan2(out.y);
    plan.sites.push(Site::new(Kind::Gas, centre, yaw));
}

/// The places out in the country, each clear of the others and of the
/// highway (`highway` says how far a point is from its middle), on ground
/// not too steep to level. The radio goes on the highest hill of those
/// looked at.
pub fn place_country(dice: &mut Dice, plan: &mut Plan, natural: &Natural, highway: &dyn Fn(Vec2) -> f64) {
    let room = EDGE - 45.0;
    let fits = |plan: &Plan, kind: Kind, c: Vec2| {
        let r = kind.size().0.length();
        c.x.abs() + r < room + 20.0
            && c.y.abs() + r < room + 20.0
            && highway(c) > r + 35.0
            && plan.sites.iter().all(|s| (s.plot.centre - c).length() > s.plot.reach() + r + 45.0)
            && plan.fields.iter().all(|f| (f.centre - c).length() > f.reach() + r + 10.0)
    };
    let steep = |c: Vec2, r: f64| {
        let hs: Vec<f64> = [(0.0, 0.0), (r, 0.0), (-r, 0.0), (0.0, r), (0.0, -r)].iter().map(|(dx, dz)| natural.height(c.x + dx, c.y + dz)).collect();
        hs.iter().cloned().fold(f64::MIN, f64::max) - hs.iter().cloned().fold(f64::MAX, f64::min)
    };
    // The radio first: the highest of many hills looked at.
    let mut best: Option<(Vec2, f64)> = None;
    for _ in 0..80 {
        let c = Vec2::new(between(dice, -room, room), between(dice, -room, room));
        if !fits(plan, Kind::Radio, c) || (c - plan.sites[0].plot.centre).length() < 150.0 {
            continue;
        }
        let h = natural.height(c.x, c.y);
        if best.is_none_or(|(_, b)| h > b) {
            best = Some((c, h));
        }
    }
    if let Some((c, _)) = best {
        plan.sites.push(Site::new(Kind::Radio, c, between(dice, 0.0, std::f64::consts::TAU)));
    }
    let wanted = [Kind::Military, Kind::Farm, Kind::Farm, Kind::Cabin, Kind::Crash, Kind::Pad];
    for kind in wanted {
        for attempt in 0..400 {
            let c = Vec2::new(between(dice, -room, room), between(dice, -room, room));
            let r = kind.size().0.length();
            // Out of town: the camp far out, the farms on gentle ground.
            let from_town = (c - plan.sites[0].plot.centre).length();
            let far_enough = match kind {
                Kind::Military => from_town > 190.0,
                _ => from_town > 130.0,
            };
            let limit = if kind == Kind::Farm { 5.0 } else { 9.0 } + f64::from(attempt / 100) * 3.0;
            if !far_enough || !fits(plan, kind, c) || steep(c, r) > limit {
                continue;
            }
            // Facing the highway, roughly: its front towards the middle of
            // the map, where the roads are.
            let to_mid = -c;
            let yaw = (-to_mid.x).atan2(-to_mid.y) + between(dice, -0.5, 0.5);
            let site = Site::new(kind, c, yaw);
            if kind == Kind::Farm {
                // A field each side, if there's room.
                for s in [-1.0, 1.0] {
                    let half = Vec2::new(between(dice, 20.0, 28.0), between(dice, 22.0, 34.0));
                    let fc = site.plot.world(Vec2::new(s * (site.plot.half.x + half.x + 4.0), between(dice, -8.0, 8.0)));
                    let field = Plot::new(fc, site.plot.yaw, half, 4.0);
                    let clear = highway(fc) > field.reach() + 12.0
                        && fc.x.abs().max(fc.y.abs()) + field.reach() < room + 30.0
                        && plan.sites.iter().all(|o| (o.plot.centre - fc).length() > o.plot.reach() + field.reach() + 12.0);
                    if clear {
                        plan.fields.push(field);
                    }
                }
            }
            plan.sites.push(site);
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_plan_puts_everything_apart_on_the_land() {
        for seed in 1..40u32 {
            let mut dice = Dice(seed);
            let mut natural = Natural::new(seed);
            let mut plan = plan(&mut dice, &mut natural);
            let (a, b) = (plan.ends[0].0, plan.ends[1].0);
            assert!((a - b).length() > 2.0 * EXTENT * 0.8, "the ends cross the map: {a:?} {b:?}");
            // A straight highway stands in for the real one.
            let highway = |p: Vec2| {
                let ab = b - a;
                let t = ((p - a).dot(ab) / ab.dot(ab)).clamp(0.0, 1.0);
                (a + ab * t - p).length()
            };
            place_country(&mut dice, &mut plan, &natural, &highway);
            let kinds: Vec<Kind> = plan.sites.iter().map(|s| s.kind).collect();
            assert!(kinds.contains(&Kind::Radio) && kinds.contains(&Kind::Military), "seed {seed}: {kinds:?}");
            assert!(kinds.len() >= 6, "seed {seed}: {kinds:?}");
            for (i, s) in plan.sites.iter().enumerate() {
                for o in &plan.sites[i + 1..] {
                    assert!((s.plot.centre - o.plot.centre).length() > s.plot.reach() + o.plot.reach(), "seed {seed}: {:?} and {:?} overlap", s.kind, o.kind);
                }
            }
        }
    }
}
