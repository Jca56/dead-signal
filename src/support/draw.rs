//! What's called down, as drawn: a flare turning end over end in the air,
//! then lying where it stopped with its red flame and the smoke going up
//! from it; a crate swinging down under its parachute; and the crate
//! down, its lid off, the parachute fallen in a heap beside it; a strip
//! of ground marked for a strafing run, outlined in red (while it's being
//! placed, in that player's pane; once it's called in, for everyone to
//! keep out of), and the plane that comes over it; the gunship, its
//! rotor turning, its gun flashing in its door, its searchlight's shaft
//! down on what it's at. And what they light of a night.

use bevy_ecs::prelude::*;
use lntrn_core::log_error;
use lntrn_math::{Mat4, Quat, Vec3};

use super::gunship::Gunship;
use super::strafe::{LONG, Strafe, Strip, WIDE};
use super::{Drop, Flare, SETTLES, STANDS};
use crate::render::{Draw, Light, MeshId, Renderer, Vertex};
use crate::world::Solid;

/// A flare's flame: how big, in the air and burning on the ground.
const FLAME: (f64, f64) = (0.45, 0.8);
/// Its light, and how far it reaches.
const GLOW: [f32; 3] = [1.0, 0.10, 0.12];
const REACH: f64 = 11.0;
/// Its smoke: how many puffs there are in the column at once, how high
/// it goes, how long a puff takes to get there, and how thick it is.
const PUFFS: usize = 14;
const COLUMN: f64 = 12.0;
const RISES: f64 = 7.0;
const SMOKE: ([f32; 3], f32) = ([0.85, 0.16, 0.14], 0.3);
/// How high the crate's top is (where its cords meet).
const CRATE_TOP: f64 = 0.6;

/// A marked strip's outline: how far apart its bars are along its sides,
/// how thick they are, how far over the ground, and its red (and what it
/// is where the strip can't be: under a roof).
const BARS: f64 = 1.5;
const BAR: (f64, f64) = (0.16, 0.07);
const OVER: f64 = 0.06;
const MARKED: [f32; 3] = [1.0, 0.08, 0.06];
const REFUSED: [f32; 3] = [0.40, 0.38, 0.36];
const PORT: [f32; 3] = [1.0, 0.1, 0.1];
const STARBOARD: [f32; 3] = [0.1, 1.0, 0.2];
/// The gunship: its searchlight's colour, how wide its shaft is and how
/// thick, how far its spot lights the ground; how fast its rotor turns,
/// how far it leans in to its circle, and where its rotor's hub and its
/// gun's muzzle are on it.
const SEARCHLIGHT: [f32; 3] = [0.75, 0.83, 1.0];
const SHAFT: (f64, f32) = (3.4, 0.2);
/// How much of its own colour what flies shows whatever the light (it's
/// seen against the night).
const SEEN: f32 = 0.6;
const SPOT: f64 = 10.0;
const SPINS: f64 = 31.0;
const LEANS: f64 = 0.16;
const HUB: Vec3 = Vec3::new(0.0, 1.85, 0.0);
const MUZZLE: Vec3 = Vec3::new(2.0, -0.45, 0.2);

/// What they look like (`airdrop.glb`), a ball for the smoke, a flare's
/// red flame, and glowing blocks: a strip's outline (red; grey where it
/// can't be), and a plane's lights.
#[derive(Resource, Clone, Copy)]
pub struct Meshes {
    flare: MeshId,
    shut: MeshId,
    open: MeshId,
    chute: MeshId,
    plane: MeshId,
    heli: MeshId,
    rotor: MeshId,
    puff: MeshId,
    /// A searchlight's shaft: a cone alight, its point at the lamp.
    shaft: MeshId,
    flame: MeshId,
    marked: MeshId,
    refused: MeshId,
    port: MeshId,
    starboard: MeshId,
}

