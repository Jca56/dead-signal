//! What a Shambler senses and decides, a fixed step at a time. It wanders
//! until it sees the player (ahead of it, near enough, nothing between) or
//! hears a shot; hunts what it saw, walking the nav grid round what's in
//! the way; lunges the last couple of metres; swipes when it's close. Lose
//! it for long enough and it searches where it last saw it, then wanders.
//! Nothing here draws or plays: it says what it wants done.

use bevy_ecs::prelude::*;
use lntrn_math::{Vec2, Vec3};

use super::figure::Clip;
use super::kind::Kind;
use super::spit;
use super::nav::NavGrid;
use super::steer::{flat_dist, level};
use crate::collide::Solids;
use crate::player::{Body, Controls, Gait};
use crate::sound::Sfx;

/// How often it looks for the player, seconds (sight is the dearest sense:
/// a ray through the world).
const LOOK_EVERY: f64 = 0.1;
/// How far it sees, and how wide (either side of straight ahead).
pub const SIGHT: f64 = 25.0;
const SIGHT_HALF: f64 = 55.0;
/// Closer than this it knows you're there, whichever way it faces.
const CLOSE: f64 = 2.5;
/// How far a shot is heard, and another's snarl on seeing the player.
pub const HEARING: f64 = 80.0;
pub const ALERT_RANGE: f64 = 15.0;
/// How far above its feet the player's feet can be and still be struck
/// (standing on a crate: the swipe takes the legs), and how far below.
pub(super) const REACH_UP: f64 = 1.0;
pub(super) const REACH_DOWN: f64 = 1.2;
/// How long a shot staggers it, and a blow (which sends it stumbling back).
const FLINCH_TIME: f64 = 0.33;
const STUMBLE_TIME: f64 = 0.6;
/// How long out of sight before it goes to look where it last saw you.
const FORGET: f64 = 8.0;
/// Wandering: how far it strays, and at what share of its walk.
const WANDER_RANGE: f64 = 12.0;
const WANDER_PACE: f64 = 0.45;
/// How fast it turns when not hunting, rad/s (hunting, its kind's).
const TURN_IDLE: f64 = 1.5;
/// How often it finds its way again, seconds, and how long it waits after
/// finding none (a search that fails is the dearest there is).
pub(super) const REPATH: f64 = 0.6;
pub(super) const NO_WAY: f64 = 2.0;
/// Cut off from what it's after, it comes this near and waits there,
/// rather than pushing into the crowd at the one nearest spot.
pub(super) const GATHER: f64 = 3.0;
/// Following a route: this near a turning point it has reached it, and it
/// aims this far ahead along the leg it's on.
pub(super) const ARRIVE: f64 = 0.3;
pub(super) const LOOK_AHEAD: f64 = 0.7;
/// Its eyes, and where it looks for yours, above the feet.
const EYE: f64 = 1.55;
const PLAYER_EYE: f64 = 1.5;
/// A stride of its walk animation covers this much ground, and of its
/// run (a Ripper's).
const STRIDE: f64 = 1.1;
const WALK_CLIP: f64 = 0.8;
const RUN_STRIDE: f64 = 3.4;
const RUN_CLIP: f64 = 0.5;
/// A shot to the head does this many times a body shot's damage (blows do
/// the same wherever they land).
pub const HEADSHOT: f64 = 3.0;
pub const BLOW_HEADSHOT: f64 = 1.5;

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
    /// A Spitter heaving up a glob: `t` into it, and whether it's thrown.
    Spit { t: f64, thrown: bool },
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
    /// How many more may find their way this step, shared by the crowd:
    /// a horde whose quarry moves all want to at once, and the searches
    /// are spread over a few steps instead.
    pub searches: &'a std::cell::Cell<u32>,
    /// How far it sees, a share of [`SIGHT`] (the player can be hard to
    /// spot).
    pub sight: f64,
}

/// A blow that landed on the player: the way it pushes, how much it takes
/// off, and what it leaves.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Blow {
    pub push: Vec3,
    pub damage: f64,
    pub leaves: Option<crate::vitals::Affliction>,
}

/// What it wants done this step.
#[derive(Default)]
pub struct Intent {
    pub controls: Controls,
    pub sounds: Vec<(Sfx, f32)>,
    /// Its blow landed on the player.
    pub hit: Option<Blow>,
    /// It has just seen the player, there, and snarled for the rest.
    pub alert: Option<Vec3>,
    /// A glob thrown: from where, at where.
    pub spit: Option<(Vec3, Vec3)>,
    /// A dead Spitter burst, just now.
    pub burst: bool,
}

