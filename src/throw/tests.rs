//! Throwing in a small built world: where a throw comes down, a Molotov's
//! fire and the dead it catches, a pipe bomb's lure and its blast.

use lntrn_math::Vec3;

use super::*;
use crate::collide::{Solids, box_tris};
use crate::zombie::kind::Kind as Dead;
use crate::zombie::looks::Theme;

fn floor() -> Solids {
    let mut s = Solids::new();
    s.add(&box_tris(Vec3::new(-60.0, -1.0, -60.0), Vec3::new(60.0, 0.0, 60.0)));
    s
}

/// A world with a floor, the player at `player`, and what the dead and
/// the throws need.
fn world(player: Vec3) -> World {
    let mut w = World::new();
    w.insert_resource(Solid(floor()));
    w.insert_resource(Horde::default());
    w.insert_resource(Booms::default());
    w.insert_resource(zombie::Noises::default());
    w.insert_resource(zombie::Heat::default());
    w.spawn((Player, Body::at(player)));
    w
}

fn steps(w: &mut World, seconds: f64) {
    for _ in 0..(seconds / STEP) as usize {
        step(w);
    }
}

#[test]
fn a_throw_comes_down_where_its_arc_says() {
    let s = floor();
    let from = Vec3::new(0.0, 1.5, 0.0);
    let vel = launch(Vec3::new(0.0, 0.0, -1.0));
    assert!(vel.y > 0.0 && vel.z < 0.0 && (vel.length() - THROW_SPEED).abs() < 1e-9);
    let (dots, lands) = arc(&s, from, vel, 0.05);
    let lands = lands.expect("it comes down");
    assert!(lands.y.abs() < 0.05 && lands.z < -10.0 && lands.z > -40.0, "{lands:?}");
    assert!(dots.len() > 10 && dots.iter().all(|d| d.y > 0.0));
    // Thrown for real, it bursts where the arc said.
    let mut w = world(Vec3::new(0.0, 0.0, 30.0));
    throw(&mut w, Throwable::Molotov, from, vel);
    steps(&mut w, 3.0);
    let fire = w.query::<&Fire>().iter(&w).next().copied().expect("a fire");
    assert!((fire.at - lands).length() < 0.3, "{:?} vs {lands:?}", fire.at);
}

#[test]
fn the_dead_walking_into_fire_burn_and_burn_on_after() {
    let mut w = world(Vec3::new(0.0, 0.0, 30.0));
    w.spawn(Fire { at: Vec3::ZERO, radius: 3.0, left: 8.0, crackle_in: 0.0 });
    zombie::spawn_kind(&mut w, Vec3::new(1.0, 0.0, 0.0), 0.0, Dead::Shambler, Theme::Drifter);
    let z = w.query_filtered::<Entity, With<Zombie>>().iter(&w).next().unwrap();
    steps(&mut w, 1.0);
    let hp = |w: &World| w.get::<Zombie>(z).unwrap().hp;
    let after = hp(&w);
    assert!(after < 150.0 - 20.0 && w.get::<Burning>(z).is_some(), "burning: {after}");
    // Out of the fire, it burns on a while, then goes out.
    w.get_mut::<Body>(z).unwrap().pos = Vec3::new(20.0, 0.0, 0.0);
    steps(&mut w, 1.0);
    assert!(hp(&w) < after - 20.0);
    steps(&mut w, 3.0);
    assert!(w.get::<Burning>(z).is_none(), "still burning");
    // Standing in it scorches the player.
    let mut w = world(Vec3::new(0.5, 0.0, 0.0));
    w.spawn(Fire { at: Vec3::ZERO, radius: 3.0, left: 8.0, crackle_in: 0.0 });
    steps(&mut w, 1.0);
    assert!(w.resource::<Booms>().scorched > 10.0);
}

#[test]
fn a_pipe_bomb_draws_the_dead_then_blows_them_apart() {
    let mut w = world(Vec3::new(0.0, 0.0, 30.0));
    for x in [2.0, -3.0, 5.0] {
        zombie::spawn_kind(&mut w, Vec3::new(x, 0.0, 0.0), 0.0, Dead::Shambler, Theme::Drifter);
    }
    zombie::spawn_kind(&mut w, Vec3::new(0.0, 0.0, 20.0), 0.0, Dead::Shambler, Theme::Drifter);
    throw(&mut w, Throwable::PipeBomb, Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.0, 1.0, 0.0));
    steps(&mut w, 1.0);
    let lures = w.resource::<Horde>().lures.clone();
    assert_eq!(lures.len(), 1, "it draws them while it beeps");
    assert!(lures[0].y < 0.2, "come to rest on the ground: {:?}", lures[0]);
    steps(&mut w, FUSE);
    let booms = w.resource::<Booms>();
    assert_eq!(booms.kills.len(), 3, "the three close by");
    assert!(booms.blasted == 0.0 && booms.shake > 0.0, "the player's out of its reach, but it shakes them");
    assert!(w.query::<&Thrown>().iter(&w).next().is_none() && w.resource::<Horde>().lures.is_empty());
}

#[test]
fn a_beeping_pipe_bomb_draws_off_one_hunting_the_player() {
    use crate::zombie::brain::{Senses, State};
    let s = floor();
    let mut z = Zombie::new(0.0, 7);
    let body = Body::at(Vec3::ZERO);
    let player = Some(Vec3::new(0.0, 0.0, -10.0));
    let lure = [Vec3::new(15.0, 0.0, 0.0)];
    let searches = std::cell::Cell::new(u32::MAX);
    let senses = |lures| Senses { lures, solids: &s, nav: None, player, noises: &[], alerts: &[], searches: &searches, sight: 1.0 };
    for _ in 0..10 {
        z.think(&body, &senses(&[]), STEP);
    }
    assert_eq!(z.state, State::Hunt);
    z.think(&body, &senses(&lure), STEP);
    assert!(matches!(z.state, State::Investigate { at, .. } if at == lure[0]), "{:?}", z.state);
}

#[test]
fn drawn_to_a_pipe_bomb_the_dead_go_quietly() {
    use crate::zombie::brain::Senses;
    let s = floor();
    // A crowd who can all see the player, a pipe bomb beeping between.
    let player = Some(Vec3::new(0.0, 0.0, -10.0));
    let lure = [Vec3::new(10.0, 0.0, -5.0)];
    let searches = std::cell::Cell::new(u32::MAX);
    let mut snarls = 0;
    for seed in 1..20u32 {
        let mut z = Zombie::new(0.0, seed * 7919);
        let body = Body::at(Vec3::new(f64::from(seed) * 0.5 - 5.0, 0.0, 0.0));
        for _ in 0..(2.0 / STEP) as usize {
            let senses = Senses { lures: &lure, solids: &s, nav: None, player, noises: &[], alerts: &[], searches: &searches, sight: 1.0 };
            snarls += z.think(&body, &senses, STEP).sounds.iter().filter(|(sfx, _)| *sfx == Sfx::Snarl).count();
        }
    }
    assert_eq!(snarls, 0, "the crowd snarled {snarls} times");
}
