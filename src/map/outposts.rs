//! What's at the places out of town that aren't farms or the cabin
//! (`homestead.rs`): each laid out in its plot's frame (x across, y along,
//! its front, towards its road, at -y), what's built there, what it's
//! fitted with (the fixtures in `sites.glb`), what's to be searched, and,
//! at the proving ground, what's to be shot at.
//!
//! - The gas station: a canopy over two pump islands, cars at the pumps,
//!   the store behind (its shop and back room) and a one-bay garage.
//! - The proving ground: lanes from a line of shooting benches to dummies
//!   and hanging plates, an earth berm behind; a range office and an
//!   armory (its ammunition caged) by the gate.
//! - The crash site: a helicopter broken in two, or a cargo plane, its
//!   pieces flung about the scorch, supply cases spilled in and round it.
//! - The military camp: a command tent (the supply cage in it), soldiers'
//!   tents, a sandbag wall along its front.

use lntrn_math::{Mat4, Quat, Vec2, Vec3};

use super::Spot;
use super::building::{Building, country, plan};
use super::scatter::{Piece, Scenery};
use super::sites::{Kind, Site};
use crate::collide::Surface;
use crate::loot::Dice;
use crate::loot::tables::Source;
use crate::targets::Kind as TargetKind;

/// A place's fixtures, as `sites.glb` has them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Fixture {
    Canopy,
    Pumps,
    GasSign,
    HeliFront,
    HeliRear,
    HeliTail,
    Rotor,
    PlaneFront,
    PlaneRear,
    PlaneWing,
    PlaneEngine,
    Tent,
    CommandTent,
    Sandbags,
    Berm,
    Bench,
    PlateFrame,
}

impl Fixture {
    pub const ALL: [Fixture; 17] = [
        Fixture::Canopy,
        Fixture::Pumps,
        Fixture::GasSign,
        Fixture::HeliFront,
        Fixture::HeliRear,
        Fixture::HeliTail,
        Fixture::Rotor,
        Fixture::PlaneFront,
        Fixture::PlaneRear,
        Fixture::PlaneWing,
        Fixture::PlaneEngine,
        Fixture::Tent,
        Fixture::CommandTent,
        Fixture::Sandbags,
        Fixture::Berm,
        Fixture::Bench,
        Fixture::PlateFrame,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Fixture::Canopy => "SITE_Canopy",
            Fixture::Pumps => "SITE_Pumps",
            Fixture::GasSign => "SITE_GasSign",
            Fixture::HeliFront => "SITE_HeliFront",
            Fixture::HeliRear => "SITE_HeliRear",
            Fixture::HeliTail => "SITE_HeliTail",
            Fixture::Rotor => "SITE_Rotor",
            Fixture::PlaneFront => "SITE_PlaneFront",
            Fixture::PlaneRear => "SITE_PlaneRear",
            Fixture::PlaneWing => "SITE_PlaneWing",
            Fixture::PlaneEngine => "SITE_PlaneEngine",
            Fixture::Tent => "SITE_Tent",
            Fixture::CommandTent => "SITE_CommandTent",
            Fixture::Sandbags => "SITE_Sandbags",
            Fixture::Berm => "SITE_Berm",
            Fixture::Bench => "SITE_Bench",
            Fixture::PlateFrame => "SITE_PlateFrame",
        }
    }

    /// What it's made of, for what a bullet kicks up.
    pub fn surface(self) -> Surface {
        match self {
            Fixture::Sandbags | Fixture::Berm => Surface::Dirt,
            Fixture::Tent | Fixture::CommandTent | Fixture::Bench => Surface::Wood,
            _ => Surface::Metal,
        }
    }
}

/// What's at a place: its buildings; its fixtures (standing on the plot's
/// level: the land's height is put under them once there's land); what's
/// to be searched, and in what; and targets, each at its rest.
#[derive(Default)]
pub struct Outpost {
    pub buildings: Vec<Building>,
    pub pieces: Vec<Piece>,
    pub containers: Vec<(Source, Spot)>,
    pub targets: Vec<(TargetKind, Mat4)>,
}

/// How far round a fixture nothing else is set down, metres.
pub fn reach(what: Scenery) -> f64 {
    match what {
        Scenery::Fixture(Fixture::Berm) => 10.0,
        Scenery::Fixture(Fixture::Canopy | Fixture::PlaneFront | Fixture::PlaneRear | Fixture::PlaneWing) => 7.0,
        Scenery::Fixture(Fixture::HeliFront | Fixture::HeliRear | Fixture::HeliTail) => 5.0,
        Scenery::Fixture(Fixture::CommandTent) => 4.5,
        Scenery::Fixture(Fixture::Tent | Fixture::Rotor | Fixture::PlaneEngine | Fixture::Sandbags) => 3.0,
        _ => 2.0,
    }
}

