//! The players as the others see them, playing together: a figure on
//! each, walking as they walk, running as they run, down on the ground
//! when they're down. For now it's the dead's own rig and clips, dressed as
//! the living: a hoodie in the player's colour, a pack, nothing torn. A
//! player's own pane never shows their own (see `render/figures.rs`).

use bevy_ecs::prelude::*;
use lntrn_math::{Mat4, Quat, Vec3};

use crate::head::View;
use crate::player::{Body, Fallen, Player};
use crate::style;
use crate::world::{Blend, Clock};
use crate::zombie::figure::{Clip, Figure, Model};
use crate::zombie::looks::{Legs, Looks, Sleeve, Top};

/// Ground covered for one time through the walk, and the run, and how long
/// each clip is.
const STRIDE: f64 = 1.4;
const WALK_CLIP: f64 = 0.8;
const RUN_STRIDE: f64 = 3.4;
const RUN_CLIP: f64 = 0.5;
/// Past this pace it's a run, under this standing still, m/s.
const RUNS_FROM: f64 = 2.5;
const STILL: f64 = 0.3;
/// Down, time's kept this far into the fall (the dead's own), past its end:
/// the clip holds its last, on the ground.
const FALL_FOR: f64 = 5.0;

/// How far through their walk a player's figure is (ground covered), and
/// how long it's been idle, or down.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct Stride {
    walked: f64,
    idle: f64,
    down: f64,
}

/// How player `seat` is dressed: as the living, in their colour.
pub fn looks(seat: usize) -> Looks {
    let c = style::player(seat);
    Looks {
        top: Top::Hoodie,
        sleeve: Sleeve::Long,
        legs: Legs::Trousers,
        pack: true,
        colors: [[0.80, 0.62, 0.50], [c.r as f32, c.g as f32, c.b as f32], [0.18, 0.20, 0.26], [0.20, 0.13, 0.08], [0.85, 0.85, 0.80]],
        ..Looks::default()
    }
}

/// Every player given a figure, to be seen by the others.
pub fn dress(world: &mut World) {
    let bare: Vec<(Entity, usize)> = world.query_filtered::<(Entity, &Player), Without<Figure>>().iter(world).map(|(e, p)| (e, p.0)).collect();
    for (e, seat) in bare {
        let looks = looks(seat);
        world.entity_mut(e).insert((Figure::of(&looks), looks, Stride::default()));
    }
}

/// A player's figure as it moves: its clip, and how far in.
fn clip(stride: &mut Stride, body: &Body, fallen: bool, dt: f64) -> (Clip, f64) {
    let pace = body.speed_flat();
    stride.walked += pace * dt;
    if fallen {
        stride.down = (stride.down + dt).min(FALL_FOR);
        return (Clip::Death, stride.down);
    }
    stride.down = 0.0;
    if pace > RUNS_FROM {
        (Clip::Run, stride.walked / RUN_STRIDE * RUN_CLIP)
    } else if pace > STILL {
        (Clip::Walk, stride.walked / STRIDE * WALK_CLIP)
    } else {
        stride.idle += dt;
        (Clip::Idle, stride.idle)
    }
}

/// A figure on a player, as they are this frame.
type Posed<'a> = (&'a Body, &'a View, Option<&'a Fallen>, &'a mut Figure, &'a Looks, &'a mut Stride);

/// Pose each player's figure for this frame: where they stand between
/// their steps, which way they face, what they're doing. (Nothing shoots
/// at them: they aren't solid.)
fn pose(model: Option<Res<Model>>, blend: Res<Blend>, clock: Res<Clock>, mut players: Query<Posed, With<Player>>) {
    let Some(model) = model else { return };
    for (body, view, fallen, mut figure, looks, mut stride) in &mut players {
        let (clip, t) = clip(&mut stride, body, fallen.is_some(), clock.dt);
        let (joints, points) = model.pose(clip, t);
        let at = body.prev + (body.pos - body.prev) * blend.0;
        let place = Mat4::from_translation(at) * Mat4::from_quat(Quat::from_rotation_y(view.yaw)) * Mat4::from_scale(Vec3::new(looks.bulk, looks.height, looks.bulk));
        if figure.parts.is_empty() {
            figure.parts = model.meshes(looks);
        }
        figure.set(place, joints, points, false);
    }
}

/// Add the figures' posing to the every-frame schedule.
pub fn install(frame: &mut Schedule) {
    frame.add_systems(pose);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_idles_walks_runs_and_goes_down_as_they_do() {
        let mut stride = Stride::default();
        let mut body = Body::at(Vec3::ZERO);
        assert_eq!(clip(&mut stride, &body, false, 0.1).0, Clip::Idle);
        body.vel = Vec3::new(0.0, 0.0, -1.5);
        assert_eq!(clip(&mut stride, &body, false, 0.1).0, Clip::Walk);
        body.vel = Vec3::new(0.0, 0.0, -6.0);
        assert_eq!(clip(&mut stride, &body, false, 0.1).0, Clip::Run);
        for _ in 0..60 {
            clip(&mut stride, &body, true, 0.1);
        }
        assert_eq!(clip(&mut stride, &body, true, 0.1), (Clip::Death, FALL_FOR), "down, and stays down");
        let c = style::player(1);
        assert_eq!(looks(1).colors[1], [c.r as f32, c.g as f32, c.b as f32], "in their colour");
    }
}
