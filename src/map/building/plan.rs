//! A building's floor plan, in its own frame: whole metres, x across its
//! front (0 to `w`), z back from it (0 to `d`), the front (at z 0) facing
//! its street. Walls stand on the whole-metre lines, so the middle of every
//! room's metre squares is where the dead's walking grid probes; every
//! doorway is centred on a square's middle, and is sure to be walked
//! through.
//!
//! Rooms are found by cutting the floor in two again and again (a doorway
//! in every cut, so every room can be got to), and each wall is where two
//! rooms (or a room and the outside) meet. A house of two storeys has a
//! hall down its side with the stairs in it.

use crate::loot::Dice;

/// Floor to floor, floor to ceiling.
pub const STOREY: f64 = 3.0;
pub const CEILING: f64 = 2.8;
/// Doorways: how wide, how high.
pub const DOOR_WIDTH: f64 = 1.2;
pub const DOOR_HEAD: f64 = 2.1;
/// Windows: how high the sill, how high the head.
pub const SILL: f64 = 0.9;
pub const WINDOW_HEAD: f64 = 2.1;
/// A room's shortest side.
pub(super) const MIN_ROOM: i32 = 3;
/// How far a wall keeps from a doorway's middle in the wall it meets.
const DOOR_ROOM: f64 = 1.5;
/// The stairs: steps, and each step's depth (their rise is a storey over
/// the steps).
pub const STEPS: usize = 15;
pub const TREAD: f64 = 0.28;
/// The hall that holds the stairs, metres across.
const HALL: i32 = 3;
/// How high a gable's ridge stands over the eaves, usually.
pub const RIDGE: f64 = 2.2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    House,
    Store,
    /// Boarded up all round: not to be got into.
    Shell,
    /// Out in the country (`country.rs`): a barn, one tall room open at
    /// both ends; a hunter's cabin, of logs.
    Barn,
    Cabin,
    /// At the places out of town (`country.rs`): a gas station's garage
    /// (one bay, a wide door), a range's office, its armory (concrete,
    /// windowless).
    Garage,
    Office,
    Armory,
}

/// What a room is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Use {
    Living,
    Kitchen,
    Bed,
    Bath,
    Hall,
    Shop,
    Back,
    /// A farmhouse's or a cabin's living room: the gun cabinet stands in
    /// it, the stove warms it.
    Den,
    /// A barn's floor: hay, tools.
    Barn,
    /// A garage's bay; an office; an armory, its ammunition caged.
    Garage,
    Office,
    Armory,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Room {
    pub storey: u8,
    pub x0: i32,
    pub z0: i32,
    pub x1: i32,
    pub z1: i32,
    pub use_: Use,
}

impl Room {
    pub fn area(&self) -> i32 {
        (self.x1 - self.x0) * (self.z1 - self.z0)
    }
}

/// A stretch of wall: along x (standing at z `at`) or along z (at x
/// `at`), from `from` to `to`, on a storey; the room on each side (the
/// lower-coordinate side first), or the outside.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Wall {
    pub storey: u8,
    pub along_x: bool,
    pub at: i32,
    pub from: i32,
    pub to: i32,
    pub sides: [Option<usize>; 2],
}

impl Wall {
    pub fn outside(&self) -> bool {
        self.sides.iter().any(Option::is_none)
    }
}

/// A gap in a wall: its middle along it, how wide, from how high to how
/// high; a doorway (else a window); and whether it's been boarded up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Opening {
    pub wall: usize,
    pub centre: f64,
    pub width: f64,
    pub sill: f64,
    pub head: f64,
    pub door: bool,
    pub boarded: bool,
}

/// A straight flight up from the ground floor, against the wall at x
/// `x` (its well from `x` to `x + 1`), its first step at z `z0` and
/// climbing towards +z.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stair {
    pub x: i32,
    pub z0: f64,
}

impl Stair {
    /// Where the flight ends (the top step's back edge).
    pub fn z1(&self) -> f64 {
        self.z0 + STEPS as f64 * TREAD
    }
}

#[derive(Clone, Debug)]
pub struct Plan {
    pub kind: Kind,
    pub w: i32,
    pub d: i32,
    pub storeys: u8,
    pub rooms: Vec<Room>,
    pub walls: Vec<Wall>,
    pub openings: Vec<Opening>,
    pub stair: Option<Stair>,
    /// A flat roof (a store's), else a gable, its ridge along x or z and
    /// standing so high over the eaves.
    pub flat_roof: bool,
    pub ridge_along_x: bool,
    pub ridge: f64,
}

