//! What flies, as drawn: a strafing run's plane coming over its strip (its
//! guns alight as they fire), and the gunship, its rotor turning, its
//! gun flashing in its door, its searchlight's shaft down on what it's
//! at.

use bevy_ecs::prelude::*;
use lntrn_math::{Mat4, Quat, Vec3};

use super::{Meshes, SEARCHLIGHT, zone};
use crate::render::{Draw, Renderer};
use crate::support::gunship::Gunship;
use crate::support::strafe::{LONG, Strafe};

/// The gunship: how wide its searchlight's shaft is and how thick; how
/// fast its rotor turns, how far it leans in to its circle, and where its
/// rotor's hub and its gun's muzzle are on it.
const SHAFT: (f64, f32) = (3.4, 0.2);
const SPINS: f64 = 31.0;
const LEANS: f64 = 0.16;
const HUB: Vec3 = Vec3::new(0.0, 1.85, 0.0);
const MUZZLE: Vec3 = Vec3::new(2.0, -0.45, 0.2);

/// Every strafing run called in: its strip, till its rounds have been
/// through it, and its plane coming over (its guns alight as they fire).
pub fn runs(world: &mut World, renderer: &mut Renderer, time: f64) {
    let (Some(m), Some(flames)) = (world.get_resource::<Meshes>().copied(), world.get_resource::<crate::throw::draw::Meshes>().copied()) else { return };
    let all: Vec<Strafe> = world.query::<&Strafe>().iter(world).copied().collect();
    for s in all {
        if s.threatens() {
            zone(world, renderer, None, &s.strip, time);
        }
        let (at, dir) = s.plane();
        // (Its nose is the model's -Z.)
        let placed = Mat4::from_translation(at) * Mat4::from_quat(Quat::from_rotation_y((-dir.x).atan2(-dir.z)));
        renderer.draw(Draw { mesh: m.plane, model: placed, emissive: 1.0, fog: 1.0, tint: [1.0; 3] });
        // A light at each wingtip, so it's seen against the night.
        for (side, mesh) in [(-6.9, m.port), (6.9, m.starboard)] {
            renderer.draw(Draw { mesh, model: placed * Mat4::from_translation(Vec3::new(side, 0.0, -0.5)) * Mat4::from_scale(Vec3::splat(0.35)), emissive: 1.0, fog: 0.3, tint: [1.0; 3] });
        }
        // (And one under its belly, blinking.)
        if (time * 2.5).fract() < 0.25 {
            renderer.draw(Draw { mesh: m.port, model: placed * Mat4::from_translation(Vec3::new(0.0, -0.85, 0.5)) * Mat4::from_scale(Vec3::splat(0.4)), emissive: 1.0, fog: 0.3, tint: [1.0; 3] });
        }
        let firing = s.front() > 0.0 && s.front() < LONG;
        if firing {
            let spit = 5.0 + 2.5 * (time * 90.0).sin().abs();
            renderer.draw(Draw { mesh: flames.flash, model: placed * Mat4::from_translation(Vec3::new(0.12, -0.62, -7.8)) * Mat4::from_quat(Quat::from_rotation_y(std::f64::consts::FRAC_PI_2)) * Mat4::from_scale(Vec3::splat(spit)), emissive: 1.0, fog: 0.5, tint: [1.0; 3] });
        }
    }
}

/// Where the gunship is, and how it's turned: its nose the way it flies,
/// leaning in to its circle while it's on it.
fn hovering(g: &Gunship) -> Mat4 {
    let (at, way) = g.place();
    let lean = if g.on_station() { -LEANS } else { 0.0 };
    Mat4::from_translation(at) * Mat4::from_quat(Quat::from_rotation_y((-way.x).atan2(-way.z)) * Quat::from_rotation_z(lean) * Quat::from_rotation_x(-0.07))
}

/// Where its searchlight's on: what it's shooting at, or (nothing to
/// shoot) sweeping the ground under its circle.
pub(super) fn searched(g: &Gunship, time: f64) -> Vec3 {
    g.lit.unwrap_or(g.over + Vec3::new((time * 0.6).sin() * g.out.0 * 0.5, 0.0, (time * 0.43).cos() * g.out.1 * 0.5)) + Vec3::new(0.0, 0.4, 0.0)
}

/// The gunship: its body and its turning rotor, its lights, its gun's
/// flash, and the shaft of its searchlight.
pub fn gunship(world: &mut World, renderer: &mut Renderer, time: f64) {
    let (Some(m), Some(flames)) = (world.get_resource::<Meshes>().copied(), world.get_resource::<crate::throw::draw::Meshes>().copied()) else { return };
    for g in world.query::<&Gunship>().iter(world) {
        let placed = hovering(g);
        // (Its own glow showing: it's seen against the night.)
        let lit = |mesh, model| Draw { mesh, model, emissive: 1.0, fog: 1.0, tint: [1.0; 3] };
        renderer.draw(lit(m.heli, placed));
        renderer.draw(lit(m.rotor, placed * Mat4::from_translation(HUB) * Mat4::from_quat(Quat::from_rotation_y(time * SPINS))));
        // A light on each skid, and one on its tail, blinking.
        let lamp = |mesh, at: Vec3, size: f64| Draw { mesh, model: placed * Mat4::from_translation(at) * Mat4::from_scale(Vec3::splat(size)), emissive: 1.0, fog: 0.3, tint: [1.0; 3] };
        renderer.draw(lamp(m.port, Vec3::new(-1.2, -1.7, -1.8), 0.22));
        renderer.draw(lamp(m.starboard, Vec3::new(1.2, -1.7, -1.8), 0.22));
        if (time * 1.8).fract() < 0.2 {
            renderer.draw(lamp(m.port, Vec3::new(0.0, 2.2, 7.9), 0.3));
        }
        if g.firing() {
            let spit = 2.2 + 1.2 * (time * 80.0).sin().abs();
            renderer.draw(Draw { mesh: flames.flash, model: placed * Mat4::from_translation(MUZZLE) * Mat4::from_quat(Quat::from_rotation_z(-0.35)) * Mat4::from_scale(Vec3::splat(spit)), emissive: 1.0, fog: 0.5, tint: [1.0; 3] });
        }
        // Its searchlight, once it's over the compound: a shaft from
        // under its nose down to where it's looking.
        if g.on_station() {
            let from = placed.transform_point(Vec3::new(0.0, -1.0, -2.6));
            let to = searched(g, time);
            // (Its point up at the lamp.)
            let Some(up) = (from - to).try_normalize() else { continue };
            let model = Mat4::from_translation((from + to) * 0.5) * Mat4::from_quat(Quat::from_rotation_arc(Vec3::Y, up)) * Mat4::from_scale(Vec3::new(SHAFT.0, (to - from).length(), SHAFT.0));
            renderer.draw_soft(Draw { mesh: m.shaft, model, emissive: 0.5, fog: 0.6, tint: SEARCHLIGHT }, SHAFT.1);
        }
    }
}
