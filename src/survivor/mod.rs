//! The players as the others see them, playing together: a survivor on
//! each (the first in a cap and a field jacket, the second with a
//! ponytail, in a hoodie; each in their own colour), moving as they move
//! and holding what they hold, where they aim. Its legs and body
//! (`body.rs`), what its hands do (`hold.rs`, with a gun `gun.rs`), all on
//! its rig (`rig.rs`). A player's own pane never shows their own figure
//! (`render/figures.rs`), nor what's in their hands (`draw`).

mod body;
mod gun;
mod hold;
mod rig;
#[cfg(test)]
mod tests;

use bevy_ecs::prelude::*;
use lntrn_math::{Mat4, Quat, Vec3};

use crate::head::View;
use crate::loot::Kind;
use crate::player::{Body, Player};
use crate::render::{Draw, PALETTE, Renderer};
use crate::style;
use crate::throw::Throwable;
use crate::weapon::Hands;
use crate::world::{Blend, Clock};
use crate::zombie::figure::Figure;
pub use rig::Rig;

/// How a survivor looks: the parts of `survivor.glb` they're put together
/// from, and each region's colour (skin, top, bottoms, accent, under; sRGB,
/// as authored).
#[derive(Component, Clone, Debug, PartialEq)]
pub struct Outfit {
    pub parts: &'static [&'static str],
    pub colors: [[f32; 3]; PALETTE],
}

const FIRST: [&str; 8] = ["head_a", "cap", "torso_jacket", "arm.L", "arm.R", "legs_cargo", "belt", "ruck"];
const SECOND: [&str; 7] = ["head_b", "hair_b", "torso_hoodie", "arm.L", "arm.R", "legs_jeans", "daypack"];

/// How player `seat` looks: the first (and third) in a cap and a field
/// jacket, the second (and fourth) in a hoodie, what's in their colour
/// theirs.
pub fn outfit(seat: usize) -> Outfit {
    let c = style::player(seat);
    let theirs = [c.r as f32, c.g as f32, c.b as f32];
    if seat.is_multiple_of(2) {
        Outfit { parts: &FIRST, colors: [[0.78, 0.58, 0.45], theirs, [0.36, 0.34, 0.26], [0.22, 0.15, 0.10], [0.62, 0.62, 0.60]] }
    } else {
        Outfit { parts: &SECOND, colors: [[0.60, 0.42, 0.31], theirs, [0.25, 0.32, 0.46], [0.10, 0.08, 0.07], [0.90, 0.88, 0.84]] }
    }
}

impl Outfit {
    /// Each region's colour, linear, to draw.
    pub fn palette(&self) -> [[f32; 3]; PALETTE] {
        self.colors.map(|c| c.map(crate::zombie::looks::linear))
    }
}

/// Where a survivor's pose is at, kept frame to frame.
#[derive(Component, Clone, Debug)]
pub struct Motion {
    /// Seconds all told (the loops' clock); through the stride (0–1); the
    /// pace (m/s), crouch (0–1) and the legs' turn from the body (radians),
    /// each easing to what it is.
    time: f64,
    phase: f64,
    pace: f64,
    crouch: f64,
    legs: f64,
    /// Seconds in the air, how tucked (0–1), and since landing.
    air: f64,
    aloft: f64,
    landed: f64,
    /// Seconds down, and bled out.
    down: f64,
    out: f64,
    /// Which way of being they're in, how long's left of fading from the
    /// last, and the pose it fades from.
    state: body::State,
    fading: f64,
    last: rig::Pose,
    /// How far into carrying a gun at a sprint (0–1), and winding up a
    /// throw (s).
    sprint: f64,
    wind: f64,
    /// A blow under way, how far in (s); and whether it's a jab (fists go
    /// a right, then a left).
    blow: Option<f64>,
    jab: bool,
}

impl Default for Motion {
    fn default() -> Self {
        Self { time: 0.0, phase: 0.0, pace: 0.0, crouch: 0.0, legs: 0.0, air: 0.0, aloft: 0.0, landed: f64::INFINITY, down: 0.0, out: 0.0, state: body::State::Up, fading: 0.0, last: Vec::new(), sprint: 0.0, wind: 0.0, blow: None, jab: false }
    }
}

