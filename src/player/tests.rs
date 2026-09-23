//! The body against small built worlds: a floor, a wall, stairs, ramps, a
//! low tunnel, a long drop. Yaw 0 faces -Z; walking "forward" goes there.

use lntrn_math::{Vec2, Vec3};

use super::*;
use crate::collide::box_tris;
use crate::testing::real_world;

fn floor() -> Solids {
    let mut s = Solids::new();
    s.add(&box_tris(Vec3::new(-60.0, -1.0, -60.0), Vec3::new(60.0, 0.0, 60.0)));
    s
}

fn forward() -> Controls {
    Controls { walk: Vec2::new(0.0, 1.0), ..Default::default() }
}

fn run(body: &mut Body, controls: &mut Controls, solids: &Solids, steps: usize) {
    for _ in 0..steps {
        step_body(body, 0.0, controls, solids, &PLAYER_GAIT, STEP);
    }
}

/// A flight of `n` steps of `rise`, 0.3 m deep and 2 m wide, going up
/// towards -Z from z = -1, with a landing at the top.
fn stairs(s: &mut Solids, rise: f64, n: usize) {
    for k in 0..n {
        let z0 = -1.0 - (k + 1) as f64 * 0.3;
        s.add(&box_tris(Vec3::new(-1.0, 0.0, z0), Vec3::new(1.0, (k + 1) as f64 * rise, z0 + 0.3)));
    }
    let z = -1.0 - n as f64 * 0.3;
    s.add(&box_tris(Vec3::new(-1.0, 0.0, z - 3.0), Vec3::new(1.0, n as f64 * rise, z)));
}

/// A ramp 2 m high at `degrees`, going up towards -Z from z = -1, with a
/// landing at the top.
fn ramp(s: &mut Solids, degrees: f64) {
    let run = 2.0 / degrees.to_radians().tan();
    let (z0, z1) = (-1.0, -1.0 - run);
    let (a, b) = (Vec3::new(-1.0, 0.0, z0), Vec3::new(1.0, 0.0, z0));
    let (c, d) = (Vec3::new(1.0, 2.0, z1), Vec3::new(-1.0, 2.0, z1));
    s.add(&[[a, b, c], [a, c, d]]);
    s.add(&box_tris(Vec3::new(-1.0, 0.0, z1 - 3.0), Vec3::new(1.0, 2.0, z1)));
}

#[test]
fn walks_up_to_speed_in_a_tenth_of_a_second() {
    let s = floor();
    let mut body = Body::at(Vec3::ZERO);
    let mut c = forward();
    run(&mut body, &mut c, &s, 6);
    assert!((body.speed_flat() - WALK).abs() < 1e-9, "full speed after 0.1 s, got {}", body.speed_flat());
    assert!(body.vel.z < 0.0 && body.grounded && body.pos.y.abs() < 0.01);
    c.walk = Vec2::ZERO;
    run(&mut body, &mut c, &s, 6);
    assert_eq!(body.speed_flat(), 0.0, "and stops as quick");
}

#[test]
fn a_jump_rises_its_height_and_lands() {
    let s = floor();
    let mut body = Body::at(Vec3::ZERO);
    let mut c = Controls { jump: true, ..Default::default() };
    let mut top: f64 = 0.0;
    for _ in 0..120 {
        step_body(&mut body, 0.0, &mut c, &s, &PLAYER_GAIT, STEP);
        top = top.max(body.pos.y);
    }
    assert!((top - JUMP_HEIGHT).abs() < 0.08, "peak {top}");
    assert!(body.grounded && body.landed > 5.0, "landed at {}", body.landed);
}

#[test]
fn sprint_is_forward_only_and_stands_you_up() {
    let s = floor();
    let mut body = Body::at(Vec3::ZERO);
    let mut c = Controls { walk: Vec2::new(0.0, -1.0), sprint: true, crouch_toggle: true, ..Default::default() };
    run(&mut body, &mut c, &s, 30);
    assert!(body.crouched && (body.speed_flat() - CROUCH).abs() < 1e-9, "backwards: no sprint, still crouched");
    c.walk = Vec2::new(0.0, 1.0);
    run(&mut body, &mut c, &s, 30);
    assert!(!body.crouched && (body.speed_flat() - SPRINT).abs() < 1e-9);
}

