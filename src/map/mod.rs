//! The map a run is played on, made afresh from a seed: the land
//! (`terrain.rs`), the places on it (`sites.rs`), the roads between them
//! (`roads.rs`), the forest and the things lying about (`scatter.rs`).
//! [`generate`] lays it all out as plain data; `build.rs` makes it solid
//! and drawable, off the main thread, while the loading screen shows.

pub mod build;
pub mod building;
pub mod homestead;
pub mod noise;
pub mod roads;
pub mod scatter;
pub mod screen;
pub mod sites;
pub mod terrain;
pub mod town;

use lntrn_math::{Vec2, Vec3};

use crate::exits::{self, Way};
use crate::loot::tables::Source;
use crate::loot::Dice;
use roads::{Network, Road};
use scatter::Piece;
use sites::Site;
use terrain::{Field, Natural, Plot, Shaped};

/// How far one can go from the middle of the map, each way.
pub const HALF: f64 = 300.0;
/// How far the land is drawn (the ridges past the edge, into the fog).
pub const EXTENT: f64 = 360.0;
/// Where the land starts rising into the ridges that close the map in.
pub const EDGE: f64 = 272.0;

/// Where a thing is set down: game x and z, which way it's turned, and a
/// height to look down from for its floor.
pub type Spot = (f64, f64, f64, f64);

pub struct Map {
    pub seed: u32,
    pub field: Field,
    pub roads: Vec<Road>,
    pub sites: Vec<Site>,
    pub fields: Vec<Plot>,
    /// Where the player starts, and which way they face.
    pub spawn: (Vec3, f64),
    pub exits: Vec<exits::Spot>,
    pub containers: Vec<(Source, Spot)>,
    pub pickups: Vec<crate::items::Spot>,
    pub scenery: Vec<Piece>,
    pub buildings: Vec<building::Building>,
    /// How thick the forest grows where.
    pub forest: scatter::Forest,
}

impl Map {
    /// What the truck is out by, in words (for the radio).
    pub fn truck_near(&self) -> &'static str {
        let Some(&(_, (x, z, _, _), _, _)) = self.exits.iter().find(|e| e.0 == Way::Truck) else { return "road" };
        let at = Vec2::new(x, z);
        self.sites.iter().find(|s| s.plot.outside(at) < 2.0).map_or("road", |s| match s.kind {
            sites::Kind::Farm => "farm",
            sites::Kind::Gas => "gas station",
            sites::Kind::Military => "army camp",
            _ => "road",
        })
    }
}

