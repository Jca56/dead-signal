//! What's strewn over the land: the forest (thick in its stands, thin in
//! the meadows, a wall of it at the edge), boulders, power poles along the
//! highway; wrecks and crates and boxes of rounds at the places and along
//! the roads; and where the player comes in.

use lntrn_math::{Vec2, Vec3};

use super::building::Building;
use super::building::furnish::Furn;
use super::outposts::Fixture;
use super::noise::Noise;
use super::roads::{Kind as RoadKind, Network};
use super::sites::{Kind as SiteKind, Site};
use super::terrain::{Field, Plot, hash, smooth};
use super::{EDGE, EXTENT, HALF, Spot};
use crate::exits::{self, Way};
use crate::loot::tables::Source;
use crate::loot::{Dice, Kind};

/// What a piece of scenery is: its object in `scenery.glb`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Scenery {
    Pine(u8),
    Dead(u8),
    Rock(u8),
    Log,
    Stump,
    Pole,
    Tower,
    Beacon,
    /// Furniture, indoors.
    Furn(Furn),
    /// What the places out of town are fitted with (`sites.glb`).
    Fixture(Fixture),
}

impl Scenery {
    /// Every piece there is.
    pub fn all() -> Vec<Scenery> {
        let mut all = vec![Scenery::Log, Scenery::Stump, Scenery::Pole, Scenery::Tower, Scenery::Beacon];
        all.extend((0..4).map(Scenery::Pine));
        all.extend((0..2).map(Scenery::Dead));
        all.extend((0..4).map(Scenery::Rock));
        all.extend(Furn::ALL.map(Scenery::Furn));
        all.extend(Fixture::ALL.map(Scenery::Fixture));
        all
    }

    pub fn name(self) -> String {
        match self {
            Scenery::Pine(i) => format!("SCENE_Pine_{i}"),
            Scenery::Dead(i) => format!("SCENE_Dead_{i}"),
            Scenery::Rock(i) => format!("SCENE_Rock_{i}"),
            Scenery::Log => "SCENE_Log".into(),
            Scenery::Stump => "SCENE_Stump".into(),
            Scenery::Pole => "SCENE_Pole".into(),
            Scenery::Tower => "SCENE_Tower".into(),
            Scenery::Beacon => "SCENE_Beacon".into(),
            Scenery::Furn(f) => f.name().into(),
            Scenery::Fixture(f) => f.name().into(),
        }
    }

    /// What it's made of, for what a bullet kicks up.
    pub fn surface(self) -> crate::collide::Surface {
        use crate::collide::Surface;
        match self {
            Scenery::Rock(_) => Surface::Stone,
            Scenery::Tower | Scenery::Beacon => Surface::Metal,
            Scenery::Furn(Furn::Bathtub | Furn::Toilet | Furn::Basin | Furn::Stove) => Surface::Stone,
            Scenery::Fixture(f) => f.surface(),
            _ => Surface::Wood,
        }
    }
}

/// One piece of scenery: what, standing where, turned and scaled, and a
/// shade of its colours (1 as made).
#[derive(Clone, Copy, Debug)]
pub struct Piece {
    pub what: Scenery,
    pub at: Vec3,
    pub yaw: f64,
    pub scale: f64,
    pub shade: f32,
}

impl Piece {
    pub fn new(what: Scenery, at: Vec3, yaw: f64, scale: f64) -> Self {
        Self { what, at, yaw, scale, shade: 1.0 }
    }

    pub fn model(&self) -> lntrn_math::Mat4 {
        use lntrn_math::{Mat4, Quat};
        Mat4::from_translation(self.at) * Mat4::from_quat(Quat::from_rotation_y(self.yaw)) * Mat4::from_scale(Vec3::splat(self.scale))
    }
}

/// How thick the forest grows where: stands and meadows.
#[derive(Clone)]
pub struct Forest(Noise);

impl Forest {
    pub fn new(seed: u32) -> Self {
        Self(Noise::new(seed ^ 0x5EED_F0E5))
    }

    /// 0 (open meadow) to 1 (a thick stand); a wall of trees at the edge.
    pub fn density(&self, x: f64, z: f64) -> f64 {
        let n = self.0.layered(x / 150.0, z / 150.0, 3);
        let stands = smooth((n + 0.12) / 0.4);
        let edge = smooth((x.abs().max(z.abs()) - (EDGE - 30.0)) / 30.0);
        stands.max(edge)
    }
}

