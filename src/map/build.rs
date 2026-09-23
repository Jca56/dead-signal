//! A map made real: its ground and roads as meshes in chunks (drawn only
//! when near and in view), everything on it solid, the containers and ways
//! out set down, and where the dead can walk. All plain data, so it's done
//! on a thread of its own while the loading screen shows; the game takes
//! it from there (`crate::app::level`).

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

use lntrn_image::Image;
use lntrn_math::{Vec2, Vec3};

use super::roads::{self, Kind as RoadKind, Network, Road};
use super::scatter::Scenery;
use super::sites::Kind as SiteKind;
use super::terrain::hash;
use super::{HALF, Map};
use crate::collide::{Solids, Surface};
use crate::containers::Placing;
use crate::exits::{self, Exits};
use crate::loot::tables::Source;
use crate::render::Vertex;
use crate::zombie::nav::NavGrid;

/// Chunks of ground, metres a side.
const CHUNK: f64 = 60.0;
/// Solid out to this far past the edge (nobody gets further).
const SOLID_REACH: f64 = HALF + 24.0;

/// The shapes things are made solid by, about their own origins: loaded
/// once, handed to every build.
#[derive(Clone, Default)]
pub struct Kit {
    pub scenery: HashMap<Scenery, Vec<[Vec3; 3]>>,
    pub containers: HashMap<Source, Vec<[Vec3; 3]>>,
    pub exits: exits::Shapes,
}

/// A piece of the ground to draw: its triangles, and the ball round them.
pub struct Chunk {
    pub vertices: Vec<Vertex>,
    pub centre: Vec3,
    pub radius: f64,
}

pub struct Built {
    pub map: Map,
    pub solids: Solids,
    pub nav: NavGrid,
    pub chunks: Vec<Chunk>,
    pub containers: Vec<Placing>,
    pub exits: Exits,
    /// The map as the map screen shows it.
    pub picture: Image,
}

/// How far along a build is, for the loading screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Land,
    Ground,
    Forest,
    Walkways,
    Done,
}

impl Stage {
    pub fn words(self) -> &'static str {
        match self {
            Stage::Land => "Surveying the land",
            Stage::Ground => "Laying the roads",
            Stage::Forest => "Growing the forest",
            Stage::Walkways => "Waking the dead",
            Stage::Done => "Ready",
        }
    }

    fn from(v: u8) -> Self {
        match v {
            0 => Stage::Land,
            1 => Stage::Ground,
            2 => Stage::Forest,
            3 => Stage::Walkways,
            _ => Stage::Done,
        }
    }
}

/// A build under way on its own thread.
pub struct Building {
    stage: Arc<AtomicU8>,
    thread: Option<std::thread::JoinHandle<Built>>,
}

impl Building {
    /// Start building the map for `seed`.
    pub fn start(seed: u32, kit: Kit) -> Self {
        let stage = Arc::new(AtomicU8::new(0));
        let told = stage.clone();
        let thread = std::thread::Builder::new().name("map".into()).spawn(move || build(seed, &kit, &|s| told.store(s as u8, Ordering::Relaxed))).expect("a thread for the map");
        Self { stage, thread: Some(thread) }
    }

    pub fn stage(&self) -> Stage {
        Stage::from(self.stage.load(Ordering::Relaxed))
    }

    /// The map, once it's built (taken, once).
    pub fn take(&mut self) -> Option<Built> {
        if !self.thread.as_ref().is_some_and(|t| t.is_finished()) {
            return None;
        }
        self.thread.take().and_then(|t| t.join().ok())
    }
}

