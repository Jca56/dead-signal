//! Pictures of things for the inventory, drawn from their own models: each
//! seen from the front, a little above and to one side, lit flat the way
//! the world is, on nothing (clear). Drawn once at start, on the CPU (a
//! handful of low poly things), a few times over size and shrunk, so the
//! edges come out smooth.

use lntrn_image::Image;
use lntrn_math::Vec3;

use crate::render::Vertex;

/// Pixels a cell's side, and how many times over it's drawn.
pub const CELL_PX: u32 = 96;
const OVER: u32 = 3;
/// How far in from the picture's edges the thing is kept, as a share.
const MARGIN: f64 = 0.1;
/// Which way it's seen: turned this far round, looked down on this much.
const YAW: f64 = -0.35;
const PITCH: f64 = 0.55;
/// Where the light comes from (the model's own space: +Y up, its front
/// towards -Z), and how much there is everywhere.
const LIGHT: Vec3 = Vec3::new(-0.45, 0.8, -0.55);
const AMBIENT: f64 = 0.45;

/// A picture of the model in `vertices` (three to a face), `cells` across
/// and down at [`CELL_PX`] a cell, as large as fits.
pub fn draw(vertices: &[Vertex], cells: (u8, u8)) -> Image {
    let (w, h) = (u32::from(cells.0) * CELL_PX, u32::from(cells.1) * CELL_PX);
    let (bw, bh) = (w * OVER, h * OVER);
    let (sy, cy) = YAW.sin_cos();
    let (sp, cp) = PITCH.sin_cos();
    // Into the view: turned about Y, then tipped about X; x right, y up,
    // z away.
    let view = |p: Vec3| {
        let x = p.x * cy + p.z * sy;
        let z = -p.x * sy + p.z * cy;
        Vec3::new(x, p.y * cp + z * sp, -p.y * sp + z * cp)
    };
    let tris: Vec<([Vec3; 3], [f64; 3])> = vertices
        .chunks_exact(3)
        .map(|t| {
            let at = [0, 1, 2].map(|k| view(Vec3::new(f64::from(t[k].pos[0]), f64::from(t[k].pos[1]), f64::from(t[k].pos[2]))));
            let n = Vec3::new(f64::from(t[0].normal[0]), f64::from(t[0].normal[1]), f64::from(t[0].normal[2]));
            let lit = AMBIENT + (1.0 - AMBIENT) * n.dot(LIGHT.normalize()).max(0.0);
            let c = t[0].color;
            (at, [f64::from(c[0]) * lit, f64::from(c[1]) * lit, f64::from(c[2]) * lit])
        })
        .collect();
    // Fit what's there into the picture, keeping its shape.
    let (mut lo, mut hi) = (Vec3::splat(f64::INFINITY), Vec3::splat(f64::NEG_INFINITY));
    for (at, _) in &tris {
        for p in at {
            lo = lo.min(*p);
            hi = hi.max(*p);
        }
    }
    let span = (hi.x - lo.x).max(1e-6) / (f64::from(bw) * (1.0 - 2.0 * MARGIN));
    let span = span.max((hi.y - lo.y).max(1e-6) / (f64::from(bh) * (1.0 - 2.0 * MARGIN)));
    let mid = (lo + hi) * 0.5;
    let to_px = |p: Vec3| Vec3::new(f64::from(bw) * 0.5 + (p.x - mid.x) / span, f64::from(bh) * 0.5 - (p.y - mid.y) / span, p.z);

    let mut colour = vec![[0.0f64; 4]; (bw * bh) as usize];
    let mut depth = vec![f64::INFINITY; (bw * bh) as usize];
    for (at, rgb) in &tris {
        let [a, b, c] = at.map(to_px);
        let area = edge(a, b, c);
        if area.abs() < 1e-9 {
            continue;
        }
        let x0 = a.x.min(b.x).min(c.x).floor().max(0.0) as u32;
        let x1 = (a.x.max(b.x).max(c.x).ceil() as u32).min(bw);
        let y0 = a.y.min(b.y).min(c.y).floor().max(0.0) as u32;
        let y1 = (a.y.max(b.y).max(c.y).ceil() as u32).min(bh);
        for y in y0..y1 {
            for x in x0..x1 {
                let p = Vec3::new(f64::from(x) + 0.5, f64::from(y) + 0.5, 0.0);
                let (wa, wb, wc) = (edge(b, c, p) / area, edge(c, a, p) / area, edge(a, b, p) / area);
                if wa < 0.0 || wb < 0.0 || wc < 0.0 {
                    continue;
                }
                let z = wa * a.z + wb * b.z + wc * c.z;
                let i = (y * bw + x) as usize;
                if z < depth[i] {
                    depth[i] = z;
                    colour[i] = [rgb[0], rgb[1], rgb[2], 1.0];
                }
            }
        }
    }
    // Shrink: each pixel the average of its block (colour weighted by
    // cover, so edges fade rather than darken).
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let mut sum = [0.0f64; 4];
            for dy in 0..OVER {
                for dx in 0..OVER {
                    let c = colour[((y * OVER + dy) * bw + x * OVER + dx) as usize];
                    for k in 0..3 {
                        sum[k] += c[k] * c[3];
                    }
                    sum[3] += c[3];
                }
            }
            let cover = sum[3] / f64::from(OVER * OVER);
            let rgb = if sum[3] > 0.0 { [sum[0] / sum[3], sum[1] / sum[3], sum[2] / sum[3]] } else { [0.0; 3] };
            rgba.extend(rgb.map(srgb));
            rgba.push((cover * 255.0).round() as u8);
        }
    }
    Image::new(w, h, rgba)
}

