//! HOLDOUT: the other way to play. No stash, no loot, no way out: a
//! pistol and a knife, an arena (`arena.rs`) and round after round of the
//! dead (`rounds.rs`) coming in by its windows. Hits and kills earn points;
//! points buy guns and kits off the walls and open the doors to more of
//! the arena. Boards torn off the windows can be nailed back, for a few
//! points more. It lasts as long as the player does.

pub mod arena;
pub mod hud;
mod land;
mod layout;
pub mod props;
mod raise;
mod relay;
pub mod rounds;

use bevy_ecs::prelude::*;
use lntrn_math::Vec3;

use crate::loot::bag::{Bag, Slot};
use crate::loot::{Kind, Stack};
use crate::sound::Sfx;
use crate::stats::Stats;
use crate::world::Solid;
use crate::zombie::breach::{Barrier, Barriers};
use arena::{Arena, Wares};
use rounds::Rounds;

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
/// A holdout's pack and pockets: room for all the rounds bought.
pub const PACK: (u8, u8) = (8, 8);
const POCKETS: (u8, u8) = (4, 2);

/// What a weapon off the wall costs; its rounds cost half.
pub fn price(kind: Kind) -> u32 {
    match kind {
        Kind::Pistol => 250,
        Kind::Rifle => 500,
        Kind::Machete => 600,
        Kind::Shotgun => 900,
        Kind::FireAxe => 1000,
        Kind::Smg => 1200,
        Kind::AssaultRifle => 1800,
        Kind::Flamethrower => 2500,
        Kind::Lmg => 3000,
        Kind::Bandage => 200,
        Kind::Medkit => 600,
        _ => 1000,
    }
}

/// A weapon's rounds, bought with it or after: so many magazines' worth
/// carried spare (fewer of an LMG's belts and a flamethrower's tanks:
/// each is a lot).
fn spare(kind: Kind) -> Option<(Kind, u32)> {
    let spec = kind.weapon()?.spec();
    let ammo = spec.ammo?;
    let mags = match kind {
        Kind::Pistol => 10,
        Kind::Lmg | Kind::Flamethrower => 4,
        _ => 12,
    };
    Some((ammo, spec.mag * mags))
}

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
}

