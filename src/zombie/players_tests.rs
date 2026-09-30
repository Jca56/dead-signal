//! The dead among the players, playing together: whom they go for (the
//! nearest standing, and they keep after them), and whose the blows are.

use lntrn_math::Vec3;

use super::brain::Zombie;
use super::tests::floor;
use crate::player::{self, Body, STEP};

#[test]
fn it_goes_for_the_nearest_player_and_its_blows_land_on_them() {
    use bevy_ecs::prelude::*;
    let mut world = World::new();
    world.insert_resource(crate::world::Solid(floor()));
    world.insert_resource(super::Nav(None));
    world.insert_resource(super::Noises::default());
    world.insert_resource(super::Horde::default());
    // The first player well off; the second a few steps from it.
    world.spawn((Body::at(Vec3::new(0.0, 0.0, 25.0)), player::Player(0)));
    world.spawn((Body::at(Vec3::new(0.0, 0.0, -6.0)), player::Player(1)));
    let mut z = Zombie::new(0.0, 7);
    z.relentless = true;
    world.spawn((z, Body::at(Vec3::ZERO), super::Beat::new(0)));
    let mut schedule = Schedule::default();
    schedule.add_systems(super::step::think);
    let mut blows = Vec::new();
    for _ in 0..(6.0 / STEP) as usize {
        schedule.run(&mut world);
        blows.append(&mut world.resource_mut::<super::Horde>().blows);
    }
    assert!(!blows.is_empty(), "it never struck");
    assert!(blows.iter().all(|(seat, _)| *seat == 1), "a blow landed on the far one: {blows:?}");
}

#[test]
fn it_leaves_the_fallen_be_and_goes_for_whoever_is_standing() {
    use bevy_ecs::prelude::*;
    let mut world = World::new();
    world.insert_resource(crate::world::Solid(floor()));
    world.insert_resource(super::Nav(None));
    world.insert_resource(super::Noises::default());
    world.insert_resource(super::Horde::default());
    // The first player down right beside it; the second well off.
    world.spawn((Body::at(Vec3::new(0.0, 0.0, -3.0)), player::Player(0), player::Fallen));
    world.spawn((Body::at(Vec3::new(0.0, 0.0, 20.0)), player::Player(1)));
    let mut z = Zombie::new(0.0, 7);
    z.relentless = true;
    let e = world.spawn((z, Body::at(Vec3::ZERO), super::Beat::new(0))).id();
    let mut schedule = Schedule::default();
    schedule.add_systems(super::step::think);
    let mut blows = Vec::new();
    for _ in 0..(8.0 / STEP) as usize {
        schedule.run(&mut world);
        blows.append(&mut world.resource_mut::<super::Horde>().blows);
    }
    assert!(blows.iter().all(|(seat, _)| *seat == 1), "it struck the one down: {blows:?}");
    assert!(world.get::<Body>(e).unwrap().pos.z > 12.0, "it went for the one standing");
}

#[test]
fn it_keeps_after_one_unless_another_is_a_good_deal_nearer() {
    let pick = |a: f64, b: f64, was| super::step::quarry(&[(0, Vec3::new(0.0, 0.0, a)), (1, Vec3::new(0.0, 0.0, -b))], Vec3::ZERO, was).map(|q| q.0);
    assert_eq!(pick(10.0, 8.0, None), Some(1), "the nearest, to begin with");
    assert_eq!(pick(10.0, 8.0, Some(0)), Some(0), "a little nearer's no reason to turn");
    assert_eq!(pick(20.0, 5.0, Some(0)), Some(1), "a good deal nearer is");
}
