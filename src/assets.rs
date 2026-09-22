//! Models from Blender (glTF) into the renderer: every node with a mesh
//! becomes a [`Prop`] placed where the file put it. Triangles are split
//! apart so each face carries its own normal: low poly facets, never
//! smoothed over.

use std::path::{Path, PathBuf};

use lntrn_math::{Mat4, Vec3};
use lntrn_model::{Gltf, Mode};

use crate::render::{MeshId, Renderer, Vertex};

/// One object of a model file, ready to draw.
#[derive(Clone, Debug)]
pub struct Prop {
    pub name: String,
    pub mesh: MeshId,
    pub model: Mat4,
    /// Its triangles in world space, for standing on (the ground).
    pub triangles: Vec<[Vec3; 3]>,
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

fn props(renderer: &mut Renderer, gltf: &Gltf, path: &Path) -> Vec<Prop> {
    let world = gltf.world_matrices(&gltf.rest_pose());
    let mut out = Vec::new();
    for (i, node) in gltf.nodes.iter().enumerate() {
        let Some(mesh) = node.mesh.and_then(|m| gltf.meshes.get(m)) else { continue };
        let mut vertices = Vec::new();
        for p in mesh.primitives.iter().filter(|p| p.mode == Mode::Triangles) {
            let material = p.material.and_then(|m| gltf.materials.get(m));
            let base = material.map_or([1.0; 4], |m| m.base_color);
            let emissive = material.map_or([0.0; 3], |m| m.emissive);
            for tri in p.indices.chunks_exact(3) {
                let corner = |k: usize| tri[k] as usize;
                let pos = [0, 1, 2].map(|k| p.positions[corner(k)]);
                let v = pos.map(|a| Vec3::new(f64::from(a[0]), f64::from(a[1]), f64::from(a[2])));
                let n = (v[1] - v[0]).cross(v[2] - v[0]).normalize();
                if !n.x.is_finite() {
                    continue; // a sliver with no area
                }
                let normal = [n.x as f32, n.y as f32, n.z as f32];
                for k in 0..3 {
                    let c = p.colors.get(corner(k)).copied().unwrap_or([1.0; 4]);
                    vertices.push(Vertex { pos: pos[k], normal, color: [c[0] * base[0], c[1] * base[1], c[2] * base[2], c[3] * base[3]], emissive });
                }
            }
        }
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
        out.push(Prop { name, mesh: renderer.add_mesh(&vertices), model, triangles });
    }
    out
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
}
