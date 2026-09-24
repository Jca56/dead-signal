//! The dead, a step at a time: each thinks (as often as its distance from
//! the player allows) and moves, the near ones keep out of each other's
//! way; and each frame, each is posed for drawing and hitting, and the
//! long dead are buried.

use bevy_ecs::prelude::*;
use lntrn_math::{Mat4, Quat, Vec2, Vec3};

use super::*;
use crate::player::{self, Body, Player, RADIUS, STEP};
use crate::world::{Blend, Solid};

#[allow(clippy::too_many_arguments)]
pub(super) fn think(
    mut dead: Query<(&mut Zombie, &mut Body, &mut Beat), Without<Player>>,
    players: Query<&Body, With<Player>>,
    solid: Res<Solid>,
    nav: Res<Nav>,
    stealth: Option<Res<Stealth>>,
    cheats: Option<Res<crate::dev::Cheats>>,
    mut noises: ResMut<Noises>,
    mut horde: ResMut<Horde>,
) {
    let player = players.iter().next().map(|b| b.pos);
    horde.tick = horde.tick.wrapping_add(1);
    let tick = horde.tick;
    let new = std::mem::take(&mut *noises);
    let fresh = |at: u32| tick.wrapping_sub(at) < FAR_EVERY;
    horde.shots.retain(|(at, _, _)| fresh(*at));
    horde.snarls.retain(|(at, _)| fresh(*at));
    for n in new.shots {
        horde.heard += 1;
        let id = horde.heard;
        horde.shots.push((tick, id, n));
    }
    horde.snarls.extend(new.snarls.into_iter().map(|n| (tick, n)));
    let shots: Vec<(u32, Vec3, f64)> = horde.shots.iter().map(|&(_, id, (at, range))| (id, at, range)).collect();
    let snarls: Vec<(Vec3, Vec3)> = horde.snarls.iter().map(|(_, n)| *n).collect();
    let searches = std::cell::Cell::new(SEARCHES);
    let lures = horde.lures.clone();
    let senses = Senses { lures: &lures, solids: &solid.0, nav: nav.0.as_ref(), player, noises: &shots, alerts: &snarls, searches: &searches, sight: if cheats.is_some_and(|c| c.ignored) { 0.0 } else { stealth.map_or(1.0, |s| s.0) } };
    for (mut z, mut body, mut beat) in &mut dead {
        // Far off, it steps less often, and further each time. (What's
        // heard is kept as long as the farthest go between steps.)
        let far = player.map_or(0.0, |p| Vec2::new(body.pos.x - p.x, body.pos.z - p.z).length());
        let n = every(far);
        if !(tick + beat.phase).is_multiple_of(n) {
            beat.since += 1;
            continue;
        }
        *beat = Beat { every: n, since: 0, ..*beat };
        let dt = STEP * f64::from(n);
        let mut intent = z.think(&body, &senses, dt);
        let voice = body.pos + Vec3::new(0.0, 1.5, 0.0);
        horde.sounds.extend(intent.sounds.drain(..).map(|(sfx, gain)| (sfx, voice, gain)));
        if let Some(blow) = intent.hit {
            horde.blows.push(blow);
        }
        if let Some(seen) = intent.alert {
            noises.snarls.push((body.pos, seen));
        }
        if let Some(throw) = intent.spit {
            horde.spits.push(throw);
        }
        if intent.burst {
            horde.bursting.push(body.pos);
            horde.sounds.push((Sfx::Burst, body.pos + Vec3::new(0.0, 0.6, 0.0), 1.0));
        }
        if z.dead() {
            body.prev = body.pos;
            continue;
        }
        let before = body.pos;
        let gait = z.gait;
        player::step_body(&mut body, z.yaw, &mut intent.controls, &solid.0, &gait, dt);
        // Kept out of the player's way.
        if let Some(p) = player {
            let off = Vec2::new(body.pos.x - p.x, body.pos.z - p.z);
            let d = off.length();
            if d < PERSONAL && d > 1e-6 {
                let out = off * ((PERSONAL - d) / d);
                body.pos.x += out.x;
                body.pos.z += out.y;
            }
        }
        z.walked += Vec2::new(body.pos.x - before.x, body.pos.z - before.z).length();
    }
    elbow(&mut dead);
}

