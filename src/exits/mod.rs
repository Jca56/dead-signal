//! The ways out, the first way to win: a call for a chopper from the radio
//! at the foot of the tower (then holding its ground a minute, the whole
//! map coming at the noise), the road out past the checkpoint south of the
//! proving ground (quiet, but blocked about half the time), and the
//! pickup by the range (fuel and a battery in it, then an engine to crank,
//! loudly). None is known till it's found. `hud.rs` draws them.

pub mod hud;

use std::collections::HashMap;
use std::ops::Range;

use bevy_ecs::prelude::*;
use lntrn_math::{Mat4, Vec3};

use crate::collide::{Solids, Surface};
use crate::containers::{in_box, seat};
use crate::loot::Dice;
use crate::render::{MeshId, Vertex};
use crate::world::{Ground, Look, Model, Placed, Solid};

/// A way out.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Way {
    Radio,
    Road,
    Truck,
}

impl Way {
    pub fn name(self) -> &'static str {
        match self {
            Way::Radio => "RADIO TOWER",
            Way::Road => "ROAD OUT",
            Way::Truck => "TRUCK",
        }
    }

    /// Its object in `exits.glb`, and the one it becomes (fixed, or shut).
    pub fn models(self) -> (&'static str, &'static str) {
        match self {
            Way::Radio => ("EXIT_Radio", "EXIT_Radio"),
            Way::Road => ("EXIT_Gate", "EXIT_Gate_Blocked"),
            Way::Truck => ("EXIT_Truck", "EXIT_Truck_Ready"),
        }
    }
}

/// Where a way out stands (game x and z, which way its front faces, a
/// height to look down for its floor from), where its zone is from there
/// (in its own frame: -Z is in front of it, +Z behind), and how wide the
/// zone is.
pub type Spot = (Way, crate::map::Spot, Vec3, f64);

/// How long each takes: the radio's wait for the chopper, the walk out
/// past the checkpoint, the engine's cranking; and how long the hands
/// take calling in and fitting a part.
pub const RADIO_WAIT: f64 = 60.0;
pub const ROAD_WAIT: f64 = 10.0;
pub const CRANK: f64 = 8.0;
pub const HANDS: f64 = 2.0;
/// Seen within this far (with nothing between), or come this near, and a
/// way out is found.
const SEEN_FROM: f64 = 45.0;
const NEAR: f64 = 20.0;
/// How far off a way out's things can be used from.
pub const REACH: f64 = 2.6;
/// The ring drawn round a zone on the ground: how many pieces, how wide,
/// how far over the ground (so it never flickers into it); how brightly it
/// glows waiting, and at most while the chopper's coming.
const RING_PIECES: usize = 128;
const RING_WIDTH: f64 = 0.3;
const RING_LIFT: f64 = 0.06;
const GLOW_WAITING: f32 = 0.35;
const GLOW_CALLED: f32 = 1.0;

/// One way out: where, what it looks like, and how it stands this run.
pub struct Exit {
    pub way: Way,
    model: Mat4,
    lo: Vec3,
    hi: Vec3,
    /// The middle of its zone, and its reach.
    pub zone: Vec3,
    pub radius: f64,
    looks: Option<(MeshId, MeshId)>,
    shown: Option<Entity>,
    /// Its zone's ring on the ground, if it has one, and drawn.
    ring: Option<MeshId>,
    ring_shown: Option<Entity>,
    pub found: bool,
    /// Open this run (the road can be blocked).
    pub open: bool,
    /// Called in, or running out, or cranking: how far along, seconds.
    pub started: bool,
    pub progress: f64,
    /// The truck's parts, fitted.
    pub fuel: bool,
    pub battery: bool,
}

impl Exit {
    /// Whether the fixed or shut look is the one to show.
    fn alt(&self) -> bool {
        match self.way {
            Way::Road => !self.open,
            Way::Truck => self.fuel && self.battery,
            Way::Radio => false,
        }
    }