/// Make the map for `seed`.
pub fn generate(seed: u32) -> Map {
    let mut dice = Dice(seed | 1);
    let mut natural = Natural::new(seed);
    let mut plan = sites::plan(&mut dice, &mut natural);
    terrain::level(&mut plan.sites[0].plot, &natural);

    // The highway: in from one edge, down the town's main street, out at
    // the other.
    let highway = {
        let town = std::slice::from_ref(&plan.sites[0].plot);
        let shaped = Shaped { natural: &natural, plots: town };
        let height = |x: f64, z: f64| shaped.height(x, z);
        let blocked = |p: Vec2| town[0].outside(p) < 6.0;
        let (a, b) = (plan.ends[0].0, plan.ends[1].0);
        let mut line = roads::route(&height, &blocked, a, plan.street.0);
        line.extend(roads::resample(&[plan.street.0, plan.street.1], roads::SPACING).into_iter().skip(1));
        line.extend(roads::route(&height, &blocked, plan.street.1, b).into_iter().skip(1));
        roads::resample(&roads::curve(&line, 2), roads::SPACING)
    };
    sites::place_gas(&mut dice, &mut plan, &highway);
    let from_highway = |p: Vec2| highway.windows(2).map(|w| distance_to_segment(p, w[0], w[1])).fold(f64::INFINITY, f64::min);
    sites::place_country(&mut dice, &mut plan, &natural, &from_highway);
    for site in plan.sites.iter_mut().skip(1) {
        terrain::level(&mut site.plot, &natural);
    }
    let plots: Vec<Plot> = plan.sites.iter().map(|s| s.plot.clone()).collect();
    let shaped = Shaped { natural: &natural, plots: &plots };
    let in_plot = |p: Vec2| plots.iter().map(|pl| (pl.weight(p), pl.height)).filter(|(w, _)| *w > 0.0).max_by(|a, b| a.0.total_cmp(&b.0));

    // The town's cross streets; then the side roads, each from the nearest
    // road already laid to its place's door (the crash has none; the gas
    // station is on the highway).
    let mut laid = vec![Road { kind: roads::Kind::Highway, points: roads::profile(roads::Kind::Highway, &highway, |x, z| shaped.height(x, z), in_plot, None) }];
    let town = town::lay_out(&mut dice, &plan.sites[0]);
    let mut buildings = town.buildings;
    for site in &plan.sites {
        buildings.extend(homestead::lay_out(&mut dice, site));
    }
    for line in &town.streets {
        let points = roads::profile(roads::Kind::Paved, line, |x, z| shaped.height(x, z), in_plot, Some(&Network::new(laid.clone())));
        laid.push(Road { kind: roads::Kind::Paved, points });
    }
    for site in &plan.sites {
        let kind = match site.kind {
            sites::Kind::Town | sites::Kind::Gas | sites::Kind::Crash => continue,
            sites::Kind::Military => roads::Kind::Paved,
            _ => roads::Kind::Dirt,
        };
        let door = site.door();
        let Some(from) = nearest_on(&laid, door) else { continue };
        let blocked = |p: Vec2| plots.iter().any(|pl| pl.outside(p) < 3.0) || plan.fields.iter().any(|f| f.outside(p) < 2.0);
        let line = roads::route(&|x, z| shaped.height(x, z), &blocked, from, door);
        // As far as its shoulder reaches into a plot, the road comes up (or
        // down) to the plot's level first: the land it claims round its end
        // is then the plot's own, and stays flat for what's built on it.
        // (Drawn to it, never pinned: a short road may have to climb
        // gently where it can't do both.)
        let reach = kind.half_width() + roads::FLAT + roads::SHOULDER;
        let at_level = |p: Vec2| plots.iter().map(|pl| ((1.0 - terrain::smooth((pl.outside(p) - reach) / pl.shoulder.max(1e-6))).min(0.98), pl.height)).filter(|(w, _)| *w > 0.0).max_by(|a, b| a.0.total_cmp(&b.0));
        let points = roads::profile(kind, &line, |x, z| shaped.height(x, z), at_level, Some(&Network::new(laid.clone())));
        laid.push(Road { kind, points });
    }
    let network = Network::new(laid);
    let height = |x: f64, z: f64| {
        let h = shaped.height(x, z);
        match network.claim(Vec2::new(x, z)) {
            Some((_, _, road, w)) => h + (road - roads::SINK - h) * w,
            None => h,
        }
    };
    let field = Field::new(seed, height);
    let forest = scatter::Forest::new(seed);

    // The ways out, then the player as far from them all as can be.
    let mut exits = Vec::new();
    let hw = &network.roads[0];
    let out_end = plan.out;
    let order: Vec<usize> = if out_end == 0 { (0..hw.points.len()).collect() } else { (0..hw.points.len()).rev().collect() };
    if let Some(&i) = order.iter().find(|&&i| hw.points[i].x.abs().max(hw.points[i].z.abs()) <= HALF - 32.0) {
        let p = hw.points[i];
        // Heading out: towards the end it leaves by.
        let h = hw.heading(i) * if out_end == 0 { -1.0 } else { 1.0 };
        let yaw = h.x.atan2(h.y);
        exits.push((Way::Road, (p.x, p.z, yaw, p.y + 6.0), Vec3::new(0.0, 0.0, 7.0), 3.5));
    }
    let mut scenery = Vec::new();
    if let Some(radio) = plan.sites.iter().find(|s| s.kind == sites::Kind::Radio) {
        let c = radio.plot.centre;
        let y = field.height_at(c.x, c.y).unwrap_or(radio.plot.height);
        scenery.push(Piece::new(scatter::Scenery::Tower, Vec3::new(c.x, y, c.y), radio.plot.yaw, 1.0));
        scenery.push(Piece::new(scatter::Scenery::Beacon, Vec3::new(c.x, y, c.y), radio.plot.yaw, 1.0));
        let front = radio.plot.world(Vec2::new(0.0, -1.0)) - c;
        let at = c + front * 6.0;
        let yaw = (-front.x).atan2(-front.y);
        exits.push((Way::Radio, (at.x, at.y, yaw, y + 3.0), Vec3::new(1.0, 0.0, -4.0), 11.0));
    }
    let truck_site = [sites::Kind::Farm, sites::Kind::Gas, sites::Kind::Military].iter().find_map(|k| plan.sites.iter().find(|s| s.kind == *k));
    if let Some(site) = truck_site {
        // In a farm's yard, clear of its house and barn.
        let spot = if site.kind == sites::Kind::Farm { homestead::FARM_YARD } else { Vec2::new(0.35, 0.3) };
        let at = site.plot.world(Vec2::new(site.plot.half.x * spot.x, site.plot.half.y * spot.y));
        let y = field.height_at(at.x, at.y).unwrap_or(site.plot.height);
        exits.push((Way::Truck, (at.x, at.y, site.plot.yaw, y + 3.0), Vec3::ZERO, 0.0));
    }
    let spawn = scatter::spawn_point(&mut dice, &field, &network, &plots, &exits);

    // What's lying about (and what's in the buildings), and the forest
    // round it all.
    let mut things = scatter::things(&mut dice, &field, &network, &plan.sites, &exits, &buildings, out_end);
    things.containers.extend(town.cars);
    for b in &buildings {
        let inside = building::furnish::furnish(b, &mut Dice(b.seed.rotate_left(9) | 1));
        scenery.extend(inside.pieces);
        things.containers.extend(inside.containers);
        things.pickups.extend(inside.pickups);
    }
    let mut keep_out: Vec<(Vec2, f64)> = exits.iter().map(|(_, (x, z, _, _), _, r)| (Vec2::new(*x, *z), r.max(6.0) + 3.0)).collect();
    keep_out.push((Vec2::new(spawn.0.x, spawn.0.z), 10.0));
    keep_out.extend(things.containers.iter().map(|(_, (x, z, _, _))| (Vec2::new(*x, *z), 4.0)));
    scenery.extend(scatter::forest(seed, &forest, &field, &network, &plots, &plan.fields, &keep_out));
    scenery.extend(scatter::poles(&field, &network, &plots));
    Map { seed, field, roads: network.roads, sites: plan.sites, fields: plan.fields, spawn, exits, containers: things.containers, pickups: things.pickups, scenery, buildings, forest }
}

