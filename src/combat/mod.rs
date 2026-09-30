//! Fighting: what the hands do turned into the world's. A shot is a ray
//! from the eye (a little wide when moving, more in the air) that stops at
//! the first solid or target; a blow is three short rays in a fan. How
//! hard and how far is the weapon's (`weapon/spec.rs`). What is
//! hit takes damage, throws chips, makes its sound where it is; a target
//! hit flashes the hitmarker. Shots kick the view; blows that land shake it.

use lntrn_math::Vec3;

mod dead;

use crate::collide::Surface;
use crate::fx::{Fx, Marker};
use crate::head::{self, View};
use crate::input::pad::Rumble;
use crate::items;
use crate::loot::Dice;
use crate::loot::bag::Slot;
use crate::loot::tables::{self, Source};
use crate::player::Body;
use crate::render::Renderer;
use crate::sound::{Sfx, Sound};
use crate::stats::Stats;
use crate::targets::{self, Kind, Target};
use crate::weapon::{Act, Clip, Falloff, Hands, Trigger, Weapon};
use crate::world::{Game, Solid};
use crate::zombie::figure::Zone;
use crate::zombie::{self, Horde, spit};

/// How long after a shot the player can't sprint, seconds.
const SPRINT_BLOCK: f64 = 0.3;

pub struct Combat {
    /// Each player's, by seat.
    pub arms: Vec<Arms>,
    pub fx: Fx,
    sound: Sound,
    seed: u32,
    /// Luck for what the dead drop: its own, so it owes nothing to the
    /// spread of shots.
    loot: Dice,
}

/// A player's side of the fight: what's in their hands, and how the
/// fight's going for them.
pub struct Arms {
    pub hands: Hands,
    sprint_block: f64,
    /// How red the edges of their screen are from a blow, 0–1, fading.
    pub hurt: f64,
    /// Melee damage and shove, a multiple of the usual (brawler).
    pub melee: f64,
    /// How their pad should shake for this frame's shots and blows.
    pub rumble: Rumble,
    /// Their hitmarker, flashing.
    pub marker: Option<Marker>,
}

impl Default for Arms {
    fn default() -> Self {
        Self { hands: Hands::default(), sprint_block: 0.0, hurt: 0.0, melee: 1.0, rumble: Rumble::default(), marker: None }
    }
}

impl Arms {
    /// Whether they may sprint (not just after a shot).
    pub fn sprint_allowed(&self) -> bool {
        self.sprint_block <= 0.0
    }
}

/// How hard a blow shoves the player, m/s, and shakes the view, metres.
const BLOW_SHOVE: f64 = 4.5;
const BLOW_SHAKE: f64 = 0.035;

/// A pellet (or round) or a blow: how far it reaches and how hard it hits,
/// whether it's a blow, how its punch fades, how hard it shoves the dead
/// and whether it can send one stumbling, and how it goes on through them.
struct Hit {
    reach: f64,
    damage: f64,
    blow: bool,
    falloff: Option<Falloff>,
    shove: f64,
    stumble: bool,
    /// The share of its damage each of the dead behind the first takes.
    pierce: &'static [f64],
    /// Kills one that never saw it coming.
    takedown: bool,
}

/// What a pellet or a blow met: anything at all, one of the dead or a
/// target, and its head.
#[derive(Clone, Copy, Debug, Default)]
struct Met {
    something: bool,
    target: bool,
    head: bool,
}

/// What's been heard of a shot's pellets (or a swing's rays) landing so
/// far: one tick for hitting something (any kill ticks again), one thud,
/// however many land; and which of the dead the pellet or swing has struck
/// (none twice by one pellet, nor by one swing).
#[derive(Default)]
struct Heard {
    confirmed: bool,
    thudded: bool,
    struck: Vec<bevy_ecs::entity::Entity>,
}

impl Heard {
    fn confirm(&mut self, sound: &Sound, killed: bool) {
        if killed || !self.confirmed {
            sound.play(Sfx::Confirm, if killed { 0.7 } else { 0.45 });
        }
        self.confirmed = true;
    }

    /// Whether this landing is the one heard.
    fn thud(&mut self) -> bool {
        !std::mem::replace(&mut self.thudded, true)
    }
}

/// Whose eye (their seat), where it is and which way it looks.
struct Aim {
    seat: usize,
    eye: Vec3,
    dir: Vec3,
    right: Vec3,
    up: Vec3,
}


impl Combat {
    pub fn new() -> Self {
        Self { arms: vec![Arms::default()], fx: Fx::default(), sound: Sound::new(), seed: 0x6C8E_9CF5, loot: Dice::default() }
    }

    pub fn init(&mut self, renderer: &mut Renderer) {
        self.fx.init(renderer);
    }

    /// A fresh run for `players`: each one's hands empty (the run says
    /// what to take up).
    pub fn reset(&mut self, players: usize) {
        self.arms = (0..players).map(|_| Arms::default()).collect();
    }

