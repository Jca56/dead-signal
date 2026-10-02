//! The town's landmarks, planned the way its houses are (`plan.rs`), each
//! on a lot of its own (`town.rs`):
//!
//! - The gun store: its floor, glass cases across it and racks on its
//!   walls, all glass at the front and blind down its sides; a back room
//!   (the locked gun cage in it) through a doorway.
//! - The police station: a lobby at the front (the front desk), the
//!   bullpen beside it; behind them the cell block (a corridor, two cells
//!   behind bars), the locker room, and the armory (windowless, its cage
//!   locked).
//! - The fire station: the engine bay (two wide doors at its front, the
//!   engine in it) down one side; the watch office, the day room and the
//!   locker room down the other.
//! - The school, of two storeys: a stairwell at one end, a corridor lined
//!   with lockers down its middle on both floors; below, the office, a
//!   classroom, the cafeteria, the nurse's office and the gym; above,
//!   classrooms.

use super::plan::{self, Cut, Kind, Opening, Plan, Room, Use};
use crate::loot::Dice;

/// One of the town's landmarks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Landmark {
    GunStore,
    Police,
    FireStation,
    School,
}

impl Landmark {
    pub const ALL: [Landmark; 4] = [Landmark::School, Landmark::GunStore, Landmark::Police, Landmark::FireStation];

    /// Its size, metres across its front and back from it.
    pub fn size(self) -> (i32, i32) {
        match self {
            Landmark::GunStore => (14, 14),
            Landmark::Police => (20, 15),
            Landmark::FireStation => (18, 16),
            Landmark::School => (30, 20),
        }
    }

    /// Its plan.
    pub fn plan(self, dice: &mut Dice) -> Plan {
        let (w, d) = self.size();
        match self {
            Landmark::GunStore => gun_store(dice, w, d),
            Landmark::Police => police(dice, w, d),
            Landmark::FireStation => fire_station(dice, w, d),
            Landmark::School => school(dice, w, d),
        }
    }
}

/// The engine bay's doors: how wide, how high.
pub const BAY_DOOR: (f64, f64) = (3.2, 2.6);

/// A cell's windows: small, and high.
const CELL_WINDOW: (f64, f64, f64) = (0.6, 1.9, 2.4);

/// How a room of each use is let in on: windows how wide, from how high
/// to how high (none, if it has none).
fn windows_of(use_: Use) -> Option<(f64, f64, f64)> {
    match use_ {
        Use::Armory | Use::CellBlock | Use::Back | Use::GunShop | Use::Corridor => None,
        Use::Cell | Use::LockerRoom => Some(CELL_WINDOW),
        Use::Bay | Use::Gym => Some(HIGH_WINDOW),
        _ => Some((1.2, plan::SILL, plan::WINDOW_HEAD)),
    }
}

/// A bay's and a gym's windows: wide, and high.
const HIGH_WINDOW: (f64, f64, f64) = (1.2, 1.8, 2.5);

/// A plan of `kind`, `w` by `d`, of `storeys`, a flat roof, with `rooms`.
fn empty(kind: Kind, w: i32, d: i32, storeys: u8, rooms: Vec<Room>) -> Plan {
    Plan { kind, w, d, storeys, cellars: 0, rooms, walls: Vec::new(), openings: Vec::new(), stairs: Vec::new(), flat_roof: true, ridge_along_x: true, ridge: plan::RIDGE, bars: Vec::new(), grim: false }
}

fn room(x0: i32, z0: i32, x1: i32, z1: i32, use_: Use) -> Room {
    Room { storey: 0, x0, z0, x1, z1, use_ }
}

fn upstairs(x0: i32, z0: i32, x1: i32, z1: i32, use_: Use) -> Room {
    Room { storey: 1, x0, z0, x1, z1, use_ }
}

/// A doorway at `centre` along an outside wall of the room `r` on the
/// line `(along_x, at)`, if there's room for one. Whether it went in.
fn door_out(dice: &mut Dice, plan: &mut Plan, r: usize, along_x: bool, at: i32) -> bool {
    let walls: Vec<usize> = (0..plan.walls.len()).filter(|&i| {
        let w = plan.walls[i];
        w.storey == 0 && w.along_x == along_x && w.at == at && w.outside() && w.sides.contains(&Some(r))
    }).collect();
    for w in walls {
        let spots = plan::spots_along(plan, w, plan::DOOR_WIDTH, 2.0);
        if !spots.is_empty() {
            let c = spots[dice.next() as usize % spots.len()];
            plan.openings.push(Opening { wall: w, centre: c, width: plan::DOOR_WIDTH, sill: 0.0, head: plan::DOOR_HEAD, door: true, boarded: false });
            return true;
        }
    }
    false
}

