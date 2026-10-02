//! HOLDOUT: the other way to play. No stash, no loot, no way out: a
//! pistol and a knife, an arena (`arena.rs`) and round after round of the
//! dead (`rounds.rs`) coming in by its windows. Hits and kills earn points;
//! points buy guns and kits off the walls and open the doors to more of
//! the arena. Boards torn off the windows can be nailed back, for a few
//! points more. It lasts as long as the player does.

pub mod arena;
pub mod drops;
mod buy;
#[cfg(test)]
pub use buy::AMPLIFY;
pub mod hud;
mod land;
pub mod lamps;
mod layout;
mod machine;
pub mod props;
mod raise;
mod relay;
pub mod rounds;

use bevy_ecs::prelude::*;
use lntrn_math::Vec3;

use crate::loot::bag::Bag;
use crate::loot::{Kind, Stack};
use crate::sound::Sfx;
use crate::stats::Stats;
use crate::world::Solid;
use crate::zombie::breach::{Barrier, Barriers};
use arena::Arena;
use buy::spare;
use rounds::Rounds;
// (What the tests of all this name, as they did when it was one file.)
#[cfg(test)]
use {arena::Wares, buy::price, crate::loot::bag::Slot};

/// Points to start with.
const START_POINTS: u32 = 500;
/// Points for a hit (a shot or a blow); for a kill (over the hit's), to
/// the head, with a blade or fist; for a board nailed back (only so many a
/// round pay).
const PER_HIT: u32 = 10;
const PER_KILL: u32 = 50;
const PER_HEADSHOT_KILL: u32 = 90;
const PER_MELEE_KILL: u32 = 120;
const PER_BOARD: u32 = 10;
const PAID_BOARDS: u32 = 20;
/// Points for killing a Juggernaut: whoever killed it, and everyone else.
const PER_JUGGERNAUT: (u32, u32) = (1500, 500);
/// Boards across each window, and seconds to nail one back.
pub const BOARDS: u8 = 6;
const NAIL_EVERY: f64 = 0.5;
/// How near a wall buy or a door is to be used (from the eye), and how
/// straight at it the player must look (the cosine of the angle off it);
/// how near a window's inside to nail its boards (flat, and up or down).
const BUY_REACH: f64 = 1.9;
const DOOR_REACH: f64 = 2.4;
const LOOKING: f64 = 0.8;
const NAIL_REACH: f64 = 1.6;
const NAIL_LEVEL: f64 = 1.0;
/// How high the eye is over the feet.
const EYE: f64 = 1.6;
/// A bandage or a medkit takes this share of its usual time in a holdout:
/// with the dead always on you there, there's no quiet moment for one.
pub const MENDING: f64 = 0.5;
/// A holdout's pack and pockets: room for all the rounds bought.
pub const PACK: (u8, u8) = (8, 8);
const POCKETS: (u8, u8) = (4, 2);

/// The count of what earns points, as it stood.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Tally {
    hits: u32,
    blows: u32,
    kills: u32,
    headshot_kills: u32,
    melee_kills: u32,
    blasts: u32,
}

impl Tally {
    fn of(s: &Stats) -> Self {
        Self { hits: s.hits, blows: s.blows_landed, kills: s.gun_kills, headshot_kills: s.headshot_kills, melee_kills: s.melee_kills, blasts: s.blast_kills + s.burst_kills }
    }
}

/// What's in front of the player to use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Aimed {
    Buy(usize),
    Door(usize),
    Window(usize),
}

/// One player's side of a holdout: their points, and all they've earned;
/// their count as it last stood (what's earned is what's new in it); a
/// window's boards they're nailing back (which, and till the next board)
/// and how many they've been paid for this round; and the points just
/// earned, each with how long it's been up.
#[derive(Clone, Debug)]
pub struct Wallet {
    pub points: u32,
    pub earned: u32,
    seen: Tally,
    nailing: Option<(usize, f64)>,
    nailed: u32,
    pub pops: Vec<(u32, f64)>,
}

impl Wallet {
    fn new() -> Self {
        Self { points: START_POINTS, earned: 0, seen: Tally::default(), nailing: None, nailed: 0, pops: Vec::new() }
    }

    fn earn(&mut self, points: u32) {
        if points > 0 {
            self.points += points;
            self.earned += points;
            self.pops.push((points, 0.0));
        }
    }
}

