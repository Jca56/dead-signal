//! What a Shambler senses and decides, a fixed step at a time. It wanders
//! until it sees the player (ahead of it, near enough, nothing between) or
//! hears a shot; hunts what it saw, walking the nav grid round what's in
//! the way; lunges the last couple of metres; swipes when it's close. Lose
//! it for long enough and it searches where it last saw it, then wanders.
//! Nothing here draws or plays: it says what it wants done.

use bevy_ecs::prelude::*;
use lntrn_math::{Vec2, Vec3};

use super::figure::Clip;
use super::nav::NavGrid;
use crate::collide::Solids;
use crate::player::{Body, Controls, Gait};
use crate::sound::Sfx;

/// How far it sees, and how wide (either side of straight ahead).
pub const SIGHT: f64 = 25.0;
const SIGHT_HALF: f64 = 55.0;
/// Closer than this it knows you're there, whichever way it faces.
const CLOSE: f64 = 2.5;
/// How far a shot is heard, and another's snarl on seeing the player.
pub const HEARING: f64 = 80.0;
pub const ALERT_RANGE: f64 = 15.0;
/// It swipes from this close, and the swipe reaches this far.
pub const ATTACK_RANGE: f64 = 1.4;
const REACH: f64 = 1.8;
/// How far above its feet the player's feet can be and still be struck
/// (standing on a crate: the swipe takes the legs), and how far below.
const REACH_UP: f64 = 1.0;
const REACH_DOWN: f64 = 1.2;
/// Seconds: the swipe, when in it the blow lands, the rest after it.
const ATTACK_TIME: f64 = 0.9;
const STRIKE_AT: f64 = 0.4;
const COOLDOWN: f64 = 1.2;
/// How long a shot staggers it, and a blow (which sends it stumbling back).
const FLINCH_TIME: f64 = 0.33;
const STUMBLE_TIME: f64 = 0.6;
/// It lunges from this close.
const LUNGE: f64 = 2.2;
/// How long out of sight before it goes to look where it last saw you.
const FORGET: f64 = 8.0;
/// Wandering: how far it strays, and at what share of its walk.
const WANDER_RANGE: f64 = 12.0;
const WANDER_PACE: f64 = 0.45;
/// How fast it turns, rad/s: hunting, and not.
const TURN_HUNT: f64 = 3.0;
const TURN_IDLE: f64 = 1.5;
/// How often it finds its way again, seconds.
const REPATH: f64 = 0.6;
/// Following a route: this near a turning point it has reached it, and it
/// aims this far ahead along the leg it's on.
const ARRIVE: f64 = 0.3;
const LOOK_AHEAD: f64 = 0.7;
/// Its eyes, and where it looks for yours, above the feet.
const EYE: f64 = 1.55;
const PLAYER_EYE: f64 = 1.5;
/// A stride of its walk animation covers this much ground.
const STRIDE: f64 = 1.1;
const WALK_CLIP: f64 = 0.8;
pub const HP: f64 = 150.0;
/// A shot to the head does this many times a body shot's damage (blows do
/// the same wherever they land).
pub const HEADSHOT: f64 = 6.0;
/// Its pace, m/s (a walk, and the lunge): most shamble, but one in
/// `FAST_SHARE` walks fast.
const SLOW: (f64, f64, f64) = (1.6, 2.0, 3.8);
const FAST: (f64, f64, f64) = (3.0, 3.4, 4.5);
const FAST_SHARE: f64 = 0.25;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    /// Heading for `goal`, or standing about `rest` seconds more.
    Wander { goal: Option<Vec3>, rest: f64 },
    Hunt,
    /// Going to look where it last saw the player.
    Search(Vec3),
    /// Going to where a shot came from, then looking about.
    Investigate { at: Vec3, looked: f64 },
    Attack { t: f64, struck: bool },
    /// Knocked off its stride for `until` seconds.
    Stagger { t: f64, until: f64 },
    Dead { t: f64 },
}

/// What it has to go on, this step.
pub struct Senses<'a> {
    pub solids: &'a Solids,
    pub nav: Option<&'a NavGrid>,
    /// Where the player stands, if there is one.
    pub player: Option<Vec3>,
    /// Noises made since the last step (a shot, a rummage): where, and
    /// how far off they're heard.
    pub noises: &'a [(Vec3, f64)],
    /// Snarls since the last step: where from, and where the player was.
    pub alerts: &'a [(Vec3, Vec3)],
}

/// What it wants done this step.
#[derive(Default)]
pub struct Intent {
    pub controls: Controls,
    pub sounds: Vec<(Sfx, f32)>,
    /// Its blow landed on the player, pushing this way.
    pub hit: Option<Vec3>,
    /// It has just seen the player, there, and snarled for the rest.
    pub alert: Option<Vec3>,
}

