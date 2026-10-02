//! A holdout's lamps, on power that's failing: one hung in every room
//! (more in the big ones), most of them lit, some flickering, some dead;
//! down in the bunker, red ones; and a few floods out on the walls. Each
//! lights its own room and no further (`render/lights.rs`). The start
//! room's never fail.

use bevy_ecs::prelude::*;
use lntrn_math::{Mat4, Vec3};

use super::arena::Arena;
use crate::render::{Light, MeshId, Vertex};
use crate::world::{Clock, Look, Model, OnMap, Placed};

/// A room's lamp, a bunker's, a flood's: its colour (linear) and how far
/// it reaches.
const ROOM: ([f32; 3], f64) = ([0.60, 0.50, 0.34], 9.0);
const BUNKER: ([f32; 3], f64) = ([0.62, 0.09, 0.06], 9.5);
const FLOOD: ([f32; 3], f64) = ([0.95, 0.58, 0.28], 14.0);
/// A lamp hung high (over a room two storeys tall) reaches this much
/// further, and is this much brighter.
const HIGH: (f64, f32) = (1.7, 1.7);
/// Of a room's lamps, the share that flicker and the share that are dead
/// (the rest are steady); of a bunker's and the floods', that flicker.
const FAILING: (f64, f64) = (0.28, 0.22);
const FAILING_RED: f64 = 0.3;
const FAILING_FLOOD: f64 = 0.4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mood {
    Steady,
    Flicker,
    Dead,
}

/// A lamp: its light at its brightest, how it's doing, and its own luck.
#[derive(Component, Clone, Copy, Debug)]
pub struct Lamp {
    pub light: Light,
    pub mood: Mood,
    seed: u32,
}

/// A number from `seed` and a count, 0–1.
fn chance(seed: u32, n: i64) -> f64 {
    let mut h = seed ^ (n as u32).wrapping_mul(0x9E37_79B9);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A_2D39);
    h ^= h >> 15;
    f64::from(h & 0xFFFF) / 65535.0
}

impl Lamp {
    /// A lamp that never fails, giving `light`.
    pub fn steady(light: Light) -> Self {
        Self { light, mood: Mood::Steady, seed: 1 }
    }

    /// How bright it is at `time`, 0–1: a failing one stutters, and now
    /// and then goes out for a few seconds.
    pub fn brightness(&self, time: f64) -> f64 {
        match self.mood {
            Mood::Steady => 1.0,
            Mood::Dead => 0.0,
            Mood::Flicker => {
                let own = f64::from(self.seed % 997) * 0.731;
                if chance(self.seed, (time * 0.3 + own).floor() as i64) < 0.2 {
                    return 0.0;
                }
                let beat = (time * 12.0 + own).floor() as i64;
                if chance(self.seed ^ 0x51ED, beat) < 0.2 { 0.2 } else { 0.8 + 0.2 * chance(self.seed ^ 0x77, beat) }
            }
        }
    }

    /// Its light at `time` (none: it's out).
    pub fn light(&self, time: f64) -> Option<Light> {
        let b = self.brightness(time) as f32;
        (b > 0.0).then(|| Light { color: self.light.color.map(|c| c * b), ..self.light })
    }
}

/// A lamp's fitting: a dark tray against the ceiling, a tube under it,
/// lit in `colour`.
fn fitting(colour: [f32; 3]) -> Vec<Vertex> {
    let tray = (Vec3::new(-0.38, 0.0, -0.11), Vec3::new(0.38, 0.05, 0.11), [0.16, 0.16, 0.17], Mat4::IDENTITY);
    let tube = (Vec3::new(-0.33, -0.05, -0.05), Vec3::new(0.33, 0.0, 0.05), [0.5, 0.5, 0.48], Mat4::IDENTITY);
    let mut v = super::props::boxes(&[tray, tube]);
    for corner in v.iter_mut().skip(36) {
        corner.emissive = colour.map(|c| c * 1.6);
    }
    v
}

