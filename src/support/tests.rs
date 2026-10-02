//! Drops in a small built world: a flare thrown into the open brings a
//! crate down on it; one under a roof gutters out and nothing comes. And
//! a strafing run: what's in its strip under open sky is hit as its
//! rounds pass, and nothing else. And the gunship: in, round its circle
//! shooting the dead it can see in the open, and off again.

use lntrn_math::Vec3;

use super::*;
use crate::collide::{Solids, box_tris};

/// A floor, with a roof over the part of it east of x = 10.
fn world() -> World {
    let mut s = Solids::new();
    s.add(&box_tris(Vec3::new(-60.0, -1.0, -60.0), Vec3::new(60.0, 0.0, 60.0)));
    s.add(&box_tris(Vec3::new(10.0, 3.0, -10.0), Vec3::new(30.0, 3.3, 10.0)));
    let mut w = World::new();
    w.insert_resource(Solid(s));
    w.insert_resource(Horde::default());
    w.insert_resource(Support::default());
    w.insert_resource(crate::throw::Booms::default());
    w
}

fn steps(w: &mut World, seconds: f64) {
    for _ in 0..(seconds / STEP).round() as usize {
        step(w);
    }
}

fn events(w: &mut World) -> Vec<Event> {
    std::mem::take(&mut w.resource_mut::<Support>().events)
}

#[test]
fn a_flare_in_the_open_comes_to_rest_and_a_crate_comes_down_on_it() {
    let mut w = world();
    throw_flare(&mut w, Call::AmmoDrop, Vec3::new(0.0, 1.5, 0.0), crate::throw::launch(Vec3::new(0.0, 0.0, -1.0)), 1);
    steps(&mut w, 4.0);
    let f = *w.query::<&Flare>().single(&w).expect("the flare");
    assert!(f.rest.is_some() && f.sky && f.pos.y < 0.2 && f.pos.z < -8.0, "{f:?}");
    assert!(events(&mut w).is_empty());
    // A while after it's settled, the crate's let go, high over it.
    steps(&mut w, COMES_IN);
    let d = *w.query::<&Drop>().single(&w).expect("the drop");
    assert!(d.down.is_none() && d.height > FROM - FALLS * (COMES_IN + 0.2) && (d.at - f.pos).length() < 0.1, "{d:?}");
    // Down it comes; it lands, once, where the flare is.
    steps(&mut w, FROM / FALLS + 0.5);
    let landed = events(&mut w);
    assert_eq!(landed.len(), 1);
    assert!(matches!(landed[0], Event::Landed { call: Call::AmmoDrop, by: 1, at } if (at - f.pos).length() < 0.1), "{landed:?}");
    assert!(w.query::<&Drop>().single(&w).is_ok_and(|d| d.down.is_some() && d.height == 0.0));
    // The flare burns out, and in time the empty crate's gone too.
    steps(&mut w, BURNS_ON + 1.0);
    assert_eq!(w.query::<&Flare>().iter(&w).count(), 0);
    steps(&mut w, STANDS);
    assert_eq!(w.query::<&Drop>().iter(&w).count(), 0);
    assert!(events(&mut w).is_empty(), "nothing more of it");
}

#[test]
fn a_flare_under_a_roof_gutters_out_and_nothing_comes() {
    let mut w = world();
    throw_flare(&mut w, Call::MedicDrop, Vec3::new(8.0, 1.5, 0.0), crate::throw::launch(Vec3::new(1.0, 0.0, 0.0)), 0);
    steps(&mut w, 3.0 + GUTTERS);
    assert_eq!(events(&mut w), [Event::Guttered { call: Call::MedicDrop, by: 0 }]);
    assert_eq!(w.query::<&Flare>().iter(&w).count(), 0);
    steps(&mut w, COMES_IN + 2.0);
    assert_eq!(w.query::<&Drop>().iter(&w).count(), 0);
    // And one thrown off the edge of everything: given up on.
    throw_flare(&mut w, Call::MedicDrop, Vec3::new(59.0, 1.5, 0.0), Vec3::new(30.0, 5.0, 0.0), 0);
    steps(&mut w, LOST + 0.5);
    assert_eq!(events(&mut w), [Event::Guttered { call: Call::MedicDrop, by: 0 }]);
    clear(&mut w);
    assert_eq!(w.query::<&Flare>().iter(&w).count(), 0);
}