    /// Set how loud everything is.
    pub fn set_mix(&self, mix: crate::sound::Mix) {
        self.sound.set_mix(mix);
    }

    /// Whether the music should be playing.
    pub fn set_music(&self, on: bool) {
        self.sound.set_music(on);
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

    /// Player `seat` takes up `weapon` (from `held`, `mag` rounds in it):
    /// it comes up into view.
    pub fn take_up(&mut self, seat: usize, held: Option<Slot>, weapon: Weapon, mag: u32) {
        self.arms[seat].hands.take_up(held, weapon, mag);
        if weapon != Weapon::Fists {
            self.sound.play(Sfx::SlideRack, 0.35);
        }
    }

    /// A frame of what flies, once for everyone (before their hands).
    pub fn update(&mut self, dt: f64) {
        self.fx.update(dt);
    }

    /// One frame of a run for player `seat`: their hands, and what they
    /// do. How much stamina their swings took.
    pub fn frame(&mut self, game: &mut Game, seat: usize, trigger: Trigger, dt: f64, stats: &mut Stats) -> f64 {
        let arms = &mut self.arms[seat];
        arms.sprint_block -= dt;
        arms.hurt = (arms.hurt - dt * 1.6).max(0.0);
        arms.marker = arms.marker.and_then(|m| m.faded(dt));
        let spec = arms.hands.spec();
        let acts = arms.hands.update(trigger, dt);
        // Down the sights, the view closes in.
        let sights = arms.hands.aim();
        if let Some(mut v) = game.player_view_mut(seat) {
            v.ads = sights;
            v.ads_zoom = spec.shot.map_or(1.0, |s| s.zoom);
        }
        if acts.is_empty() {
            return 0.0;
        }
        let Some((body, view)) = game.player(seat) else { return 0.0 };
        let aim = aim(seat, &view, &body, game.alpha());
        let mut spent = 0.0;
        for act in acts {
            match act {
                Act::Shoot => {
                    let Some(shot) = spec.shot else { continue };
                    stats.shots += 1;
                    self.sound.play(shot.sound, 0.9);
                    zombie::noise(&mut game.world, aim.eye, shot.heard);
                    self.arms[seat].sprint_block = SPRINT_BLOCK;
                    self.arms[seat].rumble.add(Rumble::shot(shot.kick));
                    let side = (self.rand() - 0.5) * shot.kick_side;
                    if let Some(mut v) = game.player_view_mut(seat) {
                        v.recoil(shot.kick, side);
                    }
                    let spread = shot.hip.toward(shot.aimed, self.arms[seat].hands.aim());
                    let spread = if !body.grounded {
                        spread.air
                    } else if body.speed_flat() > 0.5 {
                        spread.moving
                    } else {
                        spread.still
                    };
                    // Every pellet its own way; a hit counted once a shot.
                    let hit = Hit { reach: shot.range, damage: shot.damage, blow: false, falloff: shot.falloff, shove: shot.shove, stumble: shot.stumble, pierce: shot.pierce, takedown: false };
                    let mut heard = Heard::default();
                    let mut met = Met::default();
                    for _ in 0..shot.pellets {
                        // Each pellet may strike the one the last struck.
                        heard.struck.clear();
                        let dir = self.scatter(&aim, spread);
                        let m = self.strike(game, &aim, dir, &hit, &mut heard, stats);
                        met.target |= m.target;
                        met.head |= m.head;
                    }
                    stats.hits += u32::from(met.target);
                    stats.headshots += u32::from(met.head);
                }
                Act::DryFire => self.sound.play(Sfx::DryFire, 0.8),
                Act::MagOut => self.sound.play(Sfx::MagOut, 0.7),
                Act::MagIn => {
                    stats.reloads += 1;
                    self.sound.play(Sfx::MagIn, 0.8);
                }
                Act::SlideRack => self.sound.play(Sfx::SlideRack, 0.8),
                Act::Pump => {
                    // The pump that finishes a reload counts it.
                    stats.reloads += u32::from(self.arms[seat].hands.clip().0 == Clip::ReloadEnd);
                    self.sound.play(Sfx::Pump, 0.85);
                }
                Act::ShellIn => self.sound.play(Sfx::ShellIn, 0.75),
                Act::Bolt => {
                    // The bolt that finishes a reload counts it.
                    stats.reloads += u32::from(self.arms[seat].hands.clip().0 == Clip::ReloadEnd);
                    self.sound.play(Sfx::Bolt, 0.8);
                }
                Act::Swing => {
                    spent += spec.bash.stamina;
                    self.sound.play(Sfx::Whoosh, 0.7);
                }
                Act::Strike => {
                    let b = spec.bash;
                    let melee = self.arms[seat].melee;
                    let hit = Hit { reach: b.reach, damage: b.damage * melee, blow: true, falloff: None, shove: zombie::blow_shove(melee * b.shove), stumble: true, pierce: &[], takedown: b.takedown };
                    // Rays across its arc, the middle first, then out either
                    // side: the first thing met stops a lone blow; one that
                    // cleaves goes on through the arc to strike as many of
                    // the dead as it can.
                    let rays: &[f64] = if b.cleave > 1 { &[0.0, -1.0 / 3.0, 1.0 / 3.0, -2.0 / 3.0, 2.0 / 3.0, -1.0, 1.0] } else { &[0.0, -1.0, 1.0] };
                    let mut heard = Heard::default();
                    for share in rays {
                        let t = (share * b.arc * 0.5).to_radians();
                        let dir = (aim.dir * t.cos() + aim.right * t.sin()).normalize();
                        let met = self.strike(game, &aim, dir, &hit, &mut heard, stats);
                        if (b.cleave <= 1 && met.something) || heard.struck.len() >= b.cleave as usize {
                            break;
                        }
                    }
                    stats.blows_landed += heard.struck.len() as u32;
                    if !heard.struck.is_empty() {
                        self.arms[seat].rumble.add(Rumble::struck());
                    }
                    // A heavy blade biting is heard a little way off.
                    if b.heard > 0.0 && !heard.struck.is_empty() {
                        zombie::noise(&mut game.world, aim.eye, b.heard);
                    }
                }
            }
        }
        spent
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

    /// A pellet (or round) or a blow along `dir`: whatever it meets first
    /// takes `hit` (a pellet's punch fading with how far it flew). What it
    /// met.
    fn strike(&mut self, game: &mut Game, aim: &Aim, dir: Vec3, hit: &Hit, heard: &mut Heard, stats: &mut Stats) -> Met {
        let wall = game.world.resource::<Solid>().0.raycast(aim.eye, dir, hit.reach);
        let reach = wall.map_or(hit.reach, |h| h.t);
        let dead = zombie::raycast_past(&mut game.world, aim.eye, dir, reach, &heard.struck);
        let reach = dead.map_or(reach, |(_, t, _)| t);
        let target = targets::raycast(&mut game.world, aim.eye, dir, reach);
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
                let impact = zombie::Impact { damage: punch(t) * share, head, limb, blow: hit.blow, shove: hit.shove, stumble: hit.stumble && close, takedown: hit.takedown };
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

    /// Queue what flies for drawing.
    pub fn draw(&self, renderer: &mut Renderer) {
        self.fx.draw(renderer);
    }
}

/// Player `seat`'s eye and its directions, as their camera has them this
/// frame.
fn aim(seat: usize, view: &View, body: &Body, alpha: f64) -> Aim {
    let (yaw, pitch) = view.aim();
    let (sy, cy) = yaw.sin_cos();
    let (sp, cp) = pitch.sin_cos();
    let dir = Vec3::new(-sy * cp, sp, -cy * cp);
    let right = Vec3::new(cy, 0.0, -sy);
    Aim { seat, eye: head::eye_position(view, body, alpha), dir, right, up: right.cross(dir) }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A shotgun fired from the hip into one of the dead, `metres` off:
    /// the health it took.
    fn blast(metres: f64) -> f64 {
        let mut game = Game::new();
        let gltf = lntrn_model::Gltf::load(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/models/shambler.glb")).expect("shambler.glb");
        game.world.insert_resource(zombie::figure::Model::new(crate::assets::Rigged { mesh: Vec::new(), gltf, skin: 0 }).expect("the model"));
        game.spawn_player(0, 0.0, 0.0, 0.0);
        // Aimed at its chest, not over its shoulders.
        game.player_view_mut(0).unwrap().pitch = -(0.5f64 / metres).atan();
        zombie::spawn_kind(&mut game.world, Vec3::new(0.0, 0.0, -metres), 0.0, zombie::kind::Kind::Shambler, zombie::looks::Theme::Townsfolk);
        game.tick(0.0);
        let hp = |game: &mut Game| game.world.query::<&zombie::brain::Zombie>().iter(&game.world).map(|z| z.hp).sum::<f64>();
        let before = hp(&mut game);
        let mut combat = Combat::new();
        combat.take_up(0, Some(Slot::Primary), Weapon::Shotgun, 5);
        let mut stats = Stats::default();
        // Up into the hands, then the trigger pulled once.
        for frame in 0..90 {
            let trigger = Trigger { fire: frame == 60, ..Trigger::default() };
            combat.frame(&mut game, 0, trigger, 1.0 / 60.0, &mut stats);
        }
        assert_eq!(stats.shots, 1);
        before - hp(&mut game)
    }

    #[test]
    fn every_pellet_of_a_blast_can_strike_the_one_in_front() {
        let pellet = Weapon::Shotgun.spec().shot.unwrap().damage;
        let taken = blast(2.5);
        assert!(taken > pellet * 5.0, "point blank took only {taken:.0}, a pellet is {pellet:.0}");
        // Out to six metres from the hip, one drops a Shambler.
        let hp = zombie::kind::Kind::Shambler.traits().hp;
        assert!(blast(6.0) >= hp, "a blast at 6 m left it standing");
    }
}
