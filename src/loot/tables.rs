//! What turns up where: each kind of container (and the dead's pockets)
//! draws a few things from its own table, likelier things more often.

use super::grid::Grid;
use super::{Dice, Kind, Stack};

/// Where things are found.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Source {
    Crate,
    Locker,
    Car,
    Cage,
    Fridge,
    /// A chest of drawers.
    Cabinet,
    Desk,
    Wardrobe,
    /// A store's shelving.
    Shelf,
    /// A store's till counter.
    Register,
    /// In a farmhouse's den: long guns, shells; in a hunter's cabin, the
    /// hunter's own, a rifle likelier than not.
    GunCabinet,
    HunterCabinet,
    /// On one of the dead, sometimes.
    Corpse,
}

/// Every container there is (not the dead).
pub const CONTAINERS: [Source; 12] = [Source::Crate, Source::Locker, Source::Car, Source::Cage, Source::Fridge, Source::Cabinet, Source::Desk, Source::Wardrobe, Source::Shelf, Source::Register, Source::GunCabinet, Source::HunterCabinet];

/// One line of a table: what, how likely against the rest, how many.
type Line = (Kind, u32, (u32, u32));

const CRATE: &[Line] = &[
    (Kind::Rounds, 30, (8, 16)),
    (Kind::Bandage, 20, (1, 2)),
    (Kind::Beans, 20, (1, 1)),
    (Kind::Water, 15, (1, 1)),
    (Kind::Cash, 8, (1, 2)),
    (Kind::Pills, 5, (1, 1)),
    (Kind::Fuel, 4, (1, 1)),
    (Kind::Shells, 12, (4, 10)),
];

const LOCKER: &[Line] = &[
    (Kind::Rounds, 25, (10, 20)),
    (Kind::Bandage, 15, (1, 3)),
    (Kind::Medkit, 10, (1, 1)),
    (Kind::Pills, 12, (1, 1)),
    (Kind::Cash, 10, (1, 3)),
    (Kind::Watch, 8, (1, 1)),
    (Kind::Radio, 8, (1, 1)),
    (Kind::Ring, 2, (1, 1)),
    (Kind::Pistol, 5, (1, 1)),
    (Kind::Shells, 8, (4, 10)),
];

// A map has a score of wrecks: their gold is rare.
const CAR: &[Line] = &[
    (Kind::Fuel, 36, (1, 1)),
    (Kind::Battery, 28, (1, 1)),
    (Kind::Cash, 30, (1, 3)),
    (Kind::Beans, 20, (1, 1)),
    (Kind::Water, 20, (1, 1)),
    (Kind::Rounds, 20, (8, 16)),
    (Kind::Watch, 12, (1, 1)),
    (Kind::Radio, 12, (1, 1)),
    (Kind::Chain, 1, (1, 1)),
    (Kind::Pistol, 4, (1, 1)),
    (Kind::Shotgun, 2, (1, 1)),
    (Kind::Shells, 10, (4, 10)),
];

const CAGE: &[Line] = &[
    (Kind::Medkit, 20, (1, 1)),
    (Kind::Rounds, 20, (20, 30)),
    (Kind::Ring, 7, (1, 1)),
    (Kind::Chain, 7, (1, 1)),
    (Kind::Watch, 14, (1, 1)),
    (Kind::Radio, 8, (1, 1)),
    (Kind::Battery, 6, (1, 1)),
    (Kind::GoldBar, 6, (1, 1)),
    (Kind::Shotgun, 5, (1, 1)),
    (Kind::Shells, 14, (8, 16)),
    (Kind::Rifle, 4, (1, 1)),
    (Kind::RifleRounds, 10, (6, 12)),
];

const FRIDGE: &[Line] = &[
    (Kind::Water, 35, (1, 1)),
    (Kind::Beans, 25, (1, 1)),
    (Kind::Pills, 8, (1, 1)),
];

const CABINET: &[Line] = &[
    (Kind::Bandage, 20, (1, 2)),
    (Kind::Rounds, 15, (6, 12)),
    (Kind::Pills, 12, (1, 1)),
    (Kind::Cash, 12, (1, 2)),
    (Kind::Beans, 10, (1, 1)),
    (Kind::Watch, 5, (1, 1)),
];

// Houses are full of desks and wardrobes: their gold is rarer than a
// locker's.
const DESK: &[Line] = &[
    (Kind::Cash, 50, (1, 3)),
    (Kind::Rounds, 30, (6, 12)),
    (Kind::Pills, 16, (1, 1)),
    (Kind::Watch, 20, (1, 1)),
    (Kind::Radio, 16, (1, 1)),
    (Kind::Chain, 1, (1, 1)),
    (Kind::Pistol, 4, (1, 1)),
];