/// A cut to make across a region: along x (at z) or along z (at x), and
/// where its doorway is.
pub(super) struct Cut {
    pub along_x: bool,
    pub at: i32,
    pub door: f64,
}

pub(super) fn between(dice: &mut Dice, lo: i32, hi: i32) -> i32 {
    lo + (dice.next() % (hi - lo + 1) as u32) as i32
}

/// Cut `(x0, z0, x1, z1)` into rooms no bigger than about `most` square
/// metres, each cut with a doorway (`doors` are those already on the
/// region's edges, which no cut may come near).
fn cut_up(dice: &mut Dice, r: (i32, i32, i32, i32), most: i32, doors: &[(bool, i32, f64)], rooms: &mut Vec<(i32, i32, i32, i32)>, cuts: &mut Vec<Cut>) {
    let (x0, z0, x1, z1) = r;
    let (w, d) = (x1 - x0, z1 - z0);
    let can_x = w >= 2 * MIN_ROOM;
    let can_z = d >= 2 * MIN_ROOM;
    if w * d <= most || (!can_x && !can_z) {
        rooms.push(r);
        return;
    }
    // Across the longer way, mostly.
    let across_x = if can_x && can_z { w > d || (w == d && dice.unit() < 0.5) } else { can_x };
    // Where it can go: rooms either side big enough, and clear of every
    // doorway on the edges it runs to.
    let (lo, hi) = if across_x { (x0 + MIN_ROOM, x1 - MIN_ROOM) } else { (z0 + MIN_ROOM, z1 - MIN_ROOM) };
    let clear = |k: i32| doors.iter().all(|&(along_x, at, c)| {
        // A doorway in an edge the cut meets: an edge along the cut's own
        // line doesn't count.
        let meets = if across_x { along_x && (at == z0 || at == z1) } else { !along_x && (at == x0 || at == x1) };
        !meets || (f64::from(k) - c).abs() >= DOOR_ROOM
    });
    let options: Vec<i32> = (lo..=hi).filter(|&k| clear(k)).collect();
    if options.is_empty() {
        rooms.push(r);
        return;
    }
    let k = options[dice.next() as usize % options.len()];
    // The cut runs along z (at x = k) when cutting across x.
    let (from, to) = if across_x { (z0, z1) } else { (x0, x1) };
    let door = f64::from(between(dice, from + 1, to - 2)) + 0.5;
    let cut = Cut { along_x: !across_x, at: k, door };
    let mut inner: Vec<(bool, i32, f64)> = doors.to_vec();
    inner.push((cut.along_x, cut.at, cut.door));
    cuts.push(cut);
    let (a, b) = if across_x { ((x0, z0, k, z1), (k, z0, x1, z1)) } else { ((x0, z0, x1, k), (x0, k, x1, z1)) };
    cut_up(dice, a, most, &inner, rooms, cuts);
    cut_up(dice, b, most, &inner, rooms, cuts);
}

/// Every wall on a storey: wherever a room's edge is, split where what's
/// on the other side changes.
pub(super) fn walls_of(rooms: &[Room], storey: u8) -> Vec<Wall> {
    let on: Vec<(usize, &Room)> = rooms.iter().enumerate().filter(|(_, r)| r.storey == storey).collect();
    let mut lines: Vec<(bool, i32)> = Vec::new();
    for (_, r) in &on {
        lines.extend([(true, r.z0), (true, r.z1), (false, r.x0), (false, r.x1)]);
    }
    lines.sort_unstable();
    lines.dedup();
    let mut out = Vec::new();
    for (along_x, at) in lines {
        // Each room with an edge on this line: which side of it it lies,
        // and the stretch.
        let mut spans: Vec<(usize, usize, i32, i32)> = Vec::new();
        for &(i, r) in &on {
            let (lo_edge, hi_edge, from, to) = if along_x { (r.z0, r.z1, r.x0, r.x1) } else { (r.x0, r.x1, r.z0, r.z1) };
            if hi_edge == at {
                spans.push((i, 0, from, to));
            }
            if lo_edge == at {
                spans.push((i, 1, from, to));
            }
        }
        let mut cuts: Vec<i32> = spans.iter().flat_map(|s| [s.2, s.3]).collect();
        cuts.sort_unstable();
        cuts.dedup();
        for w in cuts.windows(2) {
            let (a, b) = (w[0], w[1]);
            let mut sides = [None, None];
            for &(i, side, from, to) in &spans {
                if from <= a && to >= b {
                    sides[side] = Some(i);
                }
            }
            if sides[0].is_none() && sides[1].is_none() {
                continue;
            }
            // Where the stretch goes on with the same both sides, it's one
            // wall.
            if let Some(last) = out.last_mut()
                && let Wall { storey: s, along_x: ax, at: t, to, sides: sd, .. } = last
                && *s == storey && *ax == along_x && *t == at && *to == a && *sd == sides
            {
                *to = b;
                continue;
            }
            out.push(Wall { storey, along_x, at, from: a, to: b, sides });
        }
    }
    out
}