/// Load what they look like, for `world`'s runs.
pub fn load(renderer: &mut Renderer, world: &mut World) {
    match crate::assets::load(renderer, "airdrop") {
        Ok(props) => {
            let find = |n: &str| props.iter().find(|p| p.name == n).and_then(|p| p.mesh);
            // (What flies, a little alight with its own colour.)
            let mut flying = |n: &str| {
                let p = props.iter().find(|p| p.name == n)?;
                let seen: Vec<Vertex> = p.vertices.iter().map(|v| Vertex { emissive: [v.color[0] * SEEN, v.color[1] * SEEN, v.color[2] * SEEN], ..*v }).collect();
                Some(renderer.add_mesh(&seen))
            };
            let (plane, heli, rotor) = (flying("Plane"), flying("Heli"), flying("Rotor"));
            match (find("Flare"), find("Crate"), find("Open"), find("Chute"), plane, heli, rotor) {
                (Some(flare), Some(shut), Some(open), Some(chute), Some(plane), Some(heli), Some(rotor)) => {
                    let puff = renderer.add_mesh(&crate::motes::ball());
                    let shaft = renderer.add_mesh(&cone());
                    // (What glows is its own colour, whatever it's tinted:
                    // a block of each.)
                    let mut block = |glow: [f32; 3]| renderer.add_mesh(&crate::motes::speck([1.0; 3], glow, glow));
                    let (marked, refused, port, starboard) = (block(MARKED), block(REFUSED), block(PORT), block(STARBOARD));
                    // A fire's flame, made a flare's: red, its heart
                    // white-hot.
                    let Some(flame) = crate::assets::load(renderer, "effects").ok().and_then(|fx| {
                        let burning = fx.iter().find(|p| p.name == "Flame")?;
                        let red: Vec<Vertex> = burning.vertices.iter().map(|v| Vertex { emissive: if v.color[1] > 0.7 { [1.0, 0.8, 0.75] } else { [v.color[0], v.color[1] * 0.3, v.color[1] * 0.45 + 0.05] }, ..*v }).collect();
                        Some(renderer.add_mesh(&red))
                    }) else {
                        log_error!("airdrop: no Flame to make a flare's of");
                        return;
                    };
                    world.insert_resource(Meshes { flare, shut, open, chute, plane, heli, rotor, puff, shaft, flame, marked, refused, port, starboard });
                }
                _ => log_error!("airdrop: no Flare, Crate, Open, Chute, Plane, Heli or Rotor"),
            }
        }
        Err(e) => log_error!("airdrop: {e}"),
    }
}

/// A cone a metre long and a metre across its foot, its point up at the
/// top, alight, its sides smooth all round: a shaft of light, drawn as
/// long and as wide as it is.
fn cone() -> Vec<Vertex> {
    let round = 18u32;
    let at = |k: u32| {
        let a = std::f64::consts::TAU * f64::from(k) / f64::from(round);
        (a.cos() as f32, a.sin() as f32)
    };
    let v = |pos: [f32; 3], (x, z): (f32, f32)| Vertex { pos, normal: [x, 0.0, z], color: [1.0; 4], emissive: SEARCHLIGHT };
    let mut out = Vec::new();
    for k in 0..round {
        let (a, b) = (at(k), at(k + 1));
        out.extend([v([0.0, 0.5, 0.0], a), v([b.0 * 0.5, -0.5, b.1 * 0.5], b), v([a.0 * 0.5, -0.5, a.1 * 0.5], a)]);
    }
    out
}

/// A flame's flutter at `time`, about 1.
fn flutter(time: f64, own: f64) -> f64 {
    1.0 + 0.25 * (time * 31.0 + own * 5.3).sin() + 0.15 * (time * 57.0 + own * 2.1).sin()
}

/// How a crate swings under its parachute, `age` into its fall: a turn
/// about where its cords meet the canopy.
fn swing(age: f64) -> Quat {
    Quat::from_rotation_z(0.11 * (age * 1.4).sin()) * Quat::from_rotation_x(0.07 * (age * 1.1 + 1.0).sin())
}

