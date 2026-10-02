//! The glass in windows: a pane in every window that's only for looking
//! out of (`map/building/glazing.rs`), whole till a shot, a blow or a
//! blast goes through it. Then it's gone for good, in a burst of shards,
//! with a crash the dead hear; its frame stays, and bodies are kept out of
//! the window as they were. A whole pane is drawn each frame, seen
//! through (the renderer's glass).

use bevy_ecs::prelude::*;
use lntrn_math::{Mat4, Vec3};

use crate::camera::Frustum;
use crate::fx::Fx;
use crate::render::{Draw, MeshId, Renderer, Vertex};
use crate::sound::Sfx;
use crate::zombie::{self, Horde};

/// A pane as it's drawn: how thick, its colour (sRGB, straight), and how
/// much of what's behind it it hides, looked at square on.
const THICK: f64 = 0.016;
const TINT: [f32; 3] = [0.62, 0.80, 0.84];
const FILM: f32 = 0.16;
/// How far off a pane breaking is heard by the dead, and how loud it is
/// to the players.
const HEARD: f64 = 22.0;
const CRASH: f32 = 0.9;
/// Shards to a square metre of pane, and at most.
const SHARDS: f64 = 26.0;
const MOST_SHARDS: usize = 70;

/// A pane: its box, and whether it's whole.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pane {
    pub lo: Vec3,
    pub hi: Vec3,
    pub whole: bool,
}

impl Pane {
    fn centre(&self) -> Vec3 {
        (self.lo + self.hi) * 0.5
    }

    /// Where along a ray from `from` along `dir` (a unit long) it goes in
    /// by the pane, if it does within `reach`.
    fn struck(&self, from: Vec3, dir: Vec3, reach: f64) -> Option<f64> {
        let (mut near, mut far) = (0.0f64, reach);
        for (o, d, lo, hi) in [(from.x, dir.x, self.lo.x, self.hi.x), (from.y, dir.y, self.lo.y, self.hi.y), (from.z, dir.z, self.lo.z, self.hi.z)] {
            if d.abs() < 1e-12 {
                if o < lo || o > hi {
                    return None;
                }
                continue;
            }
            let (a, b) = ((lo - o) / d, (hi - o) / d);
            (near, far) = (near.max(a.min(b)), far.min(a.max(b)));
            if near > far {
                return None;
            }
        }
        Some(near)
    }
}

/// Every pane on the map; and those broken away from where the shards
/// are thrown (by something flying through them), till they are
/// ([`settle`]): each, and the way it went.
#[derive(Resource, Default)]
pub struct Glazing {
    pub panes: Vec<Pane>,
    fresh: Vec<(Pane, Vec3)>,
}

impl Glazing {
    /// Panes in these boxes, all whole.
    pub fn new(boxes: impl IntoIterator<Item = (Vec3, Vec3)>) -> Self {
        Self { panes: boxes.into_iter().map(|(lo, hi)| Pane { lo, hi, whole: true }).collect(), fresh: Vec::new() }
    }

    /// Every whole pane a shot (or a blow) from `from` along `dir` goes
    /// through within `reach`, broken: each, as it was.
    pub fn shoot(&mut self, from: Vec3, dir: Vec3, reach: f64) -> Vec<Pane> {
        self.broken(|p| p.struck(from, dir, reach).is_some())
    }

    /// Every whole pane with any of it within `radius` of `at`, broken.
    pub fn blast(&mut self, at: Vec3, radius: f64) -> Vec<Pane> {
        self.broken(|p| (at.max(p.lo).min(p.hi) - at).length() <= radius)
    }

    fn broken(&mut self, mut hit: impl FnMut(&Pane) -> bool) -> Vec<Pane> {
        let mut out = Vec::new();
        for p in self.panes.iter_mut().filter(|p| p.whole) {
            if hit(p) {
                out.push(*p);
                p.whole = false;
            }
        }
        out
    }
}

/// A shot (or a blow) from `from` along `dir`, as far as `reach`: the
/// glass it goes through breaks, its shards flying on the way it went.
pub fn shot(world: &mut World, fx: &mut Fx, from: Vec3, dir: Vec3, reach: f64) {
    let Some(broken) = world.get_resource_mut::<Glazing>().map(|mut g| g.shoot(from, dir, reach)) else { return };
    shatter(world, fx, &broken, |_| dir);
}