pub struct Holdout {
    pub arena: Arena,
    pub rounds: Rounds,
    /// Which zones are open, which doors are.
    open: Vec<bool>,
    opened: Vec<bool>,
    /// Each player's, by seat: they earn and spend their own.
    pub wallets: Vec<Wallet>,
    /// What a purchase put out of a player's hands with no room for it in
    /// their pack (whose, and what): to be set down at their feet.
    pub spilled: Vec<(usize, Stack)>,
}

impl Holdout {
    /// The arena, its first zone open, every window boarded up, for
    /// `players`.
    pub fn new(arena: Arena, seed: u32, players: usize) -> Self {
        let mut open = vec![false; arena.zones.len()];
        open[arena.start] = true;
        let opened = vec![false; arena.doors.len()];
        let players = players.max(1);
        Self { arena, rounds: Rounds::new(seed, players), open, opened, wallets: vec![Wallet::new(); players], spilled: Vec::new() }
    }

    /// Player `seat`'s wallet.
    pub fn wallet(&self, seat: usize) -> Option<&Wallet> {
        self.wallets.get(seat)
    }

    /// What the player starts with: a pistol (loaded, and its rounds) and
    /// a knife.
    pub fn loadout() -> Bag {
        let mut bag = Bag::sized(PACK, POCKETS);
        let mut pistol = Stack::one(Kind::Pistol);
        pistol.loaded = Kind::Pistol.weapon().map_or(0, |w| w.spec().mag);
        bag.add(pistol);
        bag.add(Stack::one(Kind::Knife));
        if let Some((ammo, n)) = spare(Kind::Pistol, 0) {
            bag.add(Stack::new(ammo, n / 2));
        }
        bag
    }

    /// What's made of what a player wears: the pack's its own size, with
    /// no backpack to wear.
    pub fn fit() -> crate::loot::bag::Fit {
        crate::loot::bag::Fit { pack: Some(PACK), pockets: POCKETS, pack_bonus: (0, 0) }
    }

    /// The run begins: the windows' boards up, where the dead know them.
    pub fn begin(&mut self, world: &mut World) {
        let list = self.arena.windows.iter().map(|w| Barrier { outside: w.outside, inside: w.inside, boards: BOARDS }).collect();
        world.insert_resource(Barriers(list));
    }

    pub fn door_open(&self, door: usize) -> bool {
        self.opened[door]
    }

    /// Whatever player `seat` earned since last asked, from their count
    /// `stats`.
    pub fn score(&mut self, seat: usize, stats: &Stats) {
        let Some(wallet) = self.wallets.get_mut(seat) else { return };
        let now = Tally::of(stats);
        let was = std::mem::replace(&mut wallet.seen, now);
        let heads = now.headshot_kills - was.headshot_kills;
        let bodies = (now.kills - was.kills).saturating_sub(heads);
        let earned = PER_HIT * (now.hits - was.hits + now.blows - was.blows) + PER_KILL * (bodies + now.blasts - was.blasts) + PER_HEADSHOT_KILL * heads + PER_MELEE_KILL * (now.melee_kills - was.melee_kills);
        wallet.earn(earned);
    }

    /// A step of the holdout, the players' feet at `feet`: the rounds on
    /// (the paid boards counted afresh each round), the points flown off.
    /// What came of it: a round begun or cleared, a Juggernaut killed
    /// (which pays everyone, whoever killed it the most).
    pub fn update(&mut self, world: &mut World, feet: &[Vec3], dt: f64) -> Vec<rounds::Event> {
        for w in &mut self.wallets {
            for p in &mut w.pops {
                p.1 += dt;
            }
            w.pops.retain(|p| p.1 < hud::POP_FOR);
        }
        let mut events = Vec::new();
        for by in self.rounds.fallen(world) {
            for (seat, w) in self.wallets.iter_mut().enumerate() {
                w.earn(if by == Some(seat) { PER_JUGGERNAUT.0 } else { PER_JUGGERNAUT.1 });
            }
            events.push(rounds::Event::Felled(by));
        }
        events.extend(self.rounds.update(world, &self.arena, &self.open, feet, dt));
        if events.iter().any(|e| matches!(e, rounds::Event::Began(_))) {
            for w in &mut self.wallets {
                w.nailed = 0;
            }
        }
        events
    }

