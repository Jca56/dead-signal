//! The rounds: how many of the dead each brings (more for more players),
//! which kinds, how tough and how fast they are, and bringing them in by
//! the windows of the open zones (the nearer a player, the likelier: each
//! player in turn), never too many up at once. Later rounds bring Rippers
//! and Spitters among them; now and then a Juggernaut comes too; and
//! every few rounds isn't the dead at all but Hellhounds, out of the
//! static (`zombie/rift.rs`). A round is over when all of it is dead but
//! its Juggernaut, which stays, round after round, till it's killed; a
//! short breather (the radio saying what's next), and the next begins.

use bevy_ecs::prelude::*;
use lntrn_math::Vec3;

use super::arena::Arena;
use crate::loot::Dice;
use crate::zombie::{self, brain::{State, Zombie}, kind::Kind, looks::Theme, rift};

/// The most of the dead up at once; each player past the first, this many
/// more, and a round brings this share of its dead again.
pub const MOST_UP: usize = 20;
const MORE_UP: usize = 6;
const MORE_DEAD: f64 = 0.5;
/// Seconds before the first round, and between rounds.
const FIRST_WAIT: f64 = 4.0;
pub const BREATHER: f64 = 10.0;
/// Seconds between the dead coming in, early on, and at the quickest.
const SPAWN_EVERY: (f64, f64) = (2.0, 0.5);
/// Of the open windows, the dead come in by one of this many nearest a
/// player.
const NEAREST: usize = 4;
/// The round Rippers first come with the dead, and Spitters; the share of
/// a round each is then, how much more each round after, and at most.
pub const RIPPERS_FROM: u32 = 6;
pub const SPITTERS_FROM: u32 = 9;
const RIPPERS: (f64, f64, f64) = (0.06, 0.02, 0.2);
const SPITTERS: (f64, f64, f64) = (0.04, 0.01, 0.1);
/// The first round a Juggernaut comes with, how many rounds till the
/// next, and how many rounds on another comes with it.
const BOSS_FROM: u32 = 10;
const BOSS_EVERY: u32 = 5;
const BOSS_MORE: u32 = 10;
/// The first hound round (or the one after), and how many rounds on the
/// next (or one more).
const HOUNDS_FROM: u32 = 5;
const HOUNDS_EVERY: u32 = 4;
/// Hounds in the first hound round, more each one after, at most; how
/// many up at once (and more a player past the first); seconds between
/// rifts, early on, less each hound round, and at the quickest.
const HOUNDS: (u32, u32, u32) = (8, 2, 24);
const HOUNDS_UP: (usize, usize) = (4, 2);
/// The first hound round's are let off a little: this share as tough as
/// a hound of their round, and biting this share as hard.
pub const FIRST_PACK: (f64, f64) = (0.75, 0.85);
const RIFT_EVERY: (f64, f64, f64) = (3.0, 0.3, 1.5);
/// How fast the air goes bad as a hound round begins, and clears after,
/// a second.
const GLOOM: (f64, f64) = (0.5, 0.3);

/// What a round is: the dead by the windows (and how many Juggernauts
/// with them), or Hellhounds out of the static.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wave {
    Dead { boss: u32 },
    Hounds,
}

/// What a step of the rounds came to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Began(Wave),
    /// The last of a round is dead (and the next is known: `Rounds::next`).
    Cleared(Wave),
    /// A Juggernaut's been killed: whose kill it was (their seat).
    Felled(Option<usize>),
}

/// How many of the dead a round brings.
pub fn count(round: u32) -> u32 {
    match round {
        0 => 0,
        1..=5 => [6, 8, 11, 14, 18][round as usize - 1],
        _ => 18 + 2 * (round - 5),
    }
}

/// How many of the dead a round brings for `players`.
pub fn brings(round: u32, players: usize) -> u32 {
    crowd(count(round), players)
}

/// `n` of them, as many more as `players` call for.
fn crowd(n: u32, players: usize) -> u32 {
    (f64::from(n) * (1.0 + MORE_DEAD * players.saturating_sub(1) as f64)).round() as u32
}

/// The most of the dead up at once for `players`.
pub fn most_up(players: usize) -> usize {
    MOST_UP + MORE_UP * players.saturating_sub(1)
}

/// How much a Shambler of a round takes to kill: a hundred in the first,
/// twenty more each round to the tenth, then a twentieth more each.
/// (Gentle: it's a horde, and a horde's to be cut down. The guns are the
/// ones made for the wilds, where the dead never toughen; the Amplifier
/// answers the late rounds.)
pub fn toughness(round: u32) -> f64 {
    if round <= 10 { 100.0 + 20.0 * f64::from(round.saturating_sub(1)) } else { 280.0 * 1.05f64.powi(round as i32 - 10) }
}

