//! The lights out in the world: lamps, fires, a shot's flash. Each reaches
//! so far and fades to nothing there; a lamp's may be shut in its room
//! (nothing of it past the walls: there are no shadows cast by lamps, so
//! this is what keeps one from shining through into the next). A frame
//! says which there are; each pane is lit by the nearest few.

use lntrn_math::Vec3;

/// The most lights a pane's lit by at once (as `light.wgsl` has it).
pub const MAX_LIGHTS: usize = 24;
/// Past its room's walls a lamp fades out over this far.
const SPILL: f64 = 0.5;

/// A light: where, how far it reaches, its colour (linear), and the room
/// it's shut in, if it is (its corners).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Light {
    pub at: Vec3,
    pub radius: f64,
    pub color: [f32; 3],
    pub within: Option<(Vec3, Vec3)>,
}

impl Light {
    /// A light out in the open.
    pub fn open(at: Vec3, radius: f64, color: [f32; 3]) -> Self {
        Self { at, radius, color, within: None }
    }

    /// How much of it reaches `p`, 0–1 (whichever way what's there faces):
    /// as the shader has it.
    pub fn reaches(&self, p: Vec3) -> f64 {
        let fade = (1.0 - (self.at - p).length() / self.radius.max(1e-6)).max(0.0);
        let inside = self.within.map_or(1.0, |(lo, hi)| {
            let q = (p - lo).min(hi - p);
            (q.x.min(q.y).min(q.z) / SPILL + 1.0).clamp(0.0, 1.0)
        });
        fade * fade * inside
    }
}

/// A light as the shaders have it.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub(super) struct GpuLight {
    pos_radius: [f32; 4],
    color: [f32; 4],
    lo: [f32; 4],
    hi: [f32; 4],
}

/// The lights of `all` that matter most to an eye at `eye` (the nearest,
/// by how far their reach is from it), as the shaders have them; and how
/// many.
pub(super) fn nearest(all: &[Light], eye: Vec3) -> ([GpuLight; MAX_LIGHTS], usize) {
    let mut by: Vec<(f64, &Light)> = all.iter().filter(|l| l.radius > 0.0 && l.color.iter().any(|c| *c > 0.0)).map(|l| ((l.at - eye).length() - l.radius, l)).collect();
    by.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut out = [GpuLight::default(); MAX_LIGHTS];
    let n = by.len().min(MAX_LIGHTS);
    for (slot, (_, l)) in out.iter_mut().zip(&by) {
        let v = |p: Vec3| [p.x as f32, p.y as f32, p.z as f32, 0.0];
        let (lo, hi) = l.within.unwrap_or((Vec3::splat(-1.0e9), Vec3::splat(1.0e9)));
        *slot = GpuLight { pos_radius: [l.at.x as f32, l.at.y as f32, l.at.z as f32, l.radius as f32], color: [l.color[0], l.color[1], l.color[2], 0.0], lo: v(lo), hi: v(hi) };
    }
    (out, n)
}

/// The light of `all` on something at `at`, whichever way it faces (the
/// arms, lit by what's about them).
pub(super) fn glow(all: &[Light], at: Vec3) -> [f32; 3] {
    let mut sum = [0.0f32; 3];
    for l in all {
        let k = l.reaches(at) as f32 * 0.6;
        for (s, c) in sum.iter_mut().zip(l.color) {
            *s += c * k;
        }
    }
    sum
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_light_fades_to_its_reach_and_a_lamp_stops_at_its_walls() {
        let lamp = Light { at: Vec3::new(0.0, 2.5, 0.0), radius: 8.0, color: [1.0; 3], within: Some((Vec3::new(-3.0, 0.0, -3.0), Vec3::new(3.0, 3.0, 3.0))) };
        assert_eq!(lamp.reaches(lamp.at), 1.0);
        let (near, far) = (lamp.reaches(Vec3::new(1.0, 0.0, 0.0)), lamp.reaches(Vec3::new(2.9, 0.0, 0.0)));
        assert!(near > far && far > 0.2, "{near} {far}");
        // Just past the wall, a little spills; half a metre on, none.
        assert!(lamp.reaches(Vec3::new(3.2, 1.0, 0.0)) > 0.0 && lamp.reaches(Vec3::new(3.6, 1.0, 0.0)) == 0.0);
        let fire = Light::open(Vec3::ZERO, 5.0, [1.0, 0.5, 0.1]);
        assert!(fire.reaches(Vec3::new(3.6, 1.0, 0.0)) > 0.0 && fire.reaches(Vec3::new(5.0, 0.0, 0.0)) == 0.0);
    }

    #[test]
    fn a_pane_is_lit_by_the_nearest_lights_and_no_more_than_it_can_be() {
        let many: Vec<Light> = (0..40).map(|i| Light::open(Vec3::new(f64::from(i) * 3.0, 0.0, 0.0), 2.0, [1.0, 0.0, 0.0])).collect();
        let (picked, n) = nearest(&many, Vec3::new(60.0, 0.0, 0.0));
        assert_eq!(n, MAX_LIGHTS);
        assert!(picked[..n].iter().all(|l| (l.pos_radius[0] - 60.0).abs() <= 36.0), "the nearest two dozen");
        assert_eq!(picked[0].pos_radius[0], 60.0);
        // One that's out isn't counted.
        assert_eq!(nearest(&[Light::open(Vec3::ZERO, 4.0, [0.0; 3])], Vec3::ZERO).1, 0);
        assert!(glow(&many, Vec3::new(3.0, 0.0, 0.0))[0] > 0.5);
    }
}