/// How far a point is from every plot's edge (0 in one).
fn from_plots(plots: &[Plot], p: Vec2) -> f64 {
    plots.iter().map(|pl| pl.outside(p)).fold(f64::INFINITY, f64::min)
}

/// The forest and the boulders: a tree to every few metres of a thick
/// stand, fewer in the meadows; none on a road or its verge, on a plot, in
/// a field, or in `keep_out` (spots, and how far round them).
pub fn forest(seed: u32, forest: &Forest, field: &Field, network: &Network, plots: &[Plot], fields: &[Plot], keep_out: &[(Vec2, f64)]) -> Vec<Piece> {
    const TREE_GRID: f64 = 6.0;
    const ROCK_GRID: f64 = 18.0;
    let clear = |p: Vec2, road: f64, plot: f64| {
        network.off_road(p) > road && from_plots(plots, p) > plot && from_plots(fields, p) > 1.0 && keep_out.iter().all(|(c, r)| (p - *c).length() > *r)
    };
    let mut out = Vec::new();
    let n = (2.0 * EXTENT / TREE_GRID) as usize;
    for j in 0..n {
        for i in 0..n {
            let h = |salt| hash(seed, i, j, salt);
            let p = Vec2::new(-EXTENT + (i as f64 + h(1)) * TREE_GRID, -EXTENT + (j as f64 + h(2)) * TREE_GRID);
            let d = forest.density(p.x, p.y);
            if h(3) > d * 0.8 + 0.025 || !clear(p, 4.0, 6.0) {
                continue;
            }
            let Some(y) = field.height_at(p.x, p.y) else { continue };
            let pick = h(4);
            // Dead trees stand more in the open; stumps and logs on the
            // forest floor.
            let what = if pick < 0.86 - 0.2 * (1.0 - d) {
                Scenery::Pine((h(5) * 4.0) as u8 % 4)
            } else if pick < 0.94 {
                Scenery::Dead((h(5) * 2.0) as u8 % 2)
            } else if pick < 0.97 {
                Scenery::Stump
            } else {
                Scenery::Log
            };
            let scale = match what {
                Scenery::Pine(_) => 0.8 + 0.55 * h(6),
                Scenery::Dead(_) => 0.85 + 0.4 * h(6),
                _ => 0.9 + 0.3 * h(6),
            };
            out.push(Piece { what, at: Vec3::new(p.x, y, p.y), yaw: h(7) * std::f64::consts::TAU, scale, shade: (0.88 + 0.24 * h(8)) as f32 });
        }
    }
    let n = (2.0 * EXTENT / ROCK_GRID) as usize;
    for j in 0..n {
        for i in 0..n {
            let h = |salt| hash(seed ^ 0xB0_1DE5, i, j, salt);
            let p = Vec2::new(-EXTENT + (i as f64 + h(1)) * ROCK_GRID, -EXTENT + (j as f64 + h(2)) * ROCK_GRID);
            if h(3) > 0.22 || !clear(p, 4.0, 4.0) {
                continue;
            }
            // Sat on the lowest of the ground under it, so it never floats.
            let scale = 0.6 + 1.1 * h(6) * h(6);
            let r = 1.2 * scale;
            let low = [(0.0, 0.0), (r, 0.0), (-r, 0.0), (0.0, r), (0.0, -r)].iter().filter_map(|(dx, dz)| field.height_at(p.x + dx, p.y + dz)).fold(f64::INFINITY, f64::min);
            if !low.is_finite() {
                continue;
            }
            out.push(Piece { what: Scenery::Rock((h(5) * 4.0) as u8 % 4), at: Vec3::new(p.x, low - 0.15 * scale, p.y), yaw: h(7) * std::f64::consts::TAU, scale, shade: (0.9 + 0.2 * h(8)) as f32 });
        }
    }
    out
}

/// Power poles down one side of the highway, every so often.
pub fn poles(field: &Field, network: &Network, plots: &[Plot]) -> Vec<Piece> {
    let hw = &network.roads[0];
    let mut out = Vec::new();
    for i in (3..hw.points.len().saturating_sub(3)).step_by(11) {
        let dir = hw.heading(i);
        let side = Vec2::new(-dir.y, dir.x);
        let p = Vec2::new(hw.points[i].x, hw.points[i].z) + side * (RoadKind::Highway.half_width() + 2.5);
        if from_plots(plots, p) < 2.0 || network.off_road(p) < 1.5 {
            continue;
        }
        let Some(y) = field.height_at(p.x, p.y) else { continue };
        // Its crossarm (along its own z) along the line.
        out.push(Piece::new(Scenery::Pole, Vec3::new(p.x, y, p.y), dir.x.atan2(dir.y), 1.0));
    }
    out
}

