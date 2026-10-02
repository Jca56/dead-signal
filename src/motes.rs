//! What drifts in a night's air: embers off whatever burns (a fire, the
//! dead alight, a hound), sparks from a lamp that's failing, ash coming
//! down over everything out of doors, and mist lying low about the yard.
//! None of it touches anything; it's only seen.

use bevy_ecs::prelude::*;
use lntrn_math::{Mat4, Quat, Vec3};

use crate::camera::Camera;
use crate::holdout::lamps::{Lamp, Mood};
use crate::player::Body;
use crate::render::{Draw, MeshId, Renderer, Vertex};
use crate::throw::{Burning, Fire};
use crate::world::Solid;
use crate::zombie::brain::Zombie;
use crate::zombie::figure::Figure;
use crate::zombie::kind::Kind;

/// The most embers and sparks at once.
const MOST: usize = 500;
/// Embers a second: off a metre of fire, off one of the dead alight, off
/// a hound.
const OFF_FIRE: f64 = 9.0;
const OFF_BURNING: f64 = 6.0;
const OFF_HOUND: f64 = 7.0;
/// A failing lamp, dim or out, spits sparks this often a second.
const SPITS: f64 = 2.5;
/// Ash: how many flakes about each eye, how far out and how far up
/// they're kept, and how fast they fall (and how much faster, some).
const FLAKES: usize = 220;
const ABOUT: f64 = 13.0;
const OVER: f64 = 9.0;
const FALLS: (f64, f64) = (0.35, 0.3);
/// Mist: how many wisps, how far past the compound they lie, how wide
/// and how deep each is (and how much more, some), and how thick.
const WISPS: usize = 70;
const PAST: f64 = 18.0;
const WISP: (f64, f64, f64, f64) = (3.5, 4.0, 0.45, 0.4);
const THICK: (f32, f32) = (0.07, 0.09);
const MIST: [f32; 3] = [0.40, 0.46, 0.60];

#[derive(Clone, Copy)]
struct Meshes {
    ember: MeshId,
    spark: MeshId,
    flake: MeshId,
    wisp: MeshId,
}

/// An ember or a spark: where, how fast, how long it lasts and has left,
/// and whether it falls (a spark does; an ember rises).
struct Speck {
    pos: Vec3,
    vel: Vec3,
    life: f64,
    left: f64,
    spark: bool,
}

/// A flake of ash: where, the height it lands at, how fast it falls, and
/// what sets its drift apart.
#[derive(Clone, Copy)]
struct Flake {
    pos: Vec3,
    lands: f64,
    falls: f64,
    own: f64,
}

/// A wisp of mist: where it lies, how wide and deep, how thick, and what
/// sets its drift apart.
struct Wisp {
    at: Vec3,
    wide: f64,
    deep: f64,
    thick: f32,
    own: f64,
}

#[derive(Default)]
pub struct Motes {
    meshes: Option<Meshes>,
    specks: Vec<Speck>,
    /// Each pane's ash, about its eye.
    ash: Vec<Vec<Flake>>,
    /// The mist, and the compound it was laid about (none: not laid yet).
    mist: Vec<Wisp>,
    laid: Option<(Vec3, Vec3)>,
    seed: u32,
}

/// A box of `size`, glowing `glow` (or lit like anything else, `color`).
fn speck(size: [f32; 3], color: [f32; 3], glow: [f32; 3]) -> Vec<Vertex> {
    let [x, y, z] = size.map(|s| s * 0.5);
    let faces: [([f32; 3], [[f32; 3]; 4]); 6] = [
        ([1.0, 0.0, 0.0], [[x, -y, -z], [x, y, -z], [x, y, z], [x, -y, z]]),
        ([-1.0, 0.0, 0.0], [[-x, -y, z], [-x, y, z], [-x, y, -z], [-x, -y, -z]]),
        ([0.0, 1.0, 0.0], [[-x, y, -z], [-x, y, z], [x, y, z], [x, y, -z]]),
        ([0.0, -1.0, 0.0], [[-x, -y, z], [-x, -y, -z], [x, -y, -z], [x, -y, z]]),
        ([0.0, 0.0, 1.0], [[-x, -y, z], [x, -y, z], [x, y, z], [-x, y, z]]),
        ([0.0, 0.0, -1.0], [[x, -y, -z], [-x, -y, -z], [-x, y, -z], [x, y, -z]]),
    ];
    let mut out = Vec::new();
    for (normal, q) in faces {
        for i in [0, 1, 2, 0, 2, 3] {
            out.push(Vertex { pos: q[i], normal, color: [color[0], color[1], color[2], 1.0], emissive: glow });
        }
    }
    out
}

