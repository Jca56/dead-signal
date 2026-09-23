//! Things lying about to pick up: bandages, medkits and boxes of rounds,
//! set down on whatever is under their spots when a run begins (how many
//! rounds are in each box is down to luck). The player takes one by
//! looking at it within reach and pressing E.

use bevy_ecs::prelude::*;
use lntrn_math::{Mat4, Quat, Vec3};

use crate::render::MeshId;
use crate::vitals::Kit;
use crate::world::{Look, Model, Placed, Solid};

/// How far away something can be taken from, and how near the middle of
/// the view it must be (the cosine of the angle off it).
pub const REACH: f64 = 2.6;
const AIM: f64 = 0.965;

/// How many rounds a box can hold, fewest to most.
const ROUNDS: (u32, u32) = (8, 16);

/// Something to pick up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    Kit(Kit),
    Rounds(u32),
}

impl Item {
    /// What the prompt calls it.
    pub fn label(self) -> String {
        match self {
            Item::Kit(kit) => kit.name().to_string(),
            Item::Rounds(n) => format!("9MM ROUNDS  ×{n}"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum What {
    Bandage,
    Medkit,
    Ammo,
}

/// Where things lie: what, where (game x and z), a height above the floor
/// meant (so the one inside the building lands on its floor, not its
/// roof), and which way it's turned.
const SPOTS: [(What, f64, f64, f64, f64); 14] = [
    (What::Bandage, 4.0, 3.0, 3.0, 0.4),
    (What::Bandage, 1.0, 2.2, 37.0, 1.1),
    (What::Bandage, -1.5, 5.2, 46.0, 2.3),
    (What::Bandage, 12.0, 3.0, -46.0, 0.2),
    (What::Medkit, 0.0, 2.2, 45.5, 0.6),
    (What::Medkit, 2.5, 5.0, -36.5, 2.0),
    (What::Ammo, -5.0, 3.0, 12.0, 0.3),
    (What::Ammo, 18.0, 3.0, 2.0, 1.9),
    (What::Ammo, -20.0, 3.0, -8.0, 0.8),
    (What::Ammo, 8.0, 3.0, -28.0, 2.6),
    (What::Ammo, -16.0, 3.0, -44.0, 1.2),
    (What::Ammo, 28.0, 3.0, -24.0, 0.1),
    (What::Ammo, -1.9, 2.2, 47.0, 1.6),
    (What::Ammo, 38.0, 3.0, 36.0, 2.2),
];

/// The things' meshes.
#[derive(Resource, Clone, Copy)]
pub struct Meshes {
    pub bandage: MeshId,
    pub medkit: MeshId,
    pub ammo: MeshId,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct Pickup {
    pub item: Item,
    /// Its middle, for looking at.
    pub at: Vec3,
}

/// Set everything down in its spot (any left from before go first), the
/// boxes filled by `seed`'s luck.
pub fn scatter(world: &mut World, seed: u32) {
    clear(world);
    let Some(meshes) = world.get_resource::<Meshes>().copied() else { return };
    let mut seed = seed | 1;
    let mut rounds = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        ROUNDS.0 + seed % (ROUNDS.1 - ROUNDS.0 + 1)
    };
    let mut placed = Vec::new();
    {
        let solids = &world.resource::<Solid>().0;
        for (what, x, hint, z, yaw) in SPOTS {
            let from = Vec3::new(x, hint + 1.0, z);
            let Some(hit) = solids.raycast(from, Vec3::new(0.0, -1.0, 0.0), 20.0) else { continue };
            placed.push((what, hit.point, yaw));
        }
    }
    for (what, floor, yaw) in placed {
        let (item, mesh) = match what {
            What::Bandage => (Item::Kit(Kit::Bandage), meshes.bandage),
            What::Medkit => (Item::Kit(Kit::Medkit), meshes.medkit),
            What::Ammo => (Item::Rounds(rounds()), meshes.ammo),
        };
        let model = Mat4::from_translation(floor) * Mat4::from_quat(Quat::from_rotation_y(yaw));
        world.spawn((Pickup { item, at: floor + Vec3::new(0.0, 0.06, 0.0) }, Placed(model), Model(mesh), Look::default()));
    }
}

pub fn clear(world: &mut World) {
    let all: Vec<Entity> = world.query_filtered::<Entity, With<Pickup>>().iter(world).collect();
    for e in all {
        world.despawn(e);
    }
}

/// What the eye at `eye`, looking along `dir`, could take: the nearest
/// thing within reach, near the middle of the view, nothing in between.
pub fn in_view(world: &mut World, eye: Vec3, dir: Vec3) -> Option<(Entity, Item)> {
    let mut best: Option<(Entity, Item, f64)> = None;
    for (e, p) in world.query::<(Entity, &Pickup)>().iter(world) {
        let to = p.at - eye;
        let d = to.length();
        if !(1e-6..=REACH).contains(&d) || to.dot(dir) / d < AIM {
            continue;
        }
        if best.is_none_or(|(_, _, b)| d < b) {
            best = Some((e, p.item, d));
        }
    }
    let (e, item, d) = best?;
    let at = world.get::<Pickup>(e)?.at;
    let blocked = world.resource::<Solid>().0.raycast(eye, (at - eye) * (1.0 / d), d - 0.1).is_some();
    (!blocked).then_some((e, item))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn everything_lands_where_it_can_be_walked_to_and_seen_up_close() {
        let mut world = World::new();
        let solids = crate::testing::real_world();
        let nav = crate::zombie::nav::NavGrid::build(&solids, crate::player::capsule(false));
        world.insert_resource(Solid(solids));
        let mesh = MeshId::placeholder();
        world.insert_resource(Meshes { bandage: mesh, medkit: mesh, ammo: mesh });
        scatter(&mut world, 1234);
        let all: Vec<Pickup> = world.query::<&Pickup>().iter(&world).copied().collect();
        assert_eq!(all.len(), SPOTS.len(), "every spot had a floor under it");
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
        let inside = all.iter().find(|p| p.item == Item::Kit(Kit::Medkit) && (p.at.z - 45.5).abs() < 0.1).expect("the building's medkit");
        assert!((inside.at.y - 1.11 - 0.06).abs() < 0.05, "at {}", inside.at.y);
        let boxes: Vec<u32> = all.iter().filter_map(|p| if let Item::Rounds(n) = p.item { Some(n) } else { None }).collect();
        assert_eq!(boxes.len(), 8);
        assert!(boxes.iter().all(|n| (ROUNDS.0..=ROUNDS.1).contains(n)), "{boxes:?}");
        assert!(boxes.iter().any(|n| *n != boxes[0]), "all alike: {boxes:?}");
        // Standing over one, looking at it: that one. Looking away: none.
        let p = all[0];
        let eye = p.at + Vec3::new(0.0, 1.5, 1.0);
        let dir = (p.at - eye).normalize();
        assert!(in_view(&mut world, eye, dir).is_some_and(|(_, item)| item == p.item));
        assert!(in_view(&mut world, eye, Vec3::new(0.0, 0.0, -1.0)).is_none());
        scatter(&mut world, 99);
        assert_eq!(world.query::<&Pickup>().iter(&world).count(), SPOTS.len(), "a fresh set, not a second one");
    }
}
