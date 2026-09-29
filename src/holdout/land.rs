//! The hill a holdout's compound stands on: level where the compound is
//! (and over a cleared apron round its walls), falling away into the woods
//! on every side, rising far off into ridges lost in the fog; a dirt road
//! winding down from the gate, power poles along it; the forest.

use lntrn_math::Vec2;

use crate::map::noise::Noise;
use crate::map::roads::{self, Kind as RoadKind, Network, Road};
use crate::map::scatter::{self, Forest, Piece};
use crate::map::sites::{Kind as SiteKind, Site};
use crate::map::terrain::{Field, Plot, smooth};

/// The cleared apron round the compound's walls, and how far the land
/// takes to come back to its own past it.
const APRON: f64 = 9.0;
const SHOULDER: f64 = 18.0;
/// The road runs straight out from the gate this far before it finds its
/// own way down, to about here.
const STRAIGHT: f64 = 36.0;
const ROAD_TO: Vec2 = Vec2::new(110.0, 340.0);
/// The woods are thick all round, out this far.
const WOODS: f64 = 130.0;

pub struct Land {
    pub field: Field,
    pub roads: Vec<Road>,
    pub site: Site,
    pub scenery: Vec<Piece>,
    pub forest: Forest,
}

/// The land's own height, before the compound's levelled: the hilltop,
/// folds on its sides, the ridges far off.
fn natural(noise: &Noise, x: f64, z: f64) -> f64 {
    let d = Vec2::new(x, z * 1.15).length();
    let fall = -24.0 * smooth((d - 20.0) / 150.0);
    let ridges = 45.0 * smooth((d - 240.0) / 180.0);
    let folds = 6.0 * noise.layered(x / 120.0, z / 120.0, 3) + noise.at(x / 22.0 + 30.0, z / 22.0 - 30.0);
    fall + ridges + folds * smooth((d - 15.0) / 50.0)
}

/// Lay the land for a compound from `lo` to `hi` (on the map, flat), its
/// gate at `gate` opening `out`; no tree within reach of `keep_out`.
pub fn lay(seed: u32, lo: Vec2, hi: Vec2, gate: Vec2, out: Vec2, keep_out: &[(Vec2, f64)]) -> Land {
    let noise = Noise::new(seed);
    let plot = Plot::new((lo + hi) * 0.5, 0.0, (hi - lo) * 0.5 + Vec2::new(APRON, APRON), SHOULDER);
    let shaped = |x: f64, z: f64| {
        let h = natural(&noise, x, z);
        h + (plot.height - h) * plot.weight(Vec2::new(x, z))
    };
    // The road: straight out over the apron, then down through the woods,
    // round the steep ground.
    let leave = gate + out * STRAIGHT;
    let blocked = |p: Vec2| plot.outside(p) < 2.0;
    let mut line = roads::resample(&[gate, leave], roads::SPACING);
    line.extend(roads::route(&shaped, &blocked, leave, ROAD_TO).into_iter().skip(1));
    let line = roads::resample(&roads::curve(&line, 2), roads::SPACING);
    let points = roads::profile(RoadKind::Dirt, &line, shaped, |p| Some((plot.weight(p), plot.height)).filter(|(w, _)| *w > 0.0), None);
    let network = Network::new(vec![Road { kind: RoadKind::Dirt, points }]);
    let height = |x: f64, z: f64| {
        let h = shaped(x, z);
        match network.claim(Vec2::new(x, z)) {
            Some((_, _, road, w)) => h + (road - roads::SINK - h) * w,
            None => h,
        }
    };
    let mut field = Field::new(seed, height);
    roads::press_ground(&mut field, &network);
    let forest = Forest::round(seed, plot.centre, WOODS);
    let plots = [plot.clone()];
    let mut scenery = scatter::forest(seed, &forest, &field, &network, &plots, &[], keep_out);
    scenery.extend(scatter::poles(&field, &network, &plots));
    let site = Site { kind: SiteKind::Radio, name: "RELAY STATION", plot };
    Land { field, roads: network.roads, site, scenery, forest }
}