/// Make the map for `seed` and make it real, saying how far along it is.
pub fn build(seed: u32, kit: &Kit, stage: &dyn Fn(Stage)) -> Built {
    stage(Stage::Land);
    let map = super::generate(seed);
    stage(Stage::Ground);
    let network = Network::new(map.roads.clone());
    let mut solids = Solids::default();
    let mut buckets: HashMap<(i32, i32), Vec<Vertex>> = HashMap::new();
    let mut bucket = |tri: [Vec3; 3], colour: [f32; 3]| {
        let mid = (tri[0] + tri[1] + tri[2]) * (1.0 / 3.0);
        let key = ((mid.x / CHUNK).floor() as i32, (mid.z / CHUNK).floor() as i32);
        let n = (tri[1] - tri[0]).cross(tri[2] - tri[0]);
        if n.length() < 1e-12 {
            return;
        }
        let n = n.normalize();
        let normal = [n.x as f32, n.y as f32, n.z as f32];
        // Colours are picked as they look (sRGB), and kept as Blender
        // exports them: linear.
        let [r, g, b] = colour.map(linear);
        let out = buckets.entry(key).or_default();
        for p in tri {
            out.push(Vertex { pos: [p.x as f32, p.y as f32, p.z as f32], normal, color: [r, g, b, 1.0], emissive: [0.0; 3] });
        }
    };
    // The ground.
    let mut ground = Vec::new();
    for j in 0..map.field.n - 1 {
        for i in 0..map.field.n - 1 {
            for (k, tri) in map.field.cell(i, j).into_iter().enumerate() {
                let colour = ground_colour(&map, &network, tri, hash(seed, i, j, 10 + k as u32));
                bucket(tri, colour);
                let mid = (tri[0] + tri[1] + tri[2]) * (1.0 / 3.0);
                if mid.x.abs().max(mid.z.abs()) < SOLID_REACH {
                    ground.push(tri);
                }
            }
        }
    }
    solids.add_as(&ground, Surface::Dirt);
    // The roads over it.
    for (r, road) in map.roads.iter().enumerate() {
        let (tops, rest) = ribbon(road, seed ^ r as u32);
        let surface = if road.kind == RoadKind::Dirt { Surface::Dirt } else { Surface::Stone };
        solids.add_as(&tops.iter().map(|(t, _)| *t).collect::<Vec<_>>(), surface);
        for (tri, colour) in tops.into_iter().chain(rest) {
            bucket(tri, colour);
        }
    }
    let chunks = buckets
        .into_values()
        .map(|vertices| {
            let (mut lo, mut hi) = (Vec3::splat(f64::INFINITY), Vec3::splat(f64::NEG_INFINITY));
            for v in &vertices {
                let p = Vec3::new(f64::from(v.pos[0]), f64::from(v.pos[1]), f64::from(v.pos[2]));
                lo = lo.min(p);
                hi = hi.max(p);
            }
            Chunk { vertices, centre: (lo + hi) * 0.5, radius: (hi - lo).length() * 0.5 }
        })
        .collect();

    stage(Stage::Forest);
    let mut by_surface: HashMap<Surface, Vec<[Vec3; 3]>> = HashMap::new();
    for piece in &map.scenery {
        if piece.at.x.abs().max(piece.at.z.abs()) > HALF + 10.0 {
            continue;
        }
        let Some(hull) = kit.scenery.get(&piece.what) else { continue };
        let m = piece.model();
        by_surface.entry(piece.what.surface()).or_default().extend(hull.iter().map(|t| t.map(|p| m.transform_point(p))));
    }
    for (surface, tris) in by_surface {
        solids.add_as(&tris, surface);
    }
    let containers = crate::containers::set_down(&mut solids, &kit.containers, &map.containers);
    let exits = exits::set_down(&mut solids, &kit.exits, &map.exits);

    stage(Stage::Walkways);
    let nav = NavGrid::build(&solids, crate::player::capsule(false), HALF);
    let picture = super::screen::picture(&map, &network);
    stage(Stage::Done);
    Built { map, solids, nav, chunks, containers, exits, picture }
}

// ---- colour -------------------------------------------------------------------
//
// The title scene's own colours (sRGB, as picked in Blender), so the map
// is the same forest.

type Rgb = [f32; 3];

