//! The map screen (M): the land as a paper map (drawn once per map, as a
//! picture: the lie of the land shaded and in contour lines, the forest,
//! fields, plots and roads), the places named on it, the ways out once
//! found, and the player's arrow. The run carries on under it.

use lntrn_app::lntrn_render::ImageHandle;
use lntrn_image::Image;
use lntrn_math::{Color, Rect, Vec2, Vec3};
use lntrn_text::TextStyle;
use lntrn_ui::Ui;

use super::roads::{Kind as RoadKind, Network};
use super::scatter::Scenery;
use super::sites::Kind as SiteKind;
use super::{HALF, Map};
use crate::exits::Exit;
use crate::style;

/// The picture's size, pixels a side.
pub const PICTURE: u32 = 900;
/// Metres between contour lines.
const CONTOUR: f64 = 4.0;

type Rgb = [f64; 3];

const PAPER: Rgb = [0.80, 0.76, 0.64];
const WOODS: Rgb = [0.50, 0.58, 0.40];
const TREE: Rgb = [0.33, 0.42, 0.27];
const PLOT: Rgb = [0.66, 0.61, 0.52];
const FIELD: Rgb = [0.83, 0.74, 0.46];
const HIGHWAY: Rgb = [0.72, 0.30, 0.18];
const PAVED: Rgb = [0.93, 0.89, 0.76];
const TRACK: Rgb = [0.52, 0.40, 0.27];
const INK: Rgb = [0.20, 0.17, 0.13];

fn mix(a: Rgb, b: Rgb, t: f64) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

/// Where a point of the map is on the picture, in pixels.
fn to_px(x: f64, z: f64) -> (f64, f64) {
    let k = f64::from(PICTURE) / (2.0 * HALF);
    ((x + HALF) * k, (z + HALF) * k)
}

/// Draw the map as a picture, north (-z) up.
pub fn picture(map: &Map, network: &Network) -> Image {
    let n = PICTURE as usize;
    let m = 2.0 * HALF / f64::from(PICTURE);
    let at = |i: usize, j: usize| (-HALF + (i as f64 + 0.5) * m, -HALF + (j as f64 + 0.5) * m);
    // The land's height at every pixel, and a pixel either side past the
    // edge (for shading and contours).
    let heights: Vec<f64> = super::rows(n + 1, |j| {
        (0..=n)
            .map(|i| {
                let (x, z) = at(i, j);
                map.field.height_at(x, z).unwrap_or(0.0)
            })
            .collect()
    });
    let h = |i: usize, j: usize| heights[j * (n + 1) + i];
    let mut px: Vec<Rgb> = super::rows(n, |j| {
        (0..n)
            .map(|i| {
                let (x, z) = at(i, j);
                let p = Vec2::new(x, z);
                let mut c = mix(PAPER, WOODS, map.forest.density(x, z) * 0.8);
                for f in &map.fields {
                    if f.outside(p) <= 0.0 {
                        c = FIELD;
                    }
                }
                for s in &map.sites {
                    if s.plot.outside(p) <= 0.0 {
                        c = PLOT;
                    }
                }
                // Lit from the north-west, and a line every few metres up.
                let (dx, dz) = (h(i + 1, j) - h(i, j), h(i, j + 1) - h(i, j));
                let light = (0.5 * (-dx - dz) / m).clamp(-1.0, 1.0);
                c = c.map(|v| v * (1.0 + 0.35 * light));
                let band = |v: f64| (v / CONTOUR).floor();
                if band(h(i, j)) != band(h(i + 1, j)) || band(h(i, j)) != band(h(i, j + 1)) {
                    c = mix(c, INK, 0.25);
                }
                c
            })
            .collect()
    });
    let mut dot = |x: f64, z: f64, r: f64, colour: Rgb| {
        let (cx, cy) = to_px(x, z);
        let (i0, i1) = ((cx - r).floor().max(0.0) as usize, ((cx + r).ceil() as usize).min(n - 1));
        let (j0, j1) = ((cy - r).floor().max(0.0) as usize, ((cy + r).ceil() as usize).min(n - 1));
        for j in j0..=j1 {
            for i in i0..=i1 {
                let (ddx, ddy) = (i as f64 + 0.5 - cx, j as f64 + 0.5 - cy);
                if ddx * ddx + ddy * ddy <= r * r {
                    px[j * n + i] = colour;
                }
            }
        }
    };
    for piece in &map.scenery {
        if matches!(piece.what, Scenery::Pine(_)) {
            dot(piece.at.x, piece.at.z, 1.3, TREE);
        }
    }
    // The roads: the lesser first, the highway over them; each with a dark
    // edge.
    let mut roads: Vec<_> = network.roads.iter().collect();
    roads.sort_by_key(|r| r.kind);
    for pass in 0..2 {
        for road in &roads {
            let (colour, width) = match road.kind {
                RoadKind::Highway => (HIGHWAY, 5.0),
                RoadKind::Paved => (PAVED, 3.5),
                RoadKind::Dirt => (TRACK, 2.5),
            };
            let (colour, r) = if pass == 0 { (INK, width * 0.5 + 1.0) } else { (colour, width * 0.5) };
            for w in road.points.windows(2) {
                let steps = ((w[1] - w[0]).length() / (m * 0.5)).ceil().max(1.0) as usize;
                for s in 0..=steps {
                    let q = w[0] + (w[1] - w[0]) * (s as f64 / steps as f64);
                    dot(q.x, q.z, r, colour);
                }
            }
        }
    }
    let rgba = px.iter().flat_map(|c| [to_byte(c[0]), to_byte(c[1]), to_byte(c[2]), 255]).collect();
    Image::new(PICTURE, PICTURE, rgba)
}

