//! Buildings out of town, made the way the town's are (`plan.rs`): a
//! farmhouse, a house whose living room is a den (where the gun cabinet
//! stands); a barn, one tall room with wide doors at both ends and a few
//! high windows; and a hunter's cabin, a den and a bedroom behind it.

use super::plan::{self, Cut, Kind, Opening, Plan, Room, Use};
use crate::loot::Dice;

/// A barn's doorways: how wide, how high; its windows, set high; and how
/// far its roof's ridge stands over the eaves.
const BARN_DOOR: f64 = 3.2;
const BARN_DOOR_HEAD: f64 = 2.6;
const BARN_SILL: f64 = 1.7;
const BARN_WINDOW_HEAD: f64 = 2.4;
const BARN_RIDGE: f64 = 4.0;

/// A farmhouse `w` by `d`, of one storey or two.
pub fn farmhouse(dice: &mut Dice, w: i32, d: i32, two: bool) -> Plan {
    let mut p = plan::house(dice, w, d, two);
    for r in &mut p.rooms {
        if r.use_ == Use::Living {
            r.use_ = Use::Den;
        }
    }
    p
}

/// A barn `w` (odd, so its doorways fall on the walking grid) by `d`, its
/// ridge along its depth, doorways in the middle of both ends.
pub fn barn(dice: &mut Dice, w: i32, d: i32) -> Plan {
    let w = w | 1;
    let mut plan = Plan {
        kind: Kind::Barn,
        w,
        d,
        storeys: 1,
        rooms: vec![Room { storey: 0, x0: 0, z0: 0, x1: w, z1: d, use_: Use::Barn }],
        walls: Vec::new(),
        openings: Vec::new(),
        stair: None,
        flat_roof: false,
        ridge_along_x: false,
        ridge: BARN_RIDGE,
    };
    plan.walls = plan::walls_of(&plan.rooms, 0);
    let middle = f64::from(w) * 0.5;
    for i in 0..plan.walls.len() {
        let wall = plan.walls[i];
        if wall.along_x {
            plan.openings.push(Opening { wall: i, centre: middle, width: BARN_DOOR, sill: 0.0, head: BARN_DOOR_HEAD, door: true, boarded: false });
        }
    }
    for i in 0..plan.walls.len() {
        if plan.walls[i].along_x {
            continue;
        }
        for c in plan::spots_along(&plan, i, 0.8, 4.5) {
            plan.openings.push(Opening { wall: i, centre: c, width: 0.8, sill: BARN_SILL, head: BARN_WINDOW_HEAD, door: false, boarded: dice.unit() < 0.3 });
        }
    }
    plan
}

/// A hunter's cabin `w` by `d`: the den at the front (the way in), a
/// bedroom through a doorway at its side.
pub fn cabin(dice: &mut Dice, w: i32, d: i32) -> Plan {
    let bed = 3;
    let mut plan = Plan { kind: Kind::Cabin, w, d, storeys: 1, rooms: Vec::new(), walls: Vec::new(), openings: Vec::new(), stair: None, flat_roof: false, ridge_along_x: w >= d, ridge: plan::RIDGE };
    plan.rooms.push(Room { storey: 0, x0: 0, z0: 0, x1: w - bed, z1: d, use_: Use::Den });
    plan.rooms.push(Room { storey: 0, x0: w - bed, z0: 0, x1: w, z1: d, use_: Use::Bed });
    let door = f64::from(plan::between(dice, 1, d - 2)) + 0.5;
    plan::finish(dice, plan, &[(0, Cut { along_x: false, at: w - bed, door })])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_barn_is_open_at_both_ends_and_a_cabin_has_its_two_rooms() {
        for seed in 1..40u32 {
            let b = barn(&mut Dice(seed), 12, 16);
            assert_eq!((b.w % 2, b.rooms.len()), (1, 1));
            let doors: Vec<&Opening> = b.openings.iter().filter(|o| o.door).collect();
            assert_eq!(doors.len(), 2, "seed {seed}: a doorway each end");
            for o in doors {
                assert!(b.walls[o.wall].along_x && (o.centre - 0.5).fract().abs() < 1e-9, "seed {seed}: {o:?}");
            }
            let c = cabin(&mut Dice(seed), 8, 6);
            let uses: Vec<Use> = c.rooms.iter().map(|r| r.use_).collect();
            assert_eq!(uses, vec![Use::Den, Use::Bed], "seed {seed}");
            // The way in is at the front, into the den; and through to the
            // bedroom.
            let front = c.openings.iter().find(|o| o.door && c.walls[o.wall].outside() && c.walls[o.wall].at == 0 && c.walls[o.wall].along_x);
            assert!(front.is_some_and(|o| c.walls[o.wall].sides[1] == Some(0)), "seed {seed}: no front door into the den");
            assert!(c.openings.iter().any(|o| o.door && !c.walls[o.wall].outside()), "seed {seed}: no way through");
            let f = farmhouse(&mut Dice(seed), 11, 9, seed % 2 == 0);
            assert!(f.rooms.iter().any(|r| r.use_ == Use::Den) && !f.rooms.iter().any(|r| r.use_ == Use::Living), "seed {seed}");
        }
    }
}
