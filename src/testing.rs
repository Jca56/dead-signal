//! What more than one module's tests need: the game's own scene files,
//! loaded the way the game sorts them.

use lntrn_math::Vec3;

use crate::collide::Solids;

/// Everything solid in the game's own scene files, as the game sorts it,
/// with the containers set down in it as the game sets them.
pub fn real_world() -> Solids {
    let mut s = bare_world();
    crate::containers::set_down(&mut s, &crate::containers::shapes());
    s
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
