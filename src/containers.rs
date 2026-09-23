//! Things to search: crates, lockers, wrecked cars and the locked supply
//! cage, set down at their spots once (they're solid, part of the ground
//! the dead find their way over) and filled afresh every run. The cage
//! needs its key, and every run one locker or car has it.

use std::collections::HashMap;

use bevy_ecs::prelude::*;
use lntrn_math::{Mat4, Quat, Vec3};

use crate::collide::{Solids, Surface};
use crate::loot::grid::Grid;
use crate::loot::tables::{self, Source};
use crate::loot::{Dice, Kind, Stack};
use crate::render::MeshId;
use crate::world::{Look, Model, Placed, Solid};

/// How far off a container can be searched, from the eye to where it's
/// looked at.
pub const REACH: f64 = 2.4;

/// Where each stands: what, where (game x and z), which way its front
/// faces (a yaw: 0 faces -Z), and a height above its floor meant (the
/// locker inside the building, under its roof).
const SPOTS: [(Source, f64, f64, f64, f64); 8] = [
    (Source::Crate, -7.0, 31.5, 0.3, 3.0),
    (Source::Crate, -1.0, 38.0, -0.4, 3.0),
    (Source::Crate, 50.0, 31.0, 1.2, 3.0),
    (Source::Locker, 2.45, 46.5, std::f64::consts::FRAC_PI_2, 2.2),
    (Source::Locker, 8.6, 41.0, std::f64::consts::FRAC_PI_2, 3.0),
    (Source::Car, 33.0, 26.5, -0.3, 3.0),
    (Source::Car, 60.0, 27.5, 0.3, 3.0),
    (Source::Cage, -7.6, 45.5, -std::f64::consts::FRAC_PI_2, 3.0),
];

/// A kind of container's object name in `containers.glb`, shut; opened,
/// the same with `_Open` after it; its collision shape, with `_Hull`.
pub fn model_name(source: Source) -> &'static str {
    match source {
        Source::Crate => "CONTAINER_Crate",
        Source::Locker => "CONTAINER_Locker",
        Source::Car => "CONTAINER_Car",
        Source::Cage => "CONTAINER_Cage",
        Source::Corpse => "",
    }
}

fn surface(source: Source) -> Surface {
    if source == Source::Crate { Surface::Wood } else { Surface::Metal }
}

#[derive(Component, Clone, Debug)]
pub struct Container {
    pub source: Source,
    pub grid: Grid,
    pub searched: bool,
    pub locked: bool,
    /// Its meshes, shut and opened.
    looks: Option<(MeshId, MeshId)>,
    /// Its box in the world, for knowing it's the one looked at.
    lo: Vec3,
    hi: Vec3,
}

impl Container {
    /// The middle of its box.
    pub fn middle(&self) -> Vec3 {
        (self.lo + self.hi) * 0.5
    }
}

/// Where one was set down: what, how it's placed, its box in the world.
pub struct Placing {
    pub source: Source,
    pub model: Mat4,
    pub lo: Vec3,
    pub hi: Vec3,
}

/// Set each container down on whatever is under its spot (its lowest
/// corner on the floor, so none floats on a slope) and make it solid.
/// `shapes` holds each kind's collision hull, about its own origin.
pub fn set_down(solids: &mut Solids, shapes: &HashMap<Source, Vec<[Vec3; 3]>>) -> Vec<Placing> {
    let mut out = Vec::new();
    for (source, x, z, yaw, hint) in SPOTS {
        let Some(tris) = shapes.get(&source) else { continue };
        let (mut lo, mut hi) = (Vec3::splat(f64::INFINITY), Vec3::splat(f64::NEG_INFINITY));
        for p in tris.iter().flatten() {
            lo = lo.min(*p);
            hi = hi.max(*p);
        }
        let turn = Mat4::from_quat(Quat::from_rotation_y(yaw));
        // The floor under its middle and its four corners: it sits on the
        // lowest.
        let mut floor = f64::INFINITY;
        for (cx, cz) in [(0.0, 0.0), (lo.x, lo.z), (lo.x, hi.z), (hi.x, lo.z), (hi.x, hi.z)] {
            let at = turn.transform_point(Vec3::new(cx, 0.0, cz)) + Vec3::new(x, hint + 1.0, z);
            if let Some(hit) = solids.raycast(at, Vec3::new(0.0, -1.0, 0.0), 20.0) {
                floor = floor.min(hit.point.y);
            }
        }
        if !floor.is_finite() {
            continue;
        }
        let model = Mat4::from_translation(Vec3::new(x, floor, z)) * turn;
        let placed: Vec<[Vec3; 3]> = tris.iter().map(|t| t.map(|p| model.transform_point(p))).collect();
        let (mut wlo, mut whi) = (Vec3::splat(f64::INFINITY), Vec3::splat(f64::NEG_INFINITY));
        for p in placed.iter().flatten() {
            wlo = wlo.min(*p);
            whi = whi.max(*p);
        }
        solids.add_as(&placed, surface(source));
        out.push(Placing { source, model, lo: wlo, hi: whi });
    }
    out
}