/// Something thrown flying from `from` along `dir`, `reach` far: the
/// glass it goes through breaks (its shards thrown at the next
/// [`settle`]).
pub fn through(world: &mut World, from: Vec3, dir: Vec3, reach: f64) {
    if let Some(mut g) = world.get_resource_mut::<Glazing>() {
        let broken = g.shoot(from, dir, reach);
        g.fresh.extend(broken.into_iter().map(|p| (p, dir)));
    }
}

/// The panes broken by what flew through them since the last: their
/// shards, and the crash of each.
pub fn settle(world: &mut World, fx: &mut Fx) {
    let Some(fresh) = world.get_resource_mut::<Glazing>().map(|mut g| std::mem::take(&mut g.fresh)) else { return };
    for (pane, way) in fresh {
        shatter(world, fx, &[pane], |_| way);
    }
}

/// A blast at `at`: the glass within `radius` of it breaks, blown out
/// away from it.
pub fn blast(world: &mut World, fx: &mut Fx, at: Vec3, radius: f64) {
    let Some(broken) = world.get_resource_mut::<Glazing>().map(|mut g| g.blast(at, radius)) else { return };
    shatter(world, fx, &broken, |p| (p.centre() - at).try_normalize().unwrap_or(Vec3::Y));
}

/// Panes just broken: shards of each thrown the way `flung` says, the
/// crash of it, and the dead in earshot told.
fn shatter(world: &mut World, fx: &mut Fx, broken: &[Pane], flung: impl Fn(&Pane) -> Vec3) {
    for p in broken {
        let size = p.hi - p.lo;
        // (Its face is its two longer sides.)
        let area = size.x * size.y * size.z / size.x.min(size.y).min(size.z).max(1e-6);
        fx.shards(p.lo, p.hi, flung(p), ((area * SHARDS) as usize).clamp(8, MOST_SHARDS));
        world.resource_mut::<Horde>().sounds.push((Sfx::Shatter, p.centre(), CRASH));
        zombie::noise(world, p.centre(), HEARD);
    }
}

/// The pane's mesh: a box a unit a side about its middle, glass all over.
#[derive(Resource, Clone, Copy)]
struct Mesh(MeshId);

/// Make the pane's mesh.
pub fn load(renderer: &mut Renderer, world: &mut World) {
    let s = 0.5f32;
    let faces: [([f32; 3], [[f32; 3]; 4]); 6] = [
        ([1.0, 0.0, 0.0], [[s, -s, -s], [s, s, -s], [s, s, s], [s, -s, s]]),
        ([-1.0, 0.0, 0.0], [[-s, -s, s], [-s, s, s], [-s, s, -s], [-s, -s, -s]]),
        ([0.0, 1.0, 0.0], [[-s, s, -s], [-s, s, s], [s, s, s], [s, s, -s]]),
        ([0.0, -1.0, 0.0], [[-s, -s, s], [-s, -s, -s], [s, -s, -s], [s, -s, s]]),
        ([0.0, 0.0, 1.0], [[-s, -s, s], [s, -s, s], [s, s, s], [-s, s, s]]),
        ([0.0, 0.0, -1.0], [[s, -s, -s], [-s, -s, -s], [-s, s, -s], [s, s, -s]]),
    ];
    let c = lntrn_math::Color::rgb(f64::from(TINT[0]), f64::from(TINT[1]), f64::from(TINT[2])).to_linear();
    let color = [c.r as f32, c.g as f32, c.b as f32, 1.0];
    let verts: Vec<Vertex> = faces.iter().flat_map(|(normal, q)| [0, 1, 2, 0, 2, 3].map(|i| Vertex { pos: q[i], normal: *normal, color, emissive: [0.0; 3] })).collect();
    world.insert_resource(Mesh(renderer.add_mesh(&verts)));
}

