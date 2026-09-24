//! What a hit throws up: chips of whatever was struck (dirt, splinters,
//! concrete, sparks off steel) flying off the surface, tumbling, falling
//! and shrinking away; and the hitmarker at the crosshair.

use lntrn_math::{Color, Mat4, Quat, Vec3};

use crate::collide::Surface;
use crate::render::{Draw, MeshId, Renderer, Vertex};

const GRAVITY: f64 = 14.0;
const SIZE: f64 = 0.045;

struct Chip {
    pos: Vec3,
    vel: Vec3,
    axis: Vec3,
    spin: f64,
    angle: f64,
    life: f64,
    left: f64,
    tint: [f32; 3],
}

/// A hitmarker at the crosshair: how long it has left, and whether the hit
/// beat what it struck (drawn bigger, and red).
#[derive(Clone, Copy, Debug)]
pub struct Marker {
    pub left: f64,
    pub beaten: bool,
}

#[derive(Default)]
pub struct Fx {
    chips: Vec<Chip>,
    mesh: Option<MeshId>,
    pub marker: Option<Marker>,
    /// A little noise source, the same every run.
    seed: u32,
}

/// The chips' colour for each surface (sRGB).
pub fn chip_colour(surface: Surface) -> Color {
    match surface {
        Surface::Dirt => Color::rgb(0.34, 0.28, 0.19),
        Surface::Wood => Color::rgb(0.62, 0.50, 0.33),
        Surface::Stone => Color::rgb(0.60, 0.60, 0.58),
        Surface::Metal => Color::rgb(1.0, 0.82, 0.45),
        Surface::Flesh => Color::rgb(0.36, 0.07, 0.06),
        Surface::Bile => Color::rgb(0.52, 0.72, 0.14),
    }
}

impl Fx {
    /// Make the chip's mesh: a little cube.
    pub fn init(&mut self, renderer: &mut Renderer) {
        self.seed = 0x2F6B_5A1D;
        let s = 0.5f32;
        let faces: [([f32; 3], [[f32; 3]; 4]); 6] = [
            ([1.0, 0.0, 0.0], [[s, -s, -s], [s, s, -s], [s, s, s], [s, -s, s]]),
            ([-1.0, 0.0, 0.0], [[-s, -s, s], [-s, s, s], [-s, s, -s], [-s, -s, -s]]),
            ([0.0, 1.0, 0.0], [[-s, s, -s], [-s, s, s], [s, s, s], [s, s, -s]]),
            ([0.0, -1.0, 0.0], [[-s, -s, s], [-s, -s, -s], [s, -s, -s], [s, -s, s]]),
            ([0.0, 0.0, 1.0], [[-s, -s, s], [s, -s, s], [s, s, s], [-s, s, s]]),
            ([0.0, 0.0, -1.0], [[s, -s, -s], [-s, -s, -s], [-s, s, -s], [s, s, -s]]),
        ];
        let mut verts = Vec::new();
        for (normal, q) in faces {
            for i in [0, 1, 2, 0, 2, 3] {
                verts.push(Vertex { pos: q[i], normal, color: [1.0; 4], emissive: [0.0; 3] });
            }
        }
        self.mesh = Some(renderer.add_mesh(&verts));
    }

    fn rand(&mut self) -> f64 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        f64::from(self.seed) / f64::from(u32::MAX)
    }

    fn rand_dir(&mut self) -> Vec3 {
        let v = Vec3::new(self.rand() - 0.5, self.rand() - 0.5, self.rand() - 0.5);
        if v.length() < 1e-6 { Vec3::Y } else { v.normalize() }
    }

    /// Throw `count` chips of `surface` off a hit at `at`, out of a surface
    /// facing `normal`.
    pub fn burst(&mut self, at: Vec3, normal: Vec3, surface: Surface, count: usize) {
        let l = chip_colour(surface).to_linear();
        let speed = if surface == Surface::Metal { 5.0 } else { 3.0 };
        for _ in 0..count {
            let out = (normal * (0.6 + self.rand()) + self.rand_dir() * 0.7).normalize();
            let vel = out * speed * (0.5 + self.rand()) + Vec3::Y * self.rand();
            let life = 0.45 + 0.45 * self.rand();
            let shade = 0.8 + 0.4 * self.rand() as f32;
            let axis = self.rand_dir();
            let spin = 8.0 + 14.0 * self.rand();
            self.chips.push(Chip { pos: at + normal * 0.03, vel, axis, spin, angle: 0.0, life, left: life, tint: [l.r as f32 * shade, l.g as f32 * shade, l.b as f32 * shade] });
        }
    }

    /// Flash the hitmarker.
    pub fn mark(&mut self, beaten: bool) {
        self.marker = Some(Marker { left: if beaten { 0.35 } else { 0.15 }, beaten });
    }

    pub fn update(&mut self, dt: f64) {
        for c in &mut self.chips {
            c.vel.y -= GRAVITY * dt;
            c.vel *= (-1.5 * dt).exp();
            c.pos += c.vel * dt;
            c.angle += c.spin * dt;
            c.left -= dt;
        }
        self.chips.retain(|c| c.left > 0.0);
        if let Some(m) = self.marker.as_mut() {
            m.left -= dt;
            if m.left <= 0.0 {
                self.marker = None;
            }
        }
    }

    /// Queue the chips for this frame.
    pub fn draw(&self, renderer: &mut Renderer) {
        let Some(mesh) = self.mesh else { return };
        for c in &self.chips {
            let size = SIZE * (c.left / c.life).sqrt();
            let model = Mat4::from_translation(c.pos) * Mat4::from_quat(Quat::from_axis_angle(c.axis, c.angle)) * Mat4::from_scale(Vec3::new(size, size, size));
            renderer.draw(Draw { mesh, model, emissive: 0.0, fog: 1.0, tint: c.tint });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chips_fly_off_the_surface_fall_and_are_gone() {
        let mut fx = Fx { seed: 7, ..Fx::default() };
        fx.burst(Vec3::ZERO, Vec3::new(0.0, 0.0, 1.0), Surface::Wood, 8);
        assert_eq!(fx.chips.len(), 8);
        fx.update(0.05);
        assert!(fx.chips.iter().all(|c| c.pos.z > 0.0), "off the face, the way it faces");
        for _ in 0..60 {
            fx.update(1.0 / 60.0);
        }
        assert_eq!(fx.chips.len(), 0, "gone within a second");
        fx.mark(true);
        fx.update(0.2);
        assert!(fx.marker.is_some_and(|m| m.beaten));
        fx.update(0.2);
        assert!(fx.marker.is_none());
    }
}