/// Queue everything called down for drawing, `alpha` between the last two
/// steps, at `time`.
pub fn draw(world: &mut World, renderer: &mut Renderer, alpha: f64, time: f64) {
    let Some(m) = world.get_resource::<Meshes>().copied() else { return };
    let lit = |mesh, model| Draw { mesh, model, emissive: 0.0, fog: 1.0, tint: [1.0; 3] };
    for f in world.query::<&Flare>().iter(world) {
        let at = f.prev + (f.pos - f.prev) * alpha;
        let own = f.pos.x * 1.7 + f.pos.z;
        // In the air it tumbles; at rest it lies flat, its tip burning.
        let (turn, tip, size) = match f.rest {
            None => {
                let turn = Quat::from_rotation_y(f.age * 3.0) * Quat::from_rotation_x(f.spin);
                (turn, at + turn * Vec3::new(0.0, 0.22, 0.0), FLAME.0)
            }
            Some(_) => {
                let turn = Quat::from_rotation_y(own) * Quat::from_rotation_x(std::f64::consts::FRAC_PI_2 * 0.93);
                (turn, at + turn * Vec3::new(0.0, 0.22, 0.0), FLAME.1)
            }
        };
        renderer.draw(lit(m.flare, Mat4::from_translation(at) * Mat4::from_quat(turn)));
        let high = size * flutter(time, own);
        let flame = Mat4::from_translation(tip - Vec3::new(0.0, 0.03, 0.0)) * Mat4::from_quat(Quat::from_rotation_y(time * 5.0 + own)) * Mat4::from_scale(Vec3::new(size * 0.8, high, size * 0.8));
        renderer.draw(Draw { mesh: m.flame, model: flame, emissive: 1.0, fog: 1.0, tint: [1.0; 3] });
        // The smoke, once it's settled: puffs going up one after another,
        // each swelling, leaning with the air, and thinning as it climbs.
        if let Some(rest) = f.rest {
            for k in 0..PUFFS {
                let share = (time / RISES + k as f64 / PUFFS as f64).fract();
                // (None older than the flare's been burning.)
                if share * RISES > rest {
                    continue;
                }
                // (Each a little off the last, and its own size: no stack
                // of beads.)
                let off = Vec3::new((own + k as f64 * 2.4).sin(), 0.0, (own * 1.3 + k as f64 * 1.7).cos()) * (0.12 + 0.5 * share);
                let lean = Vec3::new(0.9, 0.0, 0.3) * (share * share * 2.2) + off;
                let wide = (0.45 + 2.3 * share) * (0.8 + 0.4 * ((k * 5) % 7) as f64 / 6.0);
                let model = Mat4::from_translation(tip + Vec3::new(0.0, 0.3 + COLUMN * share, 0.0) + lean) * Mat4::from_scale(Vec3::new(wide, wide * 1.15, wide));
                let thin = (1.0 - share).powf(1.3) * (share * 8.0).min(1.0);
                renderer.draw_soft(Draw { mesh: m.puff, model, emissive: 0.25, fog: 1.0, tint: SMOKE.0 }, SMOKE.1 * thin as f32);
            }
        }
    }
    for d in world.query::<&Drop>().iter(world) {
        match d.down {
            None => {
                let height = d.prev + (d.height - d.prev) * alpha;
                // Swung about where the cords meet the canopy, well over it.
                let pivot = Vec3::new(0.0, CRATE_TOP + 3.4, 0.0);
                let hung = Mat4::from_translation(d.at + Vec3::new(0.0, height, 0.0) + pivot) * Mat4::from_quat(swing(d.age)) * Mat4::from_translation(-pivot);
                let turned = hung * Mat4::from_quat(Quat::from_rotation_y(d.at.x + d.age * 0.15));
                renderer.draw(lit(m.shut, turned));
                renderer.draw(lit(m.chute, turned * Mat4::from_translation(Vec3::new(0.0, CRATE_TOP, 0.0))));
            }
            Some(t) => {
                // Down: open, and at the last sinking away; its parachute
                // falling in a heap to one side, then gone.
                let sunk = ((t - (STANDS - 1.5)) / 1.5).clamp(0.0, 1.0);
                let stood = Mat4::from_translation(d.at - Vec3::new(0.0, sunk * 0.7, 0.0)) * Mat4::from_quat(Quat::from_rotation_y(d.at.x + d.age * 0.15));
                renderer.draw(lit(m.open, stood));
                // (It comes down beside the crate, flat and gathered in,
                // and after a while sinks away.)
                let fall = (t / SETTLES).clamp(0.0, 1.0);
                if t < SETTLES + 6.0 {
                    let (heap, drawn) = (1.0 - 0.95 * fall * fall, 1.0 - 0.35 * fall);
                    let gone = ((t - SETTLES - 4.0) / 2.0).clamp(0.0, 1.0);
                    let model = Mat4::from_translation(d.at + Vec3::new(1.9 * fall, CRATE_TOP * (1.0 - fall) - 0.12 * fall - gone * 0.5, 1.1 * fall)) * Mat4::from_scale(Vec3::new(drawn, heap, drawn));
                    renderer.draw(lit(m.chute, model));
                }
            }
        }
    }
}