#[test]
fn a_strip_is_marked_where_the_look_lands_running_away_from_the_eye() {
    use strafe::Strip;
    let w = world();
    let solid = &w.resource::<Solid>().0;
    let eye = Vec3::new(0.0, 1.6, 0.0);
    // Looking down at the ground ahead, to the north: there, and open.
    let s = Strip::marked(solid, eye, Vec3::new(0.0, -0.2, -1.0).normalize()).expect("ground ahead");
    assert!(s.open && s.mid.y.abs() < 1e-6 && (s.mid.z + 8.0).abs() < 0.1 && s.dir == Vec3::new(0.0, 0.0, -1.0), "{s:?}");
    let (along, across) = s.place(s.mid + Vec3::new(1.5, 0.0, -5.0));
    assert!((along - (strafe::LONG * 0.5 + 5.0)).abs() < 1e-9 && across.abs() == 1.5, "{along} along, {across} across");
    assert!((s.point(along, across) - (s.mid + Vec3::new(1.5, 0.0, -5.0))).length() < 1e-9 || (s.point(along, -across) - (s.mid + Vec3::new(1.5, 0.0, -5.0))).length() < 1e-9);
    // At the sky: out ahead, on the ground. Under the roof: marked, but
    // not open.
    let s = Strip::marked(solid, eye, Vec3::new(0.0, 0.5, -1.0).normalize()).expect("out ahead");
    assert!(s.open && s.mid.y.abs() < 1e-6 && s.mid.z < -30.0);
    let s = Strip::marked(solid, Vec3::new(15.0, 1.6, 0.0), Vec3::new(1.0, -0.3, 0.0).normalize()).expect("under the roof");
    assert!(!s.open && s.mid.x > 15.0);
    assert!(Strip::marked(solid, eye, Vec3::Y).is_none(), "straight up: no way for it to run");
}

#[test]
fn a_strafing_run_cuts_down_what_s_in_its_strip_in_the_open_as_its_rounds_pass() {
    use crate::player::{Body, Player};
    use crate::zombie::brain::Zombie;
    use strafe::{LONG, RAKES, Strafe, Strip, TO_A_PLAYER, WARNS};
    let mut w = world();
    w.insert_resource(crate::world::Clock::default());
    // A strip running east from x = -20 to 10: its last metres are under
    // the roof's edge (x >= 10 is roofed).
    let strip = Strip { mid: Vec3::new(-5.0, 0.0, 0.0), dir: Vec3::X, open: true };
    let dead = |w: &mut World, at: Vec3| w.spawn((Zombie::new(0.0, 3), Body::at(at))).id();
    let near = dead(&mut w, Vec3::new(-15.0, 0.0, 1.0));
    let far = dead(&mut w, Vec3::new(5.0, 0.0, -2.0));
    let beside = dead(&mut w, Vec3::new(0.0, 0.0, 4.0));
    let roofed = dead(&mut w, Vec3::new(12.0, 0.0, 0.0));
    w.spawn((Player(1), Body::at(Vec3::new(-10.0, 0.0, 0.5))));
    w.spawn((Player(0), Body::at(Vec3::new(-10.0, 0.0, 9.0))));
    strafe::call(&mut w, strip, 1);
    let alive = |w: &World, e: Entity| !w.get::<Zombie>(e).unwrap().dead();
    // Nothing till the plane's there.
    steps(&mut w, WARNS - 0.1);
    assert!(alive(&w, near) && w.resource::<Support>().kills.is_empty() && events(&mut w).is_empty());
    assert!(w.query::<&Strafe>().single(&w).is_ok_and(|s| s.threatens() && s.front() == 0.0));
    // A third of the way through: the near one's down, the far one not yet.
    steps(&mut w, 0.1 + RAKES / 3.0);
    assert!(!alive(&w, near) && alive(&w, far));
    // All of it: the far one too; not the one beside the strip, nor the
    // one under the roof. The kills are whoever called it's.
    steps(&mut w, RAKES);
    assert!(!alive(&w, far) && alive(&w, beside) && alive(&w, roofed));
    assert_eq!(std::mem::take(&mut w.resource_mut::<Support>().kills), [(1, near), (1, far)]);
    assert_eq!(w.get::<Zombie>(near).unwrap().by, Some(1));
    // Its rounds were seen to land all along it, on the ground and on the
    // roof at its end.
    let rounds: Vec<Vec3> = events(&mut w).into_iter().filter_map(|e| if let Event::Round { at, .. } = e { Some(at) } else { None }).collect();
    assert!(rounds.len() > 100 && rounds.iter().all(|at| at.x >= -20.0 && at.x <= 10.0 + 1e-6 && at.z.abs() <= 2.5), "{} rounds", rounds.len());
    assert!(rounds.iter().any(|at| at.x < -15.0) && rounds.iter().any(|at| at.x > 5.0));
    // The player in it was hit, hard; the one beside it only shaken.
    let booms = w.resource::<crate::throw::Booms>();
    assert_eq!((booms.of(1).blasted, booms.of(0).blasted), (TO_A_PLAYER, 0.0));
    assert!(booms.of(0).shake > 0.0);
    // And then it's gone.
    assert!(!w.query::<&Strafe>().single(&w).unwrap().threatens());
    steps(&mut w, 6.0);
    assert_eq!(w.query::<&Strafe>().iter(&w).count(), 0);
    let _ = LONG;
}

