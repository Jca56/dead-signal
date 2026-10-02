//! A pellet (or a round) or a blow, along its ray: what it meets first
//! takes it (one of the dead, and on through into the next if it goes
//! through; a target; a wall), with the chips, the sound and the
//! hitmarker of it. The glass on its way breaks; an amplified gun's shot
//! leaves its streak behind it.

use lntrn_math::Vec3;

use super::{Aim, Combat, Heard, Hit, Met};
use crate::collide::Surface;
use crate::fx::Marker;
use crate::sound::Sfx;
use crate::stats::Stats;
use crate::targets::{self, Kind, Target};
use crate::world::{Game, Solid};
use crate::zombie::figure::Zone;
use crate::zombie;

impl Combat {
    /// `aim`'s direction thrown up to `degrees` off true.
    pub(super) fn scatter(&mut self, aim: &Aim, degrees: f64) -> Vec3 {
        if degrees <= 0.0 {
            return aim.dir;
        }
        let r = degrees.to_radians() * self.rand().sqrt();
        let a = self.rand() * std::f64::consts::TAU;
        (aim.dir + aim.right * (r * a.cos()) + aim.up * (r * a.sin())).normalize()
    }

    /// A pellet (or round) or a blow along `dir`: whatever it meets first
    /// takes `hit` (a pellet's punch fading with how far it flew). What it
    /// met.
    pub(super) fn strike(&mut self, game: &mut Game, aim: &Aim, dir: Vec3, hit: &Hit, heard: &mut Heard, stats: &mut Stats) -> Met {
        let wall = game.world.resource::<Solid>().0.raycast(aim.eye, dir, hit.reach);
        let reach = wall.map_or(hit.reach, |h| h.t);
        let dead = zombie::raycast_past(&mut game.world, aim.eye, dir, reach, &heard.struck);
        let reach = dead.map_or(reach, |(_, t, _)| t);
        let target = targets::raycast(&mut game.world, aim.eye, dir, reach);
        // The glass on the way breaks: as far as the first thing it stops
        // in (what goes on through one of the dead goes on to the wall).
        let stops = target.map(|(_, t, _)| t).or_else(|| dead.filter(|_| hit.pierce.is_empty()).map(|(_, t, _)| t));
        let flew = stops.unwrap_or(wall.map_or(hit.reach, |h| h.t));
        crate::glass::shot(&mut game.world, &mut self.fx, aim.eye, dir, flew);
        // (An amplified round's streak: from by the muzzle to where it
        // stopped.)
        if hit.amplified > 0 {
            self.fx.streak(aim.eye + aim.right * 0.14 - aim.up * 0.12 + dir * 0.7, aim.eye + dir * flew, hit.amplified);
        }
        let punch = |t: f64| hit.damage * hit.falloff.map_or(1.0, |f| f.at(t));
        if target.is_none()
            && let Some(first) = dead
        {
            // Through one and on into the next, weaker, as far as the
            // round goes (to the wall, if there is one).
            let (mut next, mut shares, mut share) = (Some(first), hit.pierce.iter(), 1.0);
            while let Some((e, t, zone)) = next {
                let (head, limb) = (zone == Zone::Head, zone == Zone::Limb);
                let point = aim.eye + dir * t;
                // Close enough to hurt in full, a blast staggers.
                let close = hit.falloff.is_none_or(|f| t <= f.near);
                // Plate turns it, with a spark and a clang.
                let plated = zombie::plated(&game.world, e, dir, limb);
                let impact = zombie::Impact { damage: punch(t) * share, head, limb, blow: hit.blow, shove: hit.shove, stumble: hit.stumble && close, takedown: hit.takedown, fire: false, at: Some(point) };
                if let Some(mut z) = game.world.get_mut::<zombie::brain::Zombie>(e) {
                    z.by = Some(aim.seat);
                }
                let killed = zombie::hurt(&mut game.world, e, dir, aim.eye, impact);
                stats.damage_dealt += zombie::brain::dealt(impact.damage, head, hit.blow);
                if killed {
                    if hit.blow {
                        stats.melee_kills += 1
                    } else {
                        stats.gun_kills += 1
                    }
                    stats.headshot_kills += u32::from(head && !hit.blow);
                    stats.longest_kill = stats.longest_kill.max(t);
                    self.drop_something(game, e);
                }
                self.fx.burst(point, -dir, if plated { Surface::Metal } else { Surface::Flesh }, if hit.blow { 12 } else { 9 });
                self.arms[aim.seat].marker = Some(Marker::of(killed, head && !hit.blow));
                heard.confirm(&self.sound, killed);
                if plated {
                    self.sound.play_at(Sfx::Clank, 0.9, point, aim.eye, aim.right, false);
                } else if heard.thud() {
                    self.sound.play_at(Sfx::Flesh, 1.0, point, aim.eye, aim.right, false);
                }
                if hit.blow
                    && let Some(mut v) = game.player_view_mut(aim.seat)
                {
                    v.jolt(0.02);
                }
                heard.struck.push(e);
                let Some(&s) = shares.next() else { break };
                share = s;
                next = zombie::raycast_past(&mut game.world, aim.eye, dir, wall.map_or(hit.reach, |h| h.t), &heard.struck);
            }
            return Met { something: true, target: true, head: first.2 == Zone::Head };
        }
        if let Some((e, t, head)) = target {
            let point = aim.eye + dir * t;
            let damage = punch(t);
            let Some((beaten, kind)) = game.world.get_mut::<Target>(e).map(|mut target| (target.hit(dir, point, damage, head), target.kind)) else { return Met::default() };
            match kind {
                Kind::Dummy => stats.dummies_downed += u32::from(beaten),
                Kind::Plate => stats.plates_rung += 1,
            }
            let (surface, sfx, gain) = match kind {
                Kind::Dummy => (Surface::Wood, Sfx::HitWood, 0.9),
                Kind::Plate => (Surface::Metal, Sfx::Ding, 1.0),
            };
            self.fx.burst(point, -dir, surface, if hit.blow { 10 } else { 7 });
            self.arms[aim.seat].marker = Some(Marker::of(beaten, head && !hit.blow));
            heard.confirm(&self.sound, beaten);
            if heard.thud() {
                self.sound.play_at(sfx, gain, point, aim.eye, aim.right, false);
            }
            if hit.blow
                && let Some(mut v) = game.player_view_mut(aim.seat)
            {
                v.jolt(0.02);
            }
            return Met { something: true, target: true, head };
        }
        let Some(wall) = wall else { return Met::default() };
        self.fx.burst(wall.point, wall.normal, wall.surface, if hit.blow { 8 } else { 6 });
        let (sfx, gain) = match wall.surface {
            Surface::Dirt => (Sfx::HitDirt, 0.8),
            Surface::Wood => (Sfx::HitWood, 0.8),
            Surface::Stone => (Sfx::HitStone, 0.8),
            Surface::Metal => (Sfx::Ding, 0.35),
            Surface::Flesh => (Sfx::Flesh, 0.8),
            Surface::Bile => (Sfx::Splat, 0.8),
        };
        if heard.thud() {
            self.sound.play_at(sfx, gain, wall.point, aim.eye, aim.right, false);
        }
        if hit.blow
            && let Some(mut v) = game.player_view_mut(aim.seat)
        {
            v.jolt(0.012);
        }
        Met { something: true, target: false, head: false }
    }
}