/// The nearest point to `p` on any of `roads` (flat).
fn nearest_on(roads: &[Road], p: Vec2) -> Option<Vec2> {
    let mut best: Option<(Vec2, f64)> = None;
    for road in roads {
        for w in road.points.windows(2) {
            let (a, b) = (Vec2::new(w[0].x, w[0].z), Vec2::new(w[1].x, w[1].z));
            let q = a + (b - a) * ((p - a).dot(b - a) / (b - a).dot(b - a).max(1e-9)).clamp(0.0, 1.0);
            let d = (q - p).length();
            if best.is_none_or(|(_, bd)| d < bd) {
                best = Some((q, d));
            }
        }
    }
    best.map(|(q, _)| q)
}

fn distance_to_segment(p: Vec2, a: Vec2, b: Vec2) -> f64 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.dot(ab).max(1e-9)).clamp(0.0, 1.0);
    (a + ab * t - p).length()
}

/// Work each of `n` rows out with `row` (its cells, in order), the rows
/// shared out over every core; every row's cells, one after another.
pub fn rows<T: Send>(n: usize, row: impl Fn(usize) -> Vec<T> + Sync) -> Vec<T> {
    let cores = std::thread::available_parallelism().map_or(4, |c| c.get()).min(n.max(1));
    let per = n.div_ceil(cores).max(1);
    let row = &row;
    std::thread::scope(|scope| {
        let parts: Vec<_> = (0..n).step_by(per).map(|from| scope.spawn(move || (from..(from + per).min(n)).flat_map(row).collect::<Vec<T>>())).collect();
        parts.into_iter().flat_map(|p| p.join().expect("a row panicked")).collect()
    })
}

#[cfg(test)]
mod tests;