#[derive(Component, Clone, Debug)]
pub struct Zombie {
    pub kind: Kind,
    pub hp: f64,
    pub state: State,
    pub yaw: f64,
    pub gait: Gait,
    pub(super) path: Vec<Vec3>,
    pub(super) path_goal: Vec3,
    /// Where the leg of the route it's on began.
    pub(super) leg_from: Vec3,
    /// Till it next looks, and whether it saw the player when it last did.
    look_in: f64,
    in_sight: bool,
    /// What it's after can't be got to (up on a car, say): the way found
    /// ends short of it.
    pub(super) cut_off: bool,
    pub(super) repath: f64,
    last_seen: Option<Vec3>,
    unseen: f64,
    cooldown: f64,
    /// Till a Spitter may spit again.
    spit_in: f64,
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
    /// A Shambler facing `yaw`, its own ways from `seed`.
    #[cfg(test)]
    pub fn new(yaw: f64, seed: u32) -> Self {
        Self::of(Kind::Shambler, yaw, seed)
    }

    /// One of `kind` facing `yaw`, its own ways from `seed`.
    pub fn of(kind: Kind, yaw: f64, seed: u32) -> Self {
        let t = kind.traits();
        let mut z = Self { kind, hp: t.hp, state: State::Wander { goal: None, rest: 1.0 }, yaw, gait: Gait { walk: 0.0, sprint: 0.0, crouch: 0.0 }, path: Vec::new(), path_goal: Vec3::ZERO, leg_from: Vec3::ZERO, look_in: 0.0, in_sight: false, cut_off: false, repath: 0.0, last_seen: None, unseen: 0.0, cooldown: 0.0, spit_in: 0.0, groan: 0.0, shuffle: 0.0, walked: 0.0, clip_t: 0.0, moving: false, seed: seed | 1 };
        z.groan = 2.0 + 5.0 * z.rand();
        let (lo, hi, lunge) = if z.rand() < t.fast_share { t.fast } else { t.pace };
        let walk = lo + (hi - lo) * z.rand();
        z.gait = Gait { walk, sprint: lunge, crouch: walk };
        // Each finds its way and looks about on its own beat: a crowd come
        // in together never all search at once.
        z.repath = REPATH * z.rand();
        z.look_in = LOOK_EVERY * z.rand();
        z
    }