    /// Whether a body standing at `feet` is in its zone.
    pub fn holds(&self, feet: Vec3) -> bool {
        let d = Vec3::new(feet.x - self.zone.x, 0.0, feet.z - self.zone.z).length();
        d <= self.radius && (feet.y - self.zone.y).abs() < 3.0
    }

    /// How long it takes once started.
    pub fn wait(&self) -> f64 {
        match self.way {
            Way::Radio => RADIO_WAIT,
            Way::Road => ROAD_WAIT,
            Way::Truck => CRANK,
        }
    }
}

/// Every way out, and the barricade that shuts the road (solid only when
/// it's shut).
#[derive(Resource, Default)]
pub struct Exits {
    pub list: Vec<Exit>,
    barricade: Option<Range<u32>>,
}

impl Exits {
    /// Give every zone its ring, made by `make` (from the zone's middle and
    /// reach).
    pub fn ring_zones(&mut self, mut make: impl FnMut(Vec3, f64) -> MeshId) {
        for e in self.list.iter_mut().filter(|e| e.radius > 0.0) {
            e.ring = Some(make(e.zone, e.radius));
        }
    }
}

/// A ring round `centre`, `radius` out, lying on the ground (following it
/// up and down), glowing the radio's green.
pub fn ring(ground: &Ground, centre: Vec3, radius: f64) -> Vec<Vertex> {
    let green = hud::SIGNAL_GREEN;
    let colour = [0.05, 0.12, 0.06, 1.0];
    let emissive = [green.r as f32, green.g as f32, green.b as f32];
    let at = |a: f64, r: f64| {
        let (x, z) = (centre.x + a.cos() * r, centre.z + a.sin() * r);
        let y = ground.height_at(x, z).unwrap_or(centre.y) + RING_LIFT;
        [x as f32, y as f32, z as f32]
    };
    let (inner, outer) = (radius - RING_WIDTH * 0.5, radius + RING_WIDTH * 0.5);
    let v = |pos: [f32; 3]| Vertex { pos, normal: [0.0, 1.0, 0.0], color: colour, emissive };
    let mut out = Vec::with_capacity(RING_PIECES * 6);
    for k in 0..RING_PIECES {
        let (a0, a1) = (k as f64 / RING_PIECES as f64 * std::f64::consts::TAU, (k + 1) as f64 / RING_PIECES as f64 * std::f64::consts::TAU);
        let (i0, o0, i1, o1) = (at(a0, inner), at(a0, outer), at(a1, inner), at(a1, outer));
        // Wound to face up.
        out.extend([v(i0), v(i1), v(o0), v(o0), v(i1), v(o1)]);
    }
    out
}

/// Each way out's shapes, from `exits.glb`: what it's drawn as (normal and
/// alt) and what's solid (its hull, and the road's barricade).
#[derive(Clone, Default)]
pub struct Shapes {
    pub meshes: HashMap<Way, (MeshId, MeshId)>,
    pub hulls: HashMap<Way, Vec<[Vec3; 3]>>,
    pub barricade: Vec<[Vec3; 3]>,
}

/// The ways out, set down and made solid (the road's barricade too,
/// switched off till a run shuts the road).
pub fn set_down(solids: &mut Solids, shapes: &Shapes, spots: &[Spot]) -> Exits {
    let mut exits = Exits::default();
    for &(way, at, zone, radius) in spots {
        let Some(hull) = shapes.hulls.get(&way) else { continue };
        let Some(s) = seat(solids, hull, at, way == Way::Road) else { continue };
        solids.add_as(&s.tris, if way == Way::Road { Surface::Stone } else { Surface::Metal });
        if way == Way::Road {
            let bar: Vec<[Vec3; 3]> = shapes.barricade.iter().map(|t| t.map(|p| s.model.transform_point(p))).collect();
            let range = solids.add_as(&bar, Surface::Stone);
            solids.switch(range.clone(), false);
            exits.barricade = Some(range);
        }
        let zone = s.model.transform_point(zone);
        exits.list.push(Exit { way, model: s.model, lo: s.lo, hi: s.hi, zone, radius, looks: shapes.meshes.get(&way).copied(), shown: None, ring: None, ring_shown: None, found: false, open: true, started: false, progress: 0.0, fuel: false, battery: false });
    }
    exits
}

