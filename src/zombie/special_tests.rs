//! The special dead, stepped in small built worlds: a Ripper running you
//! down, a Spitter keeping off and spitting, its globs and its burst.

use lntrn_math::Vec3;

use super::brain::{Senses, Zombie};
use super::figure::Clip;
use super::tests::{floor, run};
use crate::player::{self, Body, STEP};

#[test]
fn a_ripper_runs_you_down_and_cuts_you_open() {
    use super::kind::Kind;
    let s = floor();
    // Seen 25 m off: a Shambler's still on its way after 4 s; a Ripper's
    // there and slashing, again and again, every cut bleeding.
    let player = Some(Vec3::new(0.0, 0.0, -25.0));
    let mut shambler = Zombie::new(0.0, 7);
    let mut body = Body::at(Vec3::ZERO);
    assert_eq!(run(&mut shambler, &mut body, &s, None, player, &[], 4.0), 0);
    let mut ripper = Zombie::of(Kind::Ripper, 0.0, 7);
    let mut body = Body::at(Vec3::ZERO);
    run(&mut ripper, &mut body, &s, None, player, &[], 3.5);
    assert!((body.pos - Vec3::new(0.0, 0.0, -25.0)).length() < 2.0, "caught up: {:?}", body.pos);
    assert_eq!(ripper.clip().0, Clip::Slash, "slashing");
    // A slash every three quarters of a second.
    let cuts = run(&mut ripper, &mut body, &s, None, player, &[], 3.0);
    assert!(cuts >= 3, "{cuts} cuts in 3 s");
    assert_eq!(super::kind::Kind::Ripper.traits().swipe.leaves, Some(crate::vitals::Affliction::Bleed));
}


#[test]
fn a_spitter_keeps_off_spits_and_bursts_when_it_dies() {
    use super::kind::Kind;
    use super::spit;
    let s = floor();
    let flat = |a: Vec3, b: Vec3| ((a.x - b.x).powi(2) + (a.z - b.z).powi(2)).sqrt();
    // Too near: it backs off to its distance.
    let player = Vec3::new(0.0, 0.0, -3.0);
    let mut z = Zombie::of(Kind::Spitter, 0.0, 7);
    let mut body = Body::at(Vec3::ZERO);
    let mut spat = Vec::new();
    for _ in 0..(12.0 / STEP) as usize {
        let senses = Senses { solids: &s, nav: None, player: Some(player), noises: &[], alerts: &[], searches: &std::cell::Cell::new(u32::MAX), sight: 1.0 };
        let mut intent = z.think(&body, &senses, STEP);
        spat.extend(intent.spit);
        let gait = z.gait;
        player::step_body(&mut body, z.yaw, &mut intent.controls, &s, &gait, STEP);
    }
    let d = flat(body.pos, player);
    assert!(d > spit::KEEP_NEAR - 1.5, "backed off only to {d:.1} m");
    // And it spat at them, from its mouth, at them.
    assert!(spat.len() >= 2, "{} globs in 12 s", spat.len());
    assert!(spat.iter().all(|(from, at)| from.y > 1.0 && flat(*at, player) < 0.1));
    // Dead, it bursts once, when it's swollen.
    let senses = Senses { solids: &s, nav: None, player: Some(player), noises: &[], alerts: &[], searches: &std::cell::Cell::new(u32::MAX), sight: 1.0 };
    z.hurt(1000.0, false, false, player);
    let bursts: usize = (0..(3.0 / STEP) as usize).map(|_| usize::from(z.think(&body, &senses, STEP).burst)).sum();
    assert_eq!(bursts, 1);
}

