//! Things lying about to pick up: what's set down on whatever is under its
//! spot when a run begins (how many rounds are in each box is down to
//! luck), what the dead drop and what the player throws down. The player
//! takes one by looking at it within reach and pressing E.

use std::collections::HashMap;

use bevy_ecs::prelude::*;
use lntrn_math::{Mat4, Quat, Vec3};

use crate::loot::{Dice, Kind, Stack};
use crate::render::MeshId;
use crate::world::{Look, Model, Placed, Solid};

/// How far away something can be taken from, and how near the middle of
/// the view it must be (the cosine of the angle off it).
pub const REACH: f64 = 2.6;
const AIM: f64 = 0.965;

/// Where a thing lies: what, how many (fewest to most), where (game x
/// and z), a height to look down from for its floor (so one inside a
/// building lands on its floor, not its roof), and which way it's turned.
pub type Spot = (Kind, (u32, u32), f64, f64, f64, f64);
/// How many rounds a box can hold, fewest to most.
pub const ROUNDS: (u32, u32) = (8, 16);

/// Every kind of thing's mesh.
#[derive(Resource, Clone, Default)]
pub struct Meshes(pub HashMap<Kind, MeshId>);

#[derive(Component, Clone, Copy, Debug)]
pub struct Pickup {
    pub stack: Stack,
    /// Its middle, for looking at.
    pub at: Vec3,
}

/// Set everything down in its spot (any left from before go first), the
/// boxes filled by `seed`'s luck.
pub fn scatter(world: &mut World, seed: u32, spots: &[Spot]) {
    clear(world);
    let mut dice = Dice(seed | 1);
    for &(kind, (lo, hi), x, hint, z, yaw) in spots {
        let stack = Stack::new(kind, dice.range(lo, hi));
        set_down(world, stack, Vec3::new(x, hint, z), yaw);
    }
}

/// Lay `stack` on whatever is under `above` (from a metre over it), turned
/// `yaw`. Whether there was a floor to lay it on.
pub fn set_down(world: &mut World, stack: Stack, above: Vec3, yaw: f64) -> bool {
    let Some(mesh) = world.get_resource::<Meshes>().and_then(|m| m.0.get(&stack.kind)).copied() else { return false };
    let from = above + Vec3::new(0.0, 1.0, 0.0);
    let Some(hit) = world.resource::<Solid>().0.raycast(from, Vec3::new(0.0, -1.0, 0.0), 20.0) else { return false };
    let model = Mat4::from_translation(hit.point) * Mat4::from_quat(Quat::from_rotation_y(yaw));
    world.spawn((Pickup { stack, at: hit.point + Vec3::new(0.0, 0.06, 0.0) }, Placed(model), Model(mesh), Look::default()));
    true
}

pub fn clear(world: &mut World) {
    let all: Vec<Entity> = world.query_filtered::<Entity, With<Pickup>>().iter(world).collect();
    for e in all {
        world.despawn(e);
    }
}

/// What the eye at `eye`, looking along `dir`, could take: the nearest
/// thing within reach, near the middle of the view, nothing in between.
pub fn in_view(world: &mut World, eye: Vec3, dir: Vec3) -> Option<(Entity, Stack)> {
    let mut best: Option<(Entity, Stack, f64)> = None;
    for (e, p) in world.query::<(Entity, &Pickup)>().iter(world) {
        let to = p.at - eye;
        let d = to.length();
        if !(1e-6..=REACH).contains(&d) || to.dot(dir) / d < AIM {
            continue;
        }
        if best.is_none_or(|(_, _, b)| d < b) {
            best = Some((e, p.stack, d));
        }
    }
    let (e, stack, d) = best?;
    let at = world.get::<Pickup>(e)?.at;
    let blocked = world.resource::<Solid>().0.raycast(eye, (at - eye) * (1.0 / d), d - 0.1).is_some();
    (!blocked).then_some((e, stack))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn everything_lands_where_it_can_be_walked_to_and_seen_up_close() {
        let mut world = World::new();
        let solids = crate::testing::real_world();
        let nav = crate::zombie::nav::NavGrid::build(&solids, crate::player::capsule(false), crate::testing::COURSE_HALF);
        world.insert_resource(Solid(solids));
        world.insert_resource(Meshes(crate::loot::ALL.iter().map(|&k| (k, MeshId::placeholder())).collect()));
        scatter(&mut world, 1234, &crate::testing::COURSE_PICKUPS);
        let all: Vec<Pickup> = world.query::<&Pickup>().iter(&world).copied().collect();
        assert_eq!(all.len(), crate::testing::COURSE_PICKUPS.len(), "every spot had a floor under it");
        let solids = &world.resource::<Solid>().0;
        for p in &all {
            // Outdoors, on the ground something could stand on: not a
            // branch or a rock's top. (Indoors the grid knows only the roof.)
            if solids.raycast(p.at, Vec3::new(0.0, 1.0, 0.0), 3.0).is_some() {
                continue;
            }
            let floor = nav.height_at(p.at).unwrap_or_else(|| panic!("{:?} is nowhere to stand", p));
            assert!((p.at.y - 0.06 - floor).abs() < 0.3, "{:?} lies {:.2} m off the floor", p, p.at.y - 0.06 - floor);
        }
        // The one in the building is on its floor (the pad's top, 1.11 m),
        // not its roof.
        let inside = all.iter().find(|p| p.stack.kind == Kind::Medkit && (p.at.z - 45.5).abs() < 0.1).expect("the building's medkit");
        assert!((inside.at.y - 1.11 - 0.06).abs() < 0.05, "at {}", inside.at.y);
        let boxes: Vec<u32> = all.iter().filter(|p| p.stack.kind == Kind::Rounds).map(|p| p.stack.count).collect();
        assert_eq!(boxes.len(), 8);
        assert!(boxes.iter().all(|n| (ROUNDS.0..=ROUNDS.1).contains(n)), "{boxes:?}");
        assert!(boxes.iter().any(|n| *n != boxes[0]), "all alike: {boxes:?}");
        // Standing over one, looking at it: that one. Looking away: none.
        let p = all[0];
        let eye = p.at + Vec3::new(0.0, 1.5, 1.0);
        let dir = (p.at - eye).normalize();
        assert!(in_view(&mut world, eye, dir).is_some_and(|(_, stack)| stack == p.stack));
        assert!(in_view(&mut world, eye, Vec3::new(0.0, 0.0, -1.0)).is_none());
        scatter(&mut world, 99, &crate::testing::COURSE_PICKUPS);
        assert_eq!(world.query::<&Pickup>().iter(&world).count(), crate::testing::COURSE_PICKUPS.len(), "a fresh set, not a second one");
    }
}
