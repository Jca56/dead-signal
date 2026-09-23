//! The grid on the real map and on small built ones.

use super::*;
use crate::player::capsule;

#[test]
fn the_real_map_has_ground_and_routes_round_the_shack() {
    let solids = crate::testing::real_world();
    let started = std::time::Instant::now();
    let nav = NavGrid::build(&solids, capsule(false));
    eprintln!("nav: {} open cells in {:.0} ms", nav.open_cells(), started.elapsed().as_secs_f64() * 1000.0);
    assert!(nav.open_cells() > 30_000, "{} open cells", nav.open_cells());
    // The spawn is ground. The shack (game x 4, z -40) stands on its
    // cells: what is there is its roof, a step too tall to reach.
    let ground = nav.height_at(Vec3::new(0.0, 0.0, 6.0)).expect("the spawn is ground");
    assert!(nav.height_at(Vec3::new(4.0, 0.0, -40.0)).is_none_or(|h| h > ground + 2.0), "the shack's floor is open");
    // Round the shack: the route stays down on the ground, and comes out
    // longer than a straight line through its walls.
    let (a, b) = (Vec3::new(4.0, 0.0, -35.0), Vec3::new(4.0, 0.0, -45.0));
    let route = nav.path(a, b).expect("a way round");
    let mut length = 0.0;
    let mut prev = a;
    for p in &route {
        let mut t = 0.0;
        while t <= 1.0 {
            let q = prev + (*p - prev) * t;
            let h = nav.height_at(q).unwrap_or(f64::NAN);
            assert!(h < 2.5, "the route goes over the shack near {q:?} at {h}");
            t += 0.1;
        }
        length += (*p - prev).length();
        prev = *p;
    }
    assert!(length > 11.0, "round, not through: {length:.1} m");
    // Up onto the proving ground by its ramp: the pad is reachable.
    assert!(nav.path(Vec3::new(0.0, 0.0, 6.0), Vec3::new(0.0, 0.0, 38.0)).is_some(), "no way onto the pad");
    // And from the grass behind the building, round and up its stairs
    // onto its roof: the route climbs the stairs' column (x 3–4.2).
    let roof = Vec3::new(0.0, 4.16, 45.5);
    let route = nav.path(Vec3::new(0.0, 0.0, 51.0), roof).expect("no way up onto the roof");
    assert!((route.last().unwrap().y - 4.16).abs() < 0.1, "ends on the roof: {route:?}");
    let mut prev = Vec3::new(0.0, 0.0, 51.0);
    let mut climbed = false;
    for p in &route {
        let mut t = 0.0;
        while t <= 1.0 {
            let q = prev + (*p - prev) * t;
            if let Some(h) = nav.height_at(q)
                && h > 1.5
                && h < 4.0
            {
                climbed |= (3.0..=4.6).contains(&q.x);
            }
            t += 0.05;
        }
        prev = *p;
    }
    assert!(climbed, "reached the roof without the stairs: {route:?}");
    // But no ledge is a step: not up the pad's 1.1 m edge behind the
    // building, nor straight up onto the roof.
    let (edge, below) = (nav.cell_at(Vec3::new(8.0, 0.0, 48.0)).unwrap(), nav.cell_at(Vec3::new(8.0, 0.0, 49.0)).unwrap());
    assert!(!nav.linked(below, edge), "climbed the pad's edge");
    let (roof_edge, pad) = (nav.cell_at(Vec3::new(-3.0, 0.0, 45.0)).unwrap(), nav.cell_at(Vec3::new(-4.0, 0.0, 45.0)).unwrap());
    assert!(!nav.linked(pad, roof_edge), "climbed the wall");
}

#[test]
fn what_cannot_be_reached_is_known_and_the_way_ends_as_near_as_it_gets() {
    let solids = crate::testing::real_world();
    let nav = NavGrid::build(&solids, capsule(false));
    // The open ground is one region; crate tops and car roofs are islands
    // of their own; not every cell its own.
    let n = nav.regions.count();
    assert!((2..2000).contains(&n), "{n} regions");
    // Stood on a car's roof, none can get up: the way found ends on the
    // ground beside the car, and is found at once.
    let roof = solids.raycast(Vec3::new(33.0, 10.0, 26.5), Vec3::new(0.0, -1.0, 0.0), 20.0).unwrap().point;
    let from = Vec3::new(10.0, nav.height_at(Vec3::new(10.0, 50.0, 10.0)).unwrap(), 10.0);
    let started = std::time::Instant::now();
    let route = nav.path(from, roof).expect("a way to beside the car");
    assert!(started.elapsed().as_millis() < 5, "took {:?}", started.elapsed());
    let end = *route.last().unwrap();
    let flat = ((end.x - roof.x).powi(2) + (end.z - roof.z).powi(2)).sqrt();
    assert!(flat < 3.0 && end.y < roof.y - 0.5, "ends at {end:?}, the roof at {roof:?}");
}