/// A fresh run: nothing found or started, the road open or shut by
/// `seed`'s luck; every way out drawn.
pub fn begin(world: &mut World, seed: u32) {
    hide(world);
    let mut dice = Dice(seed | 1);
    let road_open = dice.unit() < 0.5;
    world.resource_scope(|world, mut exits: Mut<Exits>| {
        for e in &mut exits.list {
            e.found = false;
            e.started = false;
            e.progress = 0.0;
            e.fuel = false;
            e.battery = false;
            e.open = e.way != Way::Road || road_open;
        }
        if let Some(bar) = exits.barricade.clone() {
            world.resource_mut::<Solid>().0.switch(bar, !road_open);
        }
        for e in &mut exits.list {
            e.shown = e.looks.map(|(normal, alt)| world.spawn((Placed(e.model), Model(if e.alt() { alt } else { normal }), Look::default())).id());
            // (A shut road has no zone to show.)
            e.ring_shown = e.ring.filter(|_| e.open).map(|mesh| world.spawn((Placed(Mat4::IDENTITY), Model(mesh), Look { emissive: GLOW_WAITING, fog: 0.6, ..Look::default() })).id());
        }
    });
}

/// Nothing of them drawn (the title screen looks at the tower).
pub fn hide(world: &mut World) {
    let shown: Vec<Entity> = world.get_resource_mut::<Exits>().map(|mut x| x.list.iter_mut().flat_map(|e| [e.shown.take(), e.ring_shown.take()]).flatten().collect()).unwrap_or_default();
    for e in shown {
        world.despawn(e);
    }
}

/// Show `i`'s look as it now stands (the truck fixed).
pub fn refresh(world: &mut World, i: usize) {
    let Some((shown, mesh)) = world.get_resource::<Exits>().and_then(|x| x.list.get(i)).and_then(|e| Some((e.shown?, e.looks.map(|(n, a)| if e.alt() { a } else { n })?))) else { return };
    if let Some(mut m) = world.get_mut::<Model>(shown) {
        m.0 = mesh;
    }
}

/// The rings glow: dimly while waiting, pulsing brighter once under way
/// (the chopper called, the road being walked out of; `time` in seconds).
pub fn glow(world: &mut World, time: f64) {
    let Some(rings) = world.get_resource::<Exits>().map(|x| x.list.iter().filter_map(|e| Some((e.ring_shown?, e.started || e.progress > 0.0))).collect::<Vec<_>>()) else { return };
    for (e, called) in rings {
        if let Some(mut look) = world.get_mut::<Look>(e) {
            look.emissive = if called { GLOW_CALLED * (0.65 + 0.35 * (time * 4.0).sin() as f32) } else { GLOW_WAITING };
        }
    }
}

/// Find what the eye at `eye` can see or has come near. Which were found
/// just now.
pub fn look_about(world: &mut World, eye: Vec3) -> Vec<Way> {
    let mut found = Vec::new();
    world.resource_scope(|world, mut exits: Mut<Exits>| {
        let solids = &world.resource::<Solid>().0;
        for e in exits.list.iter_mut().filter(|e| !e.found) {
            let mark = (e.lo + e.hi) * 0.5 + Vec3::new(0.0, 0.5, 0.0);
            let to = mark - eye;
            let d = to.length();
            let seen = d < SEEN_FROM && solids.raycast(eye, to * (1.0 / d), d - 1.5).is_none();
            if d < NEAR || seen {
                e.found = true;
                found.push(e.way);
            }
        }
    });
    found
}

/// The way out whose things the eye at `eye`, looking along `dir`, is on.
pub fn in_view(world: &World, eye: Vec3, dir: Vec3) -> Option<usize> {
    let hit = world.resource::<Solid>().0.raycast(eye, dir, REACH)?;
    world.get_resource::<Exits>()?.list.iter().position(|e| in_box(hit.point, e.lo, e.hi, 0.1))
}

#[cfg(test)]
pub mod tests;
