//! A med station: an enamelled cabinet stood against a wall, one for each
//! stim (`stims.rs`), in that stim's colour: a lit cross on its panel, a
//! tube of the stuff either side, the tray an injector's taken from, and
//! a beacon on top to find it by. It stands there like something for sale
//! on the wall (`Wares::Stim`): a cabinet (solid, part of the map), and
//! over that its fittings and its glow.

use bevy_ecs::prelude::*;
use lntrn_math::{Mat4, Vec3};

use super::arena::{Arena, Wares};
use super::lamps::Lamp;
use super::machine::{alight, facing};
use super::props::boxes;
use super::stims::Stim;
use crate::collide::Surface;
use crate::map::building::shape::{Block, Rgb, Stuff};
use crate::render::{Light, MeshId, Vertex};
use crate::world::{Look, Model, OnMap, Placed};

/// The cabinet: how wide, how deep (out from the wall), how tall.
pub const WIDE: f64 = 0.9;
pub const DEEP: f64 = 0.42;
pub const TALL: f64 = 1.85;
/// Colours, as they look (sRGB): the cabinet's enamel, its panel, bare
/// steel.
const ENAMEL: Rgb = [0.60, 0.65, 0.60];
const PANEL: [f32; 3] = [0.09, 0.10, 0.10];
const STEEL: [f32; 3] = [0.44, 0.45, 0.44];
/// How far its glow reaches, and how bright it is.
const GLOW: (f64, f32) = (5.5, 0.85);

/// The cabinet standing against the wall at `wall` (on the wall's face,
/// `up` over the floor) facing `out`: a solid box.
pub fn cabinet(wall: Vec3, out: Vec3, up: f64) -> Block {
    let along = Vec3::new(-out.z, 0.0, out.x);
    let (p, q) = (wall - along * (WIDE * 0.5) - Vec3::new(0.0, up, 0.0), wall + along * (WIDE * 0.5) + out * DEEP + Vec3::new(0.0, TALL - up, 0.0));
    Block { lo: p.min(q), hi: p.max(q), colour: ENAMEL, stuff: Stuff::Solid(Surface::Metal) }
}

/// What's on the cabinet, about the middle of its front (`up` over the
/// floor), in `colour`: what's dull, and what's alight.
fn fittings(up: f64, colour: [f32; 3]) -> (Vec<Vertex>, Vec<Vertex>) {
    let b = |x0: f64, y0: f64, z0: f64, x1: f64, y1: f64, z1: f64, c: [f32; 3]| (Vec3::new(x0, y0, z0), Vec3::new(x1, y1, z1), c, Mat4::IDENTITY);
    let (top, foot) = (TALL - up, -up);
    let mut dull = vec![
        // The panel, and a rail under it.
        b(-0.39, -0.08, 0.0, 0.39, 0.34, 0.018, PANEL),
        b(-0.41, -0.13, 0.0, 0.41, -0.09, 0.035, STEEL),
        // The tray an injector's taken from, and its lip.
        b(-0.30, -0.50, 0.0, 0.30, -0.46, 0.17, STEEL),
        b(-0.30, -0.46, 0.15, 0.30, -0.42, 0.17, STEEL),
        // The tubes' caps, top and bottom.
        b(-0.36, 0.28, 0.018, -0.24, 0.31, 0.07, STEEL),
        b(-0.36, -0.05, 0.018, -0.24, -0.02, 0.07, STEEL),
        b(0.24, 0.28, 0.018, 0.36, 0.31, 0.07, STEEL),
        b(0.24, -0.05, 0.018, 0.36, -0.02, 0.07, STEEL),
        // The beacon's foot.
        b(-0.14, top, -0.32, 0.14, top + 0.03, -0.10, PANEL),
    ];
    // The vents down by its feet: slats.
    for k in 0..4 {
        let y = foot + 0.16 + 0.07 * f64::from(k);
        dull.push(b(-0.30, y, 0.0, 0.30, y + 0.03, 0.02, PANEL));
    }
    let lit = [
        // The cross.
        b(-0.05, 0.0, 0.018, 0.05, 0.28, 0.034, colour),
        b(-0.14, 0.09, 0.018, 0.14, 0.19, 0.034, colour),
        // A tube of it either side.
        b(-0.34, -0.02, 0.018, -0.26, 0.28, 0.06, colour),
        b(0.26, -0.02, 0.018, 0.34, 0.28, 0.06, colour),
        // The tray's back, alight; and the beacon.
        b(-0.28, -0.44, 0.0, 0.28, -0.20, 0.012, colour),
        b(-0.11, top + 0.03, -0.29, 0.11, top + 0.17, -0.13, colour),
    ];
    (boxes(&dull), alight(&lit, GLOW.1))
}

