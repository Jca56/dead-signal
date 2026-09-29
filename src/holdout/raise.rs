//! Raising a holdout's arena from its layout (`layout.rs`): each building's
//! plan made and set down, furnished; the runs of wall built block by
//! block, capped, wired along the compound's top; the ways in for the
//! dead, the doors (and the heap) to buy open, the wall buys, each in its
//! zone; what stands about set down.

use lntrn_math::{Vec2, Vec3};

use super::arena::{Buy, Door, Window};
use super::layout::{Build, BuyAt, Gap, House, Layout, Line, Prop, Run, ew, ns, world, yaw};
use crate::collide::Surface;
use crate::loot::Dice;
use crate::loot::tables::Source;
use crate::map::Spot;
use crate::map::building::Building;
use crate::map::building::furnish;
use crate::map::building::plan::{self, Opening, Plan, Room, STOREY, Stair};
use crate::map::building::shape::{Block, RAISED, Rgb, Stuff};
use crate::map::scatter::{Piece, Scenery};
use crate::zombie::nav::Gate;

/// A window the dead come in by, in a building: how wide, from how high
/// to how high; a hole in a wall for them, the same.
const WINDOW: (f64, f64, f64) = (1.2, 0.9, 2.1);
const HOLE: (f64, f64, f64) = (1.4, 0.8, 2.1);
/// Where the dead stand outside a way in, and land inside it; how far out
/// they come from.
const OUTSIDE: f64 = 0.7;
const INSIDE: f64 = 0.8;
const COME_FROM: f64 = 11.0;
/// A door: how thick, how far past its doorway it reaches.
const DOOR_THICK: f64 = 0.16;
const DOOR_LAP: f64 = 0.08;
/// A heap of junk: how far out from its wall either way, how high.
const HEAP_OUT: f64 = 0.7;
const HEAP_HIGH: f64 = 2.4;
/// How high a wall buy's middle hangs over its floor.
const BUY_HEIGHT: f64 = 1.45;
/// A building's wall, each side of its line.
const SKIN: f64 = 0.1;
/// Kept clear of furniture: round where the dead land coming in, where a
/// wall buy's stood at, a door, the start.
const CLEAR: f64 = 1.5;

/// An arena raised from its layout.
#[derive(Default)]
pub struct Raised {
    pub buildings: Vec<Building>,
    pub blocks: Vec<Block>,
    pub windows: Vec<Window>,
    pub doors: Vec<Door>,
    pub buys: Vec<Buy>,
    pub pieces: Vec<Piece>,
    pub containers: Vec<(Source, Spot)>,
    pub start: (Vec3, f64),
    pub start_zone: usize,
}

/// Raise `layout`.
pub fn raise(layout: &Layout) -> Raised {
    let mut out = Raised::default();
    for h in layout.houses {
        house(layout, h, &mut out);
    }
    for run in layout.runs {
        wall(layout, run, &mut out);
    }
    for b in layout.buys {
        buy(layout, b, &mut out);
    }
    for p in layout.props {
        prop(p, &mut out);
    }
    let (x, z, bearing) = layout.start;
    let at = Vec2::new(x, z);
    out.start = (world(at, layout.floor_at(at, 0)), yaw(bearing));
    out.start_zone = layout.zone_at(at, 0).expect("a start in a zone");
    // The furniture, clear of where the dead land coming in, where a wall
    // buy's stood at, the doors and the start.
    let mut clear: Vec<Vec3> = out.windows.iter().map(|w| w.inside).collect();
    clear.extend(out.buys.iter().map(|b| b.at + b.facing * 0.6));
    clear.extend(out.doors.iter().map(|d| (d.lo + d.hi) * 0.5));
    clear.push(out.start.0);
    let near = |p: Vec3| clear.iter().any(|c| (c.y - p.y).abs() < 2.0 && Vec2::new(c.x - p.x, c.z - p.z).length() < CLEAR);
    for b in &out.buildings {
        let inside = furnish::furnish(b, &mut Dice(b.seed.rotate_left(9) | 1));
        out.pieces.extend(inside.pieces.into_iter().filter(|p| !near(p.at)));
        out.containers.extend(inside.containers.into_iter().filter(|(_, (x, z, _, y))| !near(Vec3::new(*x, *y - 1.5, *z))));
    }
    out
}