    /// What the player at `eye`, looking along `dir`, would use.
    pub fn aimed(&self, world: &World, eye: Vec3, dir: Vec3) -> Option<Aimed> {
        let looking = |at: Vec3, reach: f64| {
            let to = at - eye;
            let d = to.length();
            d < reach && (d < 1e-6 || dir.dot(to * (1.0 / d)) > LOOKING)
        };
        if let Some(i) = self.arena.buys.iter().position(|b| self.open[b.zone] && b.facing.dot(eye - b.at) > 0.0 && looking(b.at, BUY_REACH)) {
            return Some(Aimed::Buy(i));
        }
        if let Some(i) = (0..self.arena.doors.len()).find(|&i| !self.opened[i] && looking((self.arena.doors[i].lo + self.arena.doors[i].hi) * 0.5, DOOR_REACH)) {
            return Some(Aimed::Door(i));
        }
        let boards = world.get_resource::<Barriers>()?;
        let feet = eye - Vec3::new(0.0, EYE, 0.0);
        // (On its floor: not the one below, from upstairs.)
        (0..self.arena.windows.len())
            .find(|&i| {
                let inside = self.arena.windows[i].inside;
                boards.0.get(i).is_some_and(|b| b.boards < BOARDS) && flat_dist(inside, feet) < NAIL_REACH && (inside.y - feet.y).abs() < NAIL_LEVEL
            })
            .map(Aimed::Window)
    }

    /// Door `i` open: gone from the world, the dead's ways through it
    /// open, and the zones either side of it with it.
    fn open_door(&mut self, world: &mut World, i: usize) {
        self.opened[i] = true;
        let door = &self.arena.doors[i];
        world.resource_mut::<Solid>().0.switch(door.solid.clone(), false);
        if let Some(nav) = world.resource_mut::<crate::zombie::Nav>().0.as_mut() {
            nav.shut(&door.gate, false);
        }
        let (a, b) = door.zones;
        self.open[a] = true;
        self.open[b] = true;
    }

    /// (The dev's.) Every door open.
    pub fn open_all(&mut self, world: &mut World) {
        for i in (0..self.arena.doors.len()).filter(|&i| !self.opened[i]).collect::<Vec<_>>() {
            self.open_door(world, i);
        }
    }

    /// (The dev's.) Every window boarded up again.
    pub fn board_up(world: &mut World) {
        if let Some(mut barriers) = world.get_resource_mut::<Barriers>() {
            for b in &mut barriers.0 {
                b.boards = BOARDS;
            }
        }
    }

    /// Player `seat`'s E held (or not) at `aimed`, for `dt`: nailing a
    /// window's boards back, one every so often. The sound of one going up.
    pub fn hold(&mut self, world: &mut World, seat: usize, aimed: Option<Aimed>, held: bool, dt: f64) -> Option<Sfx> {
        let wallet = self.wallets.get_mut(seat)?;
        let Some(Aimed::Window(i)) = aimed.filter(|_| held) else {
            wallet.nailing = None;
            return None;
        };
        let t = match wallet.nailing {
            Some((w, t)) if w == i => t - dt,
            _ => NAIL_EVERY - dt,
        };
        if t > 0.0 {
            wallet.nailing = Some((i, t));
            return None;
        }
        wallet.nailing = Some((i, t + NAIL_EVERY));
        let mut barriers = world.resource_mut::<Barriers>();
        let b = barriers.0.get_mut(i)?;
        if b.boards >= BOARDS {
            return None;
        }
        b.boards += 1;
        if wallet.nailed < PAID_BOARDS {
            wallet.nailed += 1;
            wallet.earn(PER_BOARD);
        }
        Some(Sfx::HitWood)
    }

    /// How far along player `seat`'s next board is (for the ring), while
    /// they're nailing.
    pub fn nail_progress(&self, seat: usize) -> Option<f64> {
        self.wallets.get(seat)?.nailing.map(|(_, t)| 1.0 - t / NAIL_EVERY)
    }
}

fn flat_dist(a: Vec3, b: Vec3) -> f64 {
    lntrn_math::Vec2::new(a.x - b.x, a.z - b.z).length()
}

#[cfg(test)]
mod amplifier_tests;
#[cfg(test)]
mod armor_tests;
#[cfg(test)]
mod cellar_tests;

#[cfg(test)]
mod dump;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod waves_tests;
