//! The game's state as an ECS: what is placed where, how it looks, and
//! the systems that move it along each frame.

use bevy_ecs::prelude::*;
use lntrn_math::{Mat4, Vec3};

use crate::assets::Prop;
use crate::render::MeshId;

/// Where a thing is.
#[derive(Component, Clone, Copy, Debug)]
pub struct Placed(pub Mat4);

/// What a thing looks like.
#[derive(Component, Clone, Copy, Debug)]
pub struct Model(pub MeshId);

/// How it shows: its own light, and how much fog swallows it.
#[derive(Component, Clone, Copy, Debug)]
pub struct Look {
    pub emissive: f32,
    pub fog: f32,
}

impl Default for Look {
    fn default() -> Self {
        Self { emissive: 1.0, fog: 1.0 }
    }
}

/// A light that stutters on and off: lit for each `(start, end)` span of
/// seconds within every `period`.
#[derive(Component, Clone, Debug)]
pub struct Blink {
    pub period: f64,
    pub lit: Vec<(f64, f64)>,
}

/// The game clock.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct Clock {
    /// Seconds since the game started.
    pub time: f64,
    /// Seconds since the last frame (capped, so a stall doesn't lurch).
    pub dt: f64,
}

/// What can be stood on, in world space.
#[derive(Resource, Clone, Debug, Default)]
pub struct Ground(pub Vec<[Vec3; 3]>);

impl Ground {
    pub fn height_at(&self, x: f64, z: f64) -> Option<f64> {
        crate::assets::height_at(&self.0, x, z)
    }
}

/// How dim a blinking light is between flashes.
const BLINK_OFF: f32 = 0.04;

fn blink(clock: Res<Clock>, mut lights: Query<(&Blink, &mut Look)>) {
    for (b, mut look) in &mut lights {
        let t = clock.time.rem_euclid(b.period);
        let on = b.lit.iter().any(|&(start, end)| t >= start && t < end);
        look.emissive = if on { 1.0 } else { BLINK_OFF };
    }
}

pub struct Game {
    pub world: World,
    schedule: Schedule,
}

impl Game {
    pub fn new() -> Self {
        let mut world = World::new();
        world.insert_resource(Clock::default());
        world.insert_resource(Ground::default());
        let mut schedule = Schedule::default();
        schedule.add_systems(blink);
        Self { world, schedule }
    }

    /// Put a model file's objects in the world. The object named `Ground`
    /// is what things stand on; `Beacon` blinks.
    pub fn spawn_props(&mut self, props: Vec<Prop>) {
        for p in props {
            let mut e = self.world.spawn((Placed(p.model), Model(p.mesh), Look::default()));
            match p.name.as_str() {
                "Ground" => {
                    self.world.resource_mut::<Ground>().0.extend(p.triangles);
                }
                "Beacon" => {
                    // A failing light: two quick flashes, then a long dark.
                    e.insert((Blink { period: 3.2, lit: vec![(0.0, 0.18), (0.42, 0.55)] }, Look { emissive: 1.0, fog: 0.35 }));
                }
                _ => {}
            }
        }
    }

    /// Advance the clock to `now` and run every system once.
    pub fn tick(&mut self, now: f64) {
        {
            let mut clock = self.world.resource_mut::<Clock>();
            clock.dt = (now - clock.time).clamp(0.0, 0.1);
            clock.time = now;
        }
        self.schedule.run(&mut self.world);
    }

    pub fn clock(&self) -> Clock {
        *self.world.resource::<Clock>()
    }

    pub fn ground(&self) -> &Ground {
        self.world.resource::<Ground>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_blink_follows_its_pattern() {
        let mut g = Game::new();
        let e = g.world.spawn((Blink { period: 2.0, lit: vec![(0.0, 0.5)] }, Look::default())).id();
        g.tick(0.25);
        assert_eq!(g.world.get::<Look>(e).unwrap().emissive, 1.0);
        g.tick(1.0);
        assert_eq!(g.world.get::<Look>(e).unwrap().emissive, BLINK_OFF);
        g.tick(2.1);
        assert_eq!(g.world.get::<Look>(e).unwrap().emissive, 1.0, "the pattern repeats");
        assert!((g.clock().dt - 0.1).abs() < 1e-12, "a long frame is capped");
    }
}
