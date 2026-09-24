//! Models from Blender (glTF) into the renderer: every node with a mesh
//! becomes a [`Prop`] placed where the file put it. Triangles are split
//! apart so each face carries its own normal: low poly facets, never
//! smoothed over.

use std::path::{Path, PathBuf};

use lntrn_math::{Mat4, Vec3};
use lntrn_model::{Gltf, Mode};

use crate::render::{FigureMeshId, MAX_JOINTS, MeshId, Renderer, SkinnedMeshId, SkinnedVertex, Vertex};

/// One object of a model file, ready to draw.
#[derive(Clone, Debug)]
pub struct Prop {
    pub name: String,
    /// What is drawn; `None` for a collision shape (named `COL_*`).
    pub mesh: Option<MeshId>,
    pub model: Mat4,
    /// Its triangles in world space, for standing on (the ground).
    pub triangles: Vec<[Vec3; 3]>,
    /// Its vertices as made, about its own origin, three to a face (for
    /// drawing it small: an icon).
    pub vertices: Vec<Vertex>,
}

/// Where the game's files are: beside the executable when installed,
/// else the project's own folder.
pub fn root() -> PathBuf {
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
    std::env::current_exe().ok().and_then(|exe| exe.parent().map(|d| d.join("assets"))).filter(|p| p.is_dir()).unwrap_or(here)
}

