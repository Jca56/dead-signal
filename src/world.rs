//! The game's state as an ECS: what is placed where, how it looks, and
//! the systems that move it along: some every frame, the simulation in
//! fixed steps (see [`crate::player::STEP`]) so it runs the same at any
//! frame rate.

use bevy_ecs::prelude::*;
use lntrn_math::{Mat4, Vec3};

use crate::assets::Prop;
use crate::collide::{Solids, Surface};
use crate::head::{self, View};
use crate::player::{self, Body, Controls, Player};
use crate::targets::{self, Kind, Target};
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

/// Everything solid: what bodies stand on and bump into.
#[derive(Resource, Clone, Debug, Default)]
pub struct Solid(pub Solids);

impl Ground {
    pub fn height_at(&self, x: f64, z: f64) -> Option<f64> {
        crate::assets::height_at(&self.0, x, z)
    }
}

/// What a scene object is made of, by its name.
fn surface_of(name: &str) -> Surface {
    match name {
        "Ground" => Surface::Dirt,
        "COL_Wood" | "SOLID_Ramps" | "SOLID_Crates" | "SOLID_DummyStands" => Surface::Wood,
        "COL_Metal" | "SOLID_Gap" | "SOLID_RangeFrames" => Surface::Metal,
        _ => Surface::Stone,
    }
}

/// Which kind of target a scene object is, by its name.
fn target_kind(name: &str) -> Option<Kind> {
    if name.starts_with("TARGET_Dummy") {
        Some(Kind::Dummy)
    } else if name.starts_with("TARGET_Plate") {
        Some(Kind::Plate)
    } else {
        None
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
    frame: Schedule,
    fixed: Schedule,
    /// Simulation time owed, seconds: what the fixed steps have yet to cover.
    owed: f64,
    /// Whether the simulation runs (a run that is not paused).
    pub simulating: bool,
}

impl Game {
    pub fn new() -> Self {
        let mut world = World::new();
        world.insert_resource(Clock::default());
        world.insert_resource(Ground::default());
        world.insert_resource(Solid::default());
        world.insert_resource(Controls::default());
        let mut frame = Schedule::default();
        let mut fixed = Schedule::default();
        frame.add_systems(blink);
        player::install(&mut fixed);
        head::install(&mut frame);
        targets::install(&mut frame);
        Self { world, frame, fixed, owed: 0.0, simulating: false }
    }

    /// Put a player on the ground at `(x, z)`, facing `yaw`, in place of
    /// any there was.
    pub fn spawn_player(&mut self, x: f64, z: f64, yaw: f64) {
        self.despawn_player();
        let y = self.ground().height_at(x, z).unwrap_or(0.0);
        self.world.spawn((Player, Body::at(Vec3::new(x, y, z)), View::facing(yaw)));
        *self.world.resource_mut::<Controls>() = Controls::default();
        self.owed = 0.0;
    }

    pub fn despawn_player(&mut self) {
        let players: Vec<Entity> = self.world.query_filtered::<Entity, With<Player>>().iter(&self.world).collect();
        for e in players {
            self.world.despawn(e);
        }
    }

    /// The player's body and view, if one is about.
    pub fn player(&mut self) -> Option<(Body, View)> {
        self.world.query_filtered::<(&Body, &View), With<Player>>().iter(&self.world).next().map(|(b, v)| (*b, *v))
    }

    /// The player's view, to turn it.
    pub fn player_view_mut(&mut self) -> Option<Mut<'_, View>> {
        self.world.query_filtered::<&mut View, With<Player>>().iter_mut(&mut self.world).next()
    }

    pub fn controls_mut(&mut self) -> Mut<'_, Controls> {
        self.world.resource_mut::<Controls>()
    }

    /// How far between its last two steps the simulation is, 0–1: what a
    /// frame drawn now should blend by.
    pub fn alpha(&self) -> f64 {
        (self.owed / player::STEP).clamp(0.0, 1.0)
    }

    /// Put a model file's objects in the world. By name: `Ground` is the
    /// terrain (drawn and solid), `SOLID_*` are drawn and solid, `COL_*` are
    /// only solid, and `Beacon` blinks.
    pub fn spawn_props(&mut self, props: Vec<Prop>) {
        for p in props {
            let name = p.name.as_str();
            if name == "Ground" || name.starts_with("SOLID_") || name.starts_with("COL_") {
                self.world.resource_mut::<Solid>().0.add_as(&p.triangles, surface_of(name));
            }
            if name == "Ground" {
                self.world.resource_mut::<Ground>().0.extend(p.triangles.iter().copied());
            }
            let Some(mesh) = p.mesh else { continue };
            let mut e = self.world.spawn((Placed(p.model), Model(mesh), Look::default()));
            if let Some(kind) = target_kind(name) {
                e.insert(Target::new(kind, p.model));
            }
            if name == "Beacon" {
                // A failing light: two quick flashes, then a long dark.
                e.insert((Blink { period: 3.2, lit: vec![(0.0, 0.18), (0.42, 0.55)] }, Look { emissive: 1.0, fog: 0.35 }));
            }
        }
    }

    /// How many solid triangles the world has.
    pub fn solid_count(&self) -> usize {
        self.world.resource::<Solid>().0.len()
    }

    /// Advance the clock to `now`, take the fixed steps owed by then (while
    /// simulating), and run the every-frame systems.
    pub fn tick(&mut self, now: f64) {
        {
            let mut clock = self.world.resource_mut::<Clock>();
            clock.dt = (now - clock.time).clamp(0.0, 0.1);
            clock.time = now;
        }
        if self.simulating {
            self.owed += self.clock().dt;
            while self.owed >= player::STEP {
                self.fixed.run(&mut self.world);
                self.owed -= player::STEP;
            }
        }
        self.frame.run(&mut self.world);
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