/// Hang the arena's lamps, their fittings made with `mesh`.
pub fn spawn(world: &mut World, arena: &Arena, mut mesh: impl FnMut(&[Vertex]) -> MeshId) {
    let fittings = [ROOM, BUNKER, FLOOD].map(|(colour, _)| mesh(&fitting(colour)));
    for (i, l) in arena.lamps.iter().enumerate() {
        let seed = (i as u32 + 1).wrapping_mul(0x85EB_CA6B) | 1;
        let roll = chance(seed, 0);
        let (kind, mood) = match (l.room.is_some(), l.red) {
            (true, false) if l.start => (0, Mood::Steady),
            (true, false) if roll < FAILING.1 => (0, Mood::Dead),
            (true, false) if roll < FAILING.1 + FAILING.0 => (0, Mood::Flicker),
            (true, false) => (0, Mood::Steady),
            (true, true) => (1, if roll < FAILING_RED { Mood::Flicker } else { Mood::Steady }),
            (false, _) => (2, if roll < FAILING_FLOOD { Mood::Flicker } else { Mood::Steady }),
        };
        let (color, radius) = [ROOM, BUNKER, FLOOD][kind];
        let high = l.room.is_some_and(|(lo, hi)| hi.y - lo.y > 4.0);
        let (color, radius) = if high { (color.map(|c| c * HIGH.1), radius * HIGH.0) } else { (color, radius) };
        let light = Light { at: l.at - Vec3::new(0.0, 0.25, 0.0), radius, color, within: l.room };
        world.spawn((Lamp { light, mood, seed }, Placed(Mat4::from_translation(l.at)), Model(fittings[kind]), Look::default(), OnMap));
    }
}

/// Each lamp's fitting as bright as its lamp is.
pub fn flicker(clock: Res<Clock>, mut lamps: Query<(&Lamp, &mut Look)>) {
    for (lamp, mut look) in &mut lamps {
        look.emissive = lamp.brightness(clock.time) as f32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failing_lamp_stutters_and_goes_out_now_and_then_and_a_dead_one_never_lights() {
        let light = Light::open(Vec3::ZERO, 8.0, [1.0; 3]);
        let lamp = |mood| Lamp { light, mood, seed: 12345 };
        assert!((0..600).all(|k| lamp(Mood::Steady).brightness(f64::from(k) / 60.0) == 1.0));
        assert!(lamp(Mood::Dead).light(3.0).is_none());
        let seen: Vec<f64> = (0..3600).map(|k| lamp(Mood::Flicker).brightness(f64::from(k) / 60.0)).collect();
        let (out, dim, lit) = (seen.iter().filter(|b| **b == 0.0).count(), seen.iter().filter(|b| **b == 0.2).count(), seen.iter().filter(|b| **b >= 0.8).count());
        assert!(out > 200 && dim > 100 && lit > 1800, "a minute of it: {out} out, {dim} dim, {lit} lit");
        // The same again at the same time: it's no one's luck but its own.
        assert_eq!(lamp(Mood::Flicker).brightness(7.3), lamp(Mood::Flicker).brightness(7.3));
    }

    #[test]
    fn every_room_of_the_arena_has_its_lamp_and_the_start_rooms_never_fail() {
        let arena = super::super::tests::built().arena.expect("the arena");
        assert!(arena.lamps.len() > 40, "{} lamps", arena.lamps.len());
        for l in &arena.lamps {
            match l.room {
                // Hung inside its room, just under its ceiling.
                Some((lo, hi)) => assert!(l.at.x > lo.x && l.at.x < hi.x && l.at.z > lo.z && l.at.z < hi.z && l.at.y < hi.y && l.at.y > hi.y - 0.5, "{l:?}"),
                None => assert!(!l.red && !l.start && l.at.y > 2.5, "{l:?}"),
            }
            // The red ones are the bunker's, and all of the bunker's are red.
            assert_eq!(l.red, l.at.y < 0.0, "{l:?}");
        }
        assert!(arena.lamps.iter().filter(|l| l.red).count() >= 8 && arena.lamps.iter().filter(|l| l.room.is_none()).count() >= 6);
        let mut world = World::new();
        spawn(&mut world, &arena, |_| MeshId::placeholder());
        let hung: Vec<(Lamp, bool, bool)> = world.query::<&Lamp>().iter(&world).copied().zip(arena.lamps.iter().map(|l| (l.start, l.red))).map(|(lamp, (start, red))| (lamp, start, red)).collect();
        assert_eq!(hung.len(), arena.lamps.len());
        assert!(hung.iter().filter(|(_, start, _)| *start).count() >= 1);
        for (lamp, start, red) in &hung {
            assert!(!start || lamp.mood == Mood::Steady, "a start room's lamp failing");
            assert!(!red || lamp.mood != Mood::Dead, "a bunker's lamp dead");
            // It lights the floor under it, and nothing the other side of its ceiling.
            assert!(lamp.light.reaches(lamp.light.at - Vec3::new(0.0, 2.0, 0.0)) > 0.3);
            assert!(lamp.light.within.is_none_or(|_| lamp.light.reaches(lamp.light.at + Vec3::new(0.0, 1.5, 0.0)) == 0.0));
        }
        let failing = |mood| hung.iter().filter(|(l, _, _)| l.mood == mood).count();
        assert!(failing(Mood::Flicker) >= 8 && failing(Mood::Dead) >= 5 && failing(Mood::Steady) > hung.len() / 3, "{} flicker, {} dead of {}", failing(Mood::Flicker), failing(Mood::Dead), hung.len());
    }
}
