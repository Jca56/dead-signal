//! How long the dead take, a crowd of them after the player on the real
//! map: run by hand (`cargo test --release bench -- --ignored --nocapture`).

use std::time::{Duration, Instant};

use bevy_ecs::prelude::*;
use lntrn_math::Vec3;

use super::brain::Zombie;
use super::figure::{Figure, Model};
use super::nav::NavGrid;
use crate::player::{Body, Player, STEP, capsule};
use crate::world::{Blend, Solid};

fn crowd(n: usize, player: Option<Vec3>) -> World {
    let solids = crate::testing::real_world();
    let nav = NavGrid::build(&solids, capsule(false), crate::testing::COURSE_HALF);
    let mut world = World::new();
    let player = player.unwrap_or(Vec3::new(0.0, nav.height_at(Vec3::new(0.0, 50.0, 6.0)).unwrap(), 6.0));
    // Round the player, 6–40 m off, every one hunting them.
    let mut placed = 0;
    let mut k = 0u32;
    while placed < n {
        k += 1;
        let a = f64::from(k) * 2.399;
        let r = 6.0 + (f64::from(k) * 7.3) % 34.0;
        let at = Vec3::new(player.x + a.cos() * r, 0.0, player.z + a.sin() * r);
        let Some(h) = nav.height_at(at) else { continue };
        let mut z = Zombie::new(0.0, k);
        z.hurt(1.0, false, false, player);
        world.spawn((z, Body::at(Vec3::new(at.x, h, at.z)), Figure::default(), super::Beat::new(k)));
        placed += 1;
    }
    world.spawn((Body::at(player), Player));
    world.insert_resource(Solid(solids));
    world.insert_resource(super::Nav(Some(nav)));
    world.insert_resource(super::Noises::default());
    world.insert_resource(super::Horde::default());
    world.insert_resource(Blend::default());
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/models/shambler.glb");
    let gltf = lntrn_model::Gltf::load(path).expect("shambler.glb");
    let rig = crate::assets::Rigged { mesh: crate::render::FigureMeshId::placeholder(), gltf, skin: 0 };
    world.insert_resource(Model::new(rig).expect("the model"));
    world
}

#[test]
#[ignore]
fn bench() {
    // Out in the open, walking; then stood on a car's roof, where none of
    // them can find a way up.
    let car_roof = {
        let solids = crate::testing::real_world();
        let hit = solids.raycast(Vec3::new(33.0, 10.0, 26.5), Vec3::new(0.0, -1.0, 0.0), 20.0).unwrap();
        hit.point
    };
    for (n, on) in [(40, None), (80, None), (80, Some(car_roof)), (160, None), (160, Some(car_roof))] {
        let mut world = crowd(n, on);
        let (mut fixed, mut frame) = (Schedule::default(), Schedule::default());
        fixed.add_systems(super::think);
        frame.add_systems(super::pose);
        // The player walks a slow circle, so the dead keep finding their way.
        let seconds = 20.0;
        let frames = (seconds / STEP) as usize;
        let (mut think, mut pose, mut worst) = (Duration::ZERO, Duration::ZERO, Duration::ZERO);
        for i in 0..frames {
            let t = i as f64 * STEP;
            let at = Vec3::new(8.0 * (t * 0.3).cos(), 0.9, 6.0 + 8.0 * (t * 0.3).sin());
            if on.is_none() {
                for mut b in world.query_filtered::<&mut Body, With<Player>>().iter_mut(&mut world) {
                    b.pos = at;
                }
            }
            let started = Instant::now();
            fixed.run(&mut world);
            let a = started.elapsed();
            let started = Instant::now();
            frame.run(&mut world);
            let b = started.elapsed();
            think += a;
            pose += b;
            // (The first run of a schedule sets it up: not counted.)
            if i > 0 {
                worst = worst.max(a + b);
            }
        }
        let states: Vec<String> = world.query::<&Zombie>().iter(&world).map(|z| format!("{:?}", z.state).chars().take(6).collect()).collect();
        let you = world.query_filtered::<&Body, With<Player>>().iter(&world).next().unwrap().pos;
        let near = world.query::<(&Zombie, &Body)>().iter(&world).filter(|(_, b)| Vec3::new(b.pos.x - you.x, 0.0, b.pos.z - you.z).length() < 6.0).count();
        eprintln!("STATES {:?} near {near}", states.iter().fold(std::collections::BTreeMap::new(), |mut m, s| { *m.entry(s.clone()).or_insert(0) += 1; m }));
        let per = |d: Duration| d.as_secs_f64() * 1000.0 / frames as f64;
        eprintln!("BENCH {n} dead{}: think {:.3} ms, pose {:.3} ms a frame; worst {:.2} ms", if on.is_some() { " (you on a car roof)" } else { "" }, per(think), per(pose), worst.as_secs_f64() * 1000.0);
    }
}