/// A ball a metre across, smooth all round (its normals out from its
/// middle): a wisp, squashed and stretched as it's drawn.
pub(crate) fn ball() -> Vec<Vertex> {
    let (rings, round) = (8u32, 14u32);
    let at = |i: u32, j: u32| {
        let (lat, lon) = (std::f64::consts::PI * f64::from(i) / f64::from(rings), std::f64::consts::TAU * f64::from(j) / f64::from(round));
        Vec3::new(lat.sin() * lon.cos(), lat.cos(), lat.sin() * lon.sin())
    };
    let v = |n: Vec3| Vertex { pos: [(n.x * 0.5) as f32, (n.y * 0.5) as f32, (n.z * 0.5) as f32], normal: [n.x as f32, n.y as f32, n.z as f32], color: [1.0; 4], emissive: [0.0; 3] };
    let mut out = Vec::new();
    for i in 0..rings {
        for j in 0..round {
            let (a, b, c, d) = (at(i, j), at(i, j + 1), at(i + 1, j + 1), at(i + 1, j));
            out.extend([v(a), v(b), v(c), v(a), v(c), v(d)]);
        }
    }
    out
}

impl Motes {
    /// Make what they're drawn with.
    pub fn init(&mut self, renderer: &mut Renderer) {
        self.seed = 0x7A3C_91E5;
        self.meshes = Some(Meshes {
            ember: renderer.add_mesh(&speck([1.0; 3], [1.0, 0.4, 0.1], [1.0, 0.42, 0.08])),
            spark: renderer.add_mesh(&speck([1.0; 3], [1.0, 0.9, 0.6], [1.0, 0.85, 0.45])),
            flake: renderer.add_mesh(&speck([1.0, 0.15, 1.0], [0.60, 0.60, 0.62], [0.0; 3])),
            wisp: renderer.add_mesh(&ball()),
        });
    }

    /// A new map: nothing in the air yet, the mist to be laid again.
    pub fn clear(&mut self) {
        self.specks.clear();
        self.ash.clear();
        self.mist.clear();
        self.laid = None;
    }