fn to_byte(v: f64) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// Draw the map screen over the view: the picture, the places named, the
/// ways out found, and where the player stands facing `yaw`.
pub fn draw(ui: &mut Ui, picture: ImageHandle, map: &Map, exits: &[Exit], player: Vec3, yaw: f64) {
    let s = ui.m.scale;
    let screen = ui.clip();
    ui.draw.rect(screen, Color::rgba(0.0, 0.0, 0.0, 0.8));
    let side = (screen.width().min(screen.height()) - 80.0 * s).max(200.0);
    let area = Rect::from_min_size(screen.center() - Vec2::splat(side * 0.5), Vec2::splat(side));
    ui.draw.image(area, picture, 0.0, Color::WHITE);
    ui.draw.stroke_rect(area, 4.0 * s, 0.0, Color::rgba(0.1, 0.08, 0.06, 1.0));
    let on = |p: Vec3| Vec2::new(area.min.x + (p.x + HALF) / (2.0 * HALF) * side, area.min.y + (p.z + HALF) / (2.0 * HALF) * side);
    let ink = Color::rgba(0.16, 0.12, 0.09, 1.0);
    let label = |ui: &mut Ui, text: &str, at: Vec2, size: f64, colour: Color| {
        let style = TextStyle::new((size * s) as f32).bold().family(style::FONT);
        let w = ui.measure(text, &style);
        let h = f64::from(style.line_height());
        let at = Vec2::new(at.x - w * 0.5, at.y - h * 0.5);
        ui.draw.rect(Rect::from_min_size(at - Vec2::new(6.0 * s, 1.0 * s), Vec2::new(w + 12.0 * s, h + 2.0 * s)), Color::rgba(0.86, 0.82, 0.70, 0.85));
        ui.text_at(text, &style, at, w + 4.0, colour);
    };
    for site in &map.sites {
        let c = site.plot.centre;
        let size = if site.kind == SiteKind::Town { 30.0 } else { 22.0 };
        label(ui, site.name, on(Vec3::new(c.x, 0.0, c.y)), size, ink);
    }
    // North, at the top.
    label(ui, "N", Vec2::new(area.center().x, area.min.y + 24.0 * s), 26.0, ink);
    // The ways out found: a diamond, and the name under it (dim if it's
    // shut).
    for e in exits.iter().filter(|e| e.found) {
        let at = on(e.zone);
        let colour = if e.open { style::SIGNAL } else { style::DIM };
        let r = 13.0 * s;
        ui.draw.triangle(at + Vec2::new(0.0, -r), at + Vec2::new(r, 0.0), at + Vec2::new(0.0, r), colour);
        ui.draw.triangle(at + Vec2::new(0.0, -r), at + Vec2::new(0.0, r), at + Vec2::new(-r, 0.0), colour);
        label(ui, e.way.name(), at + Vec2::new(0.0, 30.0 * s), 20.0, if e.open { Color::rgba(0.05, 0.35, 0.12, 1.0) } else { ink });
    }
    // The player: an arrow the way they face.
    let at = on(player);
    let f = Vec2::new(-yaw.sin(), -yaw.cos());
    let r = Vec2::new(-f.y, f.x);
    let (tip, back) = (at + f * (20.0 * s), at - f * (12.0 * s));
    let outline = 4.0 * s;
    ui.draw.triangle(tip + f * outline, back + r * (13.0 * s + outline) - f * outline, back - r * (13.0 * s + outline) - f * outline, Color::rgba(0.0, 0.0, 0.0, 1.0));
    ui.draw.triangle(tip, back + r * (13.0 * s), back - r * (13.0 * s), Color::rgba(0.85, 0.12, 0.08, 1.0));
}
