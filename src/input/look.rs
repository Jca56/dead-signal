//! Looking about with a stick: how far it's pushed turns the view that
//! fast, gently near the middle for small corrections; held hard over, the
//! turn builds, for turning about. And aim assist: with the crosshair on
//! one of the dead (or near it), the stick's turn drags a little. It only
//! ever slows the view, never moves it.

use bevy_ecs::prelude::*;
use lntrn_math::{Vec2, Vec3};

use crate::player::Body;
use crate::world::Solid;
use crate::zombie::brain::Zombie;

/// The fastest the view turns across, radians a second; up and down, a
/// share of that; the curve that keeps small pushes fine.
const TURN: f64 = 3.4;
const PITCH: f64 = 0.65;
const CURVE: f64 = 2.0;
/// Held over past this, the turn builds after a moment, over a moment
/// more, to this many times as fast.
const HARD_OVER: f64 = 0.92;
const BUILD_AFTER: f64 = 0.2;
const BUILD_FOR: f64 = 0.35;
const BUILD: f64 = 1.7;

/// How far to turn this frame (`dt`), for a stick pushed to `stick` (x
/// right, y up, its dead zone out): radians right and up. `hard_over` is
/// how long it's been held hard over, kept up to date.
pub fn stick_turn(stick: Vec2, hard_over: &mut f64, sensitivity: f64, invert: bool, dt: f64) -> Vec2 {
    let m = stick.length();
    *hard_over = if stick.x.abs() >= HARD_OVER { *hard_over + dt } else { 0.0 };
    if m < 1e-9 {
        return Vec2::ZERO;
    }
    let build = 1.0 + (BUILD - 1.0) * ((*hard_over - BUILD_AFTER) / BUILD_FOR).clamp(0.0, 1.0);
    let rate = stick * (m.min(1.0).powf(CURVE) / m * TURN * sensitivity);
    Vec2::new(rate.x * build, rate.y * PITCH * if invert { -1.0 } else { 1.0 }) * dt
}

/// Where on one of the dead the crosshair is drawn to: its chest.
const CHEST: f64 = 1.2;
/// How near the crosshair has to be to slow: this many times the angle a
/// body's half-width takes up, at most this wide; out to this far off.
const HALF_WIDTH: f64 = 0.5;
const ZONE: f64 = 2.5;
const WIDEST: f64 = 0.3;
const REACH: f64 = 40.0;
/// The least of a turn left dead on one: from the hip, and down the
/// sights.
const SLOW_HIP: f64 = 0.6;
const SLOW_AIMED: f64 = 0.45;

/// How much of a stick's turn is left, the eye at `eye` looking along
/// `dir`, `ads` down the sights (0–1): all of it with none of the dead
/// near the crosshair, less the nearer it is to one in sight.
pub fn slowdown(world: &mut World, eye: Vec3, dir: Vec3, ads: f64) -> f64 {
    let mut best: Option<(f64, Vec3, f64)> = None;
    for (z, body) in world.query::<(&Zombie, &Body)>().iter(world) {
        if z.dead() {
            continue;
        }
        let to = body.pos + Vec3::new(0.0, CHEST, 0.0) - eye;
        let d = to.length();
        if !(0.5..=REACH).contains(&d) {
            continue;
        }
        let way = to * (1.0 / d);
        let off = dir.dot(way).clamp(-1.0, 1.0).acos();
        let zone = ((HALF_WIDTH / d).atan() * ZONE).min(WIDEST);
        let pull = 1.0 - off / zone;
        if pull > best.map_or(0.0, |b| b.0) {
            best = Some((pull, way, d));
        }
    }
    let Some((pull, way, d)) = best else { return 1.0 };
    // (Not through a wall.)
    if world.get_resource::<Solid>().is_some_and(|s| s.0.raycast(eye, way, d - 0.6).is_some()) {
        return 1.0;
    }
    let slow = SLOW_HIP + (SLOW_AIMED - SLOW_HIP) * ads.clamp(0.0, 1.0);
    1.0 - (1.0 - slow) * pull
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collide::{Solids, box_tris};

    const DT: f64 = 1.0 / 60.0;

    #[test]
    fn a_small_push_turns_finely_and_a_full_one_fast() {
        let mut held = 0.0;
        let small = stick_turn(Vec2::new(0.3, 0.0), &mut held, 1.0, false, DT);
        let full = stick_turn(Vec2::new(1.0, 0.0), &mut held, 1.0, false, DT);
        assert!((full.x / DT - TURN).abs() < 1e-9, "full tilt turns at {:.2}", full.x / DT);
        assert!(small.x < full.x * 0.3 * 0.5, "a third of the way turns much less than a third as fast");
        let up = stick_turn(Vec2::new(0.0, 1.0), &mut 0.0, 1.0, false, DT);
        let inverted = stick_turn(Vec2::new(0.0, 1.0), &mut 0.0, 1.0, true, DT);
        assert!(up.y > 0.0 && up.y < full.x && inverted.y == -up.y);
    }

    #[test]
    fn held_hard_over_the_turn_builds_and_letting_up_resets_it() {
        let mut held = 0.0;
        let first = stick_turn(Vec2::new(1.0, 0.0), &mut held, 1.0, false, DT);
        let mut last = first;
        for _ in 0..60 {
            last = stick_turn(Vec2::new(1.0, 0.0), &mut held, 1.0, false, DT);
        }
        assert!((last.x / first.x - BUILD).abs() < 1e-9, "built to {:.2}×", last.x / first.x);
        stick_turn(Vec2::new(0.5, 0.0), &mut held, 1.0, false, DT);
        assert_eq!(held, 0.0);
    }

    fn world_with_one_ahead(wall: bool) -> World {
        let mut w = World::new();
        let mut s = Solids::new();
        if wall {
            s.add(&box_tris(Vec3::new(-3.0, 0.0, -6.0), Vec3::new(3.0, 3.0, -5.5)));
        }
        w.insert_resource(Solid(s));
        w.spawn((Zombie::new(0.0, 1), Body::at(Vec3::new(0.0, 0.0, -10.0))));
        w
    }

    #[test]
    fn it_drags_on_one_of_the_dead_in_sight_and_nowhere_else() {
        let eye = Vec3::new(0.0, 1.2, 0.0);
        let mut w = world_with_one_ahead(false);
        let on = slowdown(&mut w, eye, Vec3::new(0.0, 0.0, -1.0), 0.0);
        let aimed = slowdown(&mut w, eye, Vec3::new(0.0, 0.0, -1.0), 1.0);
        let near = slowdown(&mut w, eye, Vec3::new(0.08, 0.0, -1.0).normalize(), 0.0);
        let away = slowdown(&mut w, eye, Vec3::new(1.0, 0.0, 0.0), 0.0);
        assert!((on - SLOW_HIP).abs() < 1e-6 && (aimed - SLOW_AIMED).abs() < 1e-6, "{on} {aimed}");
        assert!(near > on && near < 1.0, "a little off it, a little drag: {near}");
        assert_eq!(away, 1.0);
        let mut walled = world_with_one_ahead(true);
        assert_eq!(slowdown(&mut walled, eye, Vec3::new(0.0, 0.0, -1.0), 0.0), 1.0, "through a wall");
    }
}
