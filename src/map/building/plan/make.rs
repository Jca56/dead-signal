//! The plans made by cutting a floor up: a house (of one storey or two,
//! a hall down its side with the stairs in it), a store, and a bare shell.

use super::*;

/// A house `w` by `d`, of one storey or two.
pub fn house(dice: &mut Dice, w: i32, d: i32, two: bool) -> Plan {
    let mut plan = Plan { kind: Kind::House, w, d, storeys: 1, cellars: 0, rooms: Vec::new(), walls: Vec::new(), openings: Vec::new(), stairs: Vec::new(), flat_roof: false, ridge_along_x: w >= d, ridge: RIDGE, bars: Vec::new(), grim: false };
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
        plan.stairs = vec![Stair { storey: 0, x: stair_x, z0: 1.0 }];
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
    let mut plan = Plan { kind: Kind::Store, w, d, storeys: 1, cellars: 0, rooms: Vec::new(), walls: Vec::new(), openings: Vec::new(), stairs: Vec::new(), flat_roof: true, ridge_along_x: true, ridge: RIDGE, bars: Vec::new(), grim: false };
    let back = 4.min(d - MIN_ROOM - 5).max(MIN_ROOM);
    plan.rooms.push(Room { storey: 0, x0: 0, z0: 0, x1: w, z1: d - back, use_: Use::Shop });
    plan.rooms.push(Room { storey: 0, x0: 0, z0: d - back, x1: w, z1: d, use_: Use::Back });
    let door = f64::from(between(dice, 1, w - 2)) + 0.5;
    finish(dice, plan, &[(0, Cut { along_x: true, at: d - back, door })])
}

/// A house boarded up all round, nothing inside worth drawing.
pub fn shell(dice: &mut Dice, w: i32, d: i32) -> Plan {
    let mut plan = Plan { kind: Kind::Shell, w, d, storeys: 1, cellars: 0, rooms: vec![Room { storey: 0, x0: 0, z0: 0, x1: w, z1: d, use_: Use::Hall }], walls: Vec::new(), openings: Vec::new(), stairs: Vec::new(), flat_roof: false, ridge_along_x: w >= d, ridge: RIDGE, bars: Vec::new(), grim: false };
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