#[test]
fn a_wall_stops_you_and_you_slide_along_it() {
    let mut s = floor();
    s.add(&box_tris(Vec3::new(2.0, 0.0, -30.0), Vec3::new(3.0, 3.0, 30.0)));
    let mut body = Body::at(Vec3::ZERO);
    // Forward and right: into the wall at an angle.
    let mut c = Controls { walk: Vec2::new(1.0, 1.0), ..Default::default() };
    run(&mut body, &mut c, &s, 120);
    assert!(body.pos.x <= 2.0 - RADIUS + 0.01, "stopped at the wall, x {}", body.pos.x);
    assert!(body.pos.z < -5.0, "and slid along it, z {}", body.pos.z);
    assert!(body.grounded && body.pos.y.abs() < 0.01);
}

#[test]
fn walks_up_stairs_to_the_step_limit() {
    // Long enough to reach the landing, not to walk off its far end.
    for (rise, n, steps) in [(0.2, 8, 55), (0.4, 4, 50)] {
        let mut s = floor();
        stairs(&mut s, rise, n);
        let mut body = Body::at(Vec3::ZERO);
        run(&mut body, &mut forward(), &s, steps);
        let top = rise * n as f64;
        assert!((body.pos.y - top).abs() < 0.02 && body.grounded, "{rise} m steps: at {} not {top}", body.pos.y);
        assert!(body.stepped > 0.1, "the view is told of the stairs");
    }
}

#[test]
fn a_step_too_tall_needs_a_jump() {
    let mut s = floor();
    stairs(&mut s, 0.45, 4);
    let mut body = Body::at(Vec3::ZERO);
    run(&mut body, &mut forward(), &s, 120);
    assert!(body.pos.y.abs() < 0.01, "stayed down at {}", body.pos.y);
    assert!(body.pos.z > -1.0, "stopped at the first step, z {}", body.pos.z);
    let mut c = Controls { jump: true, ..forward() };
    run(&mut body, &mut c, &s, 60);
    assert!(body.pos.y > 0.4, "a jump gets up, at {}", body.pos.y);
}

#[test]
fn walks_down_stairs_without_leaving_them() {
    let mut s = floor();
    stairs(&mut s, 0.2, 8);
    let mut body = Body::at(Vec3::new(0.0, 1.6, -5.0));
    let mut c = Controls { walk: Vec2::new(0.0, -1.0), ..Default::default() };
    for _ in 0..150 {
        step_body(&mut body, 0.0, &mut c, &s, &PLAYER_GAIT, STEP);
        assert!(body.airborne <= STEP + 1e-9, "left the stairs at z {}", body.pos.z);
    }
    assert!(body.pos.y.abs() < 0.01 && body.pos.z > 0.0, "down at {:?}", body.pos);
}

#[test]
fn climbs_45_degrees_but_not_50() {
    let mut s = floor();
    ramp(&mut s, 45.0);
    let mut body = Body::at(Vec3::ZERO);
    run(&mut body, &mut forward(), &s, 55);
    assert!((body.pos.y - 2.0).abs() < 0.05, "up the 45° ramp to {}", body.pos.y);

    let mut s = floor();
    ramp(&mut s, 50.0);
    let mut body = Body::at(Vec3::ZERO);
    run(&mut body, &mut forward(), &s, 150);
    assert!(body.pos.y < 0.6, "not up the 50° one: at {}", body.pos.y);
}

#[test]
fn slides_off_a_slope_too_steep_to_stand_on() {
    let mut s = floor();
    ramp(&mut s, 50.0);
    // Set down halfway up it, standing still.
    let run_len = 2.0 / 50f64.to_radians().tan();
    let mut body = Body::at(Vec3::new(0.0, 1.2, -1.0 - run_len * 0.6));
    body.grounded = false;
    run(&mut body, &mut Controls::default(), &s, 120);
    assert!(body.pos.y < 0.05, "slid to the bottom, at {}", body.pos.y);
}

#[test]
fn no_standing_up_in_a_low_tunnel() {
    let mut s = floor();
    // A roof 1.3 m up over z -2..-6.
    s.add(&box_tris(Vec3::new(-2.0, 1.3, -6.0), Vec3::new(2.0, 1.6, -2.0)));
    let mut body = Body::at(Vec3::ZERO);
    let mut c = Controls { crouch_toggle: true, ..forward() };
    run(&mut body, &mut c, &s, 80);
    assert!(body.crouched && body.pos.z < -3.0, "crouched in, z {}", body.pos.z);
    c.walk = Vec2::ZERO;
    c.crouch_toggle = true; // try to stand
    run(&mut body, &mut c, &s, 10);
    assert!(body.crouched && !body.want_crouch, "wants up, stays down");
    c.walk = Vec2::new(0.0, 1.0);
    run(&mut body, &mut c, &s, 90);
    assert!(!body.crouched, "stands once out, z {}", body.pos.z);
}

