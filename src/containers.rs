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
use crate::world::{Bounds, Look, Model, OnMap, Placed, Solid};

/// How far off a container can be searched, from the eye to where it's
/// looked at.
pub const REACH: f64 = 2.4;

/// A kind of container's object name in `containers.glb`, shut; opened,
/// the same with `_Open` after it; its collision shape, with `_Hull`.
pub fn model_name(source: Source) -> &'static str {
    match source {
        Source::Crate => "CONTAINER_Crate",
        Source::Locker | Source::ToolLocker => "CONTAINER_Locker",
        Source::Car => "CONTAINER_Car",
        Source::Cage | Source::AmmoCage => "CONTAINER_Cage",
        Source::Fridge => "CONTAINER_Fridge",
        Source::Cabinet => "CONTAINER_Cabinet",
        Source::Desk => "CONTAINER_Desk",
        Source::Wardrobe => "CONTAINER_Wardrobe",
        Source::Shelf => "CONTAINER_Shelf",
        Source::Register => "CONTAINER_Register",
        Source::GunCabinet | Source::HunterCabinet => "CONTAINER_GunCabinet",
        Source::SupplyCase => "CONTAINER_SupplyCase",
        Source::Corpse | Source::Soldier | Source::Juggernaut => "",
    }
}

fn surface(source: Source) -> Surface {
    match source {
        Source::Crate | Source::Cabinet | Source::Desk | Source::Wardrobe | Source::Register | Source::GunCabinet | Source::HunterCabinet => Surface::Wood,
        _ => Surface::Metal,
    }
}

/// Whether the cage's key can turn up in it.
pub fn holds_keys(source: Source) -> bool {
    matches!(source, Source::Locker | Source::Car | Source::Desk | Source::Wardrobe)
}