/// The wall on `storey` along the line `(along_x, at)` holding `centre`,
/// with a room each side (for a doorway between them).
fn wall_at(walls: &[Wall], storey: u8, along_x: bool, at: i32, centre: f64) -> Option<usize> {
    walls.iter().position(|w| w.storey == storey && w.along_x == along_x && w.at == at && f64::from(w.from) < centre && f64::from(w.to) > centre)
}

/// Spots along wall `w` for gaps `width` wide, whole-metre middles half a
/// metre in, every `every` metres, clear of the ends and of the gaps
/// already in it.
pub(super) fn spots_along(plan: &Plan, w: usize, width: f64, every: f64) -> Vec<f64> {
    let wall = plan.walls[w];
    let margin = 0.6 + width * 0.5;
    let taken: Vec<(f64, f64)> = plan.openings.iter().filter(|o| o.wall == w).map(|o| (o.centre, o.width)).collect();
    let mut out: Vec<f64> = Vec::new();
    let mut c = f64::from(wall.from) + 0.5;
    while c < f64::from(wall.to) {
        let fits = c - margin >= f64::from(wall.from) && c + margin <= f64::from(wall.to);
        let apart = taken.iter().chain(out.iter().map(|c| (*c, width)).collect::<Vec<_>>().iter()).all(|&(t, tw)| (t - c).abs() >= (tw + width) * 0.5 + every - 1.0);
        if fits && apart {
            out.push(c);
        }
        c += 1.0;
    }
    out
}

