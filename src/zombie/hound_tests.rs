//! The Hellhound: its model on bones the game knows, what a shot finds of
//! it, and what it does: runs the player down, bites, and burns.

use std::collections::BTreeSet;

use bevy_ecs::prelude::*;
use lntrn_math::{Mat4, Quat, Vec3};

use super::brain::{State, Zombie};
use super::figure::{Clip, Figure, Model};
use super::kind::Kind;
use super::looks::Looks;
use super::tests::{floor, run};
use super::{Horde, Impact};
use crate::loot::Dice;
use crate::player::Body;
use crate::sound::Sfx;

fn model() -> Model {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/models/hound.glb");
    let gltf = lntrn_model::Gltf::load(path).expect("hound.glb");
    Model::new(crate::assets::Rigged { mesh: Vec::new(), gltf, skin: 0 }).expect("its bones named as the dead's are")
}

#[test]
fn the_hounds_model_has_its_clips_and_every_part_is_worn_by_some_hound() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/models/hound.glb");
    let gltf = lntrn_model::Gltf::load(path).expect("hound.glb");
    for clip in ["Run", "Idle", "Attack", "Flinch", "Stumble", "Death"] {
        assert!(gltf.animations.iter().any(|a| a.name.as_deref() == Some(clip)), "no {clip}");
    }
    let names: BTreeSet<String> = gltf.nodes.iter().filter(|n| n.skin.is_some() && n.mesh.is_some()).filter_map(|n| n.name.clone()).collect();
    let mut worn = BTreeSet::new();
    for i in 0..40u32 {
        let l = Looks::hound(&mut Dice(i * 7919 + 1));
        assert!(l.hound && (0.9..=1.15).contains(&l.height), "{l:?}");
        for part in l.parts() {
            assert!(names.contains(&part), "no {part} in hound.glb");
            worn.insert(part);
        }
    }
    assert_eq!(worn, names, "every part worn");
}

#[test]
fn a_shot_finds_a_hounds_body_and_its_head_and_goes_over_its_back() {
    let model = model();
    let (joints, local) = model.pose(Clip::Idle, 0.0);
    let mut f = Figure::of(&Looks::hound(&mut Dice(3)));
    // Standing 10 m ahead of the eye, side on (its head to the right).
    let at = Mat4::from_translation(Vec3::new(0.0, 0.0, -10.0)) * Mat4::from_quat(Quat::from_rotation_y(-std::f64::consts::FRAC_PI_2));
    f.set(at, joints, local, true);
    let ahead = Vec3::new(0.0, 0.0, -1.0);
    let [hips, neck, head] = f.spine().expect("posed");
    assert!(hips.x < -0.2 && neck.x > 0.2 && head.x > neck.x && (0.5..0.95).contains(&hips.y), "{hips:?} {neck:?} {head:?}");
    let (t, in_head) = f.ray(Vec3::new(0.0, hips.y, 0.0), ahead, 50.0).expect("its side");
    assert!(!in_head && (9.5..10.0).contains(&t), "{t}");
    assert!(f.ray(Vec3::new(head.x + 0.05, head.y + 0.03, 0.0), ahead, 50.0).is_some_and(|(_, in_head)| in_head), "its head");
    // Over its back, a man's chest high: a miss.
    assert!(f.ray(Vec3::new(0.0, 1.3, 0.0), ahead, 50.0).is_none());
    // The bar goes over its head, the fire's number by its middle.
    assert!(f.crown().is_some_and(|c| c.y > head.y && c.y < 1.4) && f.chest().is_some_and(|c| (c.y - hips.y).abs() < 0.15));
}

#[test]
fn a_hound_runs_the_player_down_and_its_bite_burns() {
    let s = floor();
    let player = Some(Vec3::new(0.0, 0.0, -30.0));
    let mut hound = Zombie::of(Kind::Hound, 0.0, 9);
    hound.relentless = true;
    hound.state = State::Hunt;
    let mut body = Body::at(Vec3::ZERO);
    // Thirty metres in under four seconds: no outwalking it.
    run(&mut hound, &mut body, &s, None, player, &[], 2.0);
    assert_eq!(hound.clip().0, Clip::Run);
    let bites = run(&mut hound, &mut body, &s, None, player, &[], 4.0);
    assert!((body.pos - Vec3::new(0.0, 0.0, -30.0)).length() < 2.0, "caught up: {:?}", body.pos);
    assert!(bites >= 2, "{bites} bites");
    assert_eq!(Kind::Hound.traits().swipe.leaves, Some(crate::vitals::Affliction::Burn));
    // Bitten, a player's alight a moment.
    let mut world = World::new();
    let who = world.spawn((crate::player::Player(1), Body::at(Vec3::ZERO))).id();
    crate::throw::alight(&mut world, 1);
    assert!(world.get::<crate::throw::Burning>(who).is_some_and(|b| b.left > 0.5 && b.left < 3.0));
}

#[test]
fn fire_is_nothing_to_a_hound_and_dead_it_leaves_a_fire_where_it_fell() {
    let mut world = World::new();
    world.insert_resource(Horde::default());
    let mut z = Zombie::of(Kind::Hound, 0.0, 5);
    z.by = Some(1);
    let hp = z.hp;
    let e = world.spawn((z, Body::at(Vec3::new(3.0, 0.0, 4.0)))).id();
    let hit = |damage: f64, fire: bool| Impact { damage, head: false, limb: true, blow: false, shove: 0.0, stumble: false, takedown: false, fire, at: None };
    let (dir, from) = (Vec3::new(0.0, 0.0, -1.0), Vec3::new(3.0, 1.6, 8.0));
    assert!(!super::hurt(&mut world, e, dir, from, hit(500.0, true)));
    assert_eq!(world.get::<Zombie>(e).unwrap().hp, hp, "untouched");
    assert!(world.get::<super::harm::Harmed>(e).is_none(), "and no bar for it");
    let told = std::mem::take(&mut world.resource_mut::<Horde>().harms);
    assert!(told.len() == 1 && told[0].immune && told[0].amount == 0.0 && told[0].by == Some(1), "{told:?}");
    // A round does for it: a yelp, and it goes up where it fell.
    assert!(super::hurt(&mut world, e, dir, from, hit(500.0, false)));
    let fires: Vec<crate::throw::Fire> = world.query::<&crate::throw::Fire>().iter(&world).copied().collect();
    assert!(fires.len() == 1 && fires[0].at == Vec3::new(3.0, 0.0, 4.0) && fires[0].by == 1, "{fires:?}");
    let said: Vec<Sfx> = world.resource::<Horde>().sounds.iter().map(|s| s.0).collect();
    assert_eq!(said, vec![Sfx::Yelp, Sfx::Ignite]);
    // The fire it left is nothing to the next one through it.
    world.insert_resource(crate::world::Solid(floor()));
    world.insert_resource(crate::throw::Booms::default());
    let next = world.spawn((Zombie::of(Kind::Hound, 0.0, 6), Body::at(Vec3::new(3.0, 0.0, 4.0)))).id();
    for _ in 0..30 {
        crate::throw::step(&mut world);
    }
    assert!(world.get::<crate::throw::Burning>(next).is_none() && world.get::<Zombie>(next).unwrap().hp == hp);
}