/// What a round's Hellhounds and its Juggernaut are measured by instead: a
/// steeper climb (150 in the first, fifty more each round to 600 at the
/// tenth, then a twelfth or so more each). They're the fights they were
/// made to be, however quick the horde's cut down.
fn stoutness(round: u32) -> f64 {
    if round <= 10 { 150.0 + 50.0 * f64::from(round.saturating_sub(1)) } else { 600.0 * 1.08f64.powi(round as i32 - 10) }
}

/// How much one of `kind` takes to kill in `round`: a share of the
/// round's Shambler (a Ripper half, a Spitter three quarters); or, a
/// Hellhound and a Juggernaut, of the steeper measure (two fifths of it,
/// and fifteen times it).
pub fn health(round: u32, kind: Kind) -> f64 {
    match kind {
        Kind::Shambler => toughness(round),
        Kind::Ripper => toughness(round) * 0.5,
        Kind::Spitter => toughness(round) * 0.75,
        Kind::Juggernaut => stoutness(round) * 15.0,
        Kind::Hound => stoutness(round) * 0.4,
    }
}

/// The share of a round that's one of the special dead: none before the
/// round they come `from`, then `(at first, more a round, at most)`.
fn share(round: u32, from: u32, (first, more, most): (f64, f64, f64)) -> f64 {
    if round < from { 0.0 } else { (first + more * f64::from(round - from)).min(most) }
}

/// The dead a round brings for `players`, in no order: Shamblers, and in
/// time Rippers and Spitters among them, and `boss` Juggernauts besides.
pub fn muster(round: u32, players: usize, boss: u32) -> Vec<Kind> {
    let n = brings(round, players);
    let of = |s: f64| (f64::from(n) * s).round() as u32;
    let (rippers, spitters) = (of(share(round, RIPPERS_FROM, RIPPERS)), of(share(round, SPITTERS_FROM, SPITTERS)));
    let mut all = vec![Kind::Shambler; (n - rippers - spitters) as usize];
    all.extend(std::iter::repeat_n(Kind::Ripper, rippers as usize));
    all.extend(std::iter::repeat_n(Kind::Spitter, spitters as usize));
    all.extend(std::iter::repeat_n(Kind::Juggernaut, boss as usize));
    all
}

/// How many hounds the `nth` hound round (the first is 1) brings for
/// `players`.
pub fn pack(nth: u32, players: usize) -> u32 {
    crowd((HOUNDS.0 + HOUNDS.1 * nth.saturating_sub(1)).min(HOUNDS.2), players)
}

/// How fast one of a round walks, m/s, by luck (`roll`, `pick` 0–1): all
/// shamble at first; from the third round some jog, from the sixth some
/// run, more each round.
pub fn pace(round: u32, roll: f64, pick: f64) -> f64 {
    let r = f64::from(round);
    let runs = ((r - 5.0) * 0.1).clamp(0.0, 0.8);
    let jogs = ((r - 2.0) * 0.15).clamp(0.0, 0.6);
    let (lo, hi) = if roll < runs {
        (5.0, 5.6)
    } else if roll < runs + jogs * (1.0 - runs) {
        (3.0, 3.5)
    } else {
        (1.4, 1.9)
    };
    lo + (hi - lo) * pick
}

#[derive(Clone, Debug)]
pub struct Rounds {
    /// The round on (0 before the first), what it is, and what the next
    /// will be (known from the moment this one's over).
    pub round: u32,
    pub wave: Wave,
    pub next: Wave,
    /// Still to come in this round (the last comes first).
    queue: Vec<Kind>,
    /// Till the next round, while between them; since this one began.
    pub between: f64,
    pub begun: f64,
    spawn_in: f64,
    dice: Dice,
    /// How many are playing, and which of them the next of the dead comes
    /// in near.
    players: usize,
    turn: usize,
    /// The next hound round, how many there have been, and the next round
    /// a Juggernaut may come with.
    hounds_at: u32,
    packs: u32,
    boss_at: u32,
    /// How bad the air's gone (a hound round's fog), 0–1.
    pub gloom: f64,
    /// What the next round's been told to be (the dev's doing).
    forced: Option<Wave>,
    /// The Juggernauts brought in and not yet killed.
    bosses: Vec<Entity>,
}