/// `image` a quarter turn clockwise (a thing laid on its side in a grid).
pub fn turned(image: &Image) -> Image {
    let (w, h) = (image.width, image.height);
    let mut rgba = vec![0u8; image.rgba.len()];
    for y in 0..h {
        for x in 0..w {
            let from = ((y * w + x) * 4) as usize;
            // (x, y) lands at (h - 1 - y, x) in a picture h wide.
            let to = ((x * h + (h - 1 - y)) * 4) as usize;
            rgba[to..to + 4].copy_from_slice(&image.rgba[from..from + 4]);
        }
    }
    Image::new(h, w, rgba)
}

/// Twice the signed area of `a b p`: which side of `a`→`b` `p` is on.
fn edge(a: Vec3, b: Vec3, p: Vec3) -> f64 {
    (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x)
}

/// Linear light to an sRGB byte.
fn srgb(v: f64) -> u8 {
    let v = v.clamp(0.0, 1.0);
    let s = if v <= 0.003_130_8 { v * 12.92 } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 };
    (s * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quad(lo: [f32; 2], hi: [f32; 2], color: [f32; 4]) -> Vec<Vertex> {
        let v = |x: f32, y: f32| Vertex { pos: [x, y, 0.0], normal: [0.0, 0.0, -1.0], color, emissive: [0.0; 3] };
        vec![v(lo[0], lo[1]), v(hi[0], lo[1]), v(hi[0], hi[1]), v(lo[0], lo[1]), v(hi[0], hi[1]), v(lo[0], hi[1])]
    }

    #[test]
    fn it_fills_the_middle_leaves_the_edges_clear_and_turns() {
        let img = draw(&quad([-1.0, -0.5], [1.0, 0.5], [1.0, 0.0, 0.0, 1.0]), (2, 1));
        assert_eq!((img.width, img.height), (2 * CELL_PX, CELL_PX));
        fn px(img: &Image, x: u32, y: u32) -> &[u8] {
            &img.rgba[((y * img.width + x) * 4) as usize..][..4]
        }
        let mid = px(&img, img.width / 2, img.height / 2);
        assert!(mid[3] == 255 && mid[0] > 100 && mid[1] < 10, "the middle is the red thing: {mid:?}");
        assert_eq!(px(&img, 1, 1)[3], 0, "a corner is clear");
        let t = turned(&img);
        assert_eq!((t.width, t.height), (CELL_PX, 2 * CELL_PX));
        assert_eq!(px(&t, t.width / 2, t.height / 2), mid);
    }

    #[test]
    fn every_item_draws_as_something() {
        let path = format!("{}/assets/models/items.glb", env!("CARGO_MANIFEST_DIR"));
        let g = lntrn_model::Gltf::load(&path).expect("items");
        for kind in crate::loot::ALL {
            let def = kind.def();
            let node = g.nodes.iter().find(|n| n.name.as_deref() == Some(def.model)).unwrap_or_else(|| panic!("{} has no model", def.model));
            let mesh = &g.meshes[node.mesh.unwrap()];
            let mut vs = Vec::new();
            for p in &mesh.primitives {
                for t in p.indices.chunks_exact(3) {
                    let at = [t[0], t[1], t[2]].map(|k| p.positions[k as usize]);
                    let v = at.map(|a| Vec3::new(f64::from(a[0]), f64::from(a[1]), f64::from(a[2])));
                    let n = (v[1] - v[0]).cross(v[2] - v[0]);
                    if n.length() < 1e-12 {
                        continue;
                    }
                    let n = n.normalize();
                    for &k in t {
                        let c = p.colors.get(k as usize).copied().unwrap_or([1.0; 4]);
                        vs.push(Vertex { pos: p.positions[k as usize], normal: [n.x as f32, n.y as f32, n.z as f32], color: c, emissive: [0.0; 3] });
                    }
                }
            }
            let img = draw(&vs, def.size);
            let covered = img.rgba.chunks_exact(4).filter(|p| p[3] > 128).count() as f64 / (img.width * img.height) as f64;
            // (A chain or a key is mostly air.)
            assert!(covered > 0.08, "{} covers {:.0}% of its picture", def.name, covered * 100.0);
        }
    }
}