/// The containers, placed, into the world: drawn with `meshes` (shut and
/// opened), empty until a run fills them.
pub fn spawn(world: &mut World, placings: Vec<Placing>, meshes: &HashMap<Source, (MeshId, MeshId)>) {
    for p in placings {
        let (w, h) = p.source.grid();
        let looks = meshes.get(&p.source).copied();
        let container = Container { source: p.source, grid: Grid::new(w, h), searched: false, locked: false, looks, lo: p.lo, hi: p.hi };
        let mut e = world.spawn(container);
        if let Some((shut, _)) = looks {
            e.insert((Placed(p.model), Model(shut), Look::default()));
        }
    }
}

/// `e` has been searched: left open (unlocked, if it was locked), and it
/// looks it.
pub fn open_up(world: &mut World, e: Entity) {
    let Some(mut c) = world.get_mut::<Container>(e) else { return };
    c.searched = true;
    c.locked = false;
    if let Some((_, open)) = c.looks
        && let Some(mut model) = world.get_mut::<Model>(e)
    {
        model.0 = open;
    }
}

/// A fresh run: every container shut and filled anew by `seed`'s luck,
/// the cage locked, its key in one locker or car.
pub fn fill(world: &mut World, seed: u32) {
    let mut dice = Dice(seed | 1);
    let mut holders = Vec::new();
    for (e, mut c, model) in world.query::<(Entity, &mut Container, Option<&mut Model>)>().iter_mut(world) {
        c.grid = tables::fill(c.source, &mut dice);
        c.searched = false;
        c.locked = c.source == Source::Cage;
        if let (Some((shut, _)), Some(mut model)) = (c.looks, model) {
            model.0 = shut;
        }
        if matches!(c.source, Source::Locker | Source::Car) {
            holders.push(e);
        }
    }
    if holders.is_empty() {
        return;
    }
    holders.sort();
    let e = holders[dice.next() as usize % holders.len()];
    if let Some(mut c) = world.get_mut::<Container>(e) {
        // The key goes in even if something has to make way for it.
        if c.grid.place(Stack::one(Kind::Key)).count > 0 {
            c.grid.items.pop();
            c.grid.place(Stack::one(Kind::Key));
        }
    }
}

/// The container the eye at `eye`, looking along `dir`, is on (the first
/// solid it meets is part of one, within reach).
pub fn in_view(world: &mut World, eye: Vec3, dir: Vec3) -> Option<Entity> {
    let hit = world.resource::<Solid>().0.raycast(eye, dir, REACH)?;
    let near = 0.08;
    let (p, near) = (hit.point.to_array(), [near; 3]);
    world.query::<(Entity, &Container)>().iter(world).find(|(_, c)| (0..3).all(|k| p[k] >= c.lo.to_array()[k] - near[k] && p[k] <= c.hi.to_array()[k] + near[k])).map(|(e, _)| e)
}