/// A building: its plan made, set down; the windows the dead come in by
/// and the doors to buy open, each with its zones.
fn house(layout: &Layout, h: &House, out: &mut Raised) {
    let rooms: Vec<Room> = h.rooms.iter().map(|r| Room { storey: r.0, x0: r.1, z0: r.2, x1: r.3, z1: r.4, use_: r.5 }).collect();
    let walls = (0..h.storeys).flat_map(|s| plan::walls_of(&rooms, s)).collect();
    let (w, d) = h.size;
    let mut plan = Plan { kind: h.kind, w, d, storeys: h.storeys, rooms, walls, openings: Vec::new(), stair: h.stair.map(|(x, z0)| Stair { x, z0 }), flat_roof: h.flat_roof, ridge_along_x: w >= d, ridge: plan::RIDGE, bars: Vec::new() };
    let wall_of = |plan: &Plan, on: Line, storey: u8| plan::wall_at(&plan.walls, storey, on.along_x, on.at, on.u).unwrap_or_else(|| panic!("{}: no wall on {on:?} (storey {storey})", h.name));
    for door in h.doors {
        let wall = wall_of(&plan, door.0, door.1);
        plan.openings.push(Opening { wall, centre: door.0.u, width: door.2, sill: 0.0, head: plan::DOOR_HEAD, door: true, boarded: false });
    }
    for win in h.windows {
        let wall = wall_of(&plan, win.0, win.1);
        plan.openings.push(Opening { wall, centre: win.0.u, width: WINDOW.0, sill: WINDOW.1, head: WINDOW.2, door: false, boarded: false });
    }
    let corner = Vec2::new(f64::from(h.at.0), f64::from(h.at.1));
    let b = Building { plan, origin: world(corner, RAISED), quarter: 0, seed: h.seed };
    let local = |p: Vec2, y: f64| b.world(Vec3::new(p.x, y, p.y));
    for win in h.windows.iter().filter(|w| w.2) {
        let on = win.0;
        // In is the side a room's on.
        let wall = b.plan.walls[wall_of(&b.plan, on, win.1)];
        let side = if wall.sides[1].is_some() { 1.0 } else { -1.0 };
        let (mid, inward) = (on.point(), on.square(side));
        let zone = layout.zone_at(corner + mid + inward * 0.5, win.1).unwrap_or_else(|| panic!("{}: a window into no zone at {on:?}", h.name));
        let floor = f64::from(win.1) * STOREY;
        out.windows.push(Window {
            zone,
            centre: local(mid, floor + WINDOW.1),
            inward: Vec3::new(inward.x, 0.0, inward.y),
            width: WINDOW.0,
            sill: WINDOW.1,
            head: WINDOW.2,
            outside: local(mid - inward * OUTSIDE, -RAISED),
            inside: local(mid + inward * INSIDE, floor),
            from: local(mid - inward * COME_FROM, -RAISED),
        });
    }
    for door in h.doors {
        let on = door.0;
        let mid = on.point();
        let (a, b_) = (layout.zone_at(corner + mid - on.square(0.5), door.1), layout.zone_at(corner + mid + on.square(0.5), door.1));
        let (Some(a), Some(b_)) = (a, b_) else { panic!("{}: a doorway to nowhere at {on:?}", h.name) };
        let Some(cost) = door.3 else {
            assert_eq!(a, b_, "{}: an open doorway at {on:?} joins zones {a} and {b_}", h.name);
            continue;
        };
        assert_ne!(a, b_, "{}: a door at {on:?} within zone {a}", h.name);
        let floor = f64::from(door.1) * STOREY;
        let half = door.2 * 0.5 + DOOR_LAP;
        let (p, q) = (local(mid - on.along() * half - on.square(DOOR_THICK * 0.5), floor), local(mid + on.along() * half + on.square(DOOR_THICK * 0.5), floor + plan::DOOR_HEAD));
        out.doors.push(Door { lo: p.min(q), hi: p.max(q), cost, zones: (a, b_), heap: false, solid: 0..0, gate: Gate::default() });
    }
    out.buildings.push(b);
}

