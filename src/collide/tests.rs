//! The collision maths against small built solids: floors, walls, steep
//! slopes, ceilings, closest points, rays.

use super::*;

const BODY: Capsule = Capsule { radius: 0.35, height: 1.8 };

fn floor() -> Solids {
    let mut s = Solids::new();
    s.add(&box_tris(Vec3::new(-50.0, -1.0, -50.0), Vec3::new(50.0, 0.0, 50.0)));
    s
}

#[test]
fn standing_on_a_floor_is_pushed_up_onto_it() {
    let s = floor();
    let mut feet = Vec3::new(1.0, -0.2, 2.0);
    let c = s.resolve(BODY, &mut feet);
    assert!(c.floor.is_some() && !c.wall);
    assert!((feet.y - SKIN).abs() < 0.005, "at {}", feet.y);
    assert!(feet.x == 1.0 && feet.z == 2.0, "straight up, not sideways");
    assert!(s.fits(BODY, feet));
    // Resting there, the floor is still underfoot and nothing moves.
    let rest = feet;
    assert!(s.resolve(BODY, &mut feet).floor.is_some());
    assert_eq!(feet, rest);
}

#[test]
fn a_wall_pushes_out_sideways_and_says_so() {
    let mut s = floor();
    s.add(&box_tris(Vec3::new(2.0, 0.0, -5.0), Vec3::new(3.0, 3.0, 5.0)));
    let mut feet = Vec3::new(1.8, 0.0, 0.0);
    let c = s.resolve(BODY, &mut feet);
    assert!(c.wall);
    assert!((feet.x - (2.0 - 0.35)).abs() < 0.01, "against the wall at {}", feet.x);
    assert!(feet.y.abs() < 0.01, "not lifted");
}

#[test]
fn a_steep_slope_never_lifts() {
    // A 60° face: walking into it pushes back, not up it.
    let mut s = Solids::new();
    let (a, b, c) = (Vec3::new(0.0, 0.0, -5.0), Vec3::new(0.0, 0.0, 5.0), Vec3::new(-2.0, 2.0 * 3f64.sqrt(), 0.0));
    s.add(&[[a, b, c]]);
    let mut feet = Vec3::new(-0.3, 0.2, 0.0);
    let before = feet.y;
    let c = s.resolve(BODY, &mut feet);
    assert!(c.wall && c.floor.is_none());
    assert_eq!(feet.y, before);
}

#[test]
fn a_ceiling_pushes_down() {
    let mut s = floor();
    s.add(&box_tris(Vec3::new(-2.0, 1.3, -2.0), Vec3::new(2.0, 1.6, 2.0)));
    assert!(!s.fits(BODY, Vec3::ZERO), "no room to stand");
    assert!(s.fits(Capsule { radius: 0.35, height: 1.2 }, Vec3::ZERO), "room to crouch");
}

#[test]
fn closest_points_agree_with_brute_force() {
    let t = Tri { a: Vec3::new(0.0, 0.0, 0.0), b: Vec3::new(2.0, 0.0, 0.0), c: Vec3::new(0.0, 0.0, 2.0), normal: Vec3::Y, surface: Surface::Stone, off: false, ghost: false };
    // Straight above the middle, off past a corner, and crossing.
    for (p, q, want) in [
        (Vec3::new(0.5, 1.0, 0.5), Vec3::new(0.5, 3.0, 0.5), 1.0),
        (Vec3::new(3.0, 0.5, 3.0), Vec3::new(4.0, 0.5, 4.0), (2.0f64 * 2.0 + 2.0 * 2.0 + 0.25).sqrt()),
        (Vec3::new(0.5, -1.0, 0.5), Vec3::new(0.5, 1.0, 0.5), 0.0),
    ] {
        let (a, b) = closest_segment_triangle(p, q, &t);
        assert!(((a - b).length() - want).abs() < 1e-6, "{p:?}-{q:?}: {} not {want}", (a - b).length());
    }
}

#[test]
fn a_ray_finds_the_nearest_solid_across_the_grid() {
    let mut s = floor();
    // Walls 3 m and 30 m away down -Z, both across the ray.
    s.add_as(&box_tris(Vec3::new(-2.0, 0.0, -3.5), Vec3::new(2.0, 3.0, -3.0)), Surface::Wood);
    s.add_as(&box_tris(Vec3::new(-2.0, 0.0, -30.5), Vec3::new(2.0, 3.0, -30.0)), Surface::Metal);
    let eye = Vec3::new(0.0, 1.7, 0.0);
    let hit = s.raycast(eye, Vec3::new(0.0, 0.0, -1.0), 100.0).expect("a hit");
    assert!((hit.t - 3.0).abs() < 1e-9 && hit.surface == Surface::Wood && hit.normal.z > 0.99);
    // Over the near wall, the far one.
    let hit = s.raycast(Vec3::new(0.0, 3.5, 0.0), Vec3::new(0.0, -0.05, -1.0).normalize(), 100.0).expect("a hit");
    assert!(hit.surface == Surface::Metal && (hit.point.z + 30.0).abs() < 1e-6, "{hit:?}");
    // Down into the floor; nothing past `max`.
    let hit = s.raycast(eye, Vec3::new(0.0, -1.0, 0.0), 10.0).expect("the floor");
    assert!((hit.t - 1.7).abs() < 1e-9 && hit.normal.y > 0.99);
    assert!(s.raycast(eye, Vec3::new(0.0, 0.0, -1.0), 2.0).is_none());
    // Diagonally, far, through many cells.
    let d = Vec3::new(1.0, 0.0, -1.0).normalize();
    s.add_as(&box_tris(Vec3::new(40.0, 0.0, -41.0), Vec3::new(41.0, 3.0, -40.0)), Surface::Dirt);
    let hit = s.raycast(eye, d, 200.0).expect("the far block");
    assert_eq!(hit.surface, Surface::Dirt);
}

#[test]
fn a_window_barrier_stops_a_body_but_not_a_ray() {
    let mut s = floor();
    // A pane across -Z, 2 m off, from the floor up.
    s.add_barrier(&box_tris(Vec3::new(-2.0, 0.0, -2.05), Vec3::new(2.0, 2.5, -1.95)));
    let eye = Vec3::new(0.0, 1.6, 0.0);
    assert!(s.raycast(eye, Vec3::new(0.0, 0.0, -1.0), 10.0).is_none(), "seen and shot through");
    let body = Capsule { radius: 0.35, height: 1.8 };
    assert!(!s.fits(body, Vec3::new(0.0, 0.0, -2.0)), "stood in it");
    let mut feet = Vec3::new(0.0, 0.0, -1.8);
    let touched = s.resolve(body, &mut feet);
    assert!(touched.wall && feet.z > -1.7, "walked into it: {feet:?}");
}
