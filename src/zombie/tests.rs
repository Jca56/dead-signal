//! A Shambler's mind and body together, stepped in small built worlds.

use lntrn_math::Vec3;

use super::brain::{ALERT_RANGE, HEARING, HP, Senses, State, Zombie};
use super::figure::Clip;
use super::nav::NavGrid;
use crate::collide::{Solids, box_tris};
use crate::player::{self, Body, STEP, capsule};

fn floor() -> Solids {
    let mut s = Solids::new();
    s.add(&box_tris(Vec3::new(-60.0, -1.0, -60.0), Vec3::new(60.0, 0.0, 60.0)));
    s
}

/// Step a Shambler for `seconds` with the player standing at `player`; the
/// blows it landed.
fn run(z: &mut Zombie, body: &mut Body, solids: &Solids, nav: Option<&NavGrid>, player: Option<Vec3>, noises: &[Vec3], seconds: f64) -> usize {
    let mut blows = 0;
    let steps = (seconds / STEP) as usize;
    for i in 0..steps {
        let heard = if i == 0 { noises } else { &[] };
        let senses = Senses { solids, nav, player, noises: heard, alerts: &[] };
        let mut intent = z.think(body, &senses, STEP);
        blows += usize::from(intent.hit.is_some());
        if !z.dead() {
            let gait = z.gait;
            player::step_body(body, z.yaw, &mut intent.controls, solids, &gait, STEP);
        }
    }
    blows
}

#[test]
fn it_sees_ahead_not_behind_and_not_through_walls() {
    let mut s = floor();
    // Facing -Z (a yaw of 0); the player 10 m ahead, then 10 m behind.
    let mut z = Zombie::new(0.0, 7);
    let mut body = Body::at(Vec3::ZERO);
    run(&mut z, &mut body, &s, None, Some(Vec3::new(0.0, 0.0, 10.0)), &[], 0.1);
    assert!(matches!(z.state, State::Wander { .. }), "saw behind it: {:?}", z.state);
    run(&mut z, &mut body, &s, None, Some(Vec3::new(0.0, 0.0, -10.0)), &[], 0.1);
    assert_eq!(z.state, State::Hunt, "didn't see ahead");
    // A wall between: unseen.
    s.add(&box_tris(Vec3::new(-3.0, 0.0, -5.5), Vec3::new(3.0, 3.0, -5.0)));
    let mut z = Zombie::new(0.0, 7);
    let mut body = Body::at(Vec3::ZERO);
    run(&mut z, &mut body, &s, None, Some(Vec3::new(0.0, 0.0, -10.0)), &[], 0.1);
    assert!(matches!(z.state, State::Wander { .. }), "saw through a wall");
}

#[test]
fn it_hears_a_shot_within_earshot_only() {
    let s = floor();
    let behind = |d: f64| Vec3::new(0.0, 0.0, d);
    let mut z = Zombie::new(0.0, 3);
    let mut body = Body::at(Vec3::ZERO);
    run(&mut z, &mut body, &s, None, None, &[behind(HEARING + 1.0)], 0.1);
    assert!(matches!(z.state, State::Wander { .. }), "heard it too far off");
    run(&mut z, &mut body, &s, None, None, &[behind(HEARING - 1.0)], 0.1);
    assert!(matches!(z.state, State::Investigate { .. }), "didn't hear it: {:?}", z.state);
    // And goes that way.
    run(&mut z, &mut body, &s, None, None, &[], 3.0);
    assert!(body.pos.z > 2.0, "went toward the shot: {:?}", body.pos);
}

#[test]
fn it_closes_in_and_strikes() {
    let s = floor();
    let mut z = Zombie::new(0.0, 11);
    let mut body = Body::at(Vec3::ZERO);
    let player = Vec3::new(0.5, 0.0, -8.0);
    let blows = run(&mut z, &mut body, &s, None, Some(player), &[], 10.0);
    assert!(blows >= 2, "struck {blows} times");
    let d = ((body.pos.x - player.x).powi(2) + (body.pos.z - player.z).powi(2)).sqrt();
    assert!(d < 1.6, "stood {d:.2} m off");
}