/// The walls from the rooms (every storey), a doorway at every cut (on
/// its storey), the way in at the front into the room of `front`, and a
/// way out the back from `back` (most of the time).
fn doors(dice: &mut Dice, plan: &mut Plan, cuts: &[(u8, Cut)], front: Use, back: Option<Use>) {
    plan.walls = (0..plan.storeys).flat_map(|s| plan::walls_of(&plan.rooms, s)).collect();
    for (s, c) in cuts {
        if let Some(w) = plan::wall_at(&plan.walls, *s, c.along_x, c.at, c.door)
            && plan.walls[w].sides.iter().all(Option::is_some)
        {
            plan.openings.push(Opening { wall: w, centre: c.door, width: plan::DOOR_WIDTH, sill: 0.0, head: plan::DOOR_HEAD, door: true, boarded: false });
        }
    }
    if let Some(r) = find(plan, front) {
        door_out(dice, plan, r, true, 0);
    }
    if let Some(r) = back.and_then(|u| find(plan, u))
        && dice.unit() < 0.7
    {
        let d = plan.d;
        door_out(dice, plan, r, true, d);
    }
}

/// The first room of `use_` on the ground floor.
fn find(plan: &Plan, use_: Use) -> Option<usize> {
    plan.rooms.iter().position(|r| r.use_ == use_ && r.storey == 0)
}

/// Windows in every outside wall as its room has them (a shop's front all
/// glass, if it's a store).
fn windows(dice: &mut Dice, mut plan: Plan, glass_front: bool) -> Plan {
    for i in 0..plan.walls.len() {
        let w = plan.walls[i];
        if !w.outside() {
            continue;
        }
        let Some(r) = w.sides.iter().flatten().next().copied() else { continue };
        let shop_front = glass_front && w.along_x && w.at == 0 && w.storey == 0;
        let (width, sill, head, every) = match (shop_front, windows_of(plan.rooms[r].use_)) {
            (true, _) => (2.2, plan::SILL, plan::WINDOW_HEAD, 3.0),
            (false, Some((width, sill, head))) => (width, sill, head, 3.5),
            (false, None) => continue,
        };
        for c in plan::spots_along(&plan, i, width, every) {
            plan.openings.push(Opening { wall: i, centre: c, width, sill, head, door: false, boarded: dice.unit() < 0.15 });
        }
    }
    plan
}

/// A doorway's middle somewhere along `from..to` (on a square's middle,
/// clear of the ends).
fn along(dice: &mut Dice, from: i32, to: i32) -> f64 {
    f64::from(plan::between(dice, from + 1, to - 2)) + 0.5
}

/// The gun store, `w` by `d`: the floor at the front, a back room four
/// deep.
pub fn gun_store(dice: &mut Dice, w: i32, d: i32) -> Plan {
    let back = 4;
    let rooms = vec![room(0, 0, w, d - back, Use::GunShop), room(0, d - back, w, d, Use::Back)];
    let cut = Cut { along_x: true, at: d - back, door: along(dice, 0, w) };
    let mut plan = empty(Kind::GunStore, w, d, 1, rooms);
    doors(dice, &mut plan, &[(0, cut)], Use::GunShop, Some(Use::Back));
    windows(dice, plan, true)
}

