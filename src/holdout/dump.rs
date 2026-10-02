//! The arena written out to be looked at away from the game (rendered in
//! Blender, `tools/relay_views.py`): everything built of it as a coloured
//! mesh, and where each model stands. Run by hand:
//!
//! ```text
//! RELAY_DUMP=/some/dir cargo test --release holdout::dump -- --ignored
//! ```

use std::fmt::Write as _;

use lntrn_math::Vec3;

use crate::map::build::build_holdout;

/// How far out from the middle the ground's kept (x, z).
const REACH: (f32, f32) = (105.0, 62.0);

/// Light as it looks.
fn srgb(c: f32) -> u8 {
    let s = if c <= 0.003_130_8 { c * 12.92 } else { 1.055 * c.powf(1.0 / 2.4) - 0.055 };
    (s.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[test]
#[ignore]
fn dump() {
    let Ok(dir) = std::env::var("RELAY_DUMP") else { return };
    let built = build_holdout(&crate::testing::kit(), &|_| {});
    let arena = built.arena.as_ref().expect("the arena");
    // Blender's frame: x, -z, y.
    let mut verts = String::new();
    let mut n = 0;
    let mut tri = |p: [[f32; 3]; 3], c: [u8; 3]| {
        for v in p {
            let _ = writeln!(verts, "{} {} {} {} {} {}", v[0], -v[2], v[1], c[0], c[1], c[2]);
        }
        n += 1;
    };
    for chunk in &built.chunks {
        for t in chunk.vertices.chunks_exact(3) {
            if t.iter().all(|v| v.pos[0].abs() < REACH.0 && v.pos[2].abs() < REACH.1) {
                tri([t[0].pos, t[1].pos, t[2].pos], [srgb(t[0].color[0]), srgb(t[0].color[1]), srgb(t[0].color[2])]);
            }
        }
    }
    // The doors to buy, as boxes: gold, a heap brown.
    for d in &arena.doors {
        let c = if d.heap { [150, 100, 60] } else { [240, 180, 30] };
        for t in crate::collide::box_tris(d.lo, d.hi) {
            tri(t.map(|p: Vec3| [p.x as f32, p.y as f32, p.z as f32]), c);
        }
    }
    let mut ply = format!("ply\nformat ascii 1.0\nelement vertex {}\nproperty float x\nproperty float y\nproperty float z\nproperty uchar red\nproperty uchar green\nproperty uchar blue\nelement face {n}\nproperty list uchar int vertex_indices\nend_header\n", n * 3);
    ply.push_str(&verts);
    for k in 0..n {
        let _ = writeln!(ply, "3 {} {} {}", k * 3, k * 3 + 1, k * 3 + 2);
    }
    std::fs::write(format!("{dir}/world.ply"), ply).expect("the mesh written");
    // Each model: its name, where, turned how far.
    let mut list = String::new();
    for p in built.map.scenery.iter().filter(|p| (p.at.x.abs() as f32) < REACH.0 && (p.at.z.abs() as f32) < REACH.1) {
        let _ = writeln!(list, "{} {} {} {} {}", p.what.name(), p.at.x, p.at.y, p.at.z, p.yaw);
    }
    for c in &built.containers {
        let at = c.model.transform_point(Vec3::ZERO);
        let front = c.model.transform_vector(Vec3::new(0.0, 0.0, -1.0));
        let _ = writeln!(list, "{} {} {} {} {}", crate::containers::model_name(c.source), at.x, at.y, at.z, (-front.x).atan2(-front.z));
    }
    // What's on the walls, and the dead's ways in: marks to look for.
    for b in &arena.buys {
        let _ = writeln!(list, "MARK_Buy {} {} {} 0", b.at.x, b.at.y, b.at.z);
    }
    for w in &arena.windows {
        let _ = writeln!(list, "MARK_Window {} {} {} 0", w.centre.x, w.centre.y, w.centre.z);
        let _ = writeln!(list, "MARK_From {} {} {} 0", w.from.x, w.from.y, w.from.z);
    }
    std::fs::write(format!("{dir}/pieces.txt"), list).expect("the list written");
    eprintln!("dumped {n} triangles to {dir}");
}