#[test]
fn it_walks_round_a_wall_to_where_it_saw_you() {
    // A wall across the way, long enough that going round is the only way.
    let mut s = floor();
    s.add(&box_tris(Vec3::new(-6.0, 0.0, -5.5), Vec3::new(6.0, 3.0, -5.0)));
    let nav = NavGrid::build(&s, capsule(false));
    let mut z = Zombie::new(0.0, 5);
    let mut body = Body::at(Vec3::ZERO);
    let player = Vec3::new(0.0, 0.0, -12.0);
    // It saw you, then the wall hid you: it hunts where you were.
    z.hurt(1.0, false, false, player);
    // (Once there and finding nobody, it wanders off: what counts is that
    // it got there.)
    let mut closest = f64::INFINITY;
    for _ in 0..40 {
        run(&mut z, &mut body, &s, Some(&nav), None, &[], 0.5);
        closest = closest.min(((body.pos.x - player.x).powi(2) + (body.pos.z - player.z).powi(2)).sqrt());
        assert!(body.pos.z > -5.0 || body.pos.z < -5.5 || body.pos.x.abs() > 6.0, "went through the wall at {:?}", body.pos);
    }
    assert!(closest < 1.6, "got round to within {closest:.1} m");
}

#[test]
fn six_to_the_body_one_to_the_head_or_three_blows() {
    let from = Vec3::ZERO;
    let mut z = Zombie::new(0.0, 1);
    for _ in 0..5 {
        assert!(!z.hurt(25.0, false, false, from));
    }
    assert!(z.hurt(25.0, false, false, from) && z.dead(), "six body shots");
    let mut z = Zombie::new(0.0, 1);
    assert!(z.hurt(25.0, true, false, from), "one to the head");
    // Blows: the same to the head as anywhere, and each sends it reeling.
    let mut z = Zombie::new(0.0, 1);
    assert!(!z.hurt(50.0, true, true, from));
    assert!(matches!(z.state, State::Stagger { .. }) && z.clip().0 == Clip::Stumble, "a blow sends it stumbling");
    assert_eq!(z.hp, HP - 50.0);
    assert!(!z.hurt(50.0, false, true, from));
    assert!(z.hurt(50.0, false, true, from), "three blows");
}

#[test]
fn a_blow_stops_its_swipe_and_holds_it_off() {
    let s = floor();
    let mut z = Zombie::new(0.0, 11);
    let mut body = Body::at(Vec3::ZERO);
    let player = Vec3::new(0.0, 0.0, -1.0);
    // Close enough to swipe: it starts one...
    run(&mut z, &mut body, &s, None, Some(player), &[], 0.1);
    assert!(matches!(z.state, State::Attack { .. }), "{:?}", z.state);
    // ...and a blow cuts it off; for over half a second it lands nothing.
    z.hurt(50.0, false, true, player);
    assert_eq!(run(&mut z, &mut body, &s, None, Some(player), &[], 0.55), 0);
    assert!(matches!(z.state, State::Stagger { .. }), "still reeling: {:?}", z.state);
}

#[test]
fn some_walk_fast_most_shamble() {
    let fast = (0..400u32).filter(|&i| Zombie::new(0.0, i.wrapping_mul(2_654_435_761)).gait.walk > 2.5).count();
    assert!((60..140).contains(&fast), "{fast} of 400 fast");
    for i in 0..50u32 {
        let g = Zombie::new(0.0, i * 7919 + 3).gait;
        assert!((1.6..=3.4).contains(&g.walk) && g.sprint > g.walk, "{g:?}");
    }
}

