//! Things lying about to pick up: what's set down on whatever is under its
//! spot when a run begins (how many rounds are in each box is down to
//! luck), what the dead drop and what the player throws down. The player
//! takes one by looking at it within reach and pressing E. In a holdout
//! what's set down doesn't lie there long (`Ground`): it's marked by a
//! ring and a beam of light to find it by in the dark, blinks as its time
//! runs out, and is gone.

use std::collections::HashMap;

use bevy_ecs::prelude::*;
use lntrn_math::{Mat4, Quat, Vec3};

use crate::loot::{Dice, Kind, Stack};
use crate::render::{MeshId, Renderer, Vertex};
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
    /// How long it has left to lie there, if it's to go; and where it
    /// lies (it's put out of sight between blinks).
    pub left: Option<f64>,
    model: Mat4,
}

/// How what's set down on the ground here fares: how long it lies there
/// before it's gone (a holdout's: none, it stays), and then it's marked.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct Ground {
    pub fades: Option<f64>,
}

/// A thing on its way out blinks for this long at the last, this many
/// times a second at first and at the very end.
const BLINKS: f64 = 6.0;
const BLINK_RATE: (f64, f64) = (2.0, 7.0);

/// The light a thing's marked by: whose it is.
#[derive(Component, Clone, Copy, Debug)]
pub struct Mark {
    of: Entity,
    model: Mat4,
}

/// The marks' meshes, by what's marked: rounds, what mends, armor, and
/// anything else.
#[derive(Resource, Clone, Copy)]
pub struct Marks {
    ammo: MeshId,
    mend: MeshId,
    armor: MeshId,
    other: MeshId,
}

impl Marks {
    fn of(&self, kind: Kind) -> MeshId {
        // (Any gun's: belts and fuel too.)
        if crate::weapon::Weapon::ALL.iter().any(|w| w.spec().ammo == Some(kind)) {
            self.ammo
        } else if matches!(kind, Kind::Bandage | Kind::Medkit) {
            self.mend
        } else if kind == Kind::ArmorPlate || kind.gear().is_some_and(|g| g.armor > 0) {
            self.armor
        } else {
            self.other
        }
    }
}

/// Make the marks' meshes: a ring on the ground and a thin beam up from
/// its middle, alight in a colour (linear) of their own.
pub fn load_marks(renderer: &mut Renderer, world: &mut World) {
    let mut mark = |c: [f32; 3]| {
        let (r, w, tall) = (0.26, 0.03, 1.1);
        let list = [
            (Vec3::new(-r, 0.0, -r), Vec3::new(r, 0.02, -r + w)),
            (Vec3::new(-r, 0.0, r - w), Vec3::new(r, 0.02, r)),
            (Vec3::new(-r, 0.0, -r + w), Vec3::new(-r + w, 0.02, r - w)),
            (Vec3::new(r - w, 0.0, -r + w), Vec3::new(r, 0.02, r - w)),
            (Vec3::new(-0.012, 0.3, -0.012), Vec3::new(0.012, tall, 0.012)),
        ];
        let mut verts = Vec::new();
        for (lo, hi) in list {
            for [a, b, c3] in crate::map::build::box_tris(lo, hi) {
                let n = (b - a).cross(c3 - a).normalize();
                for p in [a, b, c3] {
                    verts.push(Vertex { pos: [p.x as f32, p.y as f32, p.z as f32], normal: [n.x as f32, n.y as f32, n.z as f32], color: [c[0], c[1], c[2], 1.0], emissive: c });
                }
            }
        }
        renderer.add_mesh(&verts)
    };
    let marks = Marks { ammo: mark([1.0, 0.62, 0.14]), mend: mark([0.25, 0.95, 0.38]), armor: mark([0.30, 0.62, 1.0]), other: mark([0.85, 0.85, 0.80]) };
    world.insert_resource(marks);
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
    let left = world.get_resource::<Ground>().and_then(|g| g.fades);
    let e = world.spawn((Pickup { stack, at: hit.point + Vec3::new(0.0, 0.06, 0.0), left, model }, Placed(model), Model(mesh), Look::default())).id();
    // (What's to go is marked: there's no time to hunt for it.)
    if let (Some(_), Some(marks)) = (left, world.get_resource::<Marks>().copied()) {
        let at = Mat4::from_translation(hit.point);
        world.spawn((Mark { of: e, model: at }, Placed(at), Model(marks.of(stack.kind)), Look { emissive: 1.0, fog: 0.6, tint: [1.0; 3] }));
    }
    true
}