/// A strip's outline, on the ground it lies over, at `time`: in `pane`
/// only (it's being placed), or for everyone (none: it's called in, and
/// pulses).
pub fn zone(world: &World, renderer: &mut Renderer, pane: Option<usize>, strip: &Strip, time: f64) {
    let Some(m) = world.get_resource::<Meshes>().copied() else { return };
    let solid = &world.resource::<Solid>().0;
    // (Grey where it can't be; called in, it pulses.)
    let mesh = if strip.open { m.marked } else { m.refused };
    let glows = if pane.is_none() { 0.45 + 0.55 * (time * 9.0).sin().abs() } else { 1.0 };
    // A bar from one point of it to the next, each on the ground there.
    let mut bar = |from: (f64, f64), to: (f64, f64)| {
        let (Some(a), Some(b)) = (strip.meets(solid, from.0, from.1), strip.meets(solid, to.0, to.1)) else { return };
        let (a, b) = (a + Vec3::new(0.0, OVER, 0.0), b + Vec3::new(0.0, OVER, 0.0));
        let Some(along) = (b - a).try_normalize() else { return };
        let turn = Quat::from_rotation_arc(Vec3::Z, along);
        let d = Draw { mesh, model: Mat4::from_translation((a + b) * 0.5) * Mat4::from_quat(turn) * Mat4::from_scale(Vec3::new(BAR.0, BAR.1, (b - a).length())), emissive: glows as f32, fog: 0.4, tint: [1.0; 3] };
        match pane {
            Some(p) => renderer.draw_in(p, d),
            None => renderer.draw(d),
        }
    };
    let (half, steps) = (WIDE * 0.5, (LONG / BARS).round() as usize);
    for k in 0..steps {
        let (a, b) = (LONG * k as f64 / steps as f64, LONG * (k + 1) as f64 / steps as f64);
        for side in [-half, half] {
            bar((a, side), (b, side));
        }
        // (Down its middle, a dash every other step: the way it's raked.)
        if k % 2 == 0 {
            bar((a + 0.2, 0.0), (b - 0.2, 0.0));
        }
    }
    for end in [0.0, LONG] {
        for k in 0..4 {
            bar((end, -half + WIDE * k as f64 / 4.0), (end, -half + WIDE * (k + 1) as f64 / 4.0));
        }
    }
}

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
fn spot(g: &Gunship, time: f64) -> Vec3 {
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
            let to = spot(g, time);
            // (Its point up at the lamp.)
            let Some(up) = (from - to).try_normalize() else { continue };
            let model = Mat4::from_translation((from + to) * 0.5) * Mat4::from_quat(Quat::from_rotation_arc(Vec3::Y, up)) * Mat4::from_scale(Vec3::new(SHAFT.0, (to - from).length(), SHAFT.0));
            renderer.draw_soft(Draw { mesh: m.shaft, model, emissive: 0.5, fog: 0.6, tint: SEARCHLIGHT }, SHAFT.1);
        }
    }
}

/// What they light at `time`, to `renderer`: a flare's red.
pub fn lights(world: &mut World, renderer: &mut Renderer, time: f64) {
    for f in world.query::<&Flare>().iter(world) {
        let k = 1.5 * (0.8 + 0.2 * flutter(time, f.pos.x + f.pos.z));
        renderer.light(Light::open(f.pos + Vec3::new(0.0, 0.35, 0.0), REACH, GLOW.map(|c| c * k as f32)));
    }
    // The gunship: its searchlight's spot on the ground, the glow of its
    // own cabin (so it's seen), and its gun's flash.
    for g in world.query::<&Gunship>().iter(world) {
        let (at, _) = g.place();
        renderer.light(Light::open(at - Vec3::new(0.0, 1.5, 0.0), 9.0, [0.45, 0.5, 0.55]));
        if g.on_station() {
            renderer.light(Light::open(spot(g, time) + Vec3::new(0.0, 1.2, 0.0), SPOT, SEARCHLIGHT.map(|c| c * 1.7)));
        }
        if g.firing() {
            let k = 1.0 + (time * 80.0).sin().abs();
            renderer.light(Light::open(g.gun(), 7.0, [1.0, 0.7, 0.35].map(|c| c * k as f32)));
        }
    }
    // A strafing run's rounds, where they're landing: a strobe of them.
    for s in world.query::<&Strafe>().iter(world) {
        if s.front() > 0.0 && s.front() < LONG {
            let k = 1.5 + 1.5 * (time * 70.0).sin().abs();
            renderer.light(Light::open(s.strip.point(s.front(), 0.0) + Vec3::new(0.0, 1.0, 0.0), 12.0, [1.0, 0.75, 0.4].map(|c| c * k as f32)));
        }
    }
}