#[derive(Component, Clone, Debug)]
pub struct Zombie {
    pub hp: f64,
    pub state: State,
    pub yaw: f64,
    pub gait: Gait,
    path: Vec<Vec3>,
    path_goal: Vec3,
    /// Where the leg of the route it's on began.
    leg_from: Vec3,
    repath: f64,
    last_seen: Option<Vec3>,
    unseen: f64,
    cooldown: f64,
    groan: f64,
    shuffle: f64,
    /// Ground walked, for the walk animation's stride.
    pub walked: f64,
    /// Seconds in its current animation (not the walk: that runs on
    /// `walked`), and whether it's moving.
    pub clip_t: f64,
    pub moving: bool,
    seed: u32,
}

impl Zombie {
    pub fn new(yaw: f64, seed: u32) -> Self {
        let mut z = Self { hp: HP, state: State::Wander { goal: None, rest: 1.0 }, yaw, gait: Gait { walk: 0.0, sprint: 0.0, crouch: 0.0 }, path: Vec::new(), path_goal: Vec3::ZERO, leg_from: Vec3::ZERO, repath: 0.0, last_seen: None, unseen: 0.0, cooldown: 0.0, groan: 0.0, shuffle: 0.0, walked: 0.0, clip_t: 0.0, moving: false, seed: seed | 1 };
        z.groan = 2.0 + 5.0 * z.rand();
        let (lo, hi, lunge) = if z.rand() < FAST_SHARE { FAST } else { SLOW };
        let walk = lo + (hi - lo) * z.rand();
        z.gait = Gait { walk, sprint: lunge, crouch: walk };
        z
    }