/// A house `w` by `d`, of one storey or two.
pub fn house(dice: &mut Dice, w: i32, d: i32, two: bool) -> Plan {
    let mut plan = Plan { kind: Kind::House, w, d, storeys: 1, rooms: Vec::new(), walls: Vec::new(), openings: Vec::new(), stair: None, flat_roof: false, ridge_along_x: w >= d, ridge: RIDGE };
    let two = two && d >= 8 && w >= HALL + 2 * MIN_ROOM;
    let mut cuts = Vec::new();
    let mut rects = Vec::new();
    // Two storeys: a hall down one side, the stairs in it, the same on
    // both floors; the rest cut into rooms on each.
    let hall_left = dice.unit() < 0.5;
    let rest = if two {
        plan.storeys = 2;
        let (hx0, hx1) = if hall_left { (0, HALL) } else { (w - HALL, w) };
        let at = if hall_left { HALL } else { w - HALL };
        for s in 0..2u8 {
            plan.rooms.push(Room { storey: s, x0: hx0, z0: 0, x1: hx1, z1: d, use_: Use::Hall });
        }
        let door_z = f64::from(between(dice, 1, d - 2)) + 0.5;
        cuts.push((0u8, Cut { along_x: false, at, door: door_z }));
        let stair_x = if hall_left { 0 } else { w - 1 };
        plan.stair = Some(Stair { x: stair_x, z0: 1.0 });
        if hall_left { (HALL, 0, w, d) } else { (0, 0, w - HALL, d) }
    } else {
        (0, 0, w, d)
    };
    for s in 0..plan.storeys {
        rects.clear();
        let mut here = Vec::new();
        let edge_doors: Vec<(bool, i32, f64)> = cuts.iter().filter(|(st, _)| *st == 0).map(|(_, c)| (c.along_x, c.at, c.door)).collect();
        // Upstairs, a doorway off the hall into each part cut off it comes
        // as a cut of its own.
        let most = if s == 0 { 22 } else { 16 };
        cut_up(dice, rest, most, &edge_doors, &mut rects, &mut here);
        for c in here {
            cuts.push((s, c));
        }
        let first = plan.rooms.len();
        for &(x0, z0, x1, z1) in &rects {
            plan.rooms.push(Room { storey: s, x0, z0, x1, z1, use_: Use::Bed });
        }
        // What each room is for. Downstairs: the biggest on the front the
        // living room (the way in is through it), the kitchen beside it if
        // it can be, the smallest a bathroom if there are three; the rest
        // bedrooms. Upstairs: bedrooms, the smallest a bathroom.
        let mine: Vec<usize> = (first..plan.rooms.len()).collect();
        let by_size = |v: &mut Vec<usize>, rooms: &[Room]| v.sort_by_key(|&i| (-rooms[i].area(), i));
        let mut left = mine.clone();
        by_size(&mut left, &plan.rooms);
        let take = |left: &mut Vec<usize>, pick: &dyn Fn(&Room) -> bool, rooms: &[Room]| left.iter().position(|&i| pick(&rooms[i])).map(|k| left.remove(k));
        if s == 0 {
            let living = take(&mut left, &|r: &Room| r.z0 == 0, &plan.rooms).or_else(|| take(&mut left, &|_| true, &plan.rooms));
            if let Some(l) = living {
                plan.rooms[l].use_ = Use::Living;
                let lr = plan.rooms[l];
                let touches = move |r: &Room| (r.x0 == lr.x1 || r.x1 == lr.x0) && r.z0 < lr.z1 && r.z1 > lr.z0 || (r.z0 == lr.z1 || r.z1 == lr.z0) && r.x0 < lr.x1 && r.x1 > lr.x0;
                if let Some(k) = take(&mut left, &touches, &plan.rooms).or_else(|| take(&mut left, &|_| true, &plan.rooms)) {
                    plan.rooms[k].use_ = Use::Kitchen;
                }
            }
        }
        if left.len() >= 2 || (s > 0 && !left.is_empty()) || (s == 0 && left.len() == 1 && mine.len() >= 3) {
            let smallest = left.pop().expect("not empty");
            plan.rooms[smallest].use_ = Use::Bath;
        }
        for i in left {
            plan.rooms[i].use_ = Use::Bed;
        }
        // Upstairs, every room off the hall has a doorway to it (downstairs
        // the hall's one cut does, and the rooms beyond join through their
        // own).
        if s == 1 && two {
            let at = if hall_left { HALL } else { w - HALL };
            for &(x0, z0, x1, z1) in &rects {
                if x0 == at || x1 == at {
                    let door = f64::from(between(dice, z0 + 1, z1 - 2)) + 0.5;
                    cuts.push((1, Cut { along_x: false, at, door }));
                }
            }
        }
    }
    finish(dice, plan, &cuts)
}

/// A store `w` by `d`: the shop floor at the front, a back room.
pub fn store(dice: &mut Dice, w: i32, d: i32) -> Plan {
    let mut plan = Plan { kind: Kind::Store, w, d, storeys: 1, rooms: Vec::new(), walls: Vec::new(), openings: Vec::new(), stair: None, flat_roof: true, ridge_along_x: true, ridge: RIDGE };
    let back = 4.min(d - MIN_ROOM - 5).max(MIN_ROOM);
    plan.rooms.push(Room { storey: 0, x0: 0, z0: 0, x1: w, z1: d - back, use_: Use::Shop });
    plan.rooms.push(Room { storey: 0, x0: 0, z0: d - back, x1: w, z1: d, use_: Use::Back });
    let door = f64::from(between(dice, 1, w - 2)) + 0.5;
    finish(dice, plan, &[(0, Cut { along_x: true, at: d - back, door })])
}

/// A house boarded up all round, nothing inside worth drawing.
pub fn shell(dice: &mut Dice, w: i32, d: i32) -> Plan {
    let mut plan = Plan { kind: Kind::Shell, w, d, storeys: 1, rooms: vec![Room { storey: 0, x0: 0, z0: 0, x1: w, z1: d, use_: Use::Hall }], walls: Vec::new(), openings: Vec::new(), stair: None, flat_roof: false, ridge_along_x: w >= d, ridge: RIDGE };
    plan.walls = walls_of(&plan.rooms, 0);
    for i in 0..plan.walls.len() {
        let spots = spots_along(&plan, i, 1.2, 3.0);
        for c in spots {
            plan.openings.push(Opening { wall: i, centre: c, width: 1.2, sill: SILL, head: WINDOW_HEAD, door: false, boarded: true });
        }
    }
    let _ = dice;
    plan
}

