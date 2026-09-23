//! Things lying about to pick up: bandages and medkits, set down on
//! whatever is under their spots when a run begins. The player takes one by
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

/// Where things lie: what, where (game x and z), a height above the floor
/// meant (so the one inside the building lands on its floor, not its
/// roof), and which way it's turned.
const SPOTS: [(Kit, f64, f64, f64, f64); 6] = [
    (Kit::Bandage, 4.0, 3.0, 3.0, 0.4),
    (Kit::Bandage, 1.0, 2.2, 37.0, 1.1),
    (Kit::Bandage, -1.5, 5.2, 46.0, 2.3),
    (Kit::Bandage, 12.0, 3.0, -46.0, 0.2),
    (Kit::Medkit, 0.0, 2.2, 45.5, 0.6),
    (Kit::Medkit, 2.5, 5.0, -36.5, 2.0),
];

/// The two things' meshes.
#[derive(Resource, Clone, Copy)]
pub struct Meshes {
    pub bandage: MeshId,
    pub medkit: MeshId,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct Pickup {
    pub kit: Kit,
    /// Its middle, for looking at.
    pub at: Vec3,
}

/// Set everything down in its spot (any left from before go first).
pub fn scatter(world: &mut World) {
    clear(world);
    let Some(meshes) = world.get_resource::<Meshes>().copied() else { return };
    let mut placed = Vec::new();
    {
        let solids = &world.resource::<Solid>().0;
        for (kit, x, hint, z, yaw) in SPOTS {
            let from = Vec3::new(x, hint + 1.0, z);
            let Some(hit) = solids.raycast(from, Vec3::new(0.0, -1.0, 0.0), 20.0) else { continue };
            placed.push((kit, hit.point, yaw));
        }
    }
    for (kit, floor, yaw) in placed {
        let mesh = match kit {
            Kit::Bandage => meshes.bandage,
            Kit::Medkit => meshes.medkit,
        };
        let model = Mat4::from_translation(floor) * Mat4::from_quat(Quat::from_rotation_y(yaw));
        world.spawn((Pickup { kit, at: floor + Vec3::new(0.0, 0.06, 0.0) }, Placed(model), Model(mesh), Look::default()));
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
pub fn in_view(world: &mut World, eye: Vec3, dir: Vec3) -> Option<(Entity, Kit)> {
    let mut best: Option<(Entity, Kit, f64)> = None;
    for (e, p) in world.query::<(Entity, &Pickup)>().iter(world) {
        let to = p.at - eye;
        let d = to.length();
        if !(1e-6..=REACH).contains(&d) || to.dot(dir) / d < AIM {
            continue;
        }
        if best.is_none_or(|(_, _, b)| d < b) {
            best = Some((e, p.kit, d));
        }
    }
    let (e, kit, d) = best?;
    let at = world.get::<Pickup>(e)?.at;
    let blocked = world.resource::<Solid>().0.raycast(eye, (at - eye) * (1.0 / d), d - 0.1).is_some();
    (!blocked).then_some((e, kit))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn everything_lands_on_something_and_can_be_seen_up_close() {
        let mut world = World::new();
        world.insert_resource(Solid(crate::testing::real_world()));
        let mesh = MeshId::placeholder();
        world.insert_resource(Meshes { bandage: mesh, medkit: mesh });
        scatter(&mut world);
        let all: Vec<Pickup> = world.query::<&Pickup>().iter(&world).copied().collect();
        assert_eq!(all.len(), SPOTS.len(), "every spot had a floor under it");
        // The one in the building is on its floor (the pad's top, 1.11 m),
        // not its roof.
        let inside = all.iter().find(|p| p.kit == Kit::Medkit && (p.at.z - 45.5).abs() < 0.1).expect("the building's medkit");
        assert!((inside.at.y - 1.11 - 0.06).abs() < 0.05, "at {}", inside.at.y);
        // Standing over one, looking at it: that one. Looking away: none.
        let p = all[0];
        let eye = p.at + Vec3::new(0.0, 1.5, 1.0);
        let dir = (p.at - eye).normalize();
        assert!(in_view(&mut world, eye, dir).is_some_and(|(_, kit)| kit == p.kit));
        assert!(in_view(&mut world, eye, Vec3::new(0.0, 0.0, -1.0)).is_none());
        scatter(&mut world);
        assert_eq!(world.query::<&Pickup>().iter(&world).count(), SPOTS.len(), "a fresh set, not a second one");
    }
}