/// What's lying about: containers, and boxes and kits to pick up.
pub struct Things {
    pub containers: Vec<(Source, Spot)>,
    pub pickups: Vec<crate::items::Spot>,
}

/// Containers at every place (the cage at the camp), wrecks along the
/// highway and jammed across its far end (`jammed` is the end that isn't
/// the way out), and boxes of rounds and kits about the places.
#[allow(clippy::too_many_arguments)]
pub fn things(dice: &mut Dice, field: &Field, network: &Network, sites: &[Site], exits: &[exits::Spot], buildings: &[Building], fixtures: &[(Vec2, f64)], out_end: usize) -> Things {
    let mut containers: Vec<(Source, Spot)> = Vec::new();
    let mut pickups: Vec<crate::items::Spot> = Vec::new();
    // (Nothing left out where a building stands, or its porch or steps.)
    let walled = |p: Vec2| {
        fixtures.iter().any(|(c, r)| (p - *c).length() < *r)
            || buildings.iter().any(|b| {
            let f = b.footprint();
            let (lo, hi) = f.iter().fold((Vec2::splat(f64::INFINITY), Vec2::splat(f64::NEG_INFINITY)), |(lo, hi), c| (lo.min(*c), hi.max(*c)));
            p.x > lo.x - 2.5 && p.x < hi.x + 2.5 && p.y > lo.y - 2.5 && p.y < hi.y + 2.5
        })
    };
    let taken = |containers: &[(Source, Spot)], p: Vec2, room: f64| {
        walled(p) || containers.iter().any(|(_, (x, z, _, _))| (Vec2::new(*x, *z) - p).length() < room) || exits.iter().any(|(_, (x, z, _, _), _, r)| (Vec2::new(*x, *z) - p).length() < room + r.max(4.0))
    };
    let floor = |p: Vec2| field.height_at(p.x, p.y).unwrap_or(0.0) + 3.0;
    let car_yaw = |dir: Vec2| (-dir.y).atan2(dir.x);
    for site in sites {
        let (crates, cars, pick) = match site.kind {
            // (The town's things are in its buildings.)
            SiteKind::Town => (0, 0, 1),
            // (What's at the camp, the gas station, the crash and the
            // range is in their layouts, `outposts.rs`.)
            SiteKind::Military => (0, 1, 3),
            SiteKind::Farm => (2, 1, 2),
            SiteKind::Gas => (0, 0, 2),
            SiteKind::Cabin => (1, 0, 1),
            SiteKind::Crash => (0, 0, 2),
            SiteKind::Pad => (0, 0, 1),
            SiteKind::Radio => (1, 0, 1),
        };
        let wanted: Vec<Source> = std::iter::repeat_n(Source::Crate, crates).chain(std::iter::repeat_n(Source::Car, cars)).collect();
        for source in wanted {
            for _ in 0..40 {
                // Off the town's main street, and clear of its middle
                // (where the radio mast stands).
                let l = Vec2::new((dice.unit() * 2.0 - 1.0) * site.plot.half.x * 0.8, (dice.unit() * 2.0 - 1.0) * site.plot.half.y * 0.8);
                if site.kind == SiteKind::Town && l.x.abs() < RoadKind::Highway.half_width() + 3.0 {
                    continue;
                }
                if site.kind == SiteKind::Radio && l.length() < 6.0 {
                    continue;
                }
                let p = site.plot.world(l);
                if taken(&containers, p, 5.0) || (network.off_road(p) < 1.5 && site.kind != SiteKind::Town) {
                    continue;
                }
                containers.push((source, (p.x, p.y, site.plot.yaw + (dice.unit() - 0.5) * 0.8, floor(p))));
                break;
            }
        }
        for _ in 0..pick {
            let l = Vec2::new((dice.unit() * 2.0 - 1.0) * site.plot.half.x * 0.7, (dice.unit() * 2.0 - 1.0) * site.plot.half.y * 0.7);
            let p = site.plot.world(l);
            if taken(&containers, p, 2.0) {
                continue;
            }
            pickups.push(pickup(dice, p, floor(p)));
        }
    }
    // Wrecks down the highway, and the jam across the end that isn't the
    // way out.
    let hw = &network.roads[0];
    let n = hw.points.len();
    let in_town = |p: Vec2| sites.iter().any(|s| s.kind == SiteKind::Town && s.plot.outside(p) < 10.0);
    let edgeward = |i: usize| hw.points[i].x.abs().max(hw.points[i].z.abs());
    let mut wrecks = 0;
    for _ in 0..200 {
        if wrecks >= 7 {
            break;
        }
        let i = 2 + dice.next() as usize % (n - 4);
        let p = Vec2::new(hw.points[i].x, hw.points[i].z);
        if edgeward(i) > HALF - 50.0 || in_town(p) || taken(&containers, p, 30.0) {
            continue;
        }
        let dir = hw.heading(i);
        let at = p + Vec2::new(-dir.y, dir.x) * ((dice.unit() - 0.5) * 5.0);
        containers.push((Source::Car, (at.x, at.y, car_yaw(dir) + (dice.unit() - 0.5) * 1.0, floor(at))));
        if dice.unit() < 0.6 {
            let q = at + Vec2::new(-dir.y, dir.x) * 3.2;
            pickups.push(pickup(dice, q, floor(q)));
        }
        wrecks += 1;
    }
    let jam: Vec<usize> = (0..n).filter(|&i| edgeward(i) > HALF - 42.0 && edgeward(i) < HALF - 14.0 && (if out_end == 0 { i > n / 2 } else { i < n / 2 })).collect();
    for (k, &i) in jam.iter().enumerate().step_by(2) {
        let dir = hw.heading(i);
        let lateral = if k % 4 == 0 { -2.2 } else { 2.2 };
        let at = Vec2::new(hw.points[i].x, hw.points[i].z) + Vec2::new(-dir.y, dir.x) * (lateral + (dice.unit() - 0.5));
        containers.push((Source::Car, (at.x, at.y, car_yaw(dir) + (dice.unit() - 0.5) * 2.2, floor(at))));
    }
    Things { containers, pickups }
}