/// The living dead near the player nudge each other apart (a shove, so
/// walls still stop them), and a crowd spreads round the player instead
/// of stacking.
fn elbow(dead: &mut Query<(&mut Zombie, &mut Body, &mut Beat), Without<Player>>) {
    let at: Vec<Option<Vec3>> = dead.iter().map(|(z, b, beat)| (!z.dead() && beat.every == 1).then_some(b.pos)).collect();
    let apart = RADIUS * 2.0;
    let mut nudges = vec![Vec3::ZERO; at.len()];
    for i in 0..at.len() {
        for j in i + 1..at.len() {
            let (Some(a), Some(b)) = (at[i], at[j]) else { continue };
            if (a.y - b.y).abs() > 1.5 {
                continue;
            }
            let off = Vec3::new(a.x - b.x, 0.0, a.z - b.z);
            let d = off.length();
            if d >= apart {
                continue;
            }
            // Exactly on top of each other: any way will do, as long as
            // it's opposite ways.
            let dir = if d > 1e-6 { off * (1.0 / d) } else { Vec3::new(1.0, 0.0, 0.0) };
            let nudge = dir * ((apart - d) * ELBOW);
            nudges[i] += nudge;
            nudges[j] -= nudge;
        }
    }
    for ((_, mut body, _), nudge) in dead.iter_mut().zip(nudges) {
        if nudge != Vec3::ZERO {
            body.push += nudge;
        }
    }
}

/// Pose each one for this frame: its animation, where it stands between
/// its steps, and sinking once it has lain long enough. Lost in the fog,
/// it isn't; far off, only now and then.
/// One of the dead as it's posed: what it's doing, where, its beat, its
/// figure, and how it looks.
type Posed<'a> = (&'a Zombie, &'a Body, &'a Beat, &'a mut Figure, Option<&'a Looks>);

pub(super) fn pose(model: Option<Res<Model>>, blend: Res<Blend>, players: Query<&Body, With<Player>>, mut frames: Local<u32>, mut dead: Query<Posed, Without<Player>>) {
    let Some(model) = model else { return };
    let player = players.iter().next().map(|b| b.pos);
    *frames = frames.wrapping_add(1);
    for (z, body, beat, mut figure, looks) in &mut dead {
        let far = player.map_or(0.0, |p| Vec2::new(body.pos.x - p.x, body.pos.z - p.z).length());
        if far > UNSEEN {
            figure.hide();
            continue;
        }
        if far > MID && !figure.joints.is_empty() && !(*frames + beat.phase).is_multiple_of(POSE_FAR_EVERY) {
            continue;
        }
        let (clip, t) = z.clip();
        let (joints, points) = model.pose(clip, t);
        // Between the step it took and the next it will, over as many of
        // the world's steps as it lets go by.
        let along = ((f64::from(beat.since) + blend.0) / f64::from(beat.every)).min(1.0);
        let at = body.prev + (body.pos - body.prev) * along;
        let sink = match z.state {
            State::Dead { t } => ((t - LIE_FOR) / SINK_FOR).clamp(0.0, 1.0) * 1.8,
            _ => 0.0,
        };
        let (height, mut bulk) = looks.map_or((1.0, 1.0), |l| (l.height, l.bulk));
        // A dead Spitter swells, shuddering, and once it's burst, it's gone.
        if z.kind == Kind::Spitter
            && let State::Dead { t } = z.state
        {
            if t >= spit::BURST_AT {
                figure.hide();
                continue;
            }
            let k = t / spit::BURST_AT;
            bulk *= 1.0 + 0.45 * k * k + 0.05 * k * (t * 30.0).sin();
        }
        let place = Mat4::from_translation(at - Vec3::new(0.0, sink, 0.0)) * Mat4::from_quat(Quat::from_rotation_y(z.yaw)) * Mat4::from_scale(Vec3::new(bulk, height, bulk));
        if figure.parts.is_empty() {
            figure.parts = model.meshes(looks.unwrap_or(&Looks::default()));
        }
        figure.set(place, joints, points, !z.dead());
    }
}

/// The long dead, gone.
pub(super) fn bury(mut commands: Commands, dead: Query<(Entity, &Zombie)>, mut horde: ResMut<Horde>) {
    for (e, z) in &dead {
        if matches!(z.state, State::Dead { t } if t > LIE_FOR + SINK_FOR) {
            commands.entity(e).despawn();
            horde.gone += 1;
        }
    }
}