/// Wall colours, as they look (sRGB): block, its cap, the wire along the
/// top, the gate.
const BLOCK: Rgb = [0.50, 0.49, 0.46];
const CAP: Rgb = [0.40, 0.39, 0.37];
const WIRE: Rgb = [0.17, 0.17, 0.16];
const GATE: Rgb = [0.25, 0.31, 0.25];
const GATE_DARK: Rgb = [0.16, 0.19, 0.16];
/// The wire over the compound's walls: its posts, how far apart. How high
/// over any wall (and over a heap) the ghost reaches that keeps anyone from
/// over it (off the generator, say, and onto a roof).
const WIRE_EVERY: f64 = 2.5;
const OVER: f64 = 2.5;

/// A run of wall: its stretches of block, capped; its holes (boarded by
/// the holdout, the dead coming in through them), its gate, a gap heaped
/// with junk (a door of its own); wire along the top of the compound's.
fn wall(layout: &Layout, run: &Run, out: &mut Raised) {
    let (t, h) = run.build.size();
    let line = |u: f64| if run.along_x { ew(run.at, u) } else { ns(run.at, u) };
    // A box from `a` to `b` along the run, `y0` to `y1` up, `thick`.
    let span = |a: f64, b: f64, y0: f64, y1: f64, thick: f64| {
        let (p, q) = (line(a).point() - line(a).square(thick * 0.5), line(b).point() + line(b).square(thick * 0.5));
        (world(p.min(q), y0), world(p.max(q), y1))
    };
    let mut add = |(lo, hi): (Vec3, Vec3), colour: Rgb, stuff: Stuff| out.blocks.push(Block { lo, hi, colour, stuff });
    let block = Stuff::Solid(Surface::Stone);
    let metal = Stuff::Solid(Surface::Metal);
    // Runs along x reach over the corners.
    let (from, to) = if run.along_x { (f64::from(run.from) - t * 0.5, f64::from(run.to) + t * 0.5) } else { (f64::from(run.from), f64::from(run.to)) };
    let mut along = from;
    let mut gaps: Vec<&Gap> = run.gaps.iter().collect();
    gaps.sort_by(|a, b| centre(a).total_cmp(&centre(b)));
    let perimeter = run.build == Build::Perimeter;
    let mut gate_at = None;
    for gap in gaps {
        let (u, w) = match *gap {
            Gap::Hole(u) => (u, HOLE.0),
            Gap::Gate(u, w) | Gap::Heap(u, w, _) => (u, w),
        };
        let (a, b) = (u - w * 0.5, u + w * 0.5);
        add(span(along, a, 0.0, h, t), BLOCK, block);
        add(span(along, a, h, h + 0.1, t + 0.06), CAP, block);
        along = b;
        match *gap {
            Gap::Hole(_) => {
                add(span(a, b, 0.0, HOLE.1, t), BLOCK, block);
                add(span(a, b, HOLE.2, h, t), BLOCK, block);
                add(span(a, b, h, h + 0.1, t + 0.06), CAP, block);
                add(span(a, b, HOLE.1, HOLE.2, 0.06), [0.0; 3], Stuff::Ghost);
                let mid = line(u).point();
                let inward = if layout.zone_at(mid + line(u).square(0.5), 0).is_some() { line(u).square(1.0) } else { line(u).square(-1.0) };
                let zone = layout.zone_at(mid + inward * 0.5, 0).unwrap_or_else(|| panic!("a hole into no zone at {:?}", line(u)));
                assert!(layout.zone_at(mid - inward * 0.5, 0).is_none(), "a hole at {:?} between zones", line(u));
                out.windows.push(Window {
                    zone,
                    centre: world(mid, HOLE.1),
                    inward: Vec3::new(inward.x, 0.0, inward.y),
                    width: HOLE.0,
                    sill: HOLE.1,
                    head: HOLE.2,
                    outside: world(mid - inward * OUTSIDE, 0.0),
                    inside: world(mid + inward * INSIDE, 0.0),
                    from: world(mid - inward * COME_FROM, 0.0),
                });
            }
            Gap::Gate(_, _) => {
                gate_at = Some((a, b));
                // A post each side; a kick plate low across it, bars over
                // it, rails; the leaves meeting in the middle, chained.
                for x in [a, b] {
                    add(span(x - 0.22, x + 0.22, 0.0, h + 0.3, t + 0.2), BLOCK, block);
                }
                add(span(a + 0.22, b - 0.22, 0.0, 0.9, 0.06), GATE, metal);
                let mut x = a + 0.4;
                while x < b - 0.3 {
                    add(span(x - 0.025, x + 0.025, 0.9, 2.7, 0.05), GATE, metal);
                    x += 0.18;
                }
                for y in [0.9, 1.8, 2.7] {
                    add(span(a + 0.22, b - 0.22, y, y + 0.08, 0.08), GATE_DARK, metal);
                }
                add(span(u - 0.04, u + 0.04, 0.0, 2.78, 0.1), GATE_DARK, metal);
                add(span(u - 0.12, u + 0.12, 1.15, 1.45, 0.14), WIRE, metal);
                add(span(a, b, 0.0, h + OVER, 0.1), [0.0; 3], Stuff::Ghost);
            }
            Gap::Heap(u, w, cost) => {
                let mid = line(u).point();
                let (za, zb) = (layout.zone_at(mid - line(u).square(0.5), 0), layout.zone_at(mid + line(u).square(0.5), 0));
                let (Some(za), Some(zb)) = (za, zb) else { panic!("a heap to nowhere at {:?}", line(u)) };
                assert_ne!(za, zb, "a heap at {:?} within zone {za}", line(u));
                let (lo, hi) = span(u - w * 0.5, u + w * 0.5, 0.0, HEAP_HIGH, HEAP_OUT * 2.0);
                out.doors.push(Door { lo, hi, cost, zones: (za, zb), heap: true, solid: 0..0, gate: Gate::default() });
                add(span(u - w * 0.5, u + w * 0.5, HEAP_HIGH, h + OVER, t), [0.0; 3], Stuff::Ghost);
            }
        }
    }
    add(span(along, to, 0.0, h, t), BLOCK, block);
    add(span(along, to, h, h + 0.1, t + 0.06), CAP, block);
    if perimeter {
        // Wire along the top (not over the gate): posts, two strands.
        let clear_of_gate = |a: f64, b: f64| gate_at.is_none_or(|(ga, gb)| b <= ga || a >= gb);
        let mut x = from + 0.2;
        while x < to {
            if clear_of_gate(x - 0.1, x + 0.1) {
                add(span(x - 0.03, x + 0.03, h + 0.1, h + 0.75, 0.06), WIRE, metal);
            }
            x += WIRE_EVERY;
        }
        let strands: Vec<(f64, f64)> = match gate_at {
            Some((ga, gb)) => vec![(from, ga - 0.22), (gb + 0.22, to)],
            None => vec![(from, to)],
        };
        for (a, b) in strands {
            for y in [h + 0.4, h + 0.7] {
                add(span(a, b, y, y + 0.03, 0.03), WIRE, metal);
            }
        }
    }
    // Nobody over it.
    add(span(from, to, h, h + OVER, t), [0.0; 3], Stuff::Ghost);
}

