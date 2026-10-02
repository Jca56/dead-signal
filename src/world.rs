//! The game's state as an ECS: what is placed where, how it looks, and
//! the systems that move it along: some every frame, the simulation in
//! fixed steps (see [`crate::player::STEP`]) so it runs the same at any
//! frame rate.

use bevy_ecs::prelude::*;
use lntrn_math::{Mat4, Vec3};

use crate::assets::Prop;
use crate::collide::{Solids, Surface};
use crate::head::{self, View};
use crate::player::{self, Body, Controls, Fallen, Load, Player};
use crate::targets::{self, Kind, Target};
use crate::zombie;
use crate::render::MeshId;

/// Where a thing is.
#[derive(Component, Clone, Copy, Debug)]
pub struct Placed(pub Mat4);

/// What a thing looks like.
#[derive(Component, Clone, Copy, Debug)]
pub struct Model(pub MeshId);

/// How it shows: its own light, how much fog swallows it, and a shade
/// its colours are multiplied by (white leaves them).
#[derive(Component, Clone, Copy, Debug)]
pub struct Look {
    pub emissive: f32,
    pub fog: f32,
    pub tint: [f32; 3],
}

impl Default for Look {
    fn default() -> Self {
        Self { emissive: 1.0, fog: 1.0, tint: [1.0; 3] }
    }
}

/// The ball a thing lies within, for leaving it undrawn when it's out of
/// view or lost in the fog.
#[derive(Component, Clone, Copy, Debug)]
pub struct Bounds {
    pub centre: Vec3,
    pub radius: f64,
}

/// Part of the map a run is played on: gone with it.
#[derive(Component, Clone, Copy, Debug)]
pub struct OnMap;

/// Part of the title's scene: put away while a run is on.
#[derive(Component, Clone, Copy, Debug)]
pub struct OnTitle;

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

/// How far between its last two fixed steps the simulation is (0–1), for
/// drawing things that move in steps.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct Blend(pub f64);

/// The lie of the land: the title scene's ground as triangles, or a
/// map's as its grid of heights.
#[derive(Resource, Clone, Debug)]
pub enum Ground {
    Tris(Vec<[Vec3; 3]>),
    Field(std::sync::Arc<crate::map::terrain::Field>),
}

impl Default for Ground {
    fn default() -> Self {
        Ground::Tris(Vec::new())
    }
}

/// Everything solid: what bodies stand on and bump into.
#[derive(Resource, Clone, Debug, Default)]
pub struct Solid(pub Solids);

impl Ground {
    pub fn height_at(&self, x: f64, z: f64) -> Option<f64> {
        match self {
            Ground::Tris(tris) => crate::assets::height_at(tris, x, z),
            Ground::Field(field) => field.height_at(x, z),
        }
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
        world.insert_resource(Blend::default());
        world.insert_resource(zombie::Nav::default());
        world.insert_resource(zombie::Noises::default());
        world.insert_resource(zombie::Heat::default());
        world.insert_resource(crate::dev::Cheats::default());
        world.insert_resource(crate::throw::Booms::default());
        world.insert_resource(zombie::Stealth::default());
        world.insert_resource(zombie::Horde::default());
        let mut frame = Schedule::default();
        let mut fixed = Schedule::default();
        frame.add_systems(blink);
        player::install(&mut fixed);
        head::install(&mut frame);
        targets::install(&mut frame);
        zombie::install(&mut fixed, &mut frame);
        Self { world, frame, fixed, owed: 0.0, simulating: false }
    }

    /// Put player `seat` on the ground at `(x, z)`, facing `yaw`, in place
    /// of any in that seat.
    pub fn spawn_player(&mut self, seat: usize, x: f64, z: f64, yaw: f64) {
        if let Some(e) = self.player_entity(seat) {
            self.world.despawn(e);
        }
        let y = self.ground().height_at(x, z).unwrap_or(0.0);
        self.world.spawn((Player(seat), Body::at(Vec3::new(x, y, z)), View::facing(yaw), Controls::default(), Load::default()));
        self.owed = 0.0;
    }

    /// Every player gone.
    pub fn despawn_players(&mut self) {
        let players: Vec<Entity> = self.world.query_filtered::<Entity, With<Player>>().iter(&self.world).collect();
        for e in players {
            self.world.despawn(e);
        }
    }

    fn player_entity(&mut self, seat: usize) -> Option<Entity> {
        self.world.query::<(Entity, &Player)>().iter(&self.world).find(|(_, p)| p.0 == seat).map(|(e, _)| e)
    }

    /// Player `seat`'s body and view, if they're about.
    pub fn player(&mut self, seat: usize) -> Option<(Body, View)> {
        let e = self.player_entity(seat)?;
        Some((*self.world.get::<Body>(e)?, *self.world.get::<View>(e)?))
    }

    /// Every player about: their seat, body and view, the first first.
    pub fn players(&mut self) -> Vec<(usize, Body, View)> {
        let mut all: Vec<(usize, Body, View)> = self.world.query::<(&Player, &Body, &View)>().iter(&self.world).map(|(p, b, v)| (p.0, *b, *v)).collect();
        all.sort_by_key(|(seat, _, _)| *seat);
        all
    }

    /// Put player `seat` at `at`, as they are (the dev's teleport).
    pub fn teleport(&mut self, seat: usize, at: Vec3) {
        if let Some(e) = self.player_entity(seat)
            && let Some(mut body) = self.world.get_mut::<Body>(e)
        {
            body.pos = at;
            body.prev = at;
            body.vel = Vec3::ZERO;
        }
    }