const GRASS: Rgb = [0.23, 0.26, 0.15];
const DEAD_GRASS: Rgb = [0.36, 0.33, 0.21];
const NEEDLES: Rgb = [0.20, 0.19, 0.12];
const ROCKY: Rgb = [0.33, 0.31, 0.27];
const VERGE: Rgb = [0.31, 0.28, 0.21];
const CROP: Rgb = [0.42, 0.36, 0.19];
const SOIL: Rgb = [0.27, 0.21, 0.14];
const ASPHALT: Rgb = [0.16, 0.16, 0.16];
const WORN: Rgb = [0.21, 0.21, 0.20];
const DIRT: Rgb = [0.30, 0.24, 0.17];
const PAINT_YELLOW: Rgb = [0.55, 0.45, 0.12];
const PAINT_WHITE: Rgb = [0.58, 0.58, 0.54];
const SKIRT: Rgb = [0.18, 0.16, 0.13];

/// An sRGB channel as light (what the renderer works in).
fn linear(c: f32) -> f32 {
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

fn mix(a: Rgb, b: Rgb, t: f64) -> Rgb {
    let t = t.clamp(0.0, 1.0) as f32;
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

fn shade(c: Rgb, k: f64) -> Rgb {
    let k = k as f32;
    [(c[0] * k).min(1.0), (c[1] * k).min(1.0), (c[2] * k).min(1.0)]
}

/// What a site's ground is.
fn site_ground(kind: SiteKind) -> Rgb {
    match kind {
        SiteKind::Town => [0.36, 0.33, 0.28],
        SiteKind::Gas => [0.42, 0.41, 0.38],
        SiteKind::Farm => [0.33, 0.27, 0.19],
        SiteKind::Military => [0.34, 0.31, 0.22],
        SiteKind::Cabin => [0.27, 0.22, 0.16],
        SiteKind::Crash => [0.20, 0.18, 0.15],
        SiteKind::Pad => [0.46, 0.46, 0.44],
        SiteKind::Radio => [0.38, 0.37, 0.34],
    }
}

/// A triangle of the ground's colour: grass going over to dead grass in
/// broad patches, needles under the stands, bare rock on steep faces, the
/// places' own ground on their plots, crop rows in the fields, gravel on
/// the roads' verges; each face a little off the next (`luck`, 0–1).
fn ground_colour(map: &Map, network: &Network, tri: [Vec3; 3], luck: f64) -> Rgb {
    let mid = (tri[0] + tri[1] + tri[2]) * (1.0 / 3.0);
    let p = Vec2::new(mid.x, mid.z);
    let n = (tri[1] - tri[0]).cross(tri[2] - tri[0]).normalize();
    let t = 0.5 + 0.5 * (mid.x * 0.021 + (mid.z * 0.017).cos() * 2.0).sin();
    let mut c = mix(GRASS, DEAD_GRASS, t * 0.8);
    c = mix(c, NEEDLES, map.forest.density(mid.x, mid.z) * 0.55);
    if n.y < 0.9 {
        c = mix(c, ROCKY, (0.9 - n.y) * 4.0);
    }
    for f in &map.fields {
        if f.outside(p) <= 0.0 {
            let row = (f.local(p).x / 6.0).floor() as i64;
            c = if row % 2 == 0 { CROP } else { SOIL };
        }
    }
    for site in &map.sites {
        let w = site.plot.weight(p);
        if w > 0.4 {
            let mut g = site_ground(site.kind);
            if site.kind == SiteKind::Crash {
                // Scorched towards where it came down.
                let d = site.plot.local(p).length() / site.plot.half.length();
                g = mix([0.10, 0.09, 0.08], g, d);
            }
            c = mix(c, g, ((w - 0.4) / 0.4).min(0.85));
        }
    }
    if let Some((r, d, _, _)) = network.claim(p) {
        let edge = network.roads[r].kind.half_width() + roads::FLAT;
        if d < edge {
            c = mix(c, VERGE, 0.55);
        }
    }
    shade(c, 0.9 + 0.2 * luck)
}

/// A road's strip: its top (what's walked on) and the rest (the skirt down
/// to the ground each side, painted lines); every triangle with its colour.
#[allow(clippy::type_complexity)]
fn ribbon(road: &Road, seed: u32) -> (Vec<([Vec3; 3], Rgb)>, Vec<([Vec3; 3], Rgb)>) {
    let (mut tops, mut rest) = (Vec::new(), Vec::new());
    let hw = road.kind.half_width();
    let lift = road.kind.lift();
    let up = Vec3::new(0.0, lift, 0.0);
    let across = |i: usize| {
        let d = road.tangent(i);
        Vec3::new(-d.y, 0.0, d.x)
    };
    // Facing up (or outwards, for the skirts).
    let facing = |mut t: [Vec3; 3], want: Vec3| {
        if (t[1] - t[0]).cross(t[2] - t[0]).dot(want) < 0.0 {
            t.swap(1, 2);
        }
        t
    };
    let quad = |out: &mut Vec<([Vec3; 3], Rgb)>, a: Vec3, b: Vec3, c: Vec3, d: Vec3, want: Vec3, colour: Rgb| {
        out.push((facing([a, b, c], want), colour));
        out.push((facing([a, c, d], want), colour));
    };
    let base = match road.kind {
        RoadKind::Highway => ASPHALT,
        RoadKind::Paved => WORN,
        RoadKind::Dirt => DIRT,
    };
    for i in 0..road.points.len() - 1 {
        let (p0, p1) = (road.points[i] + up, road.points[i + 1] + up);
        let (s0, s1) = (across(i), across(i + 1));
        let luck = hash(seed, i, 0, 3);
        // A patched or worn stretch now and then.
        let colour = shade(if luck < 0.12 { mix(base, WORN, 0.6) } else { base }, 0.92 + 0.16 * hash(seed, i, 1, 4));
        quad(&mut tops, p0 - s0 * hw, p0 + s0 * hw, p1 + s1 * hw, p1 - s1 * hw, Vec3::new(0.0, 1.0, 0.0), colour);
        for side in [-1.0, 1.0] {
            let (e0, e1) = (p0 + s0 * (hw * side), p1 + s1 * (hw * side));
            let drop = Vec3::new(0.0, lift + 0.45, 0.0);
            quad(&mut rest, e0, e1, e1 - drop + s1 * (0.4 * side), e0 - drop + s0 * (0.4 * side), s0 * side, SKIRT);
        }
        if road.kind == RoadKind::Highway {
            let paint = Vec3::new(0.0, 0.012, 0.0);
            let w = 0.08;
            // Dashes down the middle; solid lines along the edges.
            if i % 3 != 2 {
                quad(&mut rest, p0 - s0 * w + paint, p0 + s0 * w + paint, p1 + s1 * w + paint, p1 - s1 * w + paint, Vec3::new(0.0, 1.0, 0.0), PAINT_YELLOW);
            }
            for side in [-1.0, 1.0] {
                let (e0, e1) = (p0 + s0 * ((hw - 0.35) * side), p1 + s1 * ((hw - 0.35) * side));
                quad(&mut rest, e0 - s0 * w + paint, e0 + s0 * w + paint, e1 + s1 * w + paint, e1 - s1 * w + paint, Vec3::new(0.0, 1.0, 0.0), PAINT_WHITE);
            }
        }
    }
    (tops, rest)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The ground's colours come out as the title scene's do from Blender
    /// (which keeps colours linear).
    #[test]
    fn colours_are_kept_as_blender_keeps_them() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/models/title_scene.glb");
        let g = lntrn_model::Gltf::load(path).expect("title_scene.glb");
        let node = g.nodes.iter().find(|n| n.name.as_deref() == Some("Ground")).expect("its ground");
        let colours = &g.meshes[node.mesh.expect("a mesh")].primitives[0].colors;
        // Its brightest green: dead grass, a face jittered up a tenth.
        let brightest = colours.iter().map(|c| c[1]).fold(0.0, f32::max);
        let dead = linear(DEAD_GRASS[1]);
        assert!(brightest > dead && brightest < dead * 1.2, "dead grass {dead} vs the title's {brightest}");
    }
}