/// The walls from the rooms, the doorways the cuts made, and the doors
/// and windows out.
pub(super) fn finish(dice: &mut Dice, mut plan: Plan, cuts: &[(u8, Cut)]) -> Plan {
    for s in 0..plan.storeys {
        plan.walls.extend(walls_of(&plan.rooms, s));
    }
    for (s, c) in cuts {
        if let Some(w) = wall_at(&plan.walls, *s, c.along_x, c.at, c.door)
            && plan.walls[w].sides.iter().all(Option::is_some)
        {
            plan.openings.push(Opening { wall: w, centre: c.door, width: DOOR_WIDTH, sill: 0.0, head: DOOR_HEAD, door: true, boarded: false });
        }
    }
    let store = plan.kind == Kind::Store;
    // The way in, at the front: into the living room (or den), the hall,
    // the shop.
    let front: Vec<usize> = (0..plan.walls.len())
        .filter(|&i| {
            let w = plan.walls[i];
            w.storey == 0 && w.along_x && w.at == 0 && w.sides[1].is_some_and(|r| matches!(plan.rooms[r].use_, Use::Living | Use::Den | Use::Hall | Use::Shop | Use::Office | Use::Armory))
        })
        .collect();
    // (Whichever stretch of the front has room for it; failing that, any
    // wall out of the ground floor: there's always a way in.)
    let mut front_door = None;
    let anywhere: Vec<usize> = (0..plan.walls.len()).filter(|&i| plan.walls[i].storey == 0 && plan.walls[i].outside()).collect();
    for &w in front.iter().chain(anywhere.iter()) {
        let spots = spots_along(&plan, w, DOOR_WIDTH, 2.0);
        if !spots.is_empty() {
            let c = spots[dice.next() as usize % spots.len()];
            plan.openings.push(Opening { wall: w, centre: c, width: DOOR_WIDTH, sill: 0.0, head: DOOR_HEAD, door: true, boarded: false });
            front_door = Some(plan.openings.len() - 1);
            break;
        }
    }
    // A way out the back, most of the time (from the kitchen, the back
    // room, the hall).
    let back: Vec<usize> = (0..plan.walls.len())
        .filter(|&i| {
            let w = plan.walls[i];
            w.storey == 0 && w.along_x && w.at == plan.d && w.sides[0].is_some_and(|r| matches!(plan.rooms[r].use_, Use::Kitchen | Use::Back | Use::Hall))
        })
        .collect();
    let mut back_door = false;
    if !back.is_empty() && dice.unit() < 0.65 {
        let w = back[dice.next() as usize % back.len()];
        let spots = spots_along(&plan, w, DOOR_WIDTH, 2.0);
        if !spots.is_empty() {
            let c = spots[dice.next() as usize % spots.len()];
            plan.openings.push(Opening { wall: w, centre: c, width: DOOR_WIDTH, sill: 0.0, head: DOOR_HEAD, door: true, boarded: false });
            back_door = true;
        }
    }
    // Boarded up at the front, now and then, if there's another way in.
    if back_door
        && let Some(i) = front_door
        && dice.unit() < 0.2
    {
        plan.openings[i].boarded = true;
    }
    // Windows in every outside wall: a shop's front all glass.
    for i in 0..plan.walls.len() {
        let w = plan.walls[i];
        if !w.outside() {
            continue;
        }
        let shop_front = store && w.along_x && w.at == 0;
        let (width, every) = if shop_front { (2.2, 3.0) } else { (1.2, 3.5) };
        let bath = w.sides.iter().flatten().any(|&r| plan.rooms[r].use_ == Use::Bath);
        let width = if bath { 0.8 } else { width };
        for c in spots_along(&plan, i, width, every) {
            if dice.unit() < 0.15 {
                continue;
            }
            let sill = if bath { 1.4 } else { SILL };
            plan.openings.push(Opening { wall: i, centre: c, width, sill, head: WINDOW_HEAD, door: false, boarded: dice.unit() < 0.22 });
        }
    }
    plan
}