/// The police station, `w` (at least 18) by `d` (at least 14): the lobby
/// and the bullpen along the front, six deep; behind them the cell block
/// (its corridor three deep, two cells behind bars), the locker room and
/// the armory.
pub fn police(dice: &mut Dice, w: i32, d: i32) -> Plan {
    let front = 6;
    let (lobby, block) = (7, 8);
    let hall = front + 3;
    let lockers = block + (w - block) / 2;
    let rooms = vec![
        room(0, 0, lobby, front, Use::Lobby),
        room(lobby, 0, w, front, Use::Office),
        room(0, front, block, hall, Use::CellBlock),
        room(0, hall, block / 2, d, Use::Cell),
        room(block / 2, hall, block, d, Use::Cell),
        room(block, front, lockers, d, Use::LockerRoom),
        room(lockers, front, w, d, Use::Armory),
    ];
    let cuts = [
        // The lobby into the bullpen, and back to the cells.
        Cut { along_x: false, at: lobby, door: along(dice, 0, front) },
        Cut { along_x: true, at: front, door: along(dice, 0, lobby) },
        // Each cell's door, off the corridor.
        Cut { along_x: true, at: hall, door: f64::from(block / 4) + 0.5 },
        Cut { along_x: true, at: hall, door: f64::from(block / 2 + block / 4) + 0.5 },
        // The bullpen into the locker room and the armory.
        Cut { along_x: true, at: front, door: along(dice, block, lockers) },
        Cut { along_x: true, at: front, door: along(dice, lockers, w) },
    ]
    .map(|c| (0, c));
    let mut plan = empty(Kind::Police, w, d, 1, rooms);
    doors(dice, &mut plan, &cuts, Use::Lobby, Some(Use::LockerRoom));
    let mut plan = windows(dice, plan, false);
    // The cells' fronts are bars.
    plan.bars = (0..plan.walls.len())
        .filter(|&i| {
            let wall = plan.walls[i];
            wall.along_x && wall.at == hall && wall.sides.iter().flatten().any(|&r| plan.rooms[r].use_ == Use::Cell) && wall.sides.iter().flatten().any(|&r| plan.rooms[r].use_ == Use::CellBlock)
        })
        .collect();
    plan
}

/// The fire station, `w` (at least 16) by `d` (at least 14): the engine
/// bay down one side, ten wide, the whole depth, two wide doors in its
/// front; down the other, the watch office at the front (the way in), the
/// day room, the locker room.
pub fn fire_station(dice: &mut Dice, w: i32, d: i32) -> Plan {
    let bay = 10;
    let (office, day) = (5, 10);
    let rooms = vec![
        room(0, 0, bay, d, Use::Bay),
        room(bay, 0, w, office, Use::Office),
        room(bay, office, w, day, Use::Kitchen),
        room(bay, day, w, d, Use::LockerRoom),
    ];
    let cuts = [
        // Each room on the side into the bay, and on down the side.
        Cut { along_x: false, at: bay, door: along(dice, 0, office) },
        Cut { along_x: false, at: bay, door: along(dice, office, day) },
        Cut { along_x: false, at: bay, door: along(dice, day, d) },
        Cut { along_x: true, at: office, door: along(dice, bay, w) },
    ]
    .map(|c| (0, c));
    let mut plan = empty(Kind::FireStation, w, d, 1, rooms);
    doors(dice, &mut plan, &cuts, Use::Office, Some(Use::Bay));
    // The bay's doors, wide and high, in its front.
    if let Some(front) = (0..plan.walls.len()).find(|&i| {
        let wall = plan.walls[i];
        wall.along_x && wall.at == 0 && wall.sides[1] == Some(0)
    }) {
        for centre in [f64::from(bay) * 0.25, f64::from(bay) * 0.75] {
            plan.openings.push(Opening { wall: front, centre, width: BAY_DOOR.0, sill: 0.0, head: BAY_DOOR.1, door: true, boarded: false });
        }
    }
    windows(dice, plan, false)
}

