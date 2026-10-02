//! Drops in a small built world: a flare thrown into the open brings a
//! crate down on it; one under a roof gutters out and nothing comes.

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