    /// Shove player `seat` (a blow landing).
    pub fn push_player(&mut self, seat: usize, push: Vec3) {
        if let Some(e) = self.player_entity(seat)
            && let Some(mut body) = self.world.get_mut::<Body>(e)
        {
            body.push += push;
        }
    }

    /// Player `seat`'s view, to turn it.
    pub fn player_view_mut(&mut self, seat: usize) -> Option<Mut<'_, View>> {
        let e = self.player_entity(seat)?;
        self.world.get_mut::<View>(e)
    }

    /// What player `seat` is doing with the keys.
    pub fn controls_mut(&mut self, seat: usize) -> Option<Mut<'_, Controls>> {
        let e = self.player_entity(seat)?;
        self.world.get_mut::<Controls>(e)
    }

    /// Every player's keys let go (paused, or the run over).
    pub fn release_controls(&mut self) {
        for mut c in self.world.query::<&mut Controls>().iter_mut(&mut self.world) {
            *c = Controls::default();
        }
    }

    /// Player `seat` fallen (down, or out) or up again.
    pub fn set_fallen(&mut self, seat: usize, fallen: bool) {
        let Some(e) = self.player_entity(seat) else { return };
        if fallen {
            self.world.entity_mut(e).insert(Fallen);
        } else {
            self.world.entity_mut(e).remove::<Fallen>();
        }
    }

    /// How much of a sprint player `seat`'s gear leaves them.
    pub fn set_load(&mut self, seat: usize, share: f64) {
        if let Some(e) = self.player_entity(seat)
            && let Some(mut load) = self.world.get_mut::<Load>(e)
        {
            load.0 = share;
        }
    }

    /// How far between its last two steps the simulation is, 0–1: what a
    /// frame drawn now should blend by.
    pub fn alpha(&self) -> f64 {
        (self.owed / player::STEP).clamp(0.0, 1.0)
    }

    /// Put the title scene's objects in the world (in place of any map):
    /// its ground is the ground, and its beacon blinks. By name: `Ground`
    /// is the terrain (drawn and solid), `SOLID_*` are drawn and solid,
    /// `COL_*` are only solid.
    pub fn spawn_title(&mut self, props: &[Prop]) {
        self.clear_map();
        self.hide_title();
        let mut solids = crate::collide::Solids::default();
        let mut ground = Vec::new();
        for p in props {
            let name = p.name.as_str();
            if name == "Ground" || name.starts_with("SOLID_") || name.starts_with("COL_") {
                solids.add_as(&p.triangles, surface_of(name));
            }
            if name == "Ground" {
                ground.extend(p.triangles.iter().copied());
            }
            let Some(mesh) = p.mesh else { continue };
            let mut e = self.world.spawn((Placed(p.model), Model(mesh), Look::default(), OnTitle));
            if let Some(kind) = target_kind(name) {
                e.insert(Target::new(kind, p.model));
            }
            if name == "Beacon" {
                // A failing light: two quick flashes, then a long dark.
                e.insert((Blink { period: 3.2, lit: vec![(0.0, 0.18), (0.42, 0.55)] }, Look { emissive: 1.0, fog: 0.35, ..Look::default() }));
            }
        }
        self.world.insert_resource(Solid(solids));
        self.world.insert_resource(Ground::Tris(ground));
        self.world.resource_mut::<zombie::Nav>().0 = None;
    }

    /// Put the title scene away (a map is going in).
    pub fn hide_title(&mut self) {
        let shown: Vec<Entity> = self.world.query_filtered::<Entity, With<OnTitle>>().iter(&self.world).collect();
        for e in shown {
            self.world.despawn(e);
        }
    }

    /// Everything of the last map gone: its ground, scenery, containers,
    /// the dead, what's lying about, the ways out.
    pub fn clear_map(&mut self) {
        zombie::clear(&mut self.world);
        zombie::spit::clear(&mut self.world);
        crate::throw::clear(&mut self.world);
        crate::items::clear(&mut self.world);
        crate::exits::hide(&mut self.world);
        self.world.remove_resource::<crate::exits::Exits>();
        let all: Vec<Entity> = self.world.query_filtered::<Entity, With<OnMap>>().iter(&self.world).collect();
        for e in all {
            self.world.despawn(e);
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
        let blend = self.alpha();
        self.world.resource_mut::<Blend>().0 = blend;
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
    fn players_keep_their_own_seats() {
        let mut g = Game::new();
        g.spawn_player(0, 1.0, 2.0, 0.0);
        g.spawn_player(1, 5.0, 6.0, 0.0);
        // Seat 0 again: in place of the first, the second left be.
        g.spawn_player(0, -3.0, -4.0, 0.0);
        let seats: Vec<(usize, f64)> = g.players().iter().map(|(seat, b, _)| (*seat, b.pos.x)).collect();
        assert_eq!(seats, vec![(0, -3.0), (1, 5.0)]);
        assert!(g.controls_mut(1).is_some() && g.controls_mut(2).is_none());
        g.push_player(1, Vec3::new(0.0, 0.0, 2.0));
        assert_eq!(g.player(0).unwrap().0.push, Vec3::ZERO, "only the one shoved");
        g.despawn_players();
        assert!(g.players().is_empty());
    }

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