impl Rounds {
    pub fn new(seed: u32, players: usize) -> Self {
        let mut dice = Dice(seed | 1);
        let hounds_at = HOUNDS_FROM + dice.next() % 2;
        Self { round: 0, wave: Wave::Dead { boss: 0 }, next: Wave::Dead { boss: 0 }, queue: Vec::new(), between: FIRST_WAIT, begun: 0.0, spawn_in: 0.0, dice, players, turn: 0, hounds_at, packs: 0, boss_at: BOSS_FROM, gloom: 0.0, forced: None, bosses: Vec::new() }
    }

    /// Whether it's a breather between rounds.
    pub fn resting(&self) -> bool {
        self.between > 0.0
    }

    /// What the radio has to say, in a breather, of the round to come.
    pub fn warning(&self) -> Option<&'static str> {
        if !self.resting() {
            return None;
        }
        match (self.next, self.round + 1) {
            (Wave::Hounds, _) => Some("HOUNDS ON THE SIGNAL"),
            (Wave::Dead { boss: 1.. }, _) => Some("SOMETHING BIG IS COMING"),
            (_, RIPPERS_FROM) => Some("RIPPERS IN THE TREELINE"),
            (_, SPITTERS_FROM) => Some("SPITTERS IN THE TREELINE"),
            _ => None,
        }
    }

    /// (The dev's.) No more of this round is to come, `skip` rounds are
    /// passed over, and the next is to be `wave` (if any's said).
    pub fn force(&mut self, wave: Option<Wave>, skip: u32) {
        self.queue.clear();
        self.round += skip;
        match wave {
            Some(wave) if self.resting() => self.next = wave,
            wave => self.forced = wave,
        }
    }

    /// The Juggernauts killed since last asked: each, whose kill it was.
    pub fn fallen(&mut self, world: &World) -> Vec<Option<usize>> {
        let mut fallen = Vec::new();
        self.bosses.retain(|&e| match world.get::<Zombie>(e) {
            Some(z) if !z.dead() => true,
            gone => {
                fallen.push(gone.and_then(|z| z.by));
                false
            }
        });
        fallen
    }

    /// What `round` is to be: hounds, when their round's come; else the
    /// dead, a Juggernaut with them every so often.
    fn plan(&mut self, round: u32) -> Wave {
        if round >= self.hounds_at {
            self.hounds_at = round + HOUNDS_EVERY + self.dice.next() % 2;
            return Wave::Hounds;
        }
        if round >= self.boss_at {
            self.boss_at = round + BOSS_EVERY;
            return Wave::Dead { boss: 1 + (round - BOSS_FROM) / BOSS_MORE };
        }
        Wave::Dead { boss: 0 }
    }

    /// The round `self.round` begins: all of it, waiting to come.
    fn begin(&mut self) {
        self.wave = self.next;
        self.queue = match self.wave {
            Wave::Hounds => {
                self.packs += 1;
                vec![Kind::Hound; pack(self.packs, self.players) as usize]
            }
            Wave::Dead { boss } => {
                let mut all = muster(self.round, self.players, 0);
                for i in (1..all.len()).rev() {
                    all.swap(i, self.dice.next() as usize % (i + 1));
                }
                // A Juggernaut comes once half the round is in.
                let mid = all.len() / 2;
                all.splice(mid..mid, std::iter::repeat_n(Kind::Juggernaut, boss as usize));
                all
            }
        };
    }

    /// A step of the rounds, with the players' feet at `feet` and zones
    /// `open`: the next brought in, if it's time; the round over, or the
    /// next begun.
    pub fn update(&mut self, world: &mut World, arena: &Arena, open: &[bool], feet: &[Vec3], dt: f64) -> Option<Event> {
        let hounds = self.wave == Wave::Hounds && !self.resting();
        self.gloom = if hounds { (self.gloom + GLOOM.0 * dt).min(1.0) } else { (self.gloom - GLOOM.1 * dt).max(0.0) };
        if self.between > 0.0 {
            self.between -= dt;
            if self.between > 0.0 {
                return None;
            }
            self.round += 1;
            self.begun = 0.0;
            self.spawn_in = 0.5;
            self.begin();
            return Some(Event::Began(self.wave));
        }
        self.begun += dt;
        // (A Juggernaut still up is no part of the count: it stays, and
        // the rounds go on round it.)
        let up = zombie::alive(world) - zombie::alive_of(world, Kind::Juggernaut) + rift::pending(world);
        if self.queue.is_empty() {
            if up == 0 {
                self.between = BREATHER;
                self.next = match self.forced.take() {
                    Some(wave) => wave,
                    None => self.plan(self.round + 1),
                };
                return Some(Event::Cleared(self.wave));
            }
            return None;
        }
        self.spawn_in -= dt;
        let most = if hounds { HOUNDS_UP.0 + HOUNDS_UP.1 * self.players.saturating_sub(1) } else { most_up(self.players) };
        if self.spawn_in > 0.0 || up >= most || feet.is_empty() {
            return None;
        }
        // Each player in turn.
        let near = feet[self.turn % feet.len()];
        let kind = *self.queue.last()?;
        // (A hound with nowhere inside to come out of the air comes by a
        // window, like the rest.)
        let came = if kind == Kind::Hound { self.tear(world, near, feet) || self.bring(world, arena, open, near, kind) } else { self.bring(world, arena, open, near, kind) };
        if came {
            self.turn += 1;
            self.queue.pop();
        }
        None
    }

    /// A hound of this round: how tough, and how hard it bites (a share
    /// of a hound's bite). The first pack's are the gentler.
    fn hound(&self) -> (f64, f64) {
        let (tough, might) = if self.packs <= 1 { FIRST_PACK } else { (1.0, 1.0) };
        (health(self.round, Kind::Hound) * tough, might)
    }

    /// A rift opened for a hound near the player at `near` (no one of
    /// `feet` too close to it). Whether there was anywhere for it.
    fn tear(&mut self, world: &mut World, near: Vec3, feet: &[Vec3]) -> bool {
        let Some(at) = rift::spot(world, near, feet, || self.dice.unit()) else { return false };
        let (hp, might) = self.hound();
        rift::open(world, at, near, hp, might);
        self.spawn_in = (RIFT_EVERY.0 - RIFT_EVERY.1 * f64::from(self.packs.saturating_sub(1))).max(RIFT_EVERY.2);
        true
    }

    /// One of `kind` brought in by one of the open windows nearest the
    /// player at `near` (a Juggernaut, by the widest of them). Whether
    /// there was a window to come by.
    fn bring(&mut self, world: &mut World, arena: &Arena, open: &[bool], near: Vec3, kind: Kind) -> bool {
        self.spawn_in = (SPAWN_EVERY.0 - 0.15 * f64::from(self.round - 1)).max(SPAWN_EVERY.1);
        let mut windows: Vec<(usize, f64)> = arena.windows.iter().enumerate().filter(|(_, w)| open[w.zone]).map(|(i, w)| (i, (w.outside - near).length())).collect();
        if kind == Kind::Juggernaut {
            let widest = windows.iter().map(|&(i, _)| arena.windows[i].width).fold(0.0, f64::max);
            windows.retain(|&(i, _)| arena.windows[i].width >= widest - 1e-6);
        }
        if windows.is_empty() {
            return false;
        }
        windows.sort_by(|a, b| a.1.total_cmp(&b.1));
        windows.truncate(NEAREST);
        let (i, _) = windows[self.dice.next() as usize % windows.len()];
        let w = &arena.windows[i];
        let jitter = w.across() * ((self.dice.unit() - 0.5) * 2.0 * w.spread) - w.inward * (self.dice.unit() * w.spread);
        let at = w.from + jitter;
        let yaw = (-w.inward.x).atan2(-w.inward.z);
        let e = zombie::spawn_kind(world, at, yaw, kind, Theme::Drifter);
        let (roll, pick) = (self.dice.unit(), self.dice.unit());
        let hound = self.hound();
        if let Some(mut z) = world.get_mut::<Zombie>(e) {
            ready(&mut z, self.round, i as u8, pace(self.round, roll, pick));
            if kind == Kind::Hound {
                (z.hp, z.might) = hound;
            }
        }
        if kind == Kind::Juggernaut {
            self.bosses.push(e);
        }
        true
    }
}

/// One just come in, made a round's: as tough as the round (and its
/// kind), making for window `at`, and always knowing where the player is.
/// A Shambler's as fast as it's picked (`walk`); the special dead go at
/// their own pace.
fn ready(z: &mut Zombie, round: u32, at: u8, walk: f64) {
    z.hp = health(round, z.kind);
    z.relentless = true;
    z.barrier = Some(at);
    z.state = State::Breach { at, t: 0.0 };
    if z.kind == Kind::Shambler {
        z.gait.walk = walk;
        z.gait.crouch = walk;
        z.gait.sprint = z.gait.sprint.max(walk + 1.0);
    }
}

#[cfg(test)]
mod tests;
