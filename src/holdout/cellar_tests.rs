//! What an arena's buildings can be, tried on one made for it: a
//! blockhouse with a cellar under it (two flights of stairs in its hall,
//! one up and one down), a room two storeys tall with a rail over it, and
//! a door to buy down in the cellar.

use lntrn_math::{Vec2, Vec3};

use super::arena::{self, Wares};
use super::layout::{BuyAt, Build, DoorAt, Face, Gap, House, Layout, RailAt, RoomAt, Run, StairAt, WindowAt, Yard, ew, ns, world};
use super::tests::{open_everything, stand_at, world_of};
use super::*;
use crate::loot::Kind as Item;
use crate::map::build::{Built, build_arena};
use crate::map::building::plan::{DOOR_WIDTH, Kind as Built_, STOREY, Use};
use crate::map::building::shape::RAISED;
use crate::player::{Body, Player, STEP};
use crate::zombie::{self, Nav};

const YARD: usize = 0;
const CELLAR: usize = 1;

const BLOCKHOUSE: House = House {
    name: "the blockhouse",
    at: (-8, -10),
    kind: Built_::Relay,
    size: (12, 10),
    storeys: 2,
    cellars: 1,
    flat_roof: true,
    rooms: &[
        // A hall with the stairs in it on every floor; beside it a bay two
        // storeys tall, and under that the cellar.
        RoomAt(0, 0, 0, 4, 10, Use::Hall, YARD),
        RoomAt(0, 4, 0, 12, 10, Use::Garage, YARD),
        RoomAt(1, 0, 0, 4, 10, Use::Hall, YARD),
        RoomAt(1, 4, 0, 12, 10, Use::Void, YARD),
        RoomAt(-1, 0, 0, 4, 10, Use::Hall, YARD),
        RoomAt(-1, 4, 0, 12, 10, Use::Armory, CELLAR),
    ],
    doors: &[DoorAt(ns(0, 7.5), 0, DOOR_WIDTH, None), DoorAt(ns(4, 2.5), 0, DOOR_WIDTH, None), DoorAt(ns(4, 2.5), -1, DOOR_WIDTH, Some(500))],
    windows: &[WindowAt(ew(0, 8.5), 0, true)],
    rails: &[RailAt(ns(4, 7.0), 1, 3.0)],
    stairs: &[StairAt(0, 0, 1.0), StairAt(-1, 3, 4.8)],
    seed: 3,
};

const COMPOUND: Layout = Layout {
    zones: &["THE YARD", "THE CELLAR"],
    start: (8.0, 5.0, 0.0),
    bounds: (-12, -10, 12, 10),
    houses: &[BLOCKHOUSE],
    runs: &[
        Run { along_x: true, at: -10, from: -12, to: -8, build: Build::Perimeter, gaps: &[] },
        Run { along_x: true, at: -10, from: 4, to: 12, build: Build::Perimeter, gaps: &[] },
        Run { along_x: true, at: 10, from: -12, to: 12, build: Build::Perimeter, gaps: &[Gap::Gate(0.0, 6.0)] },
        Run { along_x: false, at: -12, from: -10, to: 10, build: Build::Perimeter, gaps: &[] },
        Run { along_x: false, at: 12, from: -10, to: 10, build: Build::Perimeter, gaps: &[Gap::Hole(5.0)] },
    ],
    yards: &[Yard(-12, -10, 12, 10, YARD)],
    buys: &[BuyAt(ew(0, 0.0), -1, Face::N, Wares::Kit(Item::Bandage))],
    props: &[],
};

fn built() -> Built {
    build_arena(arena::of(&COMPOUND), &crate::testing::kit(), &|_| {})
}

/// A spot in the blockhouse (in its own metres), on a level's floor.
fn inside(x: f64, z: f64, level: i8) -> Vec3 {
    world(Vec2::new(x - 8.0, z - 10.0), RAISED + f64::from(level) * STOREY)
}

const DOWN: Vec3 = Vec3::new(0.0, -1.0, 0.0);

#[test]
fn a_cellar_is_dug_under_its_building_and_the_ground_round_it_left() {
    let built = built();
    let hit = |from: Vec3| built.solids.raycast(from, DOWN, 20.0).map(|h| h.point.y);
    // Out in the yard, and right up against the wall: the ground (about
    // level: the road dips it a little by the gate).
    for (x, z) in [(8.5, 5.5), (4.9, -4.5), (-8.9, -4.5), (0.5, 0.4)] {
        let y = hit(Vec3::new(x, 2.0, z)).expect("ground");
        assert!(y.abs() < 0.3, "the ground at ({x}, {z}) is at {y}");
    }
    // Under the ground floor, no ground: the cellar's own floor, a storey
    // down, with head room over it (where nothing's been stood on it: in
    // the hall, and before the door).
    for (x, z) in [(1.5, 2.5), (2.0, 9.5), (4.9, 2.5)] {
        let floor = inside(x, z, -1);
        let y = hit(floor + Vec3::new(0.0, 2.6, 0.0)).expect("a floor");
        assert!((y - floor.y).abs() < 0.03, "the cellar at ({x}, {z}): a floor at {y}, not {}", floor.y);
        assert!(built.solids.fits(crate::player::capsule(false), floor + Vec3::new(0.0, 0.05, 0.0)), "no room to stand in the cellar at ({x}, {z})");
    }
    // To its far corners: its floor (or what's stood on it), never the
    // ground.
    for (x, z) in [(8.5, 5.5), (11.5, 9.5), (4.5, 0.5), (11.5, 0.5)] {
        let floor = inside(x, z, -1);
        let y = hit(floor + Vec3::new(0.0, 2.6, 0.0)).expect("a floor");
        assert!(y > floor.y - 0.03 && y < floor.y + 2.3, "the cellar at ({x}, {z}): {y} under the floor over it, its own at {}", floor.y);
    }
    assert_eq!(built.map.digs.len(), 1);
}

