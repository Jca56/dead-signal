//! Floor plans made from many seeds: every room somewhere, and got to.

use super::plan::*;
use crate::loot::Dice;

/// Every room can be walked to from the way in (through doorways not
/// boarded, and up the stairs).
fn all_rooms_reached(plan: &Plan) -> bool {
    let mut seen = vec![false; plan.rooms.len()];
    let mut todo: Vec<usize> = plan
        .openings
        .iter()
        .filter(|o| o.door && !o.boarded && plan.walls[o.wall].outside())
        .filter_map(|o| plan.walls[o.wall].sides.iter().flatten().next().copied())
        .collect();
    while let Some(r) = todo.pop() {
        if std::mem::replace(&mut seen[r], true) {
            continue;
        }
        for o in plan.openings.iter().filter(|o| o.door && !o.boarded) {
            let sides = plan.walls[o.wall].sides;
            if sides.contains(&Some(r)) {
                todo.extend(sides.iter().flatten().copied());
            }
        }
        // The stairs join the hall to the one over it.
        if plan.rooms[r].use_ == Use::Hall {
            todo.extend(plan.rooms.iter().enumerate().filter(|(_, o)| o.use_ == Use::Hall).map(|(i, _)| i));
        }
    }
    seen.iter().all(|s| *s)
}

#[test]
fn every_room_is_somewhere_and_can_be_got_to() {
    for seed in 1..300u32 {
        let mut dice = Dice(seed.wrapping_mul(2_654_435_761) | 1);
        let (w, d) = (dice.range(7, 13) as i32, dice.range(7, 12) as i32);
        let two = seed % 2 == 0;
        let plan = house(&mut dice, w, d, two);
        // The rooms tile the floor: none overlap, none too small.
        for s in 0..plan.storeys {
            let area: i32 = plan.rooms.iter().filter(|r| r.storey == s).map(Room::area).sum();
            assert_eq!(area, w * d, "seed {seed}: storey {s} isn't covered");
        }
        for r in &plan.rooms {
            assert!(r.x1 - r.x0 >= MIN_ROOM && r.z1 - r.z0 >= MIN_ROOM, "seed {seed}: a room {r:?}");
        }
        assert!(plan.openings.iter().any(|o| o.door && !o.boarded && plan.walls[o.wall].outside()), "seed {seed}: no way in");
        assert!(all_rooms_reached(&plan), "seed {seed}: a room can't be got to: {plan:#?}");
        assert!(plan.rooms.iter().any(|r| r.use_ == Use::Living) && plan.rooms.iter().any(|r| r.use_ == Use::Kitchen), "seed {seed}");
        // Every doorway sits on a square's middle, clear of its wall's
        // ends; every gap within its wall.
        for o in &plan.openings {
            let wall = plan.walls[o.wall];
            assert!(o.centre - o.width * 0.5 >= f64::from(wall.from) + 0.2 && o.centre + o.width * 0.5 <= f64::from(wall.to) - 0.2, "seed {seed}: {o:?} in {wall:?}");
            if o.door {
                assert_eq!(o.centre.fract(), 0.5, "seed {seed}");
                assert!(o.centre - f64::from(wall.from) >= 1.0 && f64::from(wall.to) - o.centre >= 1.0, "seed {seed}: {o:?} too near the end of {wall:?}");
            }
        }
        if plan.storeys == 2 {
            let st = plan.stair.expect("stairs");
            assert!(st.z1() <= f64::from(d) - 1.0, "seed {seed}: the stairs run out of the house");
        }
    }
}

#[test]
fn stores_and_shells() {
    let mut dice = Dice(5);
    let s = store(&mut dice, 12, 14);
    assert_eq!(s.rooms.len(), 2);
    assert!(all_rooms_reached(&s));
    assert!(s.openings.iter().any(|o| !o.door && o.width > 2.0), "shop windows");
    let sh = shell(&mut dice, 9, 8);
    assert!(sh.openings.iter().all(|o| o.boarded) && !sh.openings.iter().any(|o| o.door));
}
