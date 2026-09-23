//! Fighting: the pistol's events turned into the world's. A shot is a ray
//! from the eye (a little wide when moving, more in the air) that stops at
//! the first solid or target; a blow is three short rays in a fan. What is
//! hit takes damage, throws chips, makes its sound where it is; a target
//! hit flashes the hitmarker. Shots kick the view; blows that land shake it.

use lntrn_math::Vec3;

use crate::collide::Surface;
use crate::fx::Fx;
use crate::head::{self, View};
use crate::player::Body;
use crate::render::Renderer;
use crate::sound::{Sfx, Sound};
use crate::stats::Stats;
use crate::targets::{self, Kind, Target};
use crate::weapon::{Act, Pistol, Trigger};
use crate::world::{Game, Solid};
use crate::zombie::{self, Horde};

/// Damage, and how far a bullet and a blow reach, metres.
const SHOT_DAMAGE: f64 = 25.0;
const BLOW_DAMAGE: f64 = 50.0;
const SHOT_RANGE: f64 = 300.0;
const BLOW_RANGE: f64 = 1.8;
/// A shot's spread, degrees: moving, and in the air (none standing still).
const SPREAD_MOVING: f64 = 1.2;
const SPREAD_AIR: f64 = 3.0;
/// How long after a shot the player can't sprint, seconds.
const SPRINT_BLOCK: f64 = 0.3;

pub struct Combat {
    pub pistol: Pistol,
    pub fx: Fx,
    sound: Sound,
    sprint_block: f64,
    seed: u32,
    /// How red the edges of the screen are from a blow, 0–1, fading.
    pub hurt: f64,
}

/// How hard a blow shoves the player, m/s, and shakes the view, metres.
const BLOW_SHOVE: f64 = 4.5;
const BLOW_SHAKE: f64 = 0.035;

/// Where the eye is and which way it looks.
struct Aim {
    eye: Vec3,
    dir: Vec3,
    right: Vec3,
    up: Vec3,
}

impl Combat {
    pub fn new() -> Self {
        Self { pistol: Pistol::default(), fx: Fx::default(), sound: Sound::new(), sprint_block: 0.0, seed: 0x6C8E_9CF5, hurt: 0.0 }
    }

    pub fn init(&mut self, renderer: &mut Renderer) {
        self.fx.init(renderer);
    }

    /// Whether the player may sprint (not just after a shot).
    pub fn sprint_allowed(&self) -> bool {
        self.sprint_block <= 0.0
    }

    /// A fresh run: a full magazine and the spare rounds, nothing in the air.
    pub fn reset(&mut self) {
        self.pistol = Pistol::default();
        self.sprint_block = 0.0;
        self.hurt = 0.0;
    }

    /// What the dead did since last frame: play their sounds where they
    /// are, and take their blows (a shove, a shake, the edges gone red).
    /// Once the player is dead (not `alive`) it's all let go unheard. How
    /// many blows landed.
    pub fn answer_the_dead(&mut self, game: &mut Game, alive: bool) -> usize {
        let (sounds, blows) = {
            let mut horde = game.world.resource_mut::<Horde>();
            (std::mem::take(&mut horde.sounds), std::mem::take(&mut horde.blows))
        };
        if !alive {
            return 0;
        }
        let Some((body, view)) = game.player() else { return 0 };
        let aim = aim(&view, &body, game.alpha());
        let solids = &game.world.resource::<Solid>().0;
        for (sfx, at, gain) in sounds {
            // Only what could be heard at all is checked for walls between.
            let to = at - aim.eye;
            let d = to.length();
            if d >= sfx.range() {
                continue;
            }
            let blocked = d > 1.0 && solids.raycast(aim.eye, to * (1.0 / d), d - 0.5).is_some();
            self.sound.play_at(sfx, gain, at, aim.eye, aim.right, blocked);
        }
        let landed = blows.len();
        for push in blows {
            self.sound.play(Sfx::Flesh, 0.9);
            self.hurt = 1.0;
            if let Some(mut v) = game.player_view_mut() {
                v.jolt(BLOW_SHAKE);
                v.recoil(-3.0, (self.rand() - 0.5) * 6.0);
            }
            game.push_player(push * BLOW_SHOVE);
        }
        landed
    }

    /// Play a sound at the listener.
    pub fn play(&self, sfx: Sfx, gain: f32) {
        self.sound.play(sfx, gain);
    }

