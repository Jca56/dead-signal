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
fn run(z: &mut Zombie, body: &mut Body, solids: &Solids, nav: Option<&NavGrid>, player: Option<Vec3>, noises: &[(Vec3, f64)], seconds: f64) -> usize {
    let mut blows = 0;
    let steps = (seconds / STEP) as usize;
    for i in 0..steps {
        let heard = if i == 0 { noises } else { &[] };
        let senses = Senses { solids, nav, player, noises: heard, alerts: &[], searches: &std::cell::Cell::new(u32::MAX), sight: 1.0 };
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
fn light_feet_go_unseen_farther_off() {
    let s = floor();
    let player = Some(Vec3::new(0.0, 0.0, -20.0));
    let look = |sight: f64| {
        let mut z = Zombie::new(0.0, 7);
        let body = Body::at(Vec3::ZERO);
        let senses = Senses { solids: &s, nav: None, player, noises: &[], alerts: &[], searches: &std::cell::Cell::new(u32::MAX), sight };
        (0..10).any(|_| z.think(&body, &senses, STEP).alert.is_some())
    };
    assert!(look(1.0), "20 m ahead is seen");
    assert!(!look(0.55), "rank 3 light feet: not at 20 m");
}

#[test]
fn it_hears_a_shot_within_earshot_only() {
    let s = floor();
    let behind = |d: f64| Vec3::new(0.0, 0.0, d);
    let mut z = Zombie::new(0.0, 3);
    let mut body = Body::at(Vec3::ZERO);
    run(&mut z, &mut body, &s, None, None, &[(behind(HEARING + 1.0), HEARING)], 0.1);
    assert!(matches!(z.state, State::Wander { .. }), "heard it too far off");
    run(&mut z, &mut body, &s, None, None, &[(behind(HEARING - 1.0), HEARING)], 0.1);
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
fn it_cannot_reach_you_up_on_a_roof() {
    // A 3 m block; the player on it, right at its edge; the dead below.
    let mut s = floor();
    s.add(&box_tris(Vec3::new(-3.0, 0.0, -6.0), Vec3::new(3.0, 3.0, -1.2)));
    let player = Vec3::new(0.0, 3.0, -1.5);
    let mut z = Zombie::new(0.0, 11);
    let mut body = Body::at(Vec3::new(0.0, 0.0, 1.0));
    let blows = run(&mut z, &mut body, &s, None, Some(player), &[], 6.0);
    assert_eq!(blows, 0, "struck from below");
    assert!(!matches!(z.state, State::Attack { .. }), "swiping at the wall");
    // On a knee-high crate, though, the swipe takes the legs.
    let mut s = floor();
    s.add(&box_tris(Vec3::new(-1.0, 0.0, -3.0), Vec3::new(1.0, 0.8, -1.2)));
    let mut z = Zombie::new(0.0, 11);
    let mut body = Body::at(Vec3::new(0.0, 0.0, 1.0));
    assert!(run(&mut z, &mut body, &s, None, Some(Vec3::new(0.0, 0.8, -1.5)), &[], 6.0) > 0, "couldn't reach a crate");
}

#[test]
fn it_lines_up_with_stairs_it_comes_at_from_the_side() {
    // A 1.2 m wide flight of ten steps up onto a platform, climbing -Z;
    // the player up there, and the dead coming at the flight from off to
    // its side, where going straight for it means walking into its flank.
    let mut s = floor();
    let (rise, run_) = (0.19, 0.29);
    for k in 0..10 {
        let z0 = -2.0 - k as f64 * run_;
        s.add(&box_tris(Vec3::new(0.0, 0.0, z0 - run_), Vec3::new(1.2, (k + 1) as f64 * rise, z0)));
    }
    let top = 10.0 * rise;
    let edge = -2.0 - 10.0 * run_;
    s.add(&box_tris(Vec3::new(-5.0, 0.0, edge - 6.0), Vec3::new(1.2, top, edge)));
    let nav = NavGrid::build(&s, capsule(false), crate::testing::COURSE_HALF);
    let player = Vec3::new(-2.0, top, edge - 3.0);
    for (start, seed) in [Vec3::new(6.0, 0.0, -6.0), Vec3::new(5.0, 0.0, -3.5), Vec3::new(-3.0, 0.0, 2.0)].into_iter().flat_map(|s| (1..=12u32).map(move |k| (s, k * 7919))) {
        let mut z = Zombie::new(0.0, seed);
        let mut body = Body::at(start);
        z.hurt(1.0, false, false, player);
        let mut up = false;
        for _ in 0..30 {
            run(&mut z, &mut body, &s, Some(&nav), Some(player), &[], 0.5);
            up |= body.pos.y > top - 0.1;
        }
        assert!(up, "from {start:?}, walking {:.1}, it never got up: stuck at {:?}", z.gait.walk, body.pos);
    }
}

#[test]
fn on_the_real_map_it_climbs_to_you_from_any_side() {
    let solids = crate::testing::real_world();
    let nav = NavGrid::build(&solids, capsule(false), crate::testing::COURSE_HALF);
    let ground = |x: f64, z: f64| Vec3::new(x, nav.height_at(Vec3::new(x, 50.0, z)).unwrap(), z);
    // Up on the building's roof, and on the landings atop the 30° and 45°
    // ramps; each come at from around about.
    let roof = Vec3::new(0.0, 4.16, 45.5);
    let ramp30 = Vec3::new(3.5, 3.11, 39.0);
    let ramp45 = Vec3::new(6.0, 3.11, 39.8);
    let cases = [
        (roof, ground(6.0, 51.0)),
        (roof, ground(-6.0, 51.0)),
        (roof, ground(7.5, 43.0)),
        (roof, ground(-5.0, 40.0)),
        (ramp30, ground(9.0, 30.0)),
        (ramp30, ground(-2.0, 31.0)),
        (ramp45, ground(11.0, 34.0)),
        (ramp45, ground(1.0, 30.0)),
    ];
    // A shambler and a fast one each.
    let fast = (1..).find(|&i| Zombie::new(0.0, i).gait.walk > 2.5).unwrap();
    for ((player, start), seed) in cases.iter().flat_map(|&c| [(c, 5), (c, fast)]) {
        let mut z = Zombie::new(0.0, seed);
        let mut body = Body::at(start);
        z.hurt(1.0, false, false, player);
        let mut closest = f64::INFINITY;
        for _ in 0..90 {
            run(&mut z, &mut body, &solids, Some(&nav), Some(player), &[], 0.5);
            closest = closest.min((body.pos - player).length());
            if closest < 1.8 {
                break;
            }
        }
        assert!(closest < 1.8, "walking at {:.1}, from {start:?} to {player:?}: got no closer than {closest:.1} m, left at {:?}", z.gait.walk, body.pos);
    }
}

#[test]
fn it_drops_off_a_roof_rather_than_walk_round_to_the_stairs() {
    let solids = crate::testing::real_world();
    let nav = NavGrid::build(&solids, capsule(false), crate::testing::COURSE_HALF);
    // On the building's roof at its west end; the player down on the pad
    // beside its west wall, the stairs being round the far (east) side.
    let player = Vec3::new(-5.5, 1.11, 46.0);
    let mut z = Zombie::new(0.0, 5);
    let mut body = Body::at(Vec3::new(-1.5, 4.16, 46.0));
    z.hurt(1.0, false, false, player);
    let mut t = 0.0;
    while (body.pos - player).length() > 1.8 && t < 20.0 {
        run(&mut z, &mut body, &solids, Some(&nav), Some(player), &[], 0.25);
        t += 0.25;
        assert!(body.pos.x < 3.0, "went round by the stairs: {:?}", body.pos);
    }
    assert!(t < 8.0, "took {t} s to get down to you");
}

#[test]
fn it_does_not_drop_off_what_is_too_high() {
    // A 5 m block with the player at its foot; one on top, no way down but
    // a long ramp round the back.
    let mut s = floor();
    s.add(&box_tris(Vec3::new(-3.0, 0.0, -6.0), Vec3::new(3.0, 5.0, 0.0)));
    let nav = NavGrid::build(&s, capsule(false), crate::testing::COURSE_HALF);
    let (top, below) = (Vec3::new(0.0, 5.0, -1.0), Vec3::new(0.0, 0.0, 1.5));
    // (The way found goes as near as it can: the edge, up top.)
    let stays = |route: Option<Vec<Vec3>>, y: f64| route.is_some_and(|r| r.iter().all(|p| (p.y - y).abs() < 0.5));
    assert!(stays(nav.path(top, below), 5.0), "jumped 5 m");
    // Nor climbs back up a drop it would take.
    let mut s = floor();
    s.add(&box_tris(Vec3::new(-3.0, 0.0, -6.0), Vec3::new(3.0, 2.5, 0.0)));
    let nav = NavGrid::build(&s, capsule(false), crate::testing::COURSE_HALF);
    let (top, below) = (Vec3::new(0.0, 2.5, -1.0), Vec3::new(0.0, 0.0, 1.5));
    assert!(nav.path(top, below).is_some_and(|r| r.last().unwrap().y < 0.5), "wouldn't drop 2.5 m");
    assert!(stays(nav.path(below, top), 0.0), "climbed 2.5 m");
}

#[test]
fn it_walks_round_a_wall_to_where_it_saw_you() {
    // A wall across the way, long enough that going round is the only way.
    let mut s = floor();
    s.add(&box_tris(Vec3::new(-6.0, 0.0, -5.5), Vec3::new(6.0, 3.0, -5.0)));
    let nav = NavGrid::build(&s, capsule(false), crate::testing::COURSE_HALF);
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
fn six_to_the_body_two_to_the_head_or_three_blows() {
    let from = Vec3::ZERO;
    let mut z = Zombie::new(0.0, 1);
    for _ in 0..5 {
        assert!(!z.hurt(25.0, false, false, from));
    }
    assert!(z.hurt(25.0, false, false, from) && z.dead(), "six body shots");
    let mut z = Zombie::new(0.0, 1);
    assert!(!z.hurt(25.0, true, false, from), "one to the head isn't enough");
    assert!(z.hurt(25.0, true, false, from), "two to the head");
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
    let senses = Senses { solids: &s, nav: None, player: Some(player), noises: &[], alerts: &[], searches: &std::cell::Cell::new(u32::MAX), sight: 1.0 };
    // (It looks about a few times a second: give it a moment.)
    let seen = (0..10).find_map(|_| seer.think(&body, &senses, STEP).alert).expect("a snarl");
    let snarls = [(body.pos, seen)];
    // One near it, facing away from the player: it comes to look...
    let listen = |at: Vec3| {
        let mut z = Zombie::new(std::f64::consts::PI, 9);
        let b = Body::at(at);
        let senses = Senses { solids: &s, nav: None, player: Some(player), noises: &[], alerts: &snarls, searches: &std::cell::Cell::new(u32::MAX), sight: 1.0 };
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
        world.spawn((z, Body::at(Vec3::new(0.0, 0.0, -8.0)), super::Beat::new(i)));
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
    let grid = NavGrid::build(&solids, capsule(false), crate::testing::COURSE_HALF);
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