/// The key that opens it, if it's locked.
pub fn key_for(source: Source) -> Option<Kind> {
    match source {
        Source::Cage => Some(Kind::Key),
        Source::AmmoCage => Some(Kind::ArmoryKey),
        _ => None,
    }
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

/// Set each container down on whatever is under its spot (what, where in
/// game x and z, which way its front faces as a yaw (0 faces -Z), and a
/// height to look down for its floor from) and make it solid. `shapes`
/// holds each kind's collision hull, about its own origin.
pub fn set_down(solids: &mut Solids, shapes: &HashMap<Source, Vec<[Vec3; 3]>>, spots: &[(Source, crate::map::Spot)]) -> Vec<Placing> {
    let mut out = Vec::new();
    for &(source, spot) in spots {
        let Some(tris) = shapes.get(&source) else { continue };
        let Some(seat) = seat(solids, tris, spot, false) else { continue };
        solids.add_as(&seat.tris, surface(source));
        out.push(Placing { source, model: seat.model, lo: seat.lo, hi: seat.hi });
    }
    out
}

/// Where something set down came to be: how it's placed, its triangles in
/// the world, and their box.
pub struct Seat {
    pub model: Mat4,
    pub tris: Vec<[Vec3; 3]>,
    pub lo: Vec3,
    pub hi: Vec3,
}

/// Set `tris` (about their own origin) down at `(x, z)` turned `yaw`, on
/// whatever is under it looking down from `hint` + 1 m: the lowest of the
/// floor under its middle and its four corners, so none floats on a slope
/// (or the highest, for a thing with a foundation to sink).
pub fn seat(solids: &Solids, tris: &[[Vec3; 3]], (x, z, yaw, hint): (f64, f64, f64, f64), highest: bool) -> Option<Seat> {
    let (mut lo, mut hi) = (Vec3::splat(f64::INFINITY), Vec3::splat(f64::NEG_INFINITY));
    for p in tris.iter().flatten() {
        lo = lo.min(*p);
        hi = hi.max(*p);
    }
    let turn = Mat4::from_quat(Quat::from_rotation_y(yaw));
    let mut floor: Option<f64> = None;
    for (cx, cz) in [(0.0, 0.0), (lo.x, lo.z), (lo.x, hi.z), (hi.x, lo.z), (hi.x, hi.z)] {
        let at = turn.transform_point(Vec3::new(cx, 0.0, cz)) + Vec3::new(x, hint + 1.0, z);
        if let Some(hit) = solids.raycast(at, Vec3::new(0.0, -1.0, 0.0), 20.0) {
            let y = hit.point.y;
            floor = Some(floor.map_or(y, |f| if highest { f.max(y) } else { f.min(y) }));
        }
    }
    let floor = floor?;
    let model = Mat4::from_translation(Vec3::new(x, floor, z)) * turn;
    let placed: Vec<[Vec3; 3]> = tris.iter().map(|t| t.map(|p| model.transform_point(p))).collect();
    let (mut wlo, mut whi) = (Vec3::splat(f64::INFINITY), Vec3::splat(f64::NEG_INFINITY));
    for p in placed.iter().flatten() {
        wlo = wlo.min(*p);
        whi = whi.max(*p);
    }
    Some(Seat { model, tris: placed, lo: wlo, hi: whi })
}

/// Whether `p` is within `near` of the box `lo`–`hi`.
pub fn in_box(p: Vec3, lo: Vec3, hi: Vec3, near: f64) -> bool {
    p.x >= lo.x - near && p.x <= hi.x + near && p.y >= lo.y - near && p.y <= hi.y + near && p.z >= lo.z - near && p.z <= hi.z + near
}

/// The containers, placed, into the world (part of its map): drawn with
/// `meshes` (shut and opened), empty until a run fills them.
pub fn spawn(world: &mut World, placings: Vec<Placing>, meshes: &HashMap<Source, (MeshId, MeshId)>) {
    for p in placings {
        let (w, h) = p.source.grid();
        let looks = meshes.get(&p.source).copied();
        let container = Container { source: p.source, grid: Grid::new(w, h), searched: false, locked: false, looks, lo: p.lo, hi: p.hi };
        let mut e = world.spawn((container, OnMap));
        if let Some((shut, _)) = looks {
            e.insert((Placed(p.model), Model(shut), Look::default(), Bounds { centre: (p.lo + p.hi) * 0.5, radius: (p.hi - p.lo).length() * 0.5 }));
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
/// the cage locked, its key in one locker, car, desk or wardrobe.
pub fn fill(world: &mut World, seed: u32) {
    let mut dice = Dice(seed | 1);
    let (mut holders, mut cases) = (Vec::new(), Vec::new());
    for (e, mut c, model) in world.query::<(Entity, &mut Container, Option<&mut Model>)>().iter_mut(world) {
        c.grid = tables::fill(c.source, &mut dice);
        c.searched = false;
        c.locked = key_for(c.source).is_some();
        if let (Some((shut, _)), Some(mut model)) = (c.looks, model) {
            model.0 = shut;
        }
        if holds_keys(c.source) {
            holders.push(e);
        }
        if c.source == Source::SupplyCase {
            cases.push(e);
        }
    }
    // The cage's key in one of the places it can be, the armory's in one of
    // the wreck's cases (the dead soldiers carry it too, now and then).
    for (mut holders, key) in [(holders, Kind::Key), (cases, Kind::ArmoryKey)] {
        if holders.is_empty() {
            continue;
        }
        holders.sort();
        let e = holders[dice.next() as usize % holders.len()];
        if let Some(mut c) = world.get_mut::<Container>(e) {
            // The key goes in even if something has to make way for it.
            if c.grid.place(Stack::one(key)).count > 0 {
                c.grid.items.pop();
                c.grid.place(Stack::one(key));
            }
        }
    }
}

/// The container the eye at `eye`, looking along `dir`, is on (the first
/// solid it meets is part of one, within reach).
pub fn in_view(world: &mut World, eye: Vec3, dir: Vec3) -> Option<Entity> {
    let hit = world.resource::<Solid>().0.raycast(eye, dir, REACH)?;
    world.query::<(Entity, &Container)>().iter(world).find(|(_, c)| in_box(hit.point, c.lo, c.hi, 0.08)).map(|(e, _)| e)
}

/// Each kind's collision hull, as the game loads it (for tests).
#[cfg(test)]
pub fn shapes() -> HashMap<Source, Vec<[Vec3; 3]>> {
    let path = format!("{}/assets/models/containers.glb", env!("CARGO_MANIFEST_DIR"));
    let g = lntrn_model::Gltf::load(&path).expect("containers");
    let mut out = HashMap::new();
    for source in crate::loot::tables::CONTAINERS {
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
        let placed = set_down(&mut solids, &shapes(), &crate::testing::COURSE_CONTAINERS);
        assert_eq!(placed.len(), crate::testing::COURSE_CONTAINERS.len());
        let nav = crate::zombie::nav::NavGrid::build(&solids, capsule(false), crate::testing::COURSE_HALF);
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
        let vertices = |name: &str| g.nodes.iter().find(|n| n.name.as_deref() == Some(name)).map_or(0, |n| g.meshes[n.mesh.unwrap()].primitives.iter().map(|p| p.positions.len()).sum::<usize>());
        for source in crate::loot::tables::CONTAINERS {
            let (shut, open) = (bounds(model_name(source)), bounds(&format!("{}_Open", model_name(source))));
            assert!(open.0.y > shut.0.y - 0.02, "{source:?} opened sinks into the floor: {:?} vs {:?}", open.0, shut.0);
            // (A shelf searched looks it by what's gone from it.)
            let other = open != shut || vertices(model_name(source)) != vertices(&format!("{}_Open", model_name(source)));
            assert!(other, "{source:?} opened looks just the same");
            // What swings open stays near: nothing flung far off.
            assert!((open.1 - shut.1).length() < 1.2 && (open.0 - shut.0).length() < 1.2, "{source:?}: {shut:?} → {open:?}");
        }
    }

    #[test]
    fn each_run_fills_them_afresh_locks_the_cage_and_hides_its_key() {
        let mut world = World::new();
        let mut solids = crate::testing::bare_world();
        let placed = set_down(&mut solids, &shapes(), &crate::testing::COURSE_CONTAINERS);
        spawn(&mut world, placed, &HashMap::new());
        for seed in [1, 2, 3, 4, 5] {
            fill(&mut world, seed);
            let all: Vec<Container> = world.query::<&Container>().iter(&world).cloned().collect();
            assert!(all.iter().all(|c| !c.searched && !c.grid.items.is_empty()));
            assert!(all.iter().filter(|c| c.locked).all(|c| c.source == Source::Cage));
            let keys: Vec<Source> = all.iter().filter(|c| c.grid.count(Kind::Key) > 0).map(|c| c.source).collect();
            assert_eq!(keys.len(), 1, "one key, in {keys:?}");
            assert!(holds_keys(keys[0]));
        }
    }
}