/// What a player's doing this frame, as their seat has it.
pub struct Doing<'a> {
    pub seat: usize,
    pub hands: &'a Hands,
    /// A throw being wound up, and one just let go (how long ago, s).
    pub winding: Option<Throwable>,
    pub threw: Option<(Throwable, f64)>,
    /// How far the gun's lowered (patching up, rummaging), 0–1.
    pub lowered: f64,
    /// Picking someone up; down; bled out.
    pub reviving: bool,
    pub down: bool,
    pub out: bool,
}

/// What's in a player's hands, and a shot's flash, where (the world's
/// space): for the others to see.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct Held {
    pub item: Option<(Kind, Mat4)>,
    pub flash: Option<Mat4>,
}

/// Every player given a figure, to be seen by the others.
pub fn dress(world: &mut World) {
    let bare: Vec<(Entity, usize)> = world.query_filtered::<(Entity, &Player), Without<Figure>>().iter(world).map(|(e, p)| (e, p.0)).collect();
    for (e, seat) in bare {
        world.entity_mut(e).insert((Figure::default(), outfit(seat), Motion::default(), Held::default()));
    }
}

/// A figure on a player, as they are this frame.
type Posed<'a> = (&'a Player, &'a Body, &'a View, &'a Outfit, &'a mut Motion, &'a mut Figure, &'a mut Held);

/// Pose each player's figure for this frame, as `doing` says they are:
/// where they stand between their steps, which way they face, how they
/// move, what's in their hands and where they aim it. (Nothing shoots at
/// them: they aren't solid.)
pub fn pose(world: &mut World, doing: &[Doing]) {
    if !world.contains_resource::<Rig>() {
        return;
    }
    world.resource_scope(|world, rig: Mut<Rig>| {
        let dt = world.resource::<Clock>().dt;
        let blend = world.resource::<Blend>().0;
        let mut players = world.query::<Posed>();
        for (player, body, view, outfit, mut m, mut figure, mut held) in players.iter_mut(world) {
            let Some(d) = doing.iter().find(|d| d.seat == player.0) else { continue };
            let (s, c) = view.yaw.sin_cos();
            let (ahead, right) = (Vec3::new(-s, 0.0, -c), Vec3::new(c, 0.0, -s));
            let now = body::Now { going: Vec3::new(body.vel.dot(right), 0.0, body.vel.dot(ahead)), grounded: body.grounded, airborne: body.airborne, crouched: body.crouched, down: d.down, out: d.out, kneeling: d.reviving };
            let mut pose = body::pose(&rig, &mut m, &now, dt);
            let (yaw, pitch) = view.aim();
            let holding = hold::hands(&rig, &mut pose, &mut m, d, (yaw - view.yaw, pitch), body.sprinting, dt);
            let at = body.prev + (body.pos - body.prev) * blend;
            let place = Mat4::from_translation(at) * Mat4::from_quat(Quat::from_rotation_y(view.yaw));
            if figure.parts.is_empty() {
                figure.parts = rig.meshes(outfit.parts);
            }
            figure.joints = rig.joints(&rig.world(&pose));
            figure.model = place;
            figure.solid = false;
            *held = Held { item: holding.item.map(|(kind, at)| (kind, place * at)), flash: holding.flash.map(|at| place * at) };
        }
    });
}

/// Queue what's in each player's hands (and a shot's flash) for the panes
/// that don't look through their eyes (`eyes`: whose each pane's are).
pub fn draw(world: &mut World, renderer: &mut Renderer, eyes: &[usize]) {
    let held: Vec<(usize, Held)> = world.query::<(&Player, &Held)>().iter(world).map(|(p, h)| (p.0, *h)).collect();
    let Some(items) = world.get_resource::<crate::items::Meshes>() else { return };
    let flash = world.get_resource::<crate::throw::draw::Meshes>().map(|m| m.flash);
    for (seat, held) in held {
        for pane in eyes.iter().enumerate().filter(|&(_, &e)| e != seat).map(|(pane, _)| pane) {
            if let Some((kind, model)) = held.item
                && let Some(&mesh) = items.0.get(&kind)
            {
                renderer.draw_in(pane, Draw { mesh, model, emissive: 0.0, fog: 1.0, tint: [1.0; 3] });
            }
            if let (Some(model), Some(mesh)) = (held.flash, flash) {
                renderer.draw_in(pane, Draw { mesh, model, emissive: 1.0, fog: 1.0, tint: [1.0; 3] });
            }
        }
    }
}