const WARDROBE: &[Line] = &[
    (Kind::Cash, 36, (1, 2)),
    (Kind::Rounds, 30, (8, 16)),
    (Kind::Bandage, 24, (1, 2)),
    (Kind::Watch, 16, (1, 1)),
    (Kind::Medkit, 10, (1, 1)),
    (Kind::Ring, 1, (1, 1)),
    (Kind::Chain, 1, (1, 1)),
    (Kind::Pistol, 3, (1, 1)),
    (Kind::Shotgun, 2, (1, 1)),
    (Kind::Shells, 6, (4, 8)),
];

const SHELF: &[Line] = &[
    (Kind::Beans, 30, (1, 1)),
    (Kind::Water, 25, (1, 1)),
    (Kind::Bandage, 15, (1, 3)),
    (Kind::Pills, 8, (1, 1)),
    (Kind::Rounds, 8, (8, 16)),
    (Kind::Fuel, 3, (1, 1)),
    (Kind::Battery, 2, (1, 1)),
    (Kind::Shells, 6, (4, 10)),
];

const REGISTER: &[Line] = &[
    (Kind::Cash, 60, (1, 5)),
    (Kind::Watch, 5, (1, 1)),
    (Kind::Rounds, 10, (8, 16)),
];

const CORPSE: &[Line] = &[
    (Kind::Rounds, 35, (4, 8)),
    (Kind::Bandage, 20, (1, 1)),
    (Kind::Cash, 20, (1, 2)),
    (Kind::Pills, 10, (1, 1)),
    (Kind::Watch, 5, (1, 1)),
    (Kind::Ring, 2, (1, 1)),
    (Kind::Shells, 10, (2, 5)),
];

// A farmhouse's: the long gun that was kept there, likely as not.
const GUN_CABINET: &[Line] = &[
    (Kind::Shotgun, 30, (1, 1)),
    (Kind::Shells, 40, (5, 12)),
    (Kind::Rounds, 20, (10, 20)),
    (Kind::Pistol, 10, (1, 1)),
    (Kind::Cash, 8, (1, 2)),
    (Kind::Rifle, 4, (1, 1)),
    (Kind::RifleRounds, 8, (4, 10)),
];

// The hunter's: their rifle, and what it takes.
const HUNTER_CABINET: &[Line] = &[
    (Kind::Rifle, 40, (1, 1)),
    (Kind::RifleRounds, 45, (6, 14)),
    (Kind::Shells, 15, (5, 10)),
    (Kind::Shotgun, 8, (1, 1)),
    (Kind::Cash, 6, (1, 3)),
];

/// How one of the dead carries something at all.
pub const CORPSE_CHANCE: f64 = 0.2;

impl Source {
    fn table(self) -> &'static [Line] {
        match self {
            Source::Crate => CRATE,
            Source::Locker => LOCKER,
            Source::Car => CAR,
            Source::Cage => CAGE,
            Source::Fridge => FRIDGE,
            Source::Cabinet => CABINET,
            Source::Desk => DESK,
            Source::Wardrobe => WARDROBE,
            Source::Shelf => SHELF,
            Source::Register => REGISTER,
            Source::GunCabinet => GUN_CABINET,
            Source::HunterCabinet => HUNTER_CABINET,
            Source::Corpse => CORPSE,
        }
    }

    /// Whether `kind` is ever drawn here.
    #[cfg(test)]
    pub fn holds(self, kind: Kind) -> bool {
        self.table().iter().any(|(k, _, _)| *k == kind)
    }

    /// How many draws from its table.
    fn draws(self) -> (u32, u32) {
        match self {
            Source::Crate => (1, 3),
            Source::Locker => (2, 3),
            Source::Car => (2, 4),
            Source::Cage => (3, 4),
            Source::Fridge => (1, 2),
            Source::Cabinet => (1, 2),
            Source::Desk => (1, 2),
            Source::Wardrobe => (1, 2),
            Source::Shelf => (1, 3),
            Source::Register => (1, 1),
            Source::GunCabinet | Source::HunterCabinet => (2, 3),
            Source::Corpse => (1, 1),
        }
    }

    /// Its grid's size, cells across and down.
    pub fn grid(self) -> (u8, u8) {
        match self {
            Source::Crate => (4, 3),
            Source::Locker => (3, 4),
            Source::Car => (6, 3),
            Source::Cage => (5, 4),
            Source::Fridge => (3, 4),
            Source::Cabinet => (4, 2),
            Source::Desk => (4, 2),
            Source::Wardrobe => (4, 4),
            Source::Shelf => (5, 3),
            Source::Register => (3, 2),
            Source::GunCabinet | Source::HunterCabinet => (5, 3),
            Source::Corpse => (2, 2),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Source::Crate => "WOODEN CRATE",
            Source::Locker => "LOCKER",
            Source::Car => "CAR TRUNK",
            Source::Cage => "SUPPLY CAGE",
            Source::Fridge => "FRIDGE",
            Source::Cabinet => "CHEST OF DRAWERS",
            Source::Desk => "DESK",
            Source::Wardrobe => "WARDROBE",
            Source::Shelf => "STORE SHELF",
            Source::Register => "CASH REGISTER",
            Source::GunCabinet => "GUN CABINET",
            Source::HunterCabinet => "HUNTER'S GUN CABINET",
            Source::Corpse => "REMAINS",
        }
    }

    /// Seconds to search.
    pub fn search_time(self) -> f64 {
        match self {
            Source::Crate => 1.0,
            Source::Locker => 2.0,
            Source::Car => 2.5,
            Source::Cage => 3.0,
            Source::Fridge => 1.5,
            Source::Cabinet => 1.5,
            Source::Desk => 1.5,
            Source::Wardrobe => 2.0,
            Source::Shelf => 2.0,
            Source::Register => 1.5,
            Source::GunCabinet | Source::HunterCabinet => 2.0,
            Source::Corpse => 0.0,
        }
    }
}

