//! The world's light and air, a pane of the window and the camera whose
//! view fills it, and both as the shaders have them (a pane's globals).

use lntrn_core::bytes::Pod;
use lntrn_math::{Color, Mat4, Vec3};

use super::lights::{self, GpuLight, Light, MAX_LIGHTS};
use crate::camera::Camera;

/// The world's light and air.
#[derive(Clone, Copy, Debug)]
pub struct Atmosphere {
    /// The fog, which is also the sky at the horizon (sRGB).
    pub fog: Color,
    /// Fog per metre; about 1/density is where it gets thick.
    pub density: f64,
    /// The sky straight up (sRGB).
    pub zenith: Color,
    /// Towards the sun (at night, the moon).
    pub sun_dir: Vec3,
    pub sun: Color,
    pub ambient_sky: Color,
    pub ambient_ground: Color,
    /// How much shadows count, 0–1 (none are cast at 0: an overcast
    /// day's); the share of the sky's light that gets in under a roof;
    /// and how bright the stars and the moon are in the sky.
    pub shade: f64,
    pub indoor: f64,
    pub stars: f64,
}

/// A part of the window, and the camera whose view fills it; and how much
/// of the sky's light, and the sun's (or the moon's), reaches that eye
/// (for the arms: under a roof, less).
#[derive(Clone, Copy, Debug)]
pub struct Pane {
    pub camera: Camera,
    /// Where in the window, pixels: left, top, width, height.
    pub rect: [u32; 4],
    pub sky: f64,
    pub sun: f64,
}

impl Pane {
    /// How wide it is for its height.
    pub fn aspect(&self) -> f64 {
        f64::from(self.rect[2]) / f64::from(self.rect[3].max(1))
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct Globals {
    view_proj: [[f32; 4]; 4],
    camera: [f32; 4],
    cam_right: [f32; 4],
    cam_up: [f32; 4],
    cam_forward: [f32; 4],
    fog: [f32; 4],
    zenith: [f32; 4],
    sun_dir: [f32; 4],
    sun_color: [f32; 4],
    ambient_sky: [f32; 4],
    ambient_ground: [f32; 4],
    params: [f32; 4],
    moon: [[f32; 4]; 4],
    roof: [[f32; 4]; 4],
    shade: [f32; 4],
    lights: [GpuLight; MAX_LIGHTS],
}
// SAFETY: plain `f32`s.
unsafe impl Pod for Globals {}

pub(super) fn color4(c: Color, a: f64) -> [f32; 4] {
    let l = c.to_linear();
    [l.r as f32, l.g as f32, l.b as f32, a as f32]
}

pub(super) fn vec4(v: Vec3, w: f64) -> [f32; 4] {
    [v.x as f32, v.y as f32, v.z as f32, w as f32]
}

impl Globals {
    /// A pane's: its camera, the air, the lights nearest it of `lights`,
    /// and the shadow maps' views (`maps`: the moon's and the roofs', and
    /// a texel of the moon's; none: nothing's shadowed).
    pub fn of(pane: &Pane, air: &Atmosphere, time: f64, lights: &[Light], maps: Option<([Mat4; 2], f32)>) -> Self {
        let camera = &pane.camera;
        let aspect = pane.aspect();
        let (right, up, forward) = camera.basis();
        let half_h = (camera.fov_y * 0.5).tan();
        let (near, count) = lights::nearest(lights, camera.position);
        let ([moon, roof], texel) = maps.unwrap_or(([Mat4::IDENTITY; 2], 0.0));
        Self {
            view_proj: (camera.projection(aspect) * camera.view()).to_gpu(),
            camera: vec4(camera.position, 1.0),
            cam_right: vec4(right * (half_h * aspect), 0.0),
            cam_up: vec4(up * half_h, 0.0),
            cam_forward: vec4(forward, 0.0),
            fog: color4(air.fog, air.density),
            zenith: color4(air.zenith, 1.0),
            sun_dir: vec4(air.sun_dir.normalize(), 0.0),
            sun_color: color4(air.sun, 1.0),
            ambient_sky: color4(air.ambient_sky, 1.0),
            ambient_ground: color4(air.ambient_ground, 1.0),
            params: [time as f32, count as f32, 0.0, 0.0],
            moon: moon.to_gpu(),
            roof: roof.to_gpu(),
            shade: [if maps.is_some() { air.shade as f32 } else { 0.0 }, air.indoor as f32, air.stars as f32, texel],
            lights: near,
        }
    }
}