/// How high over the floor to look down from for something set inside a
/// wreck or a tent (under its roof), and out in the open. (Things are set
/// down looking from a metre over what they're given.)
const UNDER_ROOF: f64 = 1.0;
const IN_THE_OPEN: f64 = 3.0;

/// A place's frame on the map: points and directions from its plot's.
struct Frame<'a> {
    site: &'a Site,
}

impl Frame<'_> {
    fn at(&self, l: Vec2) -> Vec2 {
        self.site.plot.world(l)
    }

    /// A direction in the plot's frame, on the map.
    fn dir(&self, d: Vec2) -> Vec2 {
        self.site.plot.world(d) - self.site.plot.centre
    }

    /// The yaw that turns a piece's front (+Y as made) to face `d` (the
    /// plot's frame).
    fn facing(&self, d: Vec2) -> f64 {
        let w = self.dir(d);
        (-w.x).atan2(-w.y)
    }

    fn piece(&self, what: Fixture, l: Vec2, face: Vec2) -> Piece {
        let p = self.at(l);
        Piece::new(Scenery::Fixture(what), Vec3::new(p.x, self.site.plot.height, p.y), self.facing(face), 1.0)
    }

    /// Something to search at `l`, facing `face`, looked down on from
    /// `over` above the plot.
    fn container(&self, source: Source, l: Vec2, face: Vec2, over: f64) -> (Source, Spot) {
        let p = self.at(l);
        (source, (p.x, p.y, self.facing(face), self.site.plot.height + over))
    }
}

fn turned(v: Vec2, a: f64) -> Vec2 {
    let (s, c) = a.sin_cos();
    Vec2::new(v.x * c - v.y * s, v.x * s + v.y * c)
}

/// What's at `site`, if it's one of these places.
pub fn lay_out(dice: &mut Dice, site: &Site) -> Outpost {
    let f = Frame { site };
    match site.kind {
        Kind::Gas => gas(dice, &f),
        Kind::Pad => range(dice, &f),
        Kind::Crash => crash(dice, &f),
        Kind::Military => camp(dice, &f),
        _ => Outpost::default(),
    }
}

const TO_FRONT: Vec2 = Vec2::new(0.0, -1.0);
const TO_BACK: Vec2 = Vec2::new(0.0, 1.0);

fn gas(dice: &mut Dice, f: &Frame) -> Outpost {
    let mut out = Outpost::default();
    let plot = &f.site.plot;
    // The canopy over its pumps, cars at them.
    out.pieces.push(f.piece(Fixture::Canopy, Vec2::new(0.0, -7.0), TO_FRONT));
    for y in [-8.6, -5.4] {
        out.pieces.push(f.piece(Fixture::Pumps, Vec2::new(0.0, y), TO_FRONT));
    }
    out.pieces.push(f.piece(Fixture::GasSign, Vec2::new(-plot.half.x + 3.0, -plot.half.y + 3.0), TO_FRONT));
    for (x, y) in [(-1.0, -10.9), (1.5, -3.1)] {
        let across = f.dir(Vec2::new(1.0, 0.0));
        let p = f.at(Vec2::new(x, y));
        out.containers.push((Source::Car, (p.x, p.y, (-across.y).atan2(across.x) + (dice.unit() - 0.5) * 0.2, plot.height + IN_THE_OPEN)));
    }
    // The store and the garage behind.
    let seed = dice.next();
    out.buildings.push(Building::facing(plan::store(&mut Dice(seed | 1), 13, 10), plot, Vec2::new(-11.0, 2.0), TO_FRONT, seed));
    let seed = dice.next();
    out.buildings.push(Building::facing(country::garage(&mut Dice(seed | 1), 9, 10), plot, Vec2::new(11.0, 2.0), TO_FRONT, seed));
    out
}