/// One draw from `source`'s table (a gun found with what was left in it).
pub fn draw(source: Source, dice: &mut Dice) -> Stack {
    let table = source.table();
    let total: u32 = table.iter().map(|(_, w, _)| w).sum();
    let mut pick = dice.next() % total;
    for &(kind, weight, (lo, hi)) in table {
        if pick < weight {
            let stack = Stack::new(kind, dice.range(lo, hi));
            return match stack.magazine() {
                Some(mag) => Stack { loaded: dice.range(0, mag), ..stack },
                None => stack,
            };
        }
        pick -= weight;
    }
    unreachable!("the pick is under the total")
}

/// A fresh grid for `source`, filled with what it rolls (what doesn't fit
/// is simply not there).
pub fn fill(source: Source, dice: &mut Dice) -> Grid {
    let (w, h) = source.grid();
    let mut grid = Grid::new(w, h);
    let (lo, hi) = source.draws();
    for _ in 0..dice.range(lo, hi) {
        let stack = draw(source, dice);
        let stack = grid.top_up(stack);
        grid.place(stack);
    }
    grid
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_table_draws_what_it_lists_and_fills_its_grid() {
        let mut dice = Dice(0x1234_5678);
        for source in CONTAINERS.into_iter().chain([Source::Corpse]) {
            let mut seen = std::collections::HashSet::new();
            for _ in 0..300 {
                let g = fill(source, &mut dice);
                assert!(!g.items.is_empty(), "{source:?} came up empty");
                for i in &g.items {
                    assert!(source.table().iter().any(|(k, _, _)| *k == i.stack.kind), "{:?} in a {source:?}", i.stack.kind);
                    assert!(i.stack.count >= 1 && i.stack.count <= i.stack.kind.def().stack);
                    seen.insert(i.stack.kind);
                }
            }
            assert_eq!(seen.len(), source.table().len(), "{source:?}: every line turns up sometimes");
        }
    }

    #[test]
    fn epic_things_are_rare_over_a_whole_run() {
        // A long run: a town's houses and stores searched through, the
        // places out of town, and 60 kills' drops.
        let mut dice = Dice(0xBEEF);
        let mut epics = 0u32;
        let runs = 400;
        let run: Vec<(Source, u32)> = vec![(Source::Crate, 15), (Source::Car, 20), (Source::Cage, 1), (Source::Locker, 4), (Source::Fridge, 12), (Source::Cabinet, 20), (Source::Desk, 10), (Source::Wardrobe, 16), (Source::Shelf, 12), (Source::Register, 3)];
        for _ in 0..runs {
            for &(source, n) in &run {
                for _ in 0..n {
                    epics += fill(source, &mut dice).items.iter().filter(|i| i.stack.kind.def().rarity >= super::super::Rarity::Epic).count() as u32;
                }
            }
            for _ in 0..60 {
                if dice.unit() < CORPSE_CHANCE {
                    epics += u32::from(draw(Source::Corpse, &mut dice).kind.def().rarity >= super::super::Rarity::Epic);
                }
            }
        }
        let each = f64::from(epics) / f64::from(runs);
        assert!((0.8..2.4).contains(&each), "{each:.2} epic-or-better things a run");
    }

    #[test]
    fn the_cage_is_richer_than_a_crate() {
        let mut dice = Dice(99);
        let worth = |s: Source, dice: &mut Dice| (0..500).map(|_| fill(s, dice).value()).sum::<u32>() / 500;
        let (crate_, cage) = (worth(Source::Crate, &mut dice), worth(Source::Cage, &mut dice));
        assert!(cage > crate_ * 5, "cage {cage} vs crate {crate_}");
    }
}