#[test]
fn the_stairs_go_down_to_the_cellar_and_up_over_it_and_its_door_is_to_buy() {
    let (mut world, mut h) = world_of(built());
    let start = world.query_filtered::<&Body, With<Player>>().single(&world).expect("the player").pos;
    let (landing, cellar, upstairs, bay) = (inside(1.5, 2.5, -1), inside(8.5, 5.5, -1), inside(2.5, 8.5, 1), inside(8.5, 5.5, 0));
    {
        let nav = world.resource::<Nav>().0.as_ref().unwrap();
        for (what, at) in [("the cellar's landing", landing), ("upstairs", upstairs), ("the bay", bay)] {
            assert!(nav.height_at(at).is_some_and(|y| (y - at.y).abs() < 0.1), "{what}: no floor at {at:?} ({:?})", nav.height_at(at));
            assert!(nav.connects(start, at), "{what} can't be walked to from the yard");
        }
        assert!(!nav.connects(start, cellar), "the cellar's reached past its door");
        // The way down is by the stairs: a walk of some length, not a drop.
        let path = nav.path(start, landing).expect("a way down");
        assert!(path.len() >= 3, "{path:?}");
    }
    assert_eq!(h.arena.zones.len(), 2);
    assert!(h.arena.buys.iter().all(|b| b.zone == CELLAR));
    open_everything(&mut world, &mut h);
    let nav = world.resource::<Nav>().0.as_ref().unwrap();
    assert!(nav.connects(start, cellar), "the cellar's shut with its door open");
    assert!(nav.connects(start, stand_at(&h.arena.buys[0])), "the cellar's wall buy can't be got to");
}

#[test]
fn a_tall_room_has_no_floor_over_it_and_a_rail_to_look_down_from() {
    let built = built();
    let bay = inside(8.5, 5.5, 0);
    // Up from the bay's floor: nothing till its ceiling, two storeys up.
    let up = built.solids.raycast(bay + Vec3::new(0.0, 0.5, 0.0), Vec3::new(0.0, 1.0, 0.0), 20.0).expect("a ceiling");
    assert!(up.t > 2.0 * STOREY - 1.0, "something {:.2} m over the bay's floor", up.t + 0.5);
    assert!(!built.nav.walkable(inside(8.5, 5.5, 1)), "a floor in the air over the bay");
    // The walls go on up round it: no way out between the storeys.
    for (from, dir) in [(inside(8.5, 5.5, 1), Vec3::new(1.0, 0.0, 0.0)), (inside(8.5, 5.5, 1) - Vec3::new(0.0, 0.1, 0.0), Vec3::new(0.0, 0.0, 1.0))] {
        assert!(built.solids.raycast(from, dir, 8.0).is_some(), "no wall round the bay between its storeys ({dir:?})");
    }
    // The rail, from the hall upstairs: shot over, not walked through.
    let at_rail = inside(3.0, 7.0, 1);
    let across = Vec3::new(1.0, 0.0, 0.0);
    assert!(built.solids.raycast(at_rail + Vec3::new(0.0, 1.5, 0.0), across, 3.0).is_none(), "the rail stops a shot over it");
    assert!(built.solids.raycast(at_rail + Vec3::new(0.0, 0.5, 0.0), across, 3.0).is_some(), "no rail");
    assert!(!built.solids.fits(crate::player::capsule(false), inside(4.0, 7.0, 1) + Vec3::new(0.0, 0.05, 0.0)), "a body gets through the rail");
}

#[test]
fn the_dead_come_down_the_stairs_for_a_player_in_the_cellar() {
    let (mut world, mut h) = world_of(built());
    open_everything(&mut world, &mut h);
    let at = inside(9.5, 7.5, -1);
    for mut b in world.query_filtered::<&mut Body, With<Player>>().iter_mut(&mut world) {
        *b = Body::at(at);
    }
    let mut think = zombie::stepper();
    let mut most = 0;
    for _ in 0..(90.0 / STEP) as usize {
        h.update(&mut world, &[at], STEP);
        think.run(&mut world);
        let close = world.query::<(&zombie::brain::Zombie, &Body)>().iter(&world).filter(|(z, b)| !z.dead() && (b.pos - at).length() < 3.0).count();
        most = most.max(close);
    }
    assert!(most >= 1, "none of the dead got down to the cellar");
}