    pub(super) fn rand(&mut self) -> f64 {
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
            State::Attack { t, .. } if self.kind == Kind::Ripper => (Clip::Slash, t),
            State::Spit { t, .. } => (Clip::Spit, t),
            State::Attack { t, .. } => (Clip::Attack, t),
            State::Stagger { t, until } if until > FLINCH_TIME => (Clip::Stumble, t),
            State::Stagger { t, .. } => (Clip::Flinch, t),
            _ if self.moving && self.kind == Kind::Ripper && matches!(self.state, State::Hunt | State::Search(_)) => (Clip::Run, self.walked / RUN_STRIDE * RUN_CLIP),
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
        } else if !matches!(self.state, State::Attack { .. } | State::Stagger { .. } | State::Spit { .. }) {
            self.state = State::Hunt;
        }
        false
    }

    /// Whether it has no idea the player's there: wandering, or gone to
    /// look at a noise.
    pub fn unaware(&self) -> bool {
        matches!(self.state, State::Wander { .. } | State::Investigate { .. })
    }

    /// Sent stumbling back (a blast at close range), if it's alive.
    pub fn stumble(&mut self) {
        if !self.dead() {
            self.set(State::Stagger { t: 0.0, until: STUMBLE_TIME });
        }
    }

    /// Whether it sees a player standing at `player`, as far as `reach`
    /// of its sight.
    fn sees(&self, body: &Body, player: Vec3, solids: &Solids, reach: f64) -> bool {
        let to = Vec2::new(player.x - body.pos.x, player.z - body.pos.z);
        let d = to.length();
        if d > SIGHT * reach * self.kind.traits().sight {
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
        let traits = self.kind.traits();
        let swipe = traits.swipe;
        self.clip_t += dt;
        if let State::Dead { t } = &mut self.state {
            // A dead Spitter swells, then bursts.
            out.burst = self.kind == Kind::Spitter && *t < spit::BURST_AT && *t + dt >= spit::BURST_AT;
            *t += dt;
            self.moving = false;
            return out;
        }
        self.spit_in -= dt;
        self.cooldown -= dt;
        self.repath -= dt;
        self.groan -= dt;

        // What it sees and hears.
        // Looking takes a moment; between looks it goes on what it last saw
        // (and where the player is now, if that was them).
        self.look_in -= dt;
        if self.look_in <= 0.0 {
            self.look_in += LOOK_EVERY;
            self.in_sight = s.player.is_some_and(|p| self.sees(body, p, s.solids, s.sight));
        }
        let seen = s.player.filter(|_| self.in_sight);
        if let Some(p) = seen {
            if matches!(self.state, State::Wander { .. } | State::Search(_) | State::Investigate { .. }) {
                out.sounds.push((traits.snarl, 1.0));
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
                    self.face(p, body, traits.turn, dt);
                    if !struck && t >= swipe.strike_at {
                        struck = true;
                        let to = Vec3::new(p.x - body.pos.x, 0.0, p.z - body.pos.z);
                        let d = to.length();
                        let facing = Vec3::new(-self.yaw.sin(), 0.0, -self.yaw.cos());
                        if d <= swipe.reach && level(body.pos, p) && (d < 1e-6 || facing.dot(to * (1.0 / d)) > 0.5) {
                            let push = if d > 1e-6 { to * (1.0 / d) } else { facing };
                            out.hit = Some(Blow { push, damage: swipe.damage, leaves: swipe.leaves });
                        }
                    }
                }
                if t >= swipe.time {
                    self.cooldown = swipe.cooldown;
                    self.set(State::Hunt);
                } else {
                    self.state = State::Attack { t, struck };
                }
            }
            State::Spit { t, thrown } => {
                let t = t + dt;
                let mut thrown = thrown;
                if let Some(p) = s.player {
                    self.face(p, body, traits.turn, dt);
                    if !thrown && t >= spit::SPIT_AT {
                        thrown = true;
                        let facing = Vec3::new(-self.yaw.sin(), 0.0, -self.yaw.cos());
                        out.spit = Some((body.pos + Vec3::new(0.0, spit::MOUTH, 0.0) + facing * 0.4, p));
                        out.sounds.push((Sfx::Spit, 1.0));
                    }
                }
                if t >= spit::SPIT_TIME {
                    self.spit_in = spit::SPIT_EVERY * (0.8 + 0.4 * self.rand());
                    self.set(State::Hunt);
                } else {
                    self.state = State::Spit { t, thrown };
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
                    if seen.is_some() && d <= swipe.range && level(body.pos, p) && self.cooldown <= 0.0 {
                        out.sounds.push((traits.snarl, 0.8));
                        self.set(State::Attack { t: 0.0, struck: false });
                    } else if self.kind == Kind::Spitter {
                        // It keeps off: spits from its distance (when it can
                        // see them), backs away from too near, closes from
                        // too far; turned away, it goes by where it saw them.
                        if seen.is_some() && self.spit_in <= 0.0 && (spit::SPIT_NEAREST..=spit::SPIT_FARTHEST).contains(&d) {
                            out.sounds.push((traits.snarl, 0.7));
                            self.set(State::Spit { t: 0.0, thrown: false });
                        } else if d < spit::KEEP_NEAR {
                            let away = Vec3::new(body.pos.x - p.x, 0.0, body.pos.z - p.z) * (1.0 / d.max(1e-6));
                            goal = Some((body.pos + away * 4.0, 1.0, false, traits.turn));
                        } else if d > spit::KEEP_FAR {
                            goal = Some((p, 1.0, false, traits.turn));
                        } else {
                            self.face(p, body, traits.turn, dt);
                        }
                    } else if seen.is_some() && d < swipe.range * 0.8 && level(body.pos, p) {
                        // Right on top of them, waiting on its next swipe:
                        // it stands its ground and turns to face them (a fast
                        // one would run on through and past).
                        self.face(p, body, traits.turn, dt);
                    } else {
                        goal = Some((p, 1.0, seen.is_some() && d < traits.lunge, traits.turn));
                    }
                } else {
                    self.state = State::Wander { goal: None, rest: 1.0 };
                }
            }
            State::Search(at) => {
                if flat_dist(body.pos, at) < 1.5 {
                    self.state = State::Wander { goal: None, rest: 2.0 };
                } else {
                    goal = Some((at, 1.0, false, traits.turn));
                }
            }
            State::Investigate { at, looked } => {
                if flat_dist(body.pos, at) < 2.0 || looked > 0.0 {
                    let looked = looked + dt;
                    self.state = if looked > 3.0 { State::Wander { goal: None, rest: 1.0 } } else { State::Investigate { at, looked } };
                } else {
                    goal = Some((at, 0.8, false, traits.turn));
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
            out.controls = self.steer(body, target, pace, lunge, turn, s.nav, s.searches, dt);
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
}

/// What a hit takes off: a shot to the head many times over, a blow to
/// it half again.
pub fn dealt(damage: f64, head: bool, blow: bool) -> f64 {
    damage * match (head, blow) {
        (true, false) => HEADSHOT,
        (true, true) => BLOW_HEADSHOT,
        _ => 1.0,
    }
}
