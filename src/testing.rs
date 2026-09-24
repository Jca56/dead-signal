//! What more than one module's tests need: the old fixed course (the
//! title scene and the proving ground, with the containers, ways out and
//! pickups it had), loaded the way the game sorts it, for testing how
//! bodies, the dead and things set down get on over known ground; and a
//! generated map, built once, the way a run gets one.

use std::f64::consts::FRAC_PI_2;
use std::sync::OnceLock;

use lntrn_math::Vec3;

use crate::collide::Solids;
use crate::exits::Way;
use crate::loot::Kind;
use crate::loot::tables::Source;
use crate::map::build::{Built, Kit};

/// How far the old fixed course (the title scene and the proving ground)
/// reaches each way from its middle.
pub const COURSE_HALF: f64 = 115.0;

/// The course's containers: what, where (game x and z), which way, and a
/// height to look down from (the locker inside the building, under its
/// roof).
pub const COURSE_CONTAINERS: [(Source, crate::map::Spot); 8] = [
    (Source::Crate, (-7.0, 31.5, 0.3, 3.0)),
    (Source::Crate, (-1.0, 38.0, -0.4, 3.0)),
    (Source::Crate, (50.0, 31.0, 1.2, 3.0)),
    (Source::Locker, (2.45, 46.5, FRAC_PI_2, 2.2)),
    (Source::Locker, (8.6, 41.0, FRAC_PI_2, 3.0)),
    (Source::Car, (33.0, 26.5, -0.3, 3.0)),
    (Source::Car, (60.0, 27.5, 0.3, 3.0)),
    (Source::Cage, (-7.6, 45.5, -FRAC_PI_2, 3.0)),
];

/// The course's ways out: the radio at the tower's foot, the road
/// checkpoint south of the pad, the truck by the range.
pub const COURSE_EXITS: [crate::exits::Spot; 3] = [
    (Way::Radio, (7.6, -44.2, FRAC_PI_2, 3.0), Vec3::new(1.0, 0.0, -4.0), 11.0),
    (Way::Road, (-6.0, 61.0, -0.2, 6.0), Vec3::new(0.0, 0.0, 7.0), 3.5),
    (Way::Truck, (48.0, 35.0, 0.1, 3.0), Vec3::new(0.0, 0.0, 0.0), 0.0),
];

/// The course's pickups (one inside the building, on its floor).
pub const COURSE_PICKUPS: [crate::items::Spot; 14] = [
    (Kind::Bandage, (1, 1), 4.0, 3.0, 3.0, 0.4),
    (Kind::Bandage, (1, 1), 1.0, 2.2, 37.0, 1.1),
    (Kind::Bandage, (1, 1), -1.5, 5.2, 46.0, 2.3),
    (Kind::Bandage, (1, 1), 12.0, 3.0, -46.0, 0.2),
    (Kind::Medkit, (1, 1), 0.0, 2.2, 45.5, 0.6),
    (Kind::Medkit, (1, 1), 2.5, 5.0, -36.5, 2.0),
    (Kind::Rounds, crate::items::ROUNDS, -5.0, 3.0, 12.0, 0.3),
    (Kind::Rounds, crate::items::ROUNDS, 18.0, 3.0, 2.0, 1.9),
    (Kind::Rounds, crate::items::ROUNDS, -20.0, 3.0, -8.0, 0.8),
    (Kind::Rounds, crate::items::ROUNDS, 8.0, 3.0, -28.0, 2.6),
    (Kind::Rounds, crate::items::ROUNDS, -16.0, 3.0, -44.0, 1.2),
    (Kind::Rounds, crate::items::ROUNDS, 28.0, 3.0, -24.0, 0.1),
    (Kind::Rounds, crate::items::ROUNDS, -1.9, 2.2, 47.0, 1.6),
    (Kind::Rounds, crate::items::ROUNDS, 38.0, 3.0, 36.0, 2.2),
];

/// Everything solid on the course, as the game sorts it, with its
/// containers set down as the game sets them.
pub fn real_world() -> Solids {
    let mut s = bare_world();
    crate::containers::set_down(&mut s, &crate::containers::shapes(), &COURSE_CONTAINERS);
    s
}

/// What maps are built from, from the game's own files.
pub fn kit() -> Kit {
    let mut kit = Kit { containers: crate::containers::shapes(), exits: crate::exits::tests::shapes(), ..Kit::default() };
    for file in ["scenery", "furniture", "sites"] {
        let path = format!("{}/assets/models/{file}.glb", env!("CARGO_MANIFEST_DIR"));
        let g = lntrn_model::Gltf::load(&path).expect("a model file");
        for what in crate::map::scatter::Scenery::all() {
            let name = format!("{}_Hull", what.name());
            let Some(node) = g.nodes.iter().find(|n| n.name.as_deref() == Some(name.as_str())) else { continue };
            let mut tris = Vec::new();
            for p in &g.meshes[node.mesh.expect("a mesh")].primitives {
                let at = |k: u32| {
                    let v = p.positions[k as usize];
                    Vec3::new(f64::from(v[0]), f64::from(v[1]), f64::from(v[2]))
                };
                tris.extend(p.indices.chunks_exact(3).map(|t| [at(t[0]), at(t[1]), at(t[2])]));
            }
            kit.scenery.insert(what, tris);
        }
    }
    kit
}

/// The seed the tests' map is made from.
pub const MAP_SEED: u32 = 20_260_923;

/// A map built as a run gets one, once for all the tests that want it.
pub fn built_map() -> &'static Built {
    static BUILT: OnceLock<Built> = OnceLock::new();
    BUILT.get_or_init(|| {
        let started = std::time::Instant::now();
        let built = crate::map::build::build(MAP_SEED, &kit(), &|_| {});
        eprintln!("map: built in {:.0} ms", started.elapsed().as_secs_f64() * 1000.0);
        built
    })
}

/// The scene files' solids alone, before any container is set down.
pub fn bare_world() -> Solids {
    let mut s = Solids::new();
    for file in ["title_scene", "proving_ground"] {
        let path = format!("{}/assets/models/{file}.glb", env!("CARGO_MANIFEST_DIR"));
        let g = lntrn_model::Gltf::load(&path).expect("scene file");
        let world = g.world_matrices(&g.rest_pose());
        for (i, node) in g.nodes.iter().enumerate() {
            let name = node.name.as_deref().unwrap_or("");
            let solid = name == "Ground" || name.starts_with("SOLID_") || name.starts_with("COL_");
            let Some(mesh) = node.mesh.filter(|_| solid) else { continue };
            for p in &g.meshes[mesh].primitives {
                let at = |k: u32| {
                    let v = p.positions[k as usize];
                    world[i].transform_point(Vec3::new(f64::from(v[0]), f64::from(v[1]), f64::from(v[2])))
                };
                let tris: Vec<[Vec3; 3]> = p.indices.chunks_exact(3).map(|t| [at(t[0]), at(t[1]), at(t[2])]).collect();
                s.add(&tris);
            }
        }
    }
    s
}

/// Every target in the proving ground: its name and where it stands.
pub fn real_targets() -> Vec<(String, lntrn_math::Mat4)> {
    let path = format!("{}/assets/models/proving_ground.glb", env!("CARGO_MANIFEST_DIR"));
    let g = lntrn_model::Gltf::load(&path).expect("scene file");
    let world = g.world_matrices(&g.rest_pose());
    g.nodes.iter().enumerate().filter_map(|(i, n)| n.name.as_deref().filter(|name| name.starts_with("TARGET_")).map(|name| (name.to_owned(), world[i]))).collect()
}
