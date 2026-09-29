//! The holdout in its arena: the ways the dead can walk, the doors
//! shutting them out, points, buying, nailing boards back, and a round
//! played out.

use lntrn_math::Vec3;

use super::*;
use crate::map::build::{Built, build_holdout};
use crate::player::{Body, Player, STEP};
use bevy_ecs::world::World;

use crate::zombie::{self, Nav};

fn built() -> Built {
    build_holdout(&crate::testing::kit(), &|_| {})
}

/// The arena made real in a world of its own, the player standing at its
/// start.
fn world_of(built: Built) -> (World, Holdout) {
    let Built { map, solids, nav, arena, .. } = built;
    let mut world = World::new();
    world.insert_resource(Solid(solids));
    world.insert_resource(Nav(Some(nav)));
    world.insert_resource(zombie::Noises::default());
    world.insert_resource(zombie::Horde::default());
    world.insert_resource(zombie::Heat::default());
    world.spawn((Body::at(map.spawn.0), Player));
    let mut h = Holdout::new(arena.expect("the arena"), 7);
    h.begin(&mut world);
    (world, h)
}

fn feet(world: &mut World) -> Vec3 {
    world.query_filtered::<&Body, With<Player>>().single(world).expect("the player").pos
}

#[test]
fn the_dead_get_to_every_window_and_in_only_by_the_windows() {
    let (mut world, h) = world_of(built());
    let start = feet(&mut world);
    let nav = world.resource::<Nav>().0.as_ref().unwrap();
    for (i, w) in h.arena.windows.iter().enumerate() {
        assert!(nav.connects(w.from, w.outside), "window {i}: no way from where they come to its outside");
        assert!(!nav.connects(w.outside, w.inside), "window {i}: a way in past its boards");
        let in_start = w.zone == h.arena.start;
        assert_eq!(nav.connects(start, w.inside), in_start, "window {i} (zone {}): reached from the start", w.zone);
    }
    // The start is ground in the building, and its floor.
    assert!(nav.height_at(start).is_some_and(|y| (y - start.y).abs() < 0.3), "the start's floor: {start:?}");
}

#[test]
fn a_door_bought_open_lets_the_player_and_the_dead_through() {
    let (mut world, mut h) = world_of(built());
    let start = feet(&mut world);
    let door = h.arena.doors[0].clone();
    let beyond = h.arena.windows.iter().find(|w| w.zone != h.arena.start).expect("a window beyond the door").inside;
    let mid = (door.lo + door.hi) * 0.5;
    // Straight across it (through its doorway).
    let across = if door.hi.x - door.lo.x < door.hi.z - door.lo.z { Vec3::new(1.0, 0.0, 0.0) } else { Vec3::new(0.0, 0.0, 1.0) };
    let solid_at_door = |world: &World| world.resource::<Solid>().0.raycast(mid - across, across, 2.0).is_some();
    assert!(solid_at_door(&world), "the door isn't there");
    assert!(!world.resource::<Nav>().0.as_ref().unwrap().connects(start, beyond));
    // Short of points: nothing.
    h.points = door.cost - 1;
    let mut bag = Holdout::loadout();
    let (_, note, _) = h.press(&mut world, Aimed::Door(0), &mut bag);
    assert_eq!(note, Some("NOT ENOUGH POINTS"));
    assert!(!h.door_open(0));
    h.points = door.cost + 5;
    h.press(&mut world, Aimed::Door(0), &mut bag);
    assert!(h.door_open(0) && h.points == 5);
    assert!(!solid_at_door(&world), "the door's still solid");
    assert!(world.resource::<Nav>().0.as_ref().unwrap().connects(start, beyond), "no way through the open door");
    assert!(h.open.iter().all(|&o| o), "both rooms open");
}

#[test]
fn hits_and_kills_earn_points_the_head_and_the_blade_more() {
    let (_, mut h) = world_of(built());
    let mut stats = Stats::default();
    h.score(&stats);
    assert_eq!(h.points, START_POINTS);
    // Four hits, a body kill, a headshot kill, a knife kill.
    stats.hits = 4;
    stats.gun_kills = 2;
    stats.headshot_kills = 1;
    stats.melee_kills = 1;
    h.score(&stats);
    assert_eq!(h.points, START_POINTS + 40 + 50 + 90 + 120);
    // Counted once.
    h.score(&stats);
    assert_eq!(h.points, START_POINTS + 300);
}