fn centre(gap: &Gap) -> f64 {
    match *gap {
        Gap::Hole(u) | Gap::Gate(u, _) | Gap::Heap(u, _, _) => u,
    }
}

/// A wall buy: at its line, out from the wall's face, at eye height over
/// the floor it's bought from; its zone.
fn buy(layout: &Layout, b: &BuyAt, out: &mut Raised) {
    let BuyAt(on, storey, face, wares) = *b;
    let p = on.point();
    let front = p + face.dir() * 0.5;
    // A building's wall, or a run of block.
    let off = if layout.house_at(p + face.dir() * 0.05).is_some() || layout.house_at(p - face.dir() * 0.05).is_some() {
        SKIN
    } else {
        let run = layout.runs.iter().find(|r| r.along_x == on.along_x && r.at == on.at && on.u >= f64::from(r.from) && on.u <= f64::from(r.to)).unwrap_or_else(|| panic!("a wall buy on no wall at {on:?}"));
        run.build.size().0 * 0.5
    };
    let zone = layout.zone_at(front, storey).unwrap_or_else(|| panic!("a wall buy in no zone at {on:?}"));
    let at = world(p + face.dir() * (off + 0.01), layout.floor_at(front, storey) + BUY_HEIGHT);
    out.buys.push(Buy { at, facing: Vec3::new(face.dir().x, 0.0, face.dir().y), wares, zone });
}