/// A box of rounds (mostly) or a kit, lying at `p`.
fn pickup(dice: &mut Dice, p: Vec2, hint: f64) -> crate::items::Spot {
    let roll = dice.unit();
    let (kind, count) = if roll < 0.62 {
        (Kind::Rounds, crate::items::ROUNDS)
    } else if roll < 0.87 {
        (Kind::Bandage, (1, 1))
    } else {
        (Kind::Medkit, (1, 1))
    };
    (kind, count, p.x, hint, p.y, dice.unit() * std::f64::consts::TAU)
}

/// Where the player comes in: near an edge, off the roads and plots, on
/// ground not too steep, and as far as can be from every way out. Facing
/// the middle of the map.
pub fn spawn_point(dice: &mut Dice, field: &Field, network: &Network, plots: &[Plot], exits: &[exits::Spot]) -> (Vec3, f64) {
    let mut best: Option<(Vec2, f64)> = None;
    let edge = EDGE - 14.0;
    for _ in 0..60 {
        let along = (dice.unit() * 2.0 - 1.0) * (EDGE - 60.0);
        let p = match dice.next() % 4 {
            0 => Vec2::new(edge, along),
            1 => Vec2::new(-edge, along),
            2 => Vec2::new(along, edge),
            _ => Vec2::new(along, -edge),
        };
        if network.off_road(p) < 12.0 || from_plots(plots, p) < 20.0 {
            continue;
        }
        let Some(h) = field.height_at(p.x, p.y) else { continue };
        let steep = [(2.0, 0.0), (-2.0, 0.0), (0.0, 2.0), (0.0, -2.0)].iter().filter_map(|(dx, dz)| field.height_at(p.x + dx, p.y + dz)).any(|g| (g - h).abs() > 1.0);
        if steep {
            continue;
        }
        let far = exits.iter().map(|(way, (x, z, _, _), _, _)| (Vec2::new(*x, *z) - p).length() * if *way == Way::Road { 1.0 } else { 1.1 }).fold(f64::INFINITY, f64::min);
        if best.is_none_or(|(_, b)| far > b) {
            best = Some((p, far));
        }
    }
    let p = best.map_or(Vec2::new(0.0, 0.0), |(p, _)| p);
    let y = field.height_at(p.x, p.y).unwrap_or(0.0);
    (Vec3::new(p.x, y, p.y), p.x.atan2(p.y))
}