    fn rand(&mut self) -> f64 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        f64::from(self.seed) / f64::from(u32::MAX)
    }

    pub fn dead(&self) -> bool {
        matches!(self.state, State::Dead { .. })
    }

    fn set(&mut self, state: State) {
        self.state = state;
        self.clip_t = 0.0;
    }

    /// The animation to show and how far into it.
    pub fn clip(&self) -> (Clip, f64) {
        match self.state {
            State::Dead { t } => (Clip::Death, t),
            State::Attack { t, .. } => (Clip::Attack, t),
            State::Stagger { t, until } if until > FLINCH_TIME => (Clip::Stumble, t),
            State::Stagger { t, .. } => (Clip::Flinch, t),
            _ if self.moving => (Clip::Walk, self.walked / STRIDE * WALK_CLIP),
            _ => (Clip::Idle, self.clip_t),
        }
    }

    /// Take a hit: a shot to the `head` does [`HEADSHOT`] times the damage;
    /// a `blow` sends it stumbling, a shot to the head that doesn't kill
    /// staggers it. Being hit, it knows where from. Whether this killed it.
    pub fn hurt(&mut self, damage: f64, head: bool, blow: bool, from: Vec3) -> bool {
        if self.dead() {
            return false;
        }
        self.hp -= dealt(damage, head, blow);
        if self.hp <= 0.0 {
            self.set(State::Dead { t: 0.0 });
            return true;
        }
        self.last_seen = Some(from);
        self.unseen = 0.0;
        if blow || head {
            self.set(State::Stagger { t: 0.0, until: if blow { STUMBLE_TIME } else { FLINCH_TIME } });
        } else if !matches!(self.state, State::Attack { .. } | State::Stagger { .. }) {
            self.state = State::Hunt;
        }
        false
    }

    /// Whether it sees a player standing at `player`.
    fn sees(&self, body: &Body, player: Vec3, solids: &Solids) -> bool {
        let to = Vec2::new(player.x - body.pos.x, player.z - body.pos.z);
        let d = to.length();
        if d > SIGHT {
            return false;
        }
        if d > CLOSE {
            let facing = Vec2::new(-self.yaw.sin(), -self.yaw.cos());
            if facing.dot(to * (1.0 / d)) < SIGHT_HALF.to_radians().cos() {
                return false;
            }
        }
        let eye = body.pos + Vec3::new(0.0, EYE, 0.0);
        let look = player + Vec3::new(0.0, PLAYER_EYE, 0.0) - eye;
        let len = look.length();
        solids.raycast(eye, look * (1.0 / len), (len - 0.3).max(0.0)).is_none()
    }

    /// One fixed step of thought.
    pub fn think(&mut self, body: &Body, s: &Senses, dt: f64) -> Intent {
        let mut out = Intent::default();
        self.clip_t += dt;
        if let State::Dead { t } = &mut self.state {
            *t += dt;
            self.moving = false;
            return out;
        }
        self.cooldown -= dt;
        self.repath -= dt;
        self.groan -= dt;

        // What it sees and hears.
        let seen = s.player.filter(|&p| self.sees(body, p, s.solids));
        if let Some(p) = seen {
            if matches!(self.state, State::Wander { .. } | State::Search(_) | State::Investigate { .. }) {
                out.sounds.push((Sfx::Snarl, 1.0));
                out.alert = Some(p);
                self.state = State::Hunt;
            }
            self.last_seen = Some(p);
            self.unseen = 0.0;
        } else if let Some(&(shot, _)) = s.noises.iter().find(|(at, range)| (*at - body.pos).length() <= *range)
            && !matches!(self.state, State::Hunt | State::Attack { .. } | State::Stagger { .. })
        {
            self.state = State::Investigate { at: shot, looked: 0.0 };
        } else if let Some(&(_, seen_at)) = s.alerts.iter().find(|(from, _)| flat_dist(*from, body.pos) <= ALERT_RANGE)
            && matches!(self.state, State::Wander { .. } | State::Search(_))
        {
            // Another saw them: come and look, answering it.
            out.sounds.push((Sfx::Groan, 0.8));
            self.state = State::Investigate { at: seen_at, looked: 0.0 };
        }
        if self.groan <= 0.0 {
            out.sounds.push((Sfx::Groan, 0.9));
            self.groan = 4.0 + 5.0 * self.rand();
        }

        let mut goal: Option<(Vec3, f64, bool, f64)> = None; // where, pace, lunge, turn rate
        match self.state {
            State::Stagger { t, until } => {
                self.state = if t + dt >= until { State::Hunt } else { State::Stagger { t: t + dt, until } };
            }
            State::Attack { t, struck } => {
                let t = t + dt;
                let mut struck = struck;
                if let Some(p) = s.player {
                    self.face(p, body, TURN_HUNT, dt);
                    if !struck && t >= STRIKE_AT {
                        struck = true;
                        let to = Vec3::new(p.x - body.pos.x, 0.0, p.z - body.pos.z);
                        let d = to.length();
                        let facing = Vec3::new(-self.yaw.sin(), 0.0, -self.yaw.cos());
                        if d <= REACH && level(body.pos, p) && (d < 1e-6 || facing.dot(to * (1.0 / d)) > 0.5) {
                            out.hit = Some(if d > 1e-6 { to * (1.0 / d) } else { facing });
                        }
                    }
                }
                if t >= ATTACK_TIME {
                    self.cooldown = COOLDOWN;
                    self.set(State::Hunt);
                } else {
                    self.state = State::Attack { t, struck };
                }
            }
            State::Hunt => {
                if seen.is_none() {
                    self.unseen += dt;
                    if self.unseen > FORGET {
                        self.state = self.last_seen.map_or(State::Wander { goal: None, rest: 1.0 }, State::Search);
                    }
                }
                let target = seen.or(self.last_seen);
                if let Some(p) = target {
                    let d = Vec2::new(p.x - body.pos.x, p.z - body.pos.z).length();
                    if seen.is_some() && d <= ATTACK_RANGE && level(body.pos, p) && self.cooldown <= 0.0 {
                        out.sounds.push((Sfx::Snarl, 0.8));
                        self.set(State::Attack { t: 0.0, struck: false });
                    } else {
                        goal = Some((p, 1.0, seen.is_some() && d < LUNGE, TURN_HUNT));
                    }
                } else {
                    self.state = State::Wander { goal: None, rest: 1.0 };
                }
            }
            State::Search(at) => {
                if flat_dist(body.pos, at) < 1.5 {
                    self.state = State::Wander { goal: None, rest: 2.0 };
                } else {
                    goal = Some((at, 1.0, false, TURN_HUNT));
                }
            }
            State::Investigate { at, looked } => {
                if flat_dist(body.pos, at) < 2.0 || looked > 0.0 {
                    let looked = looked + dt;
                    self.state = if looked > 3.0 { State::Wander { goal: None, rest: 1.0 } } else { State::Investigate { at, looked } };
                } else {
                    goal = Some((at, 0.8, false, TURN_HUNT));
                }
            }
            State::Wander { goal: aim, rest } => {
                if rest > 0.0 {
                    self.state = State::Wander { goal: aim, rest: rest - dt };
                } else if let Some(g) = aim {
                    if flat_dist(body.pos, g) < 1.0 {
                        let rest = 2.0 + 4.0 * self.rand();
                        self.state = State::Wander { goal: None, rest };
                    } else {
                        goal = Some((g, WANDER_PACE, false, TURN_IDLE));
                    }
                } else {
                    let a = self.rand() * std::f64::consts::TAU;
                    let r = 4.0 + (WANDER_RANGE - 4.0) * self.rand();
                    let g = body.pos + Vec3::new(a.cos() * r, 0.0, a.sin() * r);
                    let ok = s.nav.is_none_or(|n| n.walkable(g));
                    self.state = State::Wander { goal: ok.then_some(g), rest: if ok { 0.0 } else { 0.5 } };
                }
            }
            State::Dead { .. } => {}
        }

        self.moving = false;
        if let Some((target, pace, lunge, turn)) = goal {
            out.controls = self.steer(body, target, pace, lunge, turn, s.nav, dt);
            self.moving = out.controls.walk.y > 0.05;
            if self.moving {
                self.shuffle -= dt * pace;
                if self.shuffle <= 0.0 {
                    out.sounds.push((Sfx::Shuffle, 0.35));
                    self.shuffle = 0.42;
                }
            }
        }
        out
    }

    /// Turn toward `p` at up to `rate` rad/s. How far off it still is.
    fn face(&mut self, p: Vec3, body: &Body, rate: f64, dt: f64) -> f64 {
        let (dx, dz) = (p.x - body.pos.x, p.z - body.pos.z);
        if dx.abs() + dz.abs() < 1e-6 {
            return 0.0;
        }
        let want = (-dx).atan2(-dz);
        let mut off = want - self.yaw;
        off = (off + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU) - std::f64::consts::PI;
        let step = off.clamp(-rate * dt, rate * dt);
        self.yaw += step;
        off - step
    }

    /// The keys that take it toward `target`: along the nav grid when the
    /// way isn't straight, slowed while it turns.
    #[allow(clippy::too_many_arguments)]
    fn steer(&mut self, body: &Body, target: Vec3, pace: f64, lunge: bool, turn: f64, nav: Option<&NavGrid>, dt: f64) -> Controls {
        // Close and on its level it just goes; anywhere else the grid
        // decides (up on a roof over it means going round by the stairs).
        let straight = (flat_dist(body.pos, target) < 3.0 && level(body.pos, target)) || nav.is_none_or(|n| n.clear(body.pos, target));
        let waypoint = if straight {
            self.path.clear();
            target
        } else {
            // (No way found is remembered until the next repath too: a search
            // that fails looks at the whole grid.)
            if self.repath <= 0.0 || flat_dist(self.path_goal, target) > 2.0 {
                self.path = nav.and_then(|n| n.path(body.pos, target)).unwrap_or_default();
                self.path_goal = target;
                self.leg_from = body.pos;
                self.repath = REPATH;
            }
            // Each leg is walked along its line, not cut across: a turning
            // point counts once it's truly reached (or passed), and it aims
            // a little ahead on the leg, so knocked off it, it steers back
            // on, and it comes to the foot of a flight of stairs lined up.
            while self.path.len() > 1 && (flat_dist(body.pos, self.path[0]) < ARRIVE || along(self.leg_from, self.path[0], body.pos) >= 1.0) {
                self.leg_from = self.path.remove(0);
            }
            match self.path.first() {
                Some(&to) => {
                    let len = flat_dist(self.leg_from, to);
                    let t = if len > 1e-6 { (along(self.leg_from, to, body.pos) + LOOK_AHEAD / len).clamp(0.0, 1.0) } else { 1.0 };
                    self.leg_from + (to - self.leg_from) * t
                }
                None => target,
            }
        };
        let off = self.face(waypoint, body, turn, dt);
        let go = pace * off.cos().max(0.0);
        Controls { walk: Vec2::new(0.0, go), sprint: lunge && off.abs() < 0.5, jump: false, crouch_toggle: false }
    }
}

/// What a hit takes off: a shot to the head many times over; a blow the
/// same wherever it lands.
pub fn dealt(damage: f64, head: bool, blow: bool) -> f64 {
    damage * if head && !blow { HEADSHOT } else { 1.0 }
}

/// How far along the way from `a` to `b` the point `p` is, flat: 0 at `a`,
/// 1 at `b`.
fn along(a: Vec3, b: Vec3, p: Vec3) -> f64 {
    let ab = Vec2::new(b.x - a.x, b.z - a.z);
    let len2 = ab.dot(ab);
    if len2 < 1e-9 {
        return 1.0;
    }
    Vec2::new(p.x - a.x, p.z - a.z).dot(ab) / len2
}

/// Whether feet at `b` are near enough the level of feet at `a` to strike.
fn level(a: Vec3, b: Vec3) -> bool {
    (-REACH_DOWN..=REACH_UP).contains(&(b.y - a.y))
}

fn flat_dist(a: Vec3, b: Vec3) -> f64 {
    Vec2::new(a.x - b.x, a.z - b.z).length()
}