    fn rand(&mut self) -> f64 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        f64::from(self.seed) / f64::from(u32::MAX)
    }

    /// As many as `rate` a second makes in `dt`, by luck.
    fn so_many(&mut self, rate: f64, dt: f64) -> usize {
        let n = rate * dt;
        n.floor() as usize + usize::from(self.rand() < n.fract())
    }

    fn ember(&mut self, at: Vec3, spread: f64) {
        let pos = at + Vec3::new((self.rand() - 0.5) * spread, self.rand() * 0.3, (self.rand() - 0.5) * spread);
        let vel = Vec3::new((self.rand() - 0.5) * 0.6, 0.7 + 1.1 * self.rand(), (self.rand() - 0.5) * 0.6);
        let life = 1.0 + 1.3 * self.rand();
        self.specks.push(Speck { pos, vel, life, left: life, spark: false });
    }

    /// A frame of it, a night's, the compound from `lo` to `hi`: what
    /// burns throws up its embers, what's failing spits, the ash comes
    /// down about each of `eyes`, the mist drifts; and all of it's drawn.
    pub fn frame(&mut self, world: &mut World, renderer: &mut Renderer, eyes: &[Camera], (lo, hi): (Vec3, Vec3), time: f64, dt: f64) {
        let Some(m) = self.meshes else { return };
        // Embers, off what's burning.
        let fires: Vec<(Vec3, f64)> = world.query::<&Fire>().iter(world).map(|f| (f.at, f.size())).collect();
        for (at, size) in fires {
            for _ in 0..self.so_many(OFF_FIRE * size, dt) {
                self.ember(at + Vec3::new(0.0, 0.2, 0.0), size * 1.4);
            }
        }
        let alight: Vec<Vec3> = world.query_filtered::<&Body, With<Burning>>().iter(world).map(|b| b.pos).collect();
        for at in alight {
            for _ in 0..self.so_many(OFF_BURNING, dt) {
                self.ember(at + Vec3::new(0.0, 1.0, 0.0), 0.5);
            }
        }
        let hounds: Vec<Vec3> = world.query::<(&Zombie, &Figure)>().iter(world).filter(|(z, _)| z.kind == Kind::Hound && !z.dead()).filter_map(|(_, f)| f.chest()).collect();
        for at in hounds {
            for _ in 0..self.so_many(OFF_HOUND, dt) {
                self.ember(at + Vec3::new(0.0, 0.2, 0.0), 0.5);
            }
        }
        // Sparks, from a lamp that's failing, while it's dim or out.
        let failing: Vec<Vec3> = world.query::<&Lamp>().iter(world).filter(|l| l.mood == Mood::Flicker && l.brightness(time) < 0.5).map(|l| l.light.at).collect();
        for at in failing {
            if self.so_many(SPITS, dt) == 0 {
                continue;
            }
            for _ in 0..3 + (self.rand() * 4.0) as usize {
                let vel = Vec3::new((self.rand() - 0.5) * 2.6, self.rand() * 0.8, (self.rand() - 0.5) * 2.6);
                let life = 0.4 + 0.5 * self.rand();
                self.specks.push(Speck { pos: at, vel, life, left: life, spark: true });
            }
        }
        for s in &mut self.specks {
            s.vel.y -= if s.spark { 7.0 * dt } else { 0.25 * dt };
            s.vel *= (-0.8 * dt).exp();
            s.pos += s.vel * dt;
            s.left -= dt;
        }
        self.specks.retain(|s| s.left > 0.0);
        if self.specks.len() > MOST {
            self.specks.drain(..self.specks.len() - MOST);
        }
        for s in &self.specks {
            let share = (s.left / s.life).clamp(0.0, 1.0);
            let size = if s.spark { 0.03 } else { 0.045 } * share.sqrt();
            let turn = Quat::from_axis_angle(Vec3::new(0.5, 0.7, 0.3).normalize(), s.left * 9.0);
            renderer.draw(Draw { mesh: if s.spark { m.spark } else { m.ember }, model: Mat4::from_translation(s.pos) * Mat4::from_quat(turn) * Mat4::from_scale(Vec3::splat(size)), emissive: (0.4 + 0.6 * share) as f32, fog: 0.6, tint: [1.0; 3] });
        }
        self.fall(world, renderer, m.flake, eyes, time, dt);
        self.drift(world, renderer, m.wisp, (lo, hi), time);
    }

    /// The ash: about each eye, each flake falling to where it lands (on a
    /// roof, over anywhere indoors) and then coming again from the top.
    fn fall(&mut self, world: &World, renderer: &mut Renderer, mesh: MeshId, eyes: &[Camera], time: f64, dt: f64) {
        let solid = &world.resource::<Solid>().0;
        self.ash.resize(eyes.len(), Vec::new());
        for (pane, eye) in eyes.iter().map(|c| c.position).enumerate() {
            let top = eye.y + OVER;
            // A flake somewhere about the eye: at the top, or (`anywhere`)
            // at any height over where it lands.
            let fresh = |this: &mut Self, anywhere: bool| {
                let (x, z) = (eye.x + (this.rand() - 0.5) * 2.0 * ABOUT, eye.z + (this.rand() - 0.5) * 2.0 * ABOUT);
                let lands = solid.raycast(Vec3::new(x, 70.0, z), -Vec3::Y, 120.0).map_or(-50.0, |h| h.point.y);
                let y = if anywhere { lands.max(eye.y - OVER) + this.rand() * (top - lands.max(eye.y - OVER)).max(0.0) } else { top };
                Flake { pos: Vec3::new(x, y, z), lands, falls: FALLS.0 + FALLS.1 * this.rand(), own: this.rand() * 40.0 }
            };
            let mut flakes = std::mem::take(&mut self.ash[pane]);
            while flakes.len() < FLAKES {
                flakes.push(fresh(self, true));
            }
            for f in &mut flakes {
                f.pos.y -= f.falls * dt;
                f.pos.x += (time * 0.6 + f.own).sin() * 0.3 * dt;
                f.pos.z += (time * 0.45 + f.own * 1.7).cos() * 0.3 * dt;
                let strayed = (f.pos.x - eye.x).abs() > ABOUT || (f.pos.z - eye.z).abs() > ABOUT || f.pos.y > top + 1.0;
                if strayed {
                    *f = fresh(self, true);
                } else if f.pos.y <= f.lands {
                    *f = fresh(self, false);
                }
                if f.pos.y > f.lands {
                    let turn = Quat::from_axis_angle(Vec3::new(0.3, 1.0, 0.2).normalize(), time * 0.8 + f.own) * Quat::from_rotation_x((time * 1.3 + f.own).sin() * 0.9);
                    renderer.draw_in(pane, Draw { mesh, model: Mat4::from_translation(f.pos) * Mat4::from_quat(turn) * Mat4::from_scale(Vec3::splat(0.035)), emissive: 0.0, fog: 0.7, tint: [1.0; 3] });
                }
            }
            self.ash[pane] = flakes;
        }
    }

    /// The mist: laid once about the compound, on open ground (never on a
    /// roof, nor a floor), and drifting where it lies.
    fn drift(&mut self, world: &World, renderer: &mut Renderer, mesh: MeshId, (lo, hi): (Vec3, Vec3), time: f64) {
        if self.laid != Some((lo, hi)) {
            self.laid = Some((lo, hi));
            self.mist.clear();
            let solid = &world.resource::<Solid>().0;
            for _ in 0..WISPS * 8 {
                if self.mist.len() >= WISPS {
                    break;
                }
                let (x, z) = (lo.x - PAST + self.rand() * (hi.x - lo.x + 2.0 * PAST), lo.z - PAST + self.rand() * (hi.z - lo.z + 2.0 * PAST));
                let Some(hit) = solid.raycast(Vec3::new(x, 60.0, z), -Vec3::Y, 90.0) else { continue };
                // (The ground's the first thing under the sky there, and
                // it's no higher than the yard.)
                if hit.normal.y < 0.7 || hit.point.y > 0.2 {
                    continue;
                }
                let (wide, deep, thick, own) = (WISP.0 + WISP.1 * self.rand(), WISP.2 + WISP.3 * self.rand(), THICK.0 + THICK.1 * self.rand() as f32, self.rand() * 60.0);
                self.mist.push(Wisp { at: hit.point, wide, deep, thick, own });
            }
        }
        for w in &self.mist {
            let at = w.at + Vec3::new((time * 0.045 + w.own).sin() * 2.0, 0.1 + 0.08 * (time * 0.3 + w.own).sin(), (time * 0.037 + w.own * 1.3).cos() * 2.0);
            let model = Mat4::from_translation(at) * Mat4::from_quat(Quat::from_rotation_y(w.own)) * Mat4::from_scale(Vec3::new(w.wide * 2.0, w.deep * 2.0, w.wide * 1.4));
            renderer.draw_soft(Draw { mesh, model, emissive: 0.0, fog: 1.0, tint: MIST }, w.thick);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shapes_are_whole_and_face_outward() {
        for mesh in [speck([1.0; 3], [1.0; 3], [0.0; 3]), ball()] {
            assert_eq!(mesh.len() % 3, 0);
            for t in mesh.chunks(3) {
                let p = |k: usize| Vec3::new(f64::from(t[k].pos[0]), f64::from(t[k].pos[1]), f64::from(t[k].pos[2]));
                let n = (p(1) - p(0)).cross(p(2) - p(0));
                let out = p(0) + p(1) + p(2);
                assert!(n.length() < 1e-9 || n.dot(out) > 0.0, "a face turned inward: {:?}", [p(0), p(1), p(2)]);
            }
        }
    }

    #[test]
    fn so_many_a_second_come_by_luck_but_come_to_that() {
        let mut m = Motes { seed: 99, ..Motes::default() };
        let got: usize = (0..6000).map(|_| m.so_many(7.0, 1.0 / 60.0)).sum();
        assert!((650..750).contains(&got), "{got} in 100 s");
    }
}