/// Each kind's collision hull, as the game loads it (for tests).
#[cfg(test)]
pub fn shapes() -> HashMap<Source, Vec<[Vec3; 3]>> {
    let path = format!("{}/assets/models/containers.glb", env!("CARGO_MANIFEST_DIR"));
    let g = lntrn_model::Gltf::load(&path).expect("containers");
    let mut out = HashMap::new();
    for source in [Source::Crate, Source::Locker, Source::Car, Source::Cage] {
        let hull = format!("{}_Hull", model_name(source));
        let node = g.nodes.iter().find(|n| n.name.as_deref() == Some(hull.as_str())).expect("its hull");
        let mut tris = Vec::new();
        for p in &g.meshes[node.mesh.unwrap()].primitives {
            let at = |k: u32| {
                let v = p.positions[k as usize];
                Vec3::new(f64::from(v[0]), f64::from(v[1]), f64::from(v[2]))
            };
            tris.extend(p.indices.chunks_exact(3).map(|t| [at(t[0]), at(t[1]), at(t[2])]));
        }
        out.insert(source, tris);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::capsule;

    #[test]
    fn each_stands_clear_on_its_floor_and_can_be_reached() {
        let before = crate::testing::bare_world();
        let mut solids = before.clone();
        let placed = set_down(&mut solids, &shapes());
        assert_eq!(placed.len(), SPOTS.len());
        let nav = crate::zombie::nav::NavGrid::build(&solids, capsule(false));
        for p in &placed {
            // Nothing already there pokes into it: a thin body stood
            // anywhere inside its box, clear of the floor (a car's body
            // rides this high on its wheels), fits in the old world.
            let (lo, hi) = (p.lo + Vec3::new(0.1, 0.3, 0.1), p.hi - Vec3::new(0.1, 0.0, 0.1));
            let thin = crate::collide::Capsule { radius: 0.05, height: (hi.y - lo.y).max(0.12) };
            for i in 0..=4 {
                for j in 0..=4 {
                    let at = Vec3::new(lo.x + (hi.x - lo.x) * f64::from(i) / 4.0, lo.y, lo.z + (hi.z - lo.z) * f64::from(j) / 4.0);
                    assert!(before.fits(thin, at), "{:?} at {:?} runs into something at {at:?}", p.source, p.model.transform_point(Vec3::ZERO));
                }
            }
            // Its front can be stood at, and from there it's the one seen.
            let front = p.model.transform_point(Vec3::new(0.0, 0.0, -1.0));
            let middle = (p.lo + p.hi) * 0.5;
            let stand = middle + (front - p.model.transform_point(Vec3::ZERO)) * (0.5 * (p.hi - p.lo).length().min(3.0) + 0.3);
            // (The floor under the container's own level: indoors the grid
            // knows only the roof.)
            let floor = solids.raycast(Vec3::new(stand.x, p.lo.y + 1.2, stand.z), Vec3::new(0.0, -1.0, 0.0), 3.0).map(|h| h.point.y);
            let floor = floor.unwrap_or_else(|| panic!("{:?}: nowhere to stand in front, at {stand:?}", p.source));
            let ok = nav.height_at(stand).is_some_and(|h| (h - floor).abs() < 0.3) || solids.raycast(Vec3::new(stand.x, floor + 0.1, stand.z), Vec3::new(0.0, 1.0, 0.0), 3.0).is_some();
            assert!(ok, "{:?}: in front of it, at {stand:?}, is nowhere to stand", p.source);
            let eye = Vec3::new(stand.x, floor + 1.6, stand.z);
            let mut world = World::new();
            world.insert_resource(Solid(solids.clone()));
            spawn(&mut world, placed.iter().map(|q| Placing { source: q.source, model: q.model, lo: q.lo, hi: q.hi }).collect(), &HashMap::new());
            let dir = (Vec3::new(middle.x, (middle.y).max(floor + 0.4), middle.z) - eye).normalize();
            let seen = in_view(&mut world, eye, dir).map(|e| world.get::<Container>(e).unwrap().source);
            assert_eq!(seen, Some(p.source), "looking at the {:?} from {eye:?}", p.source);
        }
    }

    #[test]
    fn each_has_an_opened_look_standing_where_it_stood() {
        let path = format!("{}/assets/models/containers.glb", env!("CARGO_MANIFEST_DIR"));
        let g = lntrn_model::Gltf::load(&path).expect("containers");
        let bounds = |name: &str| {
            let node = g.nodes.iter().find(|n| n.name.as_deref() == Some(name)).unwrap_or_else(|| panic!("no {name}"));
            let (mut lo, mut hi) = (Vec3::splat(f64::INFINITY), Vec3::splat(f64::NEG_INFINITY));
            for p in &g.meshes[node.mesh.unwrap()].primitives {
                for v in &p.positions {
                    let v = Vec3::new(f64::from(v[0]), f64::from(v[1]), f64::from(v[2]));
                    lo = lo.min(v);
                    hi = hi.max(v);
                }
            }
            (lo, hi)
        };
        for source in [Source::Crate, Source::Locker, Source::Car, Source::Cage] {
            let (shut, open) = (bounds(model_name(source)), bounds(&format!("{}_Open", model_name(source))));
            assert!(open.0.y > shut.0.y - 0.02, "{source:?} opened sinks into the floor: {:?} vs {:?}", open.0, shut.0);
            assert!(open != shut, "{source:?} opened looks just the same");
            // What swings open stays near: nothing flung far off.
            assert!((open.1 - shut.1).length() < 1.2 && (open.0 - shut.0).length() < 1.2, "{source:?}: {shut:?} → {open:?}");
        }
    }

    #[test]
    fn each_run_fills_them_afresh_locks_the_cage_and_hides_its_key() {
        let mut world = World::new();
        let mut solids = crate::testing::bare_world();
        let placed = set_down(&mut solids, &shapes());
        spawn(&mut world, placed, &HashMap::new());
        for seed in [1, 2, 3, 4, 5] {
            fill(&mut world, seed);
            let all: Vec<Container> = world.query::<&Container>().iter(&world).cloned().collect();
            assert!(all.iter().all(|c| !c.searched && !c.grid.items.is_empty()));
            assert!(all.iter().filter(|c| c.locked).all(|c| c.source == Source::Cage));
            let keys: Vec<Source> = all.iter().filter(|c| c.grid.count(Kind::Key) > 0).map(|c| c.source).collect();
            assert_eq!(keys.len(), 1, "one key, in {keys:?}");
            assert!(matches!(keys[0], Source::Locker | Source::Car));
        }
    }
}