impl Holdout {
    /// The arena, its first zone open, every window boarded up, for
    /// `players`.
    pub fn new(arena: Arena, seed: u32, players: usize) -> Self {
        let mut open = vec![false; arena.zones.len()];
        open[arena.start] = true;
        let opened = vec![false; arena.doors.len()];
        let players = players.max(1);
        Self { arena, rounds: Rounds::new(seed, players), open, opened, wallets: vec![Wallet::new(); players] }
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
        if let Some((ammo, n)) = spare(Kind::Pistol) {
            bag.add(Stack::new(ammo, n / 2));
        }
        bag
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
    /// What came of it: a round begun, or cleared.
    pub fn update(&mut self, world: &mut World, feet: &[Vec3], dt: f64) -> Option<rounds::Event> {
        for w in &mut self.wallets {
            for p in &mut w.pops {
                p.1 += dt;
            }
            w.pops.retain(|p| p.1 < hud::POP_FOR);
        }
        let event = self.rounds.update(world, &self.arena, &self.open, feet, dt);
        if let Some(rounds::Event::Began(_)) = event {
            for w in &mut self.wallets {
                w.nailed = 0;
            }
        }
        event
    }

    /// Every gun in `bag`'s slots, its rounds topped up to a full carry
    /// (what a hound round cleared is worth).
    pub fn max_ammo(bag: &mut Bag) {
        for slot in Slot::ALL {
            let Some((ammo, most)) = bag.slot(slot).and_then(|gun| spare(gun.kind)) else { continue };
            let have = bag.count(ammo);
            if have < most {
                bag.add(Stack::new(ammo, most - have));
            }
        }
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

    /// What using `aimed` would do, in words.
    pub fn prompt(&self, aimed: Aimed, bag: &Bag) -> String {
        match aimed {
            Aimed::Buy(i) => match self.arena.buys[i].wares {
                Wares::Weapon(kind) => {
                    let name = kind.def().name;
                    if !has(bag, kind) {
                        format!("BUY {name} [{}]", price(kind))
                    } else if spare(kind).is_some() {
                        format!("BUY {name} AMMO [{}]", price(kind) / 2)
                    } else {
                        format!("{name} · CARRIED")
                    }
                }
                Wares::Kit(kind) => format!("BUY {} [{}]", kind.def().name, price(kind)),
            },
            Aimed::Door(i) => {
                let door = &self.arena.doors[i];
                format!("{} [{}]", if door.heap { "CLEAR DEBRIS" } else { "OPEN DOOR" }, door.cost)
            }
            Aimed::Window(_) => "HOLD TO REBUILD BARRIER".to_string(),
        }
    }

    /// Player `seat` uses `aimed` (E pressed): buys what's on the wall
    /// (into `bag`), or opens the door, from their own points. What
    /// happened: the sound to play, a word for them, and the slot of a
    /// weapon just bought (to take it up).
    pub fn press(&mut self, world: &mut World, seat: usize, aimed: Aimed, bag: &mut Bag) -> (Option<Sfx>, Option<&'static str>, Option<Slot>) {
        let Some(points) = self.wallets.get(seat).map(|w| w.points) else { return (None, None, None) };
        match aimed {
            Aimed::Buy(i) => self.buy(i, seat, bag),
            Aimed::Door(i) => {
                let cost = self.arena.doors[i].cost;
                if points < cost {
                    return (Some(Sfx::DryFire), Some("NOT ENOUGH POINTS"), None);
                }
                self.wallets[seat].points -= cost;
                self.open_door(world, i);
                (Some(if self.arena.doors[i].heap { Sfx::Rummage } else { Sfx::Unlock }), None, None)
            }
            Aimed::Window(_) => (None, None, None),
        }
    }

    fn buy(&mut self, i: usize, seat: usize, bag: &mut Bag) -> (Option<Sfx>, Option<&'static str>, Option<Slot>) {
        let (kind, cost, ammo_only) = match self.arena.buys[i].wares {
            Wares::Weapon(kind) if has(bag, kind) => match spare(kind) {
                Some(_) => (kind, price(kind) / 2, true),
                None => return (None, None, None),
            },
            Wares::Weapon(kind) | Wares::Kit(kind) => (kind, price(kind), false),
        };
        if self.wallets[seat].points < cost {
            return (Some(Sfx::DryFire), Some("NOT ENOUGH POINTS"), None);
        }
        if let Some((ammo, most)) = spare(kind) {
            // Topped up to a full carry (none paid for that's already full).
            let have = bag.count(ammo);
            if ammo_only && have >= most {
                return (None, Some("AMMO FULL"), None);
            }
            if have < most {
                bag.add(Stack::new(ammo, most - have));
            }
        }
        let mut took = None;
        if !ammo_only && let Some(slot) = Slot::of(kind) {
            let mut weapon = Stack::one(kind);
            weapon.loaded = kind.weapon().map_or(0, |w| w.spec().mag);
            *bag.slot_mut(slot) = Some(weapon);
            took = Some(slot);
        } else if !ammo_only && bag.add(Stack::one(kind)).count > 0 {
            return (None, Some("NO ROOM"), None);
        }
        self.wallets[seat].points -= cost;
        (Some(if ammo_only || took.is_none() { Sfx::Pickup } else { Sfx::SlideRack }), None, took)
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

/// Whether `kind` is carried in its slot.
fn has(bag: &Bag, kind: Kind) -> bool {
    Slot::of(kind).is_some_and(|s| bag.slot(s).is_some_and(|st| st.kind == kind))
}

fn flat_dist(a: Vec3, b: Vec3) -> f64 {
    lntrn_math::Vec2::new(a.x - b.x, a.z - b.z).length()
}

#[cfg(test)]
mod cellar_tests;

#[cfg(test)]
mod dump;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod waves_tests;