fn range(dice: &mut Dice, f: &Frame) -> Outpost {
    let mut out = Outpost::default();
    let plot = &f.site.plot;
    // Lanes down the plot from the benches, the berm behind the plates.
    for x in [-7.5, -2.5, 2.5, 7.5] {
        out.pieces.push(f.piece(Fixture::Bench, Vec2::new(x, -9.0), TO_BACK));
    }
    let lane = f.dir(TO_BACK);
    for (k, x) in [-7.5, -2.5, 2.5, 7.5].into_iter().enumerate() {
        let y = 3.0 + 3.0 * f64::from(k as u8 % 2) + dice.unit() * 3.0;
        let p = f.at(Vec2::new(x + (dice.unit() - 0.5) * 1.5, y));
        let at = Vec3::new(p.x, plot.height, p.y);
        out.targets.push((TargetKind::Dummy, Mat4::from_translation(at) * Mat4::from_quat(Quat::from_rotation_y(f.facing(TO_FRONT)))));
    }
    for x in [-5.0, 0.0, 5.0] {
        out.pieces.push(f.piece(Fixture::PlateFrame, Vec2::new(x, 14.0), TO_FRONT));
        let p = f.at(Vec2::new(x, 14.0));
        let hinge = Vec3::new(p.x, plot.height + 1.85, p.y);
        out.targets.push((TargetKind::Plate, Mat4::from_translation(hinge) * Mat4::from_quat(Quat::from_rotation_y((-lane.y).atan2(lane.x)))));
    }
    out.pieces.push(f.piece(Fixture::Berm, Vec2::new(0.0, 18.5), TO_FRONT));
    // The office and the armory, by the gate.
    let seed = dice.next();
    out.buildings.push(Building::facing(country::office(&mut Dice(seed | 1), 7, 5), plot, Vec2::new(-13.0, -21.0), TO_FRONT, seed));
    let seed = dice.next();
    out.buildings.push(Building::facing(country::armory(&mut Dice(seed | 1), 7, 6), plot, Vec2::new(13.0, -21.0), TO_FRONT, seed));
    out
}

/// A wreck's pieces, where they lie along it (`u` down its length from
/// where it came down, `v` across), how turned from its line, degrees.
type Scatter = [(Fixture, f64, f64, f64)];

const HELI: &Scatter = &[(Fixture::HeliFront, 3.0, 0.0, 0.0), (Fixture::HeliRear, -4.0, 0.8, 22.0), (Fixture::HeliTail, -11.0, -2.5, 65.0), (Fixture::Rotor, 1.0, 6.0, 40.0), (Fixture::Rotor, -6.0, -6.0, -70.0), (Fixture::Rotor, 9.0, -5.0, 110.0)];
const PLANE: &Scatter = &[(Fixture::PlaneFront, 5.0, 0.0, 0.0), (Fixture::PlaneRear, -6.0, 1.2, 14.0), (Fixture::PlaneWing, 0.0, 8.0, 85.0), (Fixture::PlaneEngine, -3.0, -9.0, 30.0)];

fn crash(dice: &mut Dice, f: &Frame) -> Outpost {
    let mut out = Outpost::default();
    // Which came down, along which line.
    let plane = dice.unit() < 0.5;
    let line = dice.unit() * std::f64::consts::TAU;
    let along = |u: f64, v: f64| turned(Vec2::new(v, u), line);
    for &(what, u, v, turn) in if plane { PLANE } else { HELI } {
        out.pieces.push(f.piece(what, along(u, v), turned(TO_BACK, line + turn.to_radians())));
    }
    // Cases in the torn-open hold, and spilled round it.
    let hold = if plane { [(3.0, 0.0), (6.0, 0.4)] } else { [(1.5, 0.0), (-4.0, 0.8)] };
    for (u, v) in hold {
        out.containers.push(f.container(Source::SupplyCase, along(u, v), turned(TO_FRONT, line + std::f64::consts::FRAC_PI_2), UNDER_ROOF));
    }
    for k in 0..3 {
        let a = line + 1.3 + f64::from(k) * 2.1 + dice.unit() * 0.6;
        let r = 7.0 + dice.unit() * 5.0;
        out.containers.push(f.container(Source::SupplyCase, turned(Vec2::new(0.0, r), a), turned(TO_FRONT, dice.unit() * 6.0), IN_THE_OPEN));
    }
    out
}