#[test]
fn a_gun_off_the_wall_comes_loaded_with_its_rounds_and_more_rounds_cost_half() {
    let (mut world, mut h) = world_of(built());
    let smg = h.arena.buys.iter().position(|b| b.wares == Wares::Weapon(Kind::Smg)).expect("an SMG on a wall");
    let mut bag = Holdout::loadout();
    h.points = price(Kind::Smg);
    let (_, _, took) = h.press(&mut world, Aimed::Buy(smg), &mut bag);
    assert_eq!(took, Some(Slot::Sidearm));
    assert_eq!(h.points, 0);
    let held = bag.slot(Slot::Sidearm).expect("the SMG in hand");
    assert_eq!((held.kind, held.loaded), (Kind::Smg, 30));
    let (ammo, most) = spare(Kind::Smg).unwrap();
    assert_eq!(bag.count(ammo), most, "a full carry of its rounds");
    // Full up, more isn't sold.
    h.points = 10_000;
    let (_, note, _) = h.press(&mut world, Aimed::Buy(smg), &mut bag);
    assert_eq!((note, h.points), (Some("AMMO FULL"), 10_000));
    // Some spent: topped up again, for half.
    bag.remove(ammo, 50);
    h.press(&mut world, Aimed::Buy(smg), &mut bag);
    assert_eq!((bag.count(ammo), h.points), (most, 10_000 - price(Kind::Smg) / 2));
    assert!(h.prompt(Aimed::Buy(smg), &bag).contains("AMMO"));
    // A kit goes in the bag.
    let medkit = h.arena.buys.iter().position(|b| b.wares == Wares::Kit(Kind::Medkit)).expect("a medkit on a wall");
    h.press(&mut world, Aimed::Buy(medkit), &mut bag);
    assert_eq!(bag.count(Kind::Medkit), 1);
}

#[test]
fn boards_are_nailed_back_one_at_a_time_while_held_and_only_so_many_pay() {
    let (mut world, mut h) = world_of(built());
    world.resource_mut::<Barriers>().0[0].boards = 0;
    let window = Some(Aimed::Window(0));
    let before = h.points;
    for _ in 0..(3.0 * NAIL_EVERY / STEP) as usize + 4 {
        h.hold(&mut world, window, true, STEP);
    }
    assert_eq!(world.resource::<Barriers>().0[0].boards, 3);
    assert_eq!(h.points, before + 3 * PER_BOARD);
    // Let go, and it starts over.
    h.hold(&mut world, window, false, STEP);
    assert!(h.nail_progress().is_none());
    // Only so many a round pay.
    h.nailed = PAID_BOARDS;
    let before = h.points;
    for _ in 0..(NAIL_EVERY / STEP) as usize + 4 {
        h.hold(&mut world, window, true, STEP);
    }
    assert_eq!(world.resource::<Barriers>().0[0].boards, 4);
    assert_eq!(h.points, before);
}

#[test]
fn the_first_round_comes_in_by_the_windows_and_comes_for_the_player() {
    let (mut world, mut h) = world_of(built());
    let mut think = zombie::stepper();
    let player = feet(&mut world);
    let (mut came, mut inside) = (0, 0);
    // A minute of the first round, the player standing still.
    for _ in 0..(60.0 / STEP) as usize {
        h.update(&mut world, player, STEP);
        think.run(&mut world);
        came = came.max(zombie::alive(&mut world));
        inside = world.query::<(&zombie::brain::Zombie, &Body)>().iter(&world).filter(|(z, b)| z.barrier.is_none() && (b.pos - player).length() < 3.0).count().max(inside);
    }
    assert_eq!(h.rounds.round, 1);
    assert!(came >= 3, "only {came} came");
    assert!(inside >= 1, "none got in to the player");
    // Only by the start room's windows: the door's shut.
    assert!(world.resource::<Barriers>().0.iter().enumerate().all(|(i, b)| h.arena.windows[i].zone == h.arena.start || b.boards == BOARDS), "boards torn off in a room that's shut");
}

#[test]
fn every_round_ends_once_its_dead_are_dead() {
    let (mut world, mut h) = world_of(built());
    let player = feet(&mut world);
    // Past the wait, into the first round, all of it brought in.
    for _ in 0..(40.0 / STEP) as usize {
        h.update(&mut world, player, STEP);
    }
    assert_eq!(zombie::alive(&mut world), rounds::count(1) as usize);
    assert!(!h.rounds.resting());
    for mut z in world.query::<&mut zombie::brain::Zombie>().iter_mut(&mut world) {
        z.hurt(1e9, false, false, player);
    }
    h.update(&mut world, player, STEP);
    assert!(h.rounds.resting(), "the round didn't end");
    for _ in 0..(rounds::BREATHER / STEP) as usize + 2 {
        h.update(&mut world, player, STEP);
    }
    assert_eq!(h.rounds.round, 2);
    // Tougher.
    for _ in 0..(5.0 / STEP) as usize {
        h.update(&mut world, player, STEP);
    }
    let hp = world.query::<&zombie::brain::Zombie>().iter(&world).filter(|z| !z.dead()).map(|z| z.hp).next();
    assert_eq!(hp, Some(rounds::toughness(2)));
}