#[test]
fn the_gunship_comes_in_circles_the_compound_shooting_what_it_sees_in_the_open_and_goes() {
    use crate::player::{Body, Player};
    use crate::zombie::brain::Zombie;
    use gunship::{ARRIVES, Gunship, HIGH, STAYS};
    let mut w = world();
    w.insert_resource(crate::world::Clock::default());
    let bounds = (Vec3::new(-40.0, 0.0, -15.0), Vec3::new(40.0, 0.0, 15.0));
    let dead = |w: &mut World, at: Vec3| w.spawn((Zombie::new(0.0, 3), Body::at(at))).id();
    let far = dead(&mut w, Vec3::new(-20.0, 0.0, 5.0));
    let near = dead(&mut w, Vec3::new(3.0, 0.0, 2.0));
    let roofed = dead(&mut w, Vec3::new(15.0, 0.0, 0.0));
    w.spawn((Player(0), Body::at(Vec3::new(1.0, 0.0, 0.0))));
    gunship::call(&mut w, 0, bounds);
    let alive = |w: &World, e: Entity| !w.get::<Zombie>(e).unwrap().dead();
    let ship = |w: &mut World| *w.query::<&Gunship>().single(w).expect("the gunship");
    // Coming in: from well off, nothing shot yet.
    assert!((ship(&mut w).place().0 - Vec3::ZERO).length() > 100.0 && !ship(&mut w).on_station());
    steps(&mut w, ARRIVES - 0.1);
    assert!(alive(&w, near) && alive(&w, far) && events(&mut w).is_empty());
    assert_eq!(gunship::left(&mut w), STAYS);
    // Over the compound: on its circle, at its height, inside the bounds'
    // long side; and the one nearest the player is the first it's at.
    steps(&mut w, 0.12);
    let g = ship(&mut w);
    let (at, way) = g.place();
    assert!(g.on_station() && (at.y - HIGH).abs() < 1e-9 && at.x.abs() < 40.0 && way.y == 0.0, "{at:?}");
    assert!(g.firing() && g.lit.is_some_and(|p| (p - Vec3::new(3.0, 0.0, 2.0)).length() < 0.5));
    steps(&mut w, 3.0);
    assert!(!alive(&w, near), "cut down");
    // Then the next; never the one under the roof. The kills are whoever
    // called it's, and no player's touched.
    steps(&mut w, 6.0);
    assert!(!alive(&w, far) && alive(&w, roofed));
    assert_eq!(std::mem::take(&mut w.resource_mut::<Support>().kills), [(0, near), (0, far)]);
    assert!(events(&mut w).iter().filter(|e| matches!(e, Event::Round { .. })).count() > 5);
    assert_eq!(w.resource::<crate::throw::Booms>().of(0), crate::throw::Felt::default());
    assert!(!ship(&mut w).firing(), "nothing left it can see");
    // Called for again: no second one, this one stays as long again.
    gunship::call(&mut w, 1, bounds);
    assert_eq!(w.query::<&Gunship>().iter(&w).count(), 1);
    assert!((gunship::left(&mut w) - (2.0 * STAYS - 9.02)).abs() < 0.1, "{}", gunship::left(&mut w));
    // Its time up, it flies off, and is gone.
    steps(&mut w, 2.0 * STAYS - 9.0);
    assert!(!ship(&mut w).on_station() && gunship::left(&mut w) == 0.0);
    steps(&mut w, 6.0);
    assert_eq!(w.query::<&Gunship>().iter(&w).count(), 0);
}