    fn rand(&mut self) -> f64 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        f64::from(self.seed) / f64::from(u32::MAX)
    }

    /// One frame of a run: the pistol, what it does, and what flies.
    pub fn frame(&mut self, game: &mut Game, trigger: Trigger, dt: f64, stats: &mut Stats) {
        self.fx.update(dt);
        self.sprint_block -= dt;
        self.hurt = (self.hurt - dt * 1.6).max(0.0);
        let acts = self.pistol.update(trigger, dt);
        if acts.is_empty() {
            return;
        }
        let Some((body, view)) = game.player() else { return };
        let aim = aim(&view, &body, game.alpha());
        for act in acts {
            match act {
                Act::Shoot => {
                    stats.shots += 1;
                    self.sound.play(Sfx::Shot, 0.9);
                    zombie::noise(&mut game.world, aim.eye);
                    self.sprint_block = SPRINT_BLOCK;
                    let side = (self.rand() - 0.5) * 0.8;
                    if let Some(mut v) = game.player_view_mut() {
                        v.recoil(1.5, side);
                    }
                    let spread = if !body.grounded { SPREAD_AIR } else if body.speed_flat() > 0.5 { SPREAD_MOVING } else { 0.0 };
                    let dir = self.scatter(&aim, spread);
                    self.strike(game, &aim, dir, SHOT_RANGE, SHOT_DAMAGE, false, stats);
                }
                Act::DryFire => self.sound.play(Sfx::DryFire, 0.8),
                Act::MagOut => self.sound.play(Sfx::MagOut, 0.7),
                Act::MagIn => {
                    stats.reloads += 1;
                    self.sound.play(Sfx::MagIn, 0.8);
                }
                Act::SlideRack => self.sound.play(Sfx::SlideRack, 0.8),
                Act::Swing => self.sound.play(Sfx::Whoosh, 0.7),
                Act::Strike => {
                    // A fan of three: straight on, and a little either side.
                    for turn in [0.0f64, -8.0, 8.0] {
                        let t = turn.to_radians();
                        let dir = (aim.dir * t.cos() + aim.right * t.sin()).normalize();
                        if self.strike(game, &aim, dir, BLOW_RANGE, BLOW_DAMAGE, true, stats) {
                            break;
                        }
                    }
                }
            }
        }
    }

    /// `aim`'s direction thrown up to `degrees` off true.
    fn scatter(&mut self, aim: &Aim, degrees: f64) -> Vec3 {
        if degrees <= 0.0 {
            return aim.dir;
        }
        let r = degrees.to_radians() * self.rand().sqrt();
        let a = self.rand() * std::f64::consts::TAU;
        (aim.dir + aim.right * (r * a.cos()) + aim.up * (r * a.sin())).normalize()
    }

    /// A shot or a blow along `dir`: whatever it meets first. Whether it
    /// met anything.
    #[allow(clippy::too_many_arguments)]
    fn strike(&mut self, game: &mut Game, aim: &Aim, dir: Vec3, range: f64, damage: f64, blow: bool, stats: &mut Stats) -> bool {
        let wall = game.world.resource::<Solid>().0.raycast(aim.eye, dir, range);
        let reach = wall.map_or(range, |h| h.t);
        let dead = zombie::raycast(&mut game.world, aim.eye, dir, reach);
        let reach = dead.map_or(reach, |(_, t, _)| t);
        let target = targets::raycast(&mut game.world, aim.eye, dir, reach);
        if target.is_none()
            && let Some((e, t, head)) = dead
        {
            let point = aim.eye + dir * t;
            let killed = zombie::hurt(&mut game.world, e, dir, aim.eye, damage, head, blow);
            stats.damage_dealt += zombie::brain::dealt(damage, head, blow);
            if !blow {
                stats.hits += 1;
                stats.headshots += u32::from(head);
            }
            if killed {
                if blow { stats.melee_kills += 1 } else { stats.gun_kills += 1 }
                stats.longest_kill = stats.longest_kill.max(t);
            }
            self.fx.burst(point, -dir, Surface::Flesh, if blow { 12 } else { 9 });
            self.fx.mark(killed);
            self.sound.play(Sfx::Confirm, if killed { 0.7 } else { 0.45 });
            self.sound.play_at(Sfx::Flesh, 1.0, point, aim.eye, aim.right, false);
            if blow && let Some(mut v) = game.player_view_mut() {
                v.jolt(0.02);
            }
            return true;
        }
        if let Some((e, t, head)) = target {
            let point = aim.eye + dir * t;
            let Some((beaten, kind)) = game.world.get_mut::<Target>(e).map(|mut target| (target.hit(dir, point, damage, head), target.kind)) else { return false };
            if !blow {
                stats.hits += 1;
                stats.headshots += u32::from(head);
            }
            match kind {
                Kind::Dummy => stats.dummies_downed += u32::from(beaten),
                Kind::Plate => stats.plates_rung += 1,
            }
            let (surface, sfx, gain) = match kind {
                Kind::Dummy => (Surface::Wood, Sfx::HitWood, 0.9),
                Kind::Plate => (Surface::Metal, Sfx::Ding, 1.0),
            };
            self.fx.burst(point, -dir, surface, if blow { 10 } else { 7 });
            self.fx.mark(beaten);
            self.sound.play(Sfx::Confirm, if beaten { 0.7 } else { 0.45 });
            self.sound.play_at(sfx, gain, point, aim.eye, aim.right, false);
            if blow && let Some(mut v) = game.player_view_mut() {
                v.jolt(0.02);
            }
            return true;
        }
        let Some(hit) = wall else { return false };
        self.fx.burst(hit.point, hit.normal, hit.surface, if blow { 8 } else { 6 });
        let (sfx, gain) = match hit.surface {
            Surface::Dirt => (Sfx::HitDirt, 0.8),
            Surface::Wood => (Sfx::HitWood, 0.8),
            Surface::Stone => (Sfx::HitStone, 0.8),
            Surface::Metal => (Sfx::Ding, 0.35),
            Surface::Flesh => (Sfx::Flesh, 0.8),
        };
        self.sound.play_at(sfx, gain, hit.point, aim.eye, aim.right, false);
        if blow && let Some(mut v) = game.player_view_mut() {
            v.jolt(0.012);
        }
        true
    }

    /// Queue what flies for drawing.
    pub fn draw(&self, renderer: &mut Renderer) {
        self.fx.draw(renderer);
    }
}

/// The eye and its directions, as the camera has them this frame.
fn aim(view: &View, body: &Body, alpha: f64) -> Aim {
    let (yaw, pitch) = view.aim();
    let (sy, cy) = yaw.sin_cos();
    let (sp, cp) = pitch.sin_cos();
    let dir = Vec3::new(-sy * cp, sp, -cy * cp);
    let right = Vec3::new(cy, 0.0, -sy);
    Aim { eye: head::eye_position(view, body, alpha), dir, right, up: right.cross(dir) }
}