fn camp(dice: &mut Dice, f: &Frame) -> Outpost {
    let mut out = Outpost::default();
    let plot = &f.site.plot;
    // The command tent in the middle, its door to the front; the cage in
    // it at the back, cases by it.
    let middle = Vec2::new(0.0, 2.0);
    out.pieces.push(f.piece(Fixture::CommandTent, middle, TO_FRONT));
    out.containers.push(f.container(Source::Cage, middle + Vec2::new(-1.8, 1.6), TO_FRONT, UNDER_ROOF));
    out.containers.push(f.container(Source::SupplyCase, middle + Vec2::new(4.5, -1.0), TO_FRONT, IN_THE_OPEN));
    // Soldiers' tents facing in, a crate or a case in each.
    for (x, y) in [(-16.0, -9.0), (-16.0, 5.0), (16.0, -9.0)] {
        let face = Vec2::new(-f64::signum(x), 0.0);
        let at = Vec2::new(x, y);
        out.pieces.push(f.piece(Fixture::Tent, at, face));
        let source = if dice.unit() < 0.5 { Source::Crate } else { Source::SupplyCase };
        out.containers.push(f.container(source, at - face * 1.0, face, UNDER_ROOF));
    }
    // A wall of sandbags along the front, a way in at its middle; a run
    // down each side from its ends.
    let front = -plot.half.y + 5.0;
    for x in [-14.0, -10.0, -6.0, 6.0, 10.0, 14.0] {
        out.pieces.push(f.piece(Fixture::Sandbags, Vec2::new(x, front), TO_FRONT));
    }
    for x in [-16.3, 16.3] {
        for k in 0..2 {
            out.pieces.push(f.piece(Fixture::Sandbags, Vec2::new(x, front + 2.3 + 4.0 * f64::from(k)), Vec2::new(f64::signum(x), 0.0)));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::terrain::Plot;

    fn site(kind: Kind) -> Site {
        let (half, shoulder) = kind.size();
        let mut plot = Plot::new(Vec2::new(40.0, -20.0), std::f64::consts::FRAC_PI_2, half, shoulder);
        plot.height = 2.0;
        Site { kind, name: "TEST", plot }
    }

    #[test]
    fn every_place_is_fitted_within_its_plot() {
        for seed in 1..40u32 {
            for kind in [Kind::Gas, Kind::Pad, Kind::Crash, Kind::Military] {
                let s = site(kind);
                let o = lay_out(&mut Dice(seed), &s);
                assert!(!o.pieces.is_empty(), "{kind:?}: nothing there");
                for p in &o.pieces {
                    assert!(s.plot.outside(Vec2::new(p.at.x, p.at.z)) < 1.0, "seed {seed}: a {:?} off the {kind:?}", p.what);
                }
                for b in &o.buildings {
                    for c in b.footprint() {
                        assert!(s.plot.outside(c) < 0.01, "seed {seed}: a {:?} off the {kind:?}", b.plan.kind);
                    }
                }
                for (source, (x, z, _, _)) in &o.containers {
                    assert!(s.plot.outside(Vec2::new(*x, *z)) < 1.0, "seed {seed}: a {source:?} off the {kind:?}");
                }
            }
            // The range has its targets, the crash its cases, the camp its
            // cage.
            assert!(lay_out(&mut Dice(seed), &site(Kind::Pad)).targets.len() >= 6);
            assert!(lay_out(&mut Dice(seed), &site(Kind::Crash)).containers.iter().filter(|(s, _)| *s == Source::SupplyCase).count() >= 4);
            assert!(lay_out(&mut Dice(seed), &site(Kind::Military)).containers.iter().any(|(s, _)| *s == Source::Cage));
        }
    }

    #[test]
    fn a_crash_is_a_helicopter_or_a_plane() {
        let kinds: std::collections::HashSet<bool> = (1..40u32).map(|seed| lay_out(&mut Dice(seed.wrapping_mul(2_654_435_761) | 1), &site(Kind::Crash)).pieces.iter().any(|p| p.what == Scenery::Fixture(Fixture::PlaneFront))).collect();
        assert_eq!(kinds.len(), 2, "always the one kind of wreck");
    }

    #[test]
    fn a_tents_roof_peaks_in_its_middle() {
        let g = lntrn_model::Gltf::load(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/models/sites.glb")).expect("sites.glb");
        for name in ["SITE_CommandTent_Hull", "SITE_Tent_Hull"] {
            let node = g.nodes.iter().find(|n| n.name.as_deref() == Some(name)).unwrap_or_else(|| panic!("no {name}"));
            let mesh = &g.meshes[node.mesh.unwrap()];
            // The highest point near the middle, and out by the walls
            // (x across, y up as the game has it).
            let (mut middle, mut sides) = (f32::NEG_INFINITY, f32::NEG_INFINITY);
            for p in mesh.primitives.iter().flat_map(|p| p.positions.iter()) {
                if p[0].abs() < 0.4 {
                    middle = middle.max(p[1]);
                } else if p[0].abs() > 1.3 {
                    sides = sides.max(p[1]);
                }
            }
            assert!(middle > sides + 0.2, "{name}: the roof's {middle:.2} m up in the middle, {sides:.2} m by the walls");
        }
    }
}