/// The generator's colours: its skid, its housing, its radiator and
/// exhaust.
const SKID: Rgb = [0.22, 0.22, 0.21];
const HOUSING: Rgb = [0.33, 0.37, 0.24];
const GRILLE: Rgb = [0.15, 0.15, 0.14];
const PANEL: Rgb = [0.55, 0.55, 0.52];

/// Something standing about, set down.
fn prop(p: &Prop, out: &mut Raised) {
    match *p {
        Prop::Tower(x, z) => {
            let at = world(Vec2::new(x, z), 0.0);
            out.pieces.push(Piece::new(Scenery::Tower, at, 0.0, 1.0));
            out.pieces.push(Piece::new(Scenery::Beacon, at, 0.0, 1.0));
        }
        Prop::Fixture(what, x, z, bearing) => out.pieces.push(Piece::new(Scenery::Fixture(what), world(Vec2::new(x, z), 0.0), yaw(bearing), 1.0)),
        Prop::Furn(what, x, z, bearing) => out.pieces.push(Piece::new(Scenery::Furn(what), world(Vec2::new(x, z), 0.0), yaw(bearing), 1.0)),
        Prop::Thing(source, x, z, bearing, over) => {
            let at = world(Vec2::new(x, z), 0.0);
            out.containers.push((source, (at.x, at.z, yaw(bearing), over)));
        }
        Prop::Generator(x, z, along_x) => {
            // A diesel set on its skid: the housing, the radiator at one
            // end, its panel on a side, the exhaust up off the top.
            let c = world(Vec2::new(x, z), 0.0);
            let (l, w) = (3.2, 1.5);
            let at = |u0: f64, v0: f64, y0: f64, u1: f64, v1: f64, y1: f64| {
                let (a, b) = if along_x { (Vec3::new(u0, y0, v0), Vec3::new(u1, y1, v1)) } else { (Vec3::new(v0, y0, u0), Vec3::new(v1, y1, u1)) };
                (c + a, c + b)
            };
            let mut add = |(lo, hi): (Vec3, Vec3), colour: Rgb| out.blocks.push(Block { lo, hi, colour, stuff: Stuff::Solid(Surface::Metal) });
            add(at(-l * 0.5, -w * 0.5, 0.0, l * 0.5, w * 0.5, 0.18), SKID);
            add(at(-l * 0.5 + 0.1, -w * 0.5 + 0.08, 0.18, l * 0.5 - 0.35, w * 0.5 - 0.08, 1.6), HOUSING);
            add(at(l * 0.5 - 0.35, -w * 0.5 + 0.08, 0.18, l * 0.5 - 0.1, w * 0.5 - 0.08, 1.5), GRILLE);
            add(at(-0.6, w * 0.5 - 0.08, 0.7, 0.2, w * 0.5 + 0.02, 1.3), PANEL);
            add(at(-l * 0.5 + 0.5, -0.12, 1.6, -l * 0.5 + 0.74, 0.12, 2.4), GRILLE);
        }
    }
}