/// Load `models/<name>.glb` and hand its meshes to `renderer`.
pub fn load(renderer: &mut Renderer, name: &str) -> Result<Vec<Prop>, String> {
    let path = root().join("models").join(format!("{name}.glb"));
    let gltf = Gltf::load(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(props(renderer, &gltf, &path))
}

/// A mesh's triangles split apart, each face its own normal and each
/// corner its colour (slivers with no area left out).
fn faceted(gltf: &Gltf, mesh: &lntrn_model::Mesh) -> Vec<Vertex> {
    let mut vertices = Vec::new();
    for p in mesh.primitives.iter().filter(|p| p.mode == Mode::Triangles) {
        let material = p.material.and_then(|m| gltf.materials.get(m));
        let base = material.map_or([1.0; 4], |m| m.base_color);
        let emissive = material.map_or([0.0; 3], |m| m.emissive);
        for tri in p.indices.chunks_exact(3) {
            let corner = |k: usize| tri[k] as usize;
            let pos = [0, 1, 2].map(|k| p.positions[corner(k)]);
            let v = pos.map(|a| Vec3::new(f64::from(a[0]), f64::from(a[1]), f64::from(a[2])));
            let n = (v[1] - v[0]).cross(v[2] - v[0]);
            if n.length() < 1e-12 {
                continue; // a sliver with no area
            }
            let n = n.normalize();
            let normal = [n.x as f32, n.y as f32, n.z as f32];
            for (k, &at) in pos.iter().enumerate() {
                let c = p.colors.get(corner(k)).copied().unwrap_or([1.0; 4]);
                vertices.push(Vertex { pos: at, normal, color: [c[0] * base[0], c[1] * base[1], c[2] * base[2], c[3] * base[3]], emissive });
            }
        }
    }
    vertices
}

fn props(renderer: &mut Renderer, gltf: &Gltf, path: &Path) -> Vec<Prop> {
    let world = gltf.world_matrices(&gltf.rest_pose());
    let mut out = Vec::new();
    for (i, node) in gltf.nodes.iter().enumerate() {
        let Some(mesh) = node.mesh.and_then(|m| gltf.meshes.get(m)) else { continue };
        let vertices = faceted(gltf, mesh);
        if vertices.is_empty() {
            continue;
        }
        let model = world[i];
        let triangles = vertices
            .chunks_exact(3)
            .map(|t| t.iter().map(|v| model.transform_point(Vec3::new(f64::from(v.pos[0]), f64::from(v.pos[1]), f64::from(v.pos[2])))).collect::<Vec<_>>())
            .map(|t| [t[0], t[1], t[2]])
            .collect();
        let name = node.name.clone().unwrap_or_else(|| format!("{}#{i}", path.display()));
        let mesh = (!name.starts_with("COL_")).then(|| renderer.add_mesh(&vertices));
        out.push(Prop { name, mesh, model, triangles, vertices });
    }
    out
}

/// A skinned model: its mesh on the GPU, and the file itself for its
/// bones and animations.
pub struct Rigged<M> {
    pub mesh: M,
    pub gltf: Gltf,
    pub skin: usize,
}

/// A viewmodel (the arms and what they hold): drawn in the viewmodel pass.
pub fn load_viewmodel(renderer: &mut Renderer, name: &str) -> Result<Rigged<SkinnedMeshId>, String> {
    let (vertices, gltf, skin) = load_skinned(name)?;
    Ok(Rigged { mesh: renderer.add_skinned_mesh(&vertices), gltf, skin })
}

/// A figure out in the world (the dead) made of parts: every skinned mesh
/// in the file, by its node's name, all on the one skin.
pub fn load_figure(renderer: &mut Renderer, name: &str) -> Result<Rigged<Vec<(String, FigureMeshId)>>, String> {
    let path = root().join("models").join(format!("{name}.glb"));
    let gltf = Gltf::load(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let skin = skin_of(&gltf, &path)?;
    let mut parts = Vec::new();
    for node in gltf.nodes.iter().filter(|n| n.skin.is_some()) {
        let (Some(mesh), Some(part)) = (node.mesh, node.name.clone()) else { continue };
        if node.skin != Some(skin) {
            return Err(format!("{}: {part} is on a skin of its own", path.display()));
        }
        parts.push((part, renderer.add_figure_mesh(&skinned_vertices(&gltf, mesh))));
    }
    Ok(Rigged { mesh: parts, gltf, skin })
}

/// Read `models/<name>.glb`, whose first skinned mesh is the model: its
/// triangles split apart (a normal a face), the file, and which skin.
fn load_skinned(name: &str) -> Result<(Vec<SkinnedVertex>, Gltf, usize), String> {
    let path = root().join("models").join(format!("{name}.glb"));
    let gltf = Gltf::load(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let skin = skin_of(&gltf, &path)?;
    let mesh = gltf.nodes.iter().find(|n| n.skin == Some(skin) && n.mesh.is_some()).and_then(|n| n.mesh).unwrap_or(0);
    Ok((skinned_vertices(&gltf, mesh), gltf, skin))
}

/// The skin of the file's first skinned mesh, if it has few enough bones.
fn skin_of(gltf: &Gltf, path: &Path) -> Result<usize, String> {
    let skin = gltf.nodes.iter().find(|n| n.skin.is_some() && n.mesh.is_some()).and_then(|n| n.skin).ok_or_else(|| format!("{}: no skinned mesh", path.display()))?;
    if gltf.skins[skin].joints.len() > MAX_JOINTS {
        return Err(format!("{}: {} bones, at most {MAX_JOINTS}", path.display(), gltf.skins[skin].joints.len()));
    }
    Ok(skin)
}

/// A skinned mesh's triangles, split apart (a normal a face).
fn skinned_vertices(gltf: &Gltf, mesh: usize) -> Vec<SkinnedVertex> {
    let mut vertices = Vec::new();
    for p in gltf.meshes[mesh].primitives.iter().filter(|p| p.mode == Mode::Triangles) {
        let base = p.material.and_then(|m| gltf.materials.get(m)).map_or([1.0; 4], |m| m.base_color);
        for tri in p.indices.chunks_exact(3) {
            let corner = |k: usize| tri[k] as usize;
            let pos = [0, 1, 2].map(|k| p.positions[corner(k)]);
            let v = pos.map(|a| Vec3::new(f64::from(a[0]), f64::from(a[1]), f64::from(a[2])));
            let n = (v[1] - v[0]).cross(v[2] - v[0]);
            if n.length() < 1e-12 {
                continue;
            }
            let n = n.normalize();
            for (k, &at) in pos.iter().enumerate() {
                let c = p.colors.get(corner(k)).copied().unwrap_or([1.0; 4]);
                let joints = p.joints.get(corner(k)).map_or([0; 4], |j| j.map(u32::from));
                let weights = p.weights.get(corner(k)).copied().unwrap_or([1.0, 0.0, 0.0, 0.0]);
                vertices.push(SkinnedVertex { pos: at, normal: [n.x as f32, n.y as f32, n.z as f32], color: [c[0] * base[0], c[1] * base[1], c[2] * base[2], c[3] * base[3]], joints, weights });
            }
        }
    }
    vertices
}

/// The height of the highest triangle under `(x, z)`, if any.
pub fn height_at(triangles: &[[Vec3; 3]], x: f64, z: f64) -> Option<f64> {
    let mut best: Option<f64> = None;
    for [a, b, c] in triangles {
        // Barycentric coordinates in the XZ plane.
        let d = (b.z - c.z) * (a.x - c.x) + (c.x - b.x) * (a.z - c.z);
        if d.abs() < 1e-12 {
            continue;
        }
        let u = ((b.z - c.z) * (x - c.x) + (c.x - b.x) * (z - c.z)) / d;
        let v = ((c.z - a.z) * (x - c.x) + (a.x - c.x) * (z - c.z)) / d;
        let w = 1.0 - u - v;
        if u < 0.0 || v < 0.0 || w < 0.0 {
            continue;
        }
        let y = u * a.y + v * b.y + w * c.y;
        best = Some(best.map_or(y, |h| h.max(y)));
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn height_under_a_slope() {
        let tri = [[Vec3::new(0.0, 0.0, 0.0), Vec3::new(10.0, 10.0, 0.0), Vec3::new(0.0, 0.0, 10.0)]];
        assert!((height_at(&tri, 5.0, 2.0).unwrap() - 5.0).abs() < 1e-9);
        assert_eq!(height_at(&tri, -1.0, 2.0), None);
    }

    /// Every model file the game loads goes through as the game takes it
    /// (a sliver with no area in one must not stop the game).
    #[test]
    fn every_model_file_loads() {
        let dir = root().join("models");
        for name in ["title_scene", "proving_ground", "items", "containers", "exits", "sites", "furniture", "scenery"] {
            let gltf = Gltf::load(dir.join(format!("{name}.glb"))).unwrap_or_else(|e| panic!("{name}: {e}"));
            let made: usize = gltf.meshes.iter().map(|m| faceted(&gltf, m).len()).sum();
            assert!(made > 0, "{name} made nothing");
        }
    }
}