#[test]
fn a_glob_thrown_at_the_player_hits_them_with_poison_and_a_miss_leaves_a_puddle() {
    use super::spit::{self, Puddle};
    use bevy_ecs::prelude::*;
    let mut world = World::new();
    world.insert_resource(crate::world::Solid(floor()));
    world.insert_resource(super::Horde::default());
    world.spawn((Body::at(Vec3::new(0.0, 0.0, -12.0)), player::Player));
    let mut fly = IntoSystem::into_system(spit::fly);
    fly.initialize(&mut world);
    // Thrown straight at them (no luck: aimed as it's thrown).
    world.resource_mut::<super::Horde>().spits.push((Vec3::new(0.0, 1.55, 0.0), Vec3::new(0.0, 1.1, -12.0)));
    let mut hit = None;
    for _ in 0..200 {
        fly.run((), &mut world).expect("the globs fly");
        world.flush();
        if let Some(b) = world.resource_mut::<super::Horde>().blows.pop() {
            hit = Some(b);
            break;
        }
    }
    // (Luck throws it up to OFF metres wide: it hits or it lands close.)
    let puddles = world.query::<&Puddle>().iter(&world).count();
    match hit {
        Some(b) => assert_eq!(b.leaves, Some(crate::vitals::Affliction::Poison)),
        None => assert_eq!(puddles, 1, "missed, and no puddle"),
    }
}

#[test]
fn a_juggernaut_roars_charges_and_hits_hard() {
    use super::brain::State;
    use super::kind::Kind;
    let s = floor();
    let player = Some(Vec3::new(0.0, 0.0, -15.0));
    let mut j = Zombie::of(Kind::Juggernaut, 0.0, 7);
    let mut body = Body::at(Vec3::ZERO);
    // It sees them, roars, and comes: 15 m in a couple of seconds.
    let mut roared = false;
    let mut blow = None;
    for _ in 0..(4.0 / STEP) as usize {
        let senses = Senses { solids: &s, nav: None, player, noises: &[], alerts: &[], searches: &std::cell::Cell::new(u32::MAX), sight: 1.0 };
        let mut intent = j.think(&body, &senses, STEP);
        roared |= matches!(j.state, State::Roar { .. });
        if let Some(b) = intent.hit {
            blow = Some(b);
            break;
        }
        let gait = j.gait;
        player::step_body(&mut body, j.yaw, &mut intent.controls, &s, &gait, STEP);
    }
    assert!(roared, "no warning");
    let blow = blow.expect("the charge hit");
    assert!(blow.damage >= 40.0 && blow.push.length() > 2.0, "{blow:?}");
}

#[test]
fn a_charge_into_a_wall_dazes_it_and_dazed_it_takes_double() {
    use super::brain::State;
    use super::kind::Kind;
    use crate::collide::box_tris;
    let mut s = floor();
    // A wall between, low enough to see over: it charges straight into it.
    s.add(&box_tris(Vec3::new(-4.0, 0.0, -7.0), Vec3::new(4.0, 1.0, -6.5)));
    let player = Some(Vec3::new(0.0, 0.0, -14.0));
    let mut j = Zombie::of(Kind::Juggernaut, 0.0, 7);
    let mut body = Body::at(Vec3::ZERO);
    // (Seen over it: it must charge, not find it's in the way.)
    j.state = State::Roar { t: 0.0 };
    run(&mut j, &mut body, &s, None, player, &[], 2.0);
    assert!(matches!(j.state, State::Dazed { .. }), "not dazed: {:?} at {:?}", j.state, body.pos);
    let forward = Vec3::new(0.0, 0.0, 1.0);
    assert_eq!(j.plating(forward, false), 2.0);
}

#[test]
fn its_plate_turns_shots_from_the_front_but_not_the_back_or_the_legs() {
    use super::kind::Kind;
    // Facing -Z: a shot from in front travels +Z.
    let j = Zombie::of(Kind::Juggernaut, 0.0, 7);
    let from_front = Vec3::new(0.0, 0.0, 1.0);
    let from_behind = Vec3::new(0.0, 0.0, -1.0);
    assert!(j.plating(from_front, false) < 0.5);
    assert_eq!(j.plating(from_behind, false), 1.0);
    assert_eq!(j.plating(from_front, true), 1.0, "the legs aren't plated");
    assert_eq!(Zombie::of(Kind::Shambler, 0.0, 7).plating(from_front, false), 1.0);
    // Headshots and blows don't stagger it.
    let mut j = Zombie::of(Kind::Juggernaut, 0.0, 7);
    j.hurt(30.0, true, true, Vec3::new(0.0, 0.0, -5.0));
    assert!(!matches!(j.state, super::brain::State::Stagger { .. }));
}