#[test]
fn a_long_fall_never_goes_through_a_thin_floor() {
    let mut s = Solids::new();
    s.add(&box_tris(Vec3::new(-5.0, 0.0, -5.0), Vec3::new(5.0, 0.05, 5.0)));
    let mut body = Body::at(Vec3::new(0.0, 60.0, 0.0));
    body.grounded = false;
    run(&mut body, &mut Controls::default(), &s, 300);
    assert!(body.grounded && (body.pos.y - 0.05).abs() < 0.01, "on the slab at {}", body.pos.y);
    assert!(body.landed > 30.0, "a hard landing: {}", body.landed);
}

#[test]
fn crouch_walks_up_a_full_step() {
    let mut s = floor();
    stairs(&mut s, 0.4, 4);
    let mut body = Body::at(Vec3::ZERO);
    let mut c = Controls { crouch_toggle: true, ..forward() };
    run(&mut body, &mut c, &s, 70);
    assert!(body.crouched && (body.pos.y - 1.6).abs() < 0.02, "crouched up to {}", body.pos.y);
}

#[test]
fn the_real_world_holds_you_up_and_the_ramp_leads_onto_the_pad() {
    let s = real_world();
    assert!(s.len() > 10_000, "{} solid triangles", s.len());
    // At the spawn, dropped from a little above the ground: lands, stays.
    let mut body = Body::at(Vec3::new(0.0, 5.0, 6.0));
    body.grounded = false;
    run(&mut body, &mut Controls::default(), &s, 120);
    let spawn_y = body.pos.y;
    assert!(body.grounded && spawn_y.abs() < 3.0, "stood at the spawn at {spawn_y}");
    run(&mut body, &mut Controls::default(), &s, 120);
    assert!((body.pos.y - spawn_y).abs() < 1e-6, "and stays put");
    // At the foot of the access ramp east of the pad, walking west (-X, yaw
    // a quarter turn) up it and onto the concrete (its top is 1.11 m).
    let mut body = Body::at(Vec3::new(17.5, 5.0, 38.0));
    body.grounded = false;
    run(&mut body, &mut Controls::default(), &s, 60);
    let mut c = forward();
    for _ in 0..90 {
        step_body(&mut body, std::f64::consts::FRAC_PI_2, &mut c, &s, &PLAYER_GAIT, STEP);
    }
    assert!(body.grounded && (body.pos.y - 1.11).abs() < 0.05, "on the pad at {:?}", body.pos);
    assert!(body.pos.x < 10.0, "past its edge, x {}", body.pos.x);
}

#[test]
fn climbs_the_building_stairs_without_shaking() {
    // The proving ground's roof stairs: 0.19 m steps, 0.29 m deep, against
    // the building's wall. Once climbing, the body never drops back.
    let s = real_world();
    let mut body = Body::at(Vec3::new(3.6, 3.0, 41.5));
    body.grounded = false;
    run(&mut body, &mut Controls::default(), &s, 60);
    let mut c = forward();
    let mut highest = body.pos.y;
    for _ in 0..50 {
        step_body(&mut body, std::f64::consts::PI, &mut c, &s, &PLAYER_GAIT, STEP);
        assert!(body.pos.y > highest - 0.02, "dropped back from {highest} to {} at z {}", body.pos.y, body.pos.z);
        highest = highest.max(body.pos.y);
    }
    assert!((body.pos.y - 4.16).abs() < 0.03, "up at the roof's height, {}", body.pos.y);
}

#[test]
fn a_shove_moves_you_and_fades() {
    let s = floor();
    let mut body = Body::at(Vec3::ZERO);
    body.push = Vec3::new(0.0, 0.0, 5.0);
    run(&mut body, &mut Controls::default(), &s, 60);
    assert!(body.pos.z > 0.5 && body.pos.z < 1.2, "shoved {} m", body.pos.z);
    assert!(body.push.length() < 0.1 && body.speed_flat() < 1e-9, "and it fades, leaving no walk of its own");
    // Into a wall: stopped by it, no bounce back.
    let mut s = floor();
    s.add(&box_tris(Vec3::new(-2.0, 0.0, 0.5), Vec3::new(2.0, 3.0, 1.0)));
    let mut body = Body::at(Vec3::ZERO);
    body.push = Vec3::new(0.0, 0.0, 5.0);
    run(&mut body, &mut Controls::default(), &s, 60);
    assert!((body.pos.z - (0.5 - RADIUS)).abs() < 0.02, "against the wall at {}", body.pos.z);
}