/// The school, `w` (at least 26) by `d` (at least 18), of two storeys: the
/// stairwell at its left end, three across, the whole depth (the way in
/// at its front); a corridor three deep down its middle, the rooms off it
/// front and back. Below, the office, a classroom, the cafeteria and the
/// nurse's office, and the gym at its right end, the whole depth; above,
/// classrooms.
pub fn school(dice: &mut Dice, w: i32, d: i32) -> Plan {
    let stairs = 3;
    let (c0, c1) = (d / 2 - 2, d / 2 + 1);
    let gym = w - 11;
    let mid = stairs + (gym - stairs) / 2;
    let third = |k: i32| stairs + (w - stairs) * k / 3;
    let mut rooms = vec![
        room(0, 0, stairs, d, Use::Hall),
        upstairs(0, 0, stairs, d, Use::Hall),
        room(stairs, c0, gym, c1, Use::Corridor),
        upstairs(stairs, c0, w, c1, Use::Corridor),
        room(stairs, 0, mid, c0, Use::Office),
        room(mid, 0, gym, c0, Use::Classroom),
        room(stairs, c1, mid + 2, d, Use::Kitchen),
        room(mid + 2, c1, gym, d, Use::Nurse),
        room(gym, 0, w, d, Use::Gym),
    ];
    for k in 0..3 {
        rooms.push(upstairs(third(k), 0, third(k + 1), c0, Use::Classroom));
        rooms.push(upstairs(third(k), c1, third(k + 1), d, Use::Classroom));
    }
    let door = f64::from(c0) + 1.5;
    let mut cuts = vec![
        // The stairwell into the corridor, on both floors; the gym off its
        // end below.
        (0, Cut { along_x: false, at: stairs, door }),
        (1, Cut { along_x: false, at: stairs, door }),
        (0, Cut { along_x: false, at: gym, door }),
    ];
    // Every room off the corridor has its door into it.
    for r in &rooms {
        if matches!(r.use_, Use::Hall | Use::Corridor | Use::Gym) {
            continue;
        }
        let at = if r.z1 == c0 { c0 } else { c1 };
        cuts.push((r.storey, Cut { along_x: true, at, door: along(dice, r.x0, r.x1) }));
    }
    let mut plan = empty(Kind::School, w, d, 2, rooms);
    plan.stairs = vec![plan::Stair { storey: 0, x: 0, z0: f64::from(c1) + 1.0 }];
    doors(dice, &mut plan, &cuts, Use::Hall, Some(Use::Kitchen));
    // The gym's own doors out, at its front.
    if let Some(g) = find(&plan, Use::Gym) {
        door_out(dice, &mut plan, g, true, 0);
    }
    windows(dice, plan, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The rooms of `plan` joined by its doorways (and its stairs, the
    /// halls on each floor), from the way in: how many can be got to.
    fn reached(plan: &Plan) -> usize {
        let mut seen = vec![false; plan.rooms.len()];
        let mut todo: Vec<usize> = plan.openings.iter().filter(|o| o.door && !o.boarded && plan.walls[o.wall].outside() && plan.walls[o.wall].at == 0 && plan.walls[o.wall].along_x).filter_map(|o| plan.walls[o.wall].sides.iter().flatten().next().copied()).collect();
        while let Some(r) = todo.pop() {
            if std::mem::replace(&mut seen[r], true) {
                continue;
            }
            for o in plan.openings.iter().filter(|o| o.door && !o.boarded) {
                let s = plan.walls[o.wall].sides;
                if s.contains(&Some(r)) {
                    todo.extend(s.iter().flatten().copied());
                }
            }
            if !plan.stairs.is_empty() && plan.rooms[r].use_ == Use::Hall {
                todo.extend((0..plan.rooms.len()).filter(|&k| plan.rooms[k].use_ == Use::Hall));
            }
        }
        seen.iter().filter(|&&s| s).count()
    }

    #[test]
    fn every_room_of_a_landmark_can_be_got_to_from_its_front_door() {
        for seed in 1..40u32 {
            for landmark in Landmark::ALL {
                let p = landmark.plan(&mut Dice(seed));
                assert_eq!(reached(&p), p.rooms.len(), "seed {seed}: the {landmark:?}");
                assert!(p.flat_roof);
                // No two rooms on a floor overlap.
                for (i, a) in p.rooms.iter().enumerate() {
                    for b in &p.rooms[i + 1..] {
                        let apart = a.storey != b.storey || a.x1 <= b.x0 || b.x1 <= a.x0 || a.z1 <= b.z0 || b.z1 <= a.z0;
                        assert!(apart, "seed {seed}: the {landmark:?}'s rooms meet: {a:?} {b:?}");
                    }
                }
            }
            let p = Landmark::Police.plan(&mut Dice(seed));
            // Two cells, each behind bars with a doorway in them.
            assert_eq!(p.rooms.iter().filter(|r| r.use_ == Use::Cell).count(), 2);
            assert_eq!(p.bars.len(), 2, "seed {seed}: {:?}", p.bars);
            for &b in &p.bars {
                assert!(p.openings.iter().any(|o| o.wall == b && o.door), "seed {seed}: a cell with no door");
            }
            // The armory has no windows.
            let armory = p.rooms.iter().position(|r| r.use_ == Use::Armory).unwrap();
            assert!(!p.openings.iter().any(|o| !o.door && p.walls[o.wall].sides.contains(&Some(armory))), "seed {seed}: a window in the armory");
            // The engine bay opens wide at the front, twice.
            let f = Landmark::FireStation.plan(&mut Dice(seed));
            assert_eq!(f.openings.iter().filter(|o| o.door && o.width >= BAY_DOOR.0).count(), 2, "seed {seed}");
            // The school has its stairs, and its six classrooms upstairs.
            let sc = Landmark::School.plan(&mut Dice(seed));
            assert!(sc.stairs.len() == 1 && sc.storeys == 2);
            assert_eq!(sc.rooms.iter().filter(|r| r.use_ == Use::Classroom && r.storey == 1).count(), 6);
        }
    }
}