/// A step of what lies about (while the game goes on): what's to go has
/// that much less time, blinks as it runs out, and goes; and a mark goes
/// with what it marks (taken, or gone).
pub fn fade(mut commands: Commands, mut things: Query<(Entity, &mut Pickup, &mut Placed), Without<Mark>>, mut marks: Query<(Entity, &Mark, &mut Placed), Without<Pickup>>) {
    let nothing = Mat4::from_scale(Vec3::splat(0.0));
    let mut shown: HashMap<Entity, bool> = HashMap::new();
    for (e, mut p, mut placed) in &mut things {
        let Some(left) = p.left else {
            shown.insert(e, true);
            continue;
        };
        let left = left - crate::player::STEP;
        p.left = Some(left);
        if left <= 0.0 {
            commands.entity(e).despawn();
            continue;
        }
        // (Quicker and quicker, the less there is left.)
        let rate = BLINK_RATE.1 + (BLINK_RATE.0 - BLINK_RATE.1) * (left / BLINKS).clamp(0.0, 1.0);
        let on = left > BLINKS || (left * rate).fract() < 0.6;
        placed.0 = if on { p.model } else { nothing };
        shown.insert(e, on);
    }
    for (e, mark, mut placed) in &mut marks {
        match shown.get(&mark.of) {
            Some(&on) => placed.0 = if on { mark.model } else { nothing },
            None => commands.entity(e).despawn(),
        }
    }
}

pub fn clear(world: &mut World) {
    let all: Vec<Entity> = world.query_filtered::<Entity, Or<(With<Pickup>, With<Mark>)>>().iter(world).collect();
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

    #[test]
    fn what_s_set_down_in_a_holdout_blinks_and_is_gone_and_its_mark_with_it() {
        let mut world = World::new();
        world.insert_resource(Solid(crate::testing::bare_world()));
        world.insert_resource(Meshes(crate::loot::ALL.iter().map(|&k| (k, MeshId::placeholder())).collect()));
        let m = MeshId::placeholder();
        world.insert_resource(Marks { ammo: m, mend: m, armor: m, other: m });
        let mut step = Schedule::default();
        step.add_systems(fade);
        let count = |world: &mut World| (world.query::<&Pickup>().iter(world).count(), world.query::<&Mark>().iter(world).count());
        // Out in the wilds, it lies there for good, unmarked.
        assert!(set_down(&mut world, Stack::one(Kind::Medkit), Vec3::new(2.0, 1.0, 2.0), 0.0));
        for _ in 0..600 {
            step.run(&mut world);
        }
        assert_eq!(count(&mut world), (1, 0));
        // In a holdout: marked, and gone in ten seconds (say).
        world.insert_resource(Ground { fades: Some(10.0) });
        set_down(&mut world, Stack::new(Kind::Rounds, 24), Vec3::new(4.0, 1.0, 2.0), 0.0);
        set_down(&mut world, Stack::one(Kind::Bandage), Vec3::new(6.0, 1.0, 2.0), 0.0);
        assert_eq!(count(&mut world), (3, 2));
        let steps = |seconds: f64| (seconds / crate::player::STEP) as usize;
        let mut seen = (0, 0);
        for k in 0..steps(9.5) {
            step.run(&mut world);
            // (In its last seconds it's there, then not, then there.)
            if k > steps(4.5) {
                let shown = world.query::<(&Pickup, &Placed)>().iter(&world).filter(|(p, _)| p.left.is_some()).all(|(p, placed)| placed.0 == p.model);
                if shown { seen.0 += 1 } else { seen.1 += 1 }
            }
        }
        assert!(seen.0 > 0 && seen.1 > 0, "it blinks: shown {} steps, hidden {}", seen.0, seen.1);
        assert_eq!(count(&mut world), (3, 2), "not yet");
        // One taken: its mark goes with it. The other's time runs out.
        let taken = world.query::<(Entity, &Pickup)>().iter(&world).find(|(_, p)| p.stack.kind == Kind::Bandage).map(|(e, _)| e).unwrap();
        world.despawn(taken);
        step.run(&mut world);
        assert_eq!(count(&mut world), (2, 1));
        for _ in 0..steps(1.0) {
            step.run(&mut world);
        }
        assert_eq!(count(&mut world), (1, 0), "only what was there for good");
    }
}