/// Queue every whole pane for drawing, in each pane of the window
/// (`sights`) that sees it.
pub fn draw(world: &World, renderer: &mut Renderer, sights: &[Frustum]) {
    let (Some(glazing), Some(&Mesh(mesh))) = (world.get_resource::<Glazing>(), world.get_resource::<Mesh>()) else { return };
    for p in glazing.panes.iter().filter(|p| p.whole) {
        let size = p.hi - p.lo;
        let (centre, radius) = (p.centre(), size.length() * 0.5);
        // Thin the way its box is thinnest.
        let thin = size.x.min(size.y).min(size.z);
        let scale = Vec3::new(if size.x <= thin { THICK } else { size.x }, if size.y <= thin { THICK } else { size.y }, if size.z <= thin { THICK } else { size.z });
        let d = Draw { mesh, model: Mat4::from_translation(centre) * Mat4::from_scale(scale), emissive: 0.0, fog: 1.0, tint: [1.0; 3] };
        for (i, sight) in sights.iter().enumerate() {
            if sight.sees(centre, radius) {
                renderer.draw_glass_in(i, d, FILM);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A window's pane across x at z = 5: 1.2 wide, from 0.9 to 2.1 up.
    fn pane() -> (Vec3, Vec3) {
        (Vec3::new(0.0, 0.9, 4.97), Vec3::new(1.2, 2.1, 5.03))
    }

    #[test]
    fn a_shot_through_a_pane_breaks_it_once_and_a_miss_doesnt() {
        let mut g = Glazing::new([pane(), (Vec3::new(3.0, 0.9, 4.97), Vec3::new(4.2, 2.1, 5.03))]);
        let eye = Vec3::new(0.6, 1.6, 0.0);
        let ahead = Vec3::new(0.0, 0.0, 1.0);
        assert!(g.shoot(eye, ahead, 4.0).is_empty(), "it stops short (a wall, something hit)");
        assert!(g.shoot(eye, Vec3::new(0.0, 1.0, 0.0), 50.0).is_empty(), "nor straight up");
        assert!(g.shoot(eye, -ahead, 50.0).is_empty(), "nor the other way");
        let broken = g.shoot(eye, ahead, 50.0);
        assert_eq!(broken.len(), 1);
        assert_eq!((broken[0].lo, g.panes[0].whole, g.panes[1].whole), (pane().0, false, true));
        assert!(g.shoot(eye, ahead, 50.0).is_empty(), "there's nothing left of it to break");
        // From outside, slantwise, through the other.
        let from = Vec3::new(1.0, 1.0, 9.0);
        let to = (Vec3::new(3.6, 1.5, 5.0) - from).normalize();
        assert_eq!(g.shoot(from, to, 50.0).len(), 1);
        assert!(g.panes.iter().all(|p| !p.whole));
    }

    #[test]
    fn a_blast_breaks_what_s_near_it() {
        let mut g = Glazing::new([pane(), (Vec3::new(20.0, 0.9, 4.97), Vec3::new(21.2, 2.1, 5.03))]);
        assert!(g.blast(Vec3::new(0.6, 0.5, 12.0), 6.0).is_empty(), "too far off");
        assert_eq!(g.blast(Vec3::new(4.0, 0.5, 2.0), 6.0).len(), 1, "its near corner's in reach");
        assert_eq!((g.panes[0].whole, g.panes[1].whole), (false, true));
    }

    #[test]
    fn a_broken_pane_is_heard_and_leaves_shards() {
        let mut world = World::new();
        world.insert_resource(Horde::default());
        world.insert_resource(zombie::Noises::default());
        world.insert_resource(zombie::Heat::default());
        world.insert_resource(Glazing::new([pane()]));
        let mut fx = Fx::default();
        shot(&mut world, &mut fx, Vec3::new(0.6, 1.6, 0.0), Vec3::new(0.0, 0.0, 1.0), 50.0);
        assert_eq!(world.resource::<Horde>().sounds.iter().map(|s| s.0).collect::<Vec<_>>(), [Sfx::Shatter]);
        assert!(fx.chip_count() >= 20, "{} shards", fx.chip_count());
        // And nothing the second time.
        shot(&mut world, &mut fx, Vec3::new(0.6, 1.6, 0.0), Vec3::new(0.0, 0.0, 1.0), 50.0);
        assert_eq!(world.resource::<Horde>().sounds.len(), 1);
        // Something thrown through one breaks it there and then; its
        // shards and its crash come as it's settled.
        world.insert_resource(Glazing::new([pane()]));
        let before = fx.chip_count();
        through(&mut world, Vec3::new(0.6, 1.6, 4.8), Vec3::new(0.0, 0.0, 1.0), 0.4);
        assert!(!world.resource::<Glazing>().panes[0].whole && fx.chip_count() == before);
        settle(&mut world, &mut fx);
        assert!(fx.chip_count() > before && world.resource::<Horde>().sounds.len() == 2);
        settle(&mut world, &mut fx);
        assert_eq!(world.resource::<Horde>().sounds.len(), 2, "once");
    }
}