#[test]
fn a_snarl_brings_the_others_near_it() {
    let s = floor();
    // It sees the player and snarls where it stands.
    let mut seer = Zombie::new(0.0, 7);
    let body = Body::at(Vec3::ZERO);
    let player = Vec3::new(0.0, 0.0, -10.0);
    let senses = Senses { solids: &s, nav: None, player: Some(player), noises: &[], alerts: &[] };
    let seen = seer.think(&body, &senses, STEP).alert.expect("a snarl");
    let snarls = [(body.pos, seen)];
    // One near it, facing away from the player: it comes to look...
    let listen = |at: Vec3| {
        let mut z = Zombie::new(std::f64::consts::PI, 9);
        let b = Body::at(at);
        let senses = Senses { solids: &s, nav: None, player: Some(player), noises: &[], alerts: &snarls };
        z.think(&b, &senses, STEP);
        z.state
    };
    assert_eq!(listen(Vec3::new(ALERT_RANGE - 2.0, 0.0, 0.0)), State::Investigate { at: player, looked: 0.0 });
    // ...one further off never knew.
    assert!(matches!(listen(Vec3::new(ALERT_RANGE + 2.0, 0.0, 0.0)), State::Wander { .. }));
}

#[test]
fn a_crowd_spreads_round_you_instead_of_stacking() {
    use bevy_ecs::prelude::*;
    let mut world = World::new();
    world.insert_resource(crate::world::Solid(floor()));
    world.insert_resource(super::Nav(None));
    world.insert_resource(super::Noises::default());
    world.insert_resource(super::Horde::default());
    world.spawn((Body::at(Vec3::ZERO), player::Player));
    // Ten on one spot, all after the player.
    for i in 0..10 {
        let mut z = Zombie::new(0.0, 100 + i);
        z.hurt(1.0, false, false, Vec3::ZERO);
        world.spawn((z, Body::at(Vec3::new(0.0, 0.0, -8.0))));
    }
    let mut schedule = Schedule::default();
    schedule.add_systems(super::think);
    for _ in 0..(8.0 / STEP) as usize {
        schedule.run(&mut world);
    }
    let at: Vec<Vec3> = world.query::<(&Zombie, &Body)>().iter(&world).map(|(_, b)| b.pos).collect();
    for (i, a) in at.iter().enumerate() {
        let d = (a.x * a.x + a.z * a.z).sqrt();
        assert!(d < 4.0, "one hung back {d:.1} m off");
        for b in &at[i + 1..] {
            let gap = ((a.x - b.x).powi(2) + (a.z - b.z).powi(2)).sqrt();
            assert!(gap > 0.45, "two stood {gap:.2} m apart");
        }
    }
}

#[test]
fn a_new_one_comes_from_out_of_sight_and_can_reach_you() {
    use bevy_ecs::prelude::*;
    let solids = crate::testing::real_world();
    let grid = NavGrid::build(&solids, capsule(false));
    let mut world = World::new();
    world.insert_resource(crate::world::Solid(solids.clone()));
    world.insert_resource(super::Nav(Some(grid)));
    world.insert_resource(super::Horde::default());
    // At the spawn, looking at the tower (down -Z).
    let (feet, forward) = (Vec3::new(0.0, 0.0, 6.0), Vec3::new(0.0, 0.0, -1.0));
    let eye = feet + Vec3::new(0.0, 1.6, 0.0);
    for _ in 0..5 {
        assert!(super::spawn_unseen(&mut world, eye, forward), "nowhere to come from");
    }
    let bodies: Vec<Vec3> = world.query::<&Body>().iter(&world).map(|b| b.pos).collect();
    let nav = world.resource::<super::Nav>().0.as_ref().expect("the grid");
    assert_eq!(bodies.len(), 5);
    for at in bodies {
        let off = at - feet;
        let d = (off.x * off.x + off.z * off.z).sqrt();
        assert!((34.0..61.0).contains(&d), "came {d:.1} m off");
        let chest = at + Vec3::new(0.0, 1.2, 0.0) - eye;
        let dir = chest.normalize();
        let hidden = solids.raycast(eye, dir, chest.length()).is_some();
        assert!(hidden || dir.dot(forward) < 0.3, "in plain sight at {at:?}");
        assert!(nav.path(at, feet).is_some(), "no way to the player from {at:?}");
    }
}

