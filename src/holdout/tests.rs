//! The holdout in its arena: the ways the dead can walk, the doors
//! shutting them out, points, buying, nailing boards back, and a round
//! played out.

use lntrn_math::Vec3;

use super::*;
use crate::map::build::{Built, build_holdout};
use crate::player::{Body, Player, STEP};
use bevy_ecs::world::World;

use crate::zombie::{self, Nav};

pub(super) fn built() -> Built {
    build_holdout(&crate::testing::kit(), &|_| {})
}

/// The arena made real in a world of its own, the player standing at its
/// start.
pub(super) fn world_of(built: Built) -> (World, Holdout) {
    let Built { map, solids, nav, arena, .. } = built;
    let mut world = World::new();
    world.insert_resource(Solid(solids));
    world.insert_resource(Nav(Some(nav)));
    world.insert_resource(zombie::Noises::default());
    world.insert_resource(zombie::Horde::default());
    world.insert_resource(zombie::Heat::default());
    world.spawn((Body::at(map.spawn.0), Player(0)));
    let mut h = Holdout::new(arena.expect("the arena"), 7, 1);
    h.begin(&mut world);
    (world, h)
}

pub(super) fn feet(world: &mut World) -> Vec3 {
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

/// Every door bought open.
pub(super) fn open_everything(world: &mut World, h: &mut Holdout) {
    let mut bag = Holdout::loadout();
    for i in 0..h.arena.doors.len() {
        h.wallets[0].points = 1_000_000;
        h.press(world, 0, Aimed::Door(i), &mut bag, None);
    }
}

/// Where a wall buy is stood at to use it (on its floor).
pub(super) fn stand_at(b: &arena::Buy) -> Vec3 {
    b.at + b.facing * 0.6 - Vec3::new(0.0, 1.45, 0.0)
}

#[test]
fn with_every_door_open_all_of_it_is_reached_and_the_dead_still_only_come_in_by_the_windows() {
    let (mut world, mut h) = world_of(built());
    let start = feet(&mut world);
    open_everything(&mut world, &mut h);
    assert!(h.open.iter().all(|&o| o), "a zone never opened");
    let nav = world.resource::<Nav>().0.as_ref().unwrap();
    for (i, w) in h.arena.windows.iter().enumerate() {
        assert!(nav.connects(start, w.inside), "window {i} (zone {}): its inside never reached", h.arena.zones[w.zone]);
        assert!(!nav.connects(w.outside, start) && !nav.connects(w.from, start), "window {i}: a way in from outside but by it");
    }
    for (i, b) in h.arena.buys.iter().enumerate() {
        assert!(nav.connects(start, stand_at(b)), "wall buy {i} ({:?}) can't be got to", b.wares);
    }
}

#[test]
fn every_zone_opens_from_the_start_a_door_at_a_time_and_the_dead_have_a_way_into_each() {
    let (_, arena) = arena::generate();
    let mut open = vec![false; arena.zones.len()];
    open[arena.start] = true;
    loop {
        let mut grew = false;
        for d in &arena.doors {
            let (a, b) = d.zones;
            assert_ne!(a, b, "a door within a zone");
            if open[a] != open[b] {
                open[a] = true;
                open[b] = true;
                grew = true;
            }
        }
        if !grew {
            break;
        }
    }
    let ways_in = |z: usize| arena.windows.iter().any(|w| w.zone == z);
    for (z, name) in arena.zones.iter().enumerate() {
        assert!(open[z], "{name} can't be opened from the start");
        // (Upstairs they come up from the floor below: its ways in.)
        let from_next = arena.doors.iter().any(|d| (d.zones.0 == z && ways_in(d.zones.1)) || (d.zones.1 == z && ways_in(d.zones.0)));
        assert!(ways_in(z) || from_next, "no way in for the dead to {name}");
    }
    assert!(arena.buys.iter().all(|b| b.zone < arena.zones.len()));
}

#[test]
fn nothing_stands_where_the_dead_come_or_climb_in_or_where_a_wall_buy_is_used() {
    let built = built();
    let arena = built.arena.as_ref().unwrap();
    let body = crate::player::capsule(false);
    let fits = |at: Vec3| built.solids.fits(body, at + Vec3::new(0.0, 0.05, 0.0));
    for (i, w) in arena.windows.iter().enumerate() {
        for (what, at) in [("comes from", w.from), ("stands outside", w.outside), ("lands inside", w.inside)] {
            assert!(fits(at), "window {i} (zone {}): something's where the dead {what}, at {at:?}", arena.zones[w.zone]);
        }
    }
    for (i, b) in arena.buys.iter().enumerate() {
        assert!(fits(stand_at(b)), "wall buy {i} ({:?}): something's where it's used, at {:?}", b.wares, stand_at(b));
    }
    assert!(fits(built.map.spawn.0), "something's on the start");
}

#[test]
fn a_door_bought_open_lets_the_player_and_the_dead_through() {
    let (mut world, mut h) = world_of(built());
    let start = feet(&mut world);
    let start_zone = h.arena.start;
    let (i, door) = h.arena.doors.iter().enumerate().find(|(_, d)| d.zones.0 == start_zone || d.zones.1 == start_zone).map(|(i, d)| (i, d.clone())).expect("a door out of the start");
    let other = if door.zones.0 == start_zone { door.zones.1 } else { door.zones.0 };
    let beyond = h.arena.windows.iter().find(|w| w.zone == other).expect("a window beyond the door").inside;
    let mid = (door.lo + door.hi) * 0.5;
    // Straight across it (through its doorway).
    let across = if door.hi.x - door.lo.x < door.hi.z - door.lo.z { Vec3::new(1.0, 0.0, 0.0) } else { Vec3::new(0.0, 0.0, 1.0) };
    let solid_at_door = |world: &World| world.resource::<Solid>().0.raycast(mid - across, across, 2.0).is_some();
    assert!(solid_at_door(&world), "the door isn't there");
    assert!(!world.resource::<Nav>().0.as_ref().unwrap().connects(start, beyond));
    // Short of points: nothing.
    h.wallets[0].points = door.cost - 1;
    let mut bag = Holdout::loadout();
    let (_, note, _) = h.press(&mut world, 0, Aimed::Door(i), &mut bag, None);
    assert_eq!(note, Some("NOT ENOUGH POINTS"));
    assert!(!h.door_open(i));
    h.wallets[0].points = door.cost + 5;
    h.press(&mut world, 0, Aimed::Door(i), &mut bag, None);
    assert!(h.door_open(i) && h.wallets[0].points == 5);
    assert!(!solid_at_door(&world), "the door's still solid");
    assert!(world.resource::<Nav>().0.as_ref().unwrap().connects(start, beyond), "no way through the open door");
    assert!(h.open[start_zone] && h.open[other], "the zones either side open");
}

#[test]
fn hits_and_kills_earn_points_the_head_and_the_blade_more() {
    let (_, mut h) = world_of(built());
    let mut stats = Stats::default();
    h.score(0, &stats);
    assert_eq!(h.wallets[0].points, START_POINTS);
    // Four hits, a body kill, a headshot kill; two blows, the second a
    // kill.
    stats.hits = 4;
    stats.gun_kills = 2;
    stats.headshot_kills = 1;
    stats.blows_landed = 2;
    stats.melee_kills = 1;
    h.score(0, &stats);
    assert_eq!(h.wallets[0].points, START_POINTS + 40 + 50 + 90 + 20 + 120);
    // Counted once.
    h.score(0, &stats);
    assert_eq!(h.wallets[0].points, START_POINTS + 320);
}

#[test]
fn each_player_earns_and_spends_their_own_points() {
    let Built { arena, .. } = built();
    let mut h = Holdout::new(arena.expect("the arena"), 7, 2);
    let mut world = World::new();
    h.begin(&mut world);
    let (mut first, mut second) = (Stats::default(), Stats::default());
    first.hits = 3;
    second.hits = 1;
    second.gun_kills = 1;
    h.score(0, &first);
    h.score(1, &second);
    assert_eq!((h.wallets[0].points, h.wallets[1].points), (START_POINTS + 30, START_POINTS + 10 + 50));
    // Counted once each.
    h.score(0, &first);
    h.score(1, &second);
    assert_eq!((h.wallets[0].points, h.wallets[1].points), (START_POINTS + 30, START_POINTS + 60));
    // A door out of the second's pocket, the first's left be.
    world.insert_resource(Solid(crate::collide::Solids::default()));
    world.insert_resource(Nav(None));
    h.wallets[1].points = h.arena.doors[0].cost + 5;
    h.press(&mut world, 1, Aimed::Door(0), &mut Holdout::loadout(), None);
    assert!(h.door_open(0));
    assert_eq!((h.wallets[0].points, h.wallets[1].points), (START_POINTS + 30, 5));
}

#[test]
fn a_gun_off_the_wall_comes_loaded_with_its_rounds_and_more_rounds_cost_half() {
    let (mut world, mut h) = world_of(built());
    let smg = h.arena.buys.iter().position(|b| b.wares == Wares::Weapon(Kind::Smg)).expect("an SMG on a wall");
    let mut bag = Holdout::loadout();
    h.wallets[0].points = price(Kind::Smg);
    let (_, _, took) = h.press(&mut world, 0, Aimed::Buy(smg), &mut bag, None);
    assert_eq!(took, Some(Slot::Sidearm));
    assert_eq!(h.wallets[0].points, 0);
    let held = bag.slot(Slot::Sidearm).expect("the SMG in hand");
    assert_eq!((held.kind, held.loaded), (Kind::Smg, 30));
    let (ammo, most) = spare(Kind::Smg, 0).unwrap();
    assert_eq!(bag.count(ammo), most, "a full carry of its rounds");
    // Full up, more isn't sold.
    h.wallets[0].points = 10_000;
    let (_, note, _) = h.press(&mut world, 0, Aimed::Buy(smg), &mut bag, None);
    assert_eq!((note, h.wallets[0].points), (Some("AMMO FULL"), 10_000));
    // Some spent: topped up again, for half.
    bag.remove(ammo, 50);
    h.press(&mut world, 0, Aimed::Buy(smg), &mut bag, None);
    assert_eq!((bag.count(ammo), h.wallets[0].points), (most, 10_000 - price(Kind::Smg) / 2));
    assert!(h.prompt(Aimed::Buy(smg), &bag, None).contains("AMMO"));
    // A kit goes in the bag.
    let medkit = h.arena.buys.iter().position(|b| b.wares == Wares::Kit(Kind::Medkit)).expect("a medkit on a wall");
    h.press(&mut world, 0, Aimed::Buy(medkit), &mut bag, None);
    assert_eq!(bag.count(Kind::Medkit), 1);
    // The pistol it took the place of is kept, in the pack, its rounds in it.
    let kept = bag.pack.items.iter().find(|i| i.stack.kind == Kind::Pistol).map(|i| i.stack.loaded);
    assert_eq!((kept, h.spilled.len()), (Some(12), 0));
    // With no room there for what's put out of hand, it's for the ground
    // at their feet.
    while bag.pack.place(Stack::one(Kind::Battery)).count == 0 {}
    while bag.pockets.place(Stack::one(Kind::Watch)).count == 0 {}
    let rifle = h.arena.buys.iter().position(|b| b.wares == Wares::Weapon(Kind::Rifle)).expect("a rifle on a wall");
    *bag.slot_mut(Slot::Primary) = Some(Stack::gun(Kind::Shotgun, 3));
    h.press(&mut world, 0, Aimed::Buy(rifle), &mut bag, None);
    assert_eq!(bag.slot(Slot::Primary).map(|g| g.kind), Some(Kind::Rifle));
    assert_eq!(h.spilled, vec![(0, Stack::gun(Kind::Shotgun, 3))]);
}

#[test]
fn the_heavy_weapons_are_on_no_wall_and_come_with_fewer_belts_and_tanks_to_carry() {
    let (_, mut h) = world_of(built());
    for (kind, carried) in [(Kind::Lmg, 400), (Kind::Flamethrower, 800), (Kind::Rpk, 300)] {
        assert!(!h.arena.buys.iter().any(|b| b.wares == Wares::Weapon(kind)), "{kind:?} is a mystery drop's");
        // Taken from one, it's in hand with what's a full carry of its.
        let mut bag = Holdout::loadout();
        assert_eq!(h.take(0, &mut bag, Stack::one(kind)), Some(Slot::Primary), "{kind:?}");
        let (ammo, most) = spare(kind, 0).unwrap();
        assert_eq!((most, bag.count(ammo)), (carried, carried), "{kind:?}");
    }
}

#[test]
fn the_cheap_guns_are_by_the_start_and_twice_over_and_the_dear_ones_deep_in_once() {
    let (_, h) = world_of(built());
    let zones = |kind: Kind| h.arena.buys.iter().filter(|b| b.wares == Wares::Weapon(kind)).map(|b| h.arena.zones[b.zone]).collect::<Vec<_>>();
    assert_eq!(zones(Kind::Pistol), ["CONTROL ROOM"]);
    assert_eq!(zones(Kind::Pistol45), ["CONTROL ROOM", "THE YARD"]);
    assert_eq!(zones(Kind::MiniUzi), ["EQUIPMENT ROOM", "EAST COURT"]);
    assert_eq!(zones(Kind::Shotgun), ["THE YARD", "MOTOR POOL"]);
    assert_eq!(zones(Kind::Smg), ["STATION HOUSE", "GENERATOR PEN"]);
    assert_eq!(zones(Kind::Rifle), ["MOTOR POOL"]);
    assert_eq!(zones(Kind::Sniper), ["BARRACKS"]);
    assert_eq!(zones(Kind::AssaultRifle), ["BROADCAST FLOOR"]);
    assert_eq!(zones(Kind::Ak47), ["THE BUNKER"]);
    // Where a holdout starts there are the two pistols, and no other gun;
    // and the dearest of them all is the furthest in.
    let start: Vec<Kind> = h.arena.buys.iter().filter(|b| h.arena.zones[b.zone] == "CONTROL ROOM").filter_map(|b| if let Wares::Weapon(k) = b.wares { Some(k) } else { None }).collect();
    assert_eq!(start, [Kind::Pistol45, Kind::Pistol]);
    let dearest = h.arena.buys.iter().filter_map(|b| if let Wares::Weapon(k) = b.wares { Some(k) } else { None }).max_by_key(|k| price(*k));
    assert_eq!(dearest, Some(Kind::Ak47));
}

#[test]
fn a_window_is_nailed_up_from_its_own_floor_not_the_one_over_it() {
    let (mut world, h) = world_of(built());
    let up = Vec3::new(0.0, 1.0, 0.0);
    for i in 0..h.arena.windows.len() {
        world.resource_mut::<Barriers>().0[i].boards = 0;
        let inside = h.arena.windows[i].inside;
        let eye = |feet: Vec3| feet + Vec3::new(0.0, EYE, 0.0);
        assert_eq!(h.aimed(&world, eye(inside), up), Some(Aimed::Window(i)), "window {i} from its own floor");
        let above = inside + Vec3::new(0.0, crate::map::building::plan::STOREY, 0.0);
        assert_ne!(h.aimed(&world, eye(above), up), Some(Aimed::Window(i)), "window {i} from the floor over it");
        world.resource_mut::<Barriers>().0[i].boards = BOARDS;
    }
}

#[test]
fn boards_are_nailed_back_one_at_a_time_while_held_and_only_so_many_pay() {
    let (mut world, mut h) = world_of(built());
    world.resource_mut::<Barriers>().0[0].boards = 0;
    let window = Some(Aimed::Window(0));
    let before = h.wallets[0].points;
    for _ in 0..(3.0 * NAIL_EVERY / STEP) as usize + 4 {
        h.hold(&mut world, 0, window, true, STEP);
    }
    assert_eq!(world.resource::<Barriers>().0[0].boards, 3);
    assert_eq!(h.wallets[0].points, before + 3 * PER_BOARD);
    // Let go, and it starts over.
    h.hold(&mut world, 0, window, false, STEP);
    assert!(h.nail_progress(0).is_none());
    // Only so many a round pay.
    h.wallets[0].nailed = PAID_BOARDS;
    let before = h.wallets[0].points;
    for _ in 0..(NAIL_EVERY / STEP) as usize + 4 {
        h.hold(&mut world, 0, window, true, STEP);
    }
    assert_eq!(world.resource::<Barriers>().0[0].boards, 4);
    assert_eq!(h.wallets[0].points, before);
}

#[test]
fn the_first_round_comes_in_by_the_windows_and_comes_for_the_player() {
    let (mut world, mut h) = world_of(built());
    let mut think = zombie::stepper();
    let player = feet(&mut world);
    let (mut came, mut inside) = (0, 0);
    // A minute of the first round, the player standing still.
    for _ in 0..(60.0 / STEP) as usize {
        h.update(&mut world, &[player], STEP);
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

/// A round played with the whole arena open and the player standing at
/// `at`: how many of the dead got to them (at most, at once) in `seconds`.
fn reach_the_player_at(at: Vec3, seconds: f64) -> usize {
    let (mut world, mut h) = world_of(built());
    open_everything(&mut world, &mut h);
    for mut b in world.query_filtered::<&mut Body, With<Player>>().iter_mut(&mut world) {
        *b = Body::at(at);
    }
    let mut think = zombie::stepper();
    let mut most = 0;
    for _ in 0..(seconds / STEP) as usize {
        h.update(&mut world, &[at], STEP);
        think.run(&mut world);
        let close = world.query::<(&zombie::brain::Zombie, &Body)>().iter(&world).filter(|(z, b)| !z.dead() && (b.pos - at).length() < 3.0).count();
        most = most.max(close);
    }
    most
}

#[test]
fn out_in_the_yard_by_the_mast_they_come_for_the_player_round_everything_in_it() {
    let got = reach_the_player_at(Vec3::new(0.5, 0.0, 18.5), 60.0);
    assert!(got >= 2, "only {got} got to the player in the yard");
}

#[test]
fn up_in_the_bunkhouse_dorm_they_come_for_the_player_up_the_stairs() {
    let got = reach_the_player_at(Vec3::new(30.5, 0.25 + crate::map::building::plan::STOREY, -19.5), 60.0);
    assert!(got >= 2, "only {got} got up to the dorm");
}

/// A spot in the relay station (on its grid) on a building's `level`.
fn indoors(x: f64, z: f64, level: i8) -> Vec3 {
    Vec3::new(x + 0.5, 0.25 + f64::from(level) * crate::map::building::plan::STOREY, z + 0.5)
}

#[test]
fn up_on_the_motor_pools_mezzanine_they_come_for_the_player_up_its_stairs() {
    let got = reach_the_player_at(indoors(-56.0, 23.0, 1), 90.0);
    assert!(got >= 2, "only {got} got up to the mezzanine");
}

#[test]
fn up_in_the_stations_studio_they_come_for_the_player() {
    let got = reach_the_player_at(indoors(62.0, 23.0, 1), 90.0);
    assert!(got >= 2, "only {got} got up to the studio");
}

#[test]
fn down_in_the_bunker_they_come_for_the_player_by_the_stairs_alone() {
    let got = reach_the_player_at(indoors(62.0, 14.0, -1), 90.0);
    assert!(got >= 2, "only {got} got down to the ops room");
    // There's no way in under the ground: every one of them came down
    // the stairs.
    let (_, h) = world_of(built());
    assert!(h.arena.windows.iter().all(|w| w.centre.y > -1.0), "a way in under the ground");
    let bunker = h.arena.zones.iter().position(|z| *z == "THE BUNKER").expect("a bunker");
    assert_eq!(h.arena.windows.iter().filter(|w| w.zone == bunker).count(), 0);
}

#[test]
fn out_in_the_lots_either_side_they_come_for_the_player() {
    for (what, at) in [("the west lot", Vec3::new(-55.5, 0.0, -16.5)), ("the east court", Vec3::new(68.5, 0.0, -17.5))] {
        let got = reach_the_player_at(at, 60.0);
        assert!(got >= 2, "only {got} got to the player in {what}");
    }
}

#[test]
fn every_round_ends_once_its_dead_are_dead() {
    let (mut world, mut h) = world_of(built());
    let player = feet(&mut world);
    // Past the wait, into the first round, all of it brought in.
    for _ in 0..(40.0 / STEP) as usize {
        h.update(&mut world, &[player], STEP);
    }
    assert_eq!(zombie::alive(&mut world), rounds::count(1) as usize);
    assert!(!h.rounds.resting());
    for mut z in world.query::<&mut zombie::brain::Zombie>().iter_mut(&mut world) {
        z.hurt(1e9, false, false, player);
    }
    h.update(&mut world, &[player], STEP);
    assert!(h.rounds.resting(), "the round didn't end");
    for _ in 0..(rounds::BREATHER / STEP) as usize + 2 {
        h.update(&mut world, &[player], STEP);
    }
    assert_eq!(h.rounds.round, 2);
    // Tougher.
    for _ in 0..(5.0 / STEP) as usize {
        h.update(&mut world, &[player], STEP);
    }
    let hp = world.query::<&zombie::brain::Zombie>().iter(&world).filter(|z| !z.dead()).map(|z| z.hp).next();
    assert_eq!(hp, Some(rounds::toughness(2)));
}

/// The arena in numbers: run by hand (`cargo test --release
/// arena_in_numbers -- --ignored --nocapture`).
#[test]
#[ignore]
fn arena_in_numbers() {
    let t = std::time::Instant::now();
    let built = built();
    eprintln!("built in {:.0} ms", t.elapsed().as_secs_f64() * 1000.0);
    let a = built.arena.as_ref().unwrap();
    for (z, name) in a.zones.iter().enumerate() {
        eprintln!("{name}: {} ways in, {} buys, doors {:?}", a.windows.iter().filter(|w| w.zone == z).count(), a.buys.iter().filter(|b| b.zone == z).count(), a.doors.iter().filter(|d| d.zones.0 == z || d.zones.1 == z).map(|d| (d.cost, d.heap, d.gate.is_empty())).collect::<Vec<_>>());
    }
    let m = &built.map;
    let near = |p: Vec3, r: f64| p.x.abs() < r && p.z.abs() < r;
    let trees = m.scenery.iter().filter(|p| matches!(p.what, crate::map::scatter::Scenery::Pine(_) | crate::map::scatter::Scenery::Dead(_))).collect::<Vec<_>>();
    eprintln!("scenery {}, trees {} ({} within 80 m, {} within 45 m), blocks {}, buildings {}, containers {}", m.scenery.len(), trees.len(), trees.iter().filter(|p| near(p.at, 80.0)).count(), trees.iter().filter(|p| near(p.at, 45.0)).count(), m.blocks.len(), m.buildings.len(), built.containers.len());
    eprintln!("road: {} points, from {:?} to {:?}", m.roads[0].points.len(), m.roads[0].points.first(), m.roads[0].points.last());
    eprintln!("spawn {:?}; nav cells {}", m.spawn, built.nav.open_cells());
    for (x, z) in [(0.5, 0.5), (0.5, 30.0), (0.5, 60.0), (40.0, 0.0), (0.0, -60.0), (100.0, 100.0), (250.0, 0.0)] {
        eprintln!("ground at ({x}, {z}): {:?}", m.field.height_at(x, z));
    }
}