/// Set every med station's fittings down (wherever the arena has one),
/// its glow with them; their meshes made with `mesh`.
pub fn spawn(world: &mut World, arena: &Arena, up: f64, mut mesh: impl FnMut(&[Vertex]) -> MeshId) {
    let bright = Look { emissive: 1.0, fog: 0.8, tint: [1.0; 3] };
    let mut made: Vec<(Stim, MeshId, MeshId)> = Vec::new();
    for b in &arena.buys {
        let Wares::Stim(stim) = b.wares else { continue };
        let (dull, lit) = match made.iter().find(|m| m.0 == stim) {
            Some(&(_, dull, lit)) => (dull, lit),
            None => {
                let (dull, lit) = fittings(up, stim.colour());
                made.push((stim, mesh(&dull), mesh(&lit)));
                (made[made.len() - 1].1, made[made.len() - 1].2)
            }
        };
        let frame = facing(b.at, b.facing);
        world.spawn((Placed(frame), Model(dull), Look::default(), OnMap));
        // Its glow lights the room it's in, and no further.
        let at = b.at + b.facing * 0.5 + Vec3::new(0.0, 0.2, 0.0);
        let room = arena.lamps.iter().filter_map(|l| l.room).find(|(lo, hi)| at.x > lo.x && at.x < hi.x && at.y > lo.y && at.y < hi.y && at.z > lo.z && at.z < hi.z);
        let light = Light { at, radius: GLOW.0, color: stim.colour().map(|c| c * GLOW.1), within: room };
        world.spawn((Lamp::steady(light), Placed(frame), Model(lit), bright, OnMap));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cabinet_stands_on_the_floor_out_from_its_wall_its_fittings_on_its_front() {
        // Against a wall on x = 4, facing east, at a buy's height over a
        // floor at y = 0.2.
        let c = cabinet(Vec3::new(4.0, 0.2 + 1.45, 9.0), Vec3::new(1.0, 0.0, 0.0), 1.45);
        assert!((c.lo - Vec3::new(4.0, 0.2, 9.0 - WIDE * 0.5)).length() < 1e-9, "{:?}", c.lo);
        assert!((c.hi - Vec3::new(4.0 + DEEP, 0.2 + TALL, 9.0 + WIDE * 0.5)).length() < 1e-9, "{:?}", c.hi);
        for stim in super::super::stims::ALL {
            let (dull, lit) = fittings(1.45, stim.colour());
            assert!(!dull.is_empty() && lit.iter().all(|v| v.emissive.iter().any(|&e| e > 0.0)));
            // None of it behind the cabinet's front (but the beacon, on
            // top), nor wider than it, nor under the floor.
            let all = || dull.iter().chain(&lit);
            assert!(all().filter(|v| f64::from(v.pos[1]) < TALL - 1.45 - 0.01).all(|v| v.pos[2] >= 0.0));
            assert!(all().all(|v| f64::from(v.pos[0].abs()) <= WIDE * 0.5 && f64::from(v.pos[1]) >= -1.45));
        }
    }
}
