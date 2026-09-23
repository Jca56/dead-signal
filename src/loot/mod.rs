//! What can be carried: every kind of thing, how much room it takes in a
//! grid, how many stack in one cell, how rare it is and what it's worth;
//! where things are kept (`grid.rs`, `bag.rs`) and what turns up where
//! (`tables.rs`).

pub mod bag;
pub mod grid;
pub mod tables;

use lntrn_math::Color;

use crate::weapon::Weapon;

/// How rare a thing is: the classic five.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

impl Rarity {
    pub fn name(self) -> &'static str {
        match self {
            Rarity::Common => "COMMON",
            Rarity::Uncommon => "UNCOMMON",
            Rarity::Rare => "RARE",
            Rarity::Epic => "EPIC",
            Rarity::Legendary => "LEGENDARY",
        }
    }

    pub fn colour(self) -> Color {
        match self {
            Rarity::Common => Color::rgb(0.62, 0.62, 0.60),
            Rarity::Uncommon => Color::rgb(0.36, 0.70, 0.30),
            Rarity::Rare => Color::rgb(0.28, 0.52, 0.88),
            Rarity::Epic => Color::rgb(0.64, 0.36, 0.86),
            Rarity::Legendary => Color::rgb(0.93, 0.68, 0.18),
        }
    }
}

/// Every kind of thing there is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Rounds,
    Bandage,
    Medkit,
    Beans,
    Water,
    Pills,
    Cash,
    Watch,
    Radio,
    Battery,
    Fuel,
    Ring,
    Chain,
    Key,
    GoldBar,
    Pistol,
}

pub const ALL: [Kind; 16] = [
    Kind::Rounds,
    Kind::Bandage,
    Kind::Medkit,
    Kind::Beans,
    Kind::Water,
    Kind::Pills,
    Kind::Cash,
    Kind::Watch,
    Kind::Radio,
    Kind::Battery,
    Kind::Fuel,
    Kind::Ring,
    Kind::Chain,
    Kind::Key,
    Kind::GoldBar,
    Kind::Pistol,
];

impl Kind {
    /// Its name in a save file: never to change, whatever it's called on
    /// screen.
    pub fn key(self) -> &'static str {
        match self {
            Kind::Rounds => "rounds_9mm",
            Kind::Bandage => "bandage",
            Kind::Medkit => "medkit",
            Kind::Beans => "canned_beans",
            Kind::Water => "water",
            Kind::Pills => "antibiotics",
            Kind::Cash => "cash",
            Kind::Watch => "watch",
            Kind::Radio => "radio",
            Kind::Battery => "car_battery",
            Kind::Fuel => "fuel_can",
            Kind::Ring => "gold_ring",
            Kind::Chain => "gold_chain",
            Kind::Key => "cage_key",
            Kind::GoldBar => "gold_bar",
            Kind::Pistol => "pistol",
        }
    }

    pub fn from_key(key: &str) -> Option<Kind> {
        ALL.into_iter().find(|k| k.key() == key)
    }

    /// The weapon it is, if it's one.
    pub fn weapon(self) -> Option<Weapon> {
        match self {
            Kind::Pistol => Some(Weapon::Pistol),
            _ => None,
        }
    }
}

/// What a kind of thing is like.
pub struct Def {
    pub name: &'static str,
    /// Cells across and down, lying as it's found.
    pub size: (u8, u8),
    /// How many share one place.
    pub stack: u32,
    pub rarity: Rarity,
    /// What one is worth, got out.
    pub value: u32,
    /// Its object in `items.glb`.
    pub model: &'static str,
}

impl Kind {
    pub fn def(self) -> Def {
        use Rarity::*;
        let d = |name, size, stack, rarity, value, model| Def { name, size, stack, rarity, value, model };
        match self {
            Kind::Rounds => d("9MM ROUNDS", (1, 1), 30, Common, 2, "ITEM_Ammo"),
            Kind::Bandage => d("BANDAGE", (1, 1), 3, Common, 15, "ITEM_Bandage"),
            Kind::Medkit => d("MEDKIT", (2, 1), 1, Uncommon, 60, "ITEM_Medkit"),
            Kind::Beans => d("CANNED BEANS", (1, 1), 1, Common, 10, "ITEM_Beans"),
            Kind::Water => d("WATER", (1, 2), 1, Common, 12, "ITEM_Water"),
            Kind::Pills => d("ANTIBIOTICS", (1, 1), 1, Uncommon, 45, "ITEM_Pills"),
            Kind::Cash => d("CASH", (1, 1), 5, Uncommon, 50, "ITEM_Cash"),
            Kind::Watch => d("WATCH", (1, 1), 1, Rare, 120, "ITEM_Watch"),
            Kind::Radio => d("RADIO", (1, 2), 1, Rare, 150, "ITEM_Radio"),
            Kind::Battery => d("CAR BATTERY", (2, 2), 1, Rare, 180, "ITEM_Battery"),
            Kind::Fuel => d("FUEL CAN", (2, 2), 1, Uncommon, 90, "ITEM_Fuel"),
            Kind::Ring => d("GOLD RING", (1, 1), 1, Epic, 300, "ITEM_Ring"),
            Kind::Chain => d("GOLD CHAIN", (1, 1), 1, Epic, 350, "ITEM_Chain"),
            Kind::Key => d("CAGE KEY", (1, 1), 1, Rare, 25, "ITEM_Key"),
            Kind::GoldBar => d("GOLD BAR", (2, 1), 1, Legendary, 1000, "ITEM_GoldBar"),
            Kind::Pistol => d("PISTOL", (2, 1), 1, Uncommon, 150, "ITEM_Pistol"),
        }
    }
}

/// Some of one kind of thing, together.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stack {
    pub kind: Kind,
    pub count: u32,
    /// The rounds in it, a gun (it keeps them, whoever carries it).
    pub loaded: u32,
}

impl Stack {
    pub fn new(kind: Kind, count: u32) -> Self {
        Self { kind, count, loaded: 0 }
    }

    pub fn one(kind: Kind) -> Self {
        Self::new(kind, 1)
    }

    /// A gun with `loaded` rounds in it.
    pub fn gun(kind: Kind, loaded: u32) -> Self {
        Self { loaded, ..Self::one(kind) }
    }

    /// As many as `count`, but otherwise the same (a gun's rounds kept).
    pub fn with_count(self, count: u32) -> Self {
        Self { count, ..self }
    }

    /// How many rounds a gun holds, if it's a gun.
    pub fn magazine(self) -> Option<u32> {
        self.kind.weapon().map(|w| w.spec().mag).filter(|&m| m > 0)
    }

    /// What it all is worth.
    pub fn value(self) -> u32 {
        self.kind.def().value * self.count
    }

    /// What it's called, with how many when more than one could be.
    pub fn label(self) -> String {
        let def = self.kind.def();
        if let Some(mag) = self.magazine() {
            format!("{}  {}/{}", def.name, self.loaded, mag)
        } else if def.stack > 1 {
            format!("{}  ×{}", def.name, self.count)
        } else {
            def.name.to_string()
        }
    }
}

/// A small shared source of luck.
#[derive(Clone, Copy, Debug)]
pub struct Dice(pub u32);

impl Default for Dice {
    fn default() -> Self {
        Self(0x2545_F491)
    }
}

impl Dice {
    /// 0 to 1.
    pub fn unit(&mut self) -> f64 {
        f64::from(self.next()) / f64::from(u32::MAX)
    }

    pub fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }

    /// `lo` to `hi`, both included.
    pub fn range(&mut self, lo: u32, hi: u32) -> u32 {
        lo + self.next() % (hi - lo + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::tables::Source;
    use super::*;

    /// `Items.md` is the list kept for people: every thing in it, as the
    /// game has it (size, stack, rarity, worth, where it's found).
    #[test]
    fn items_md_matches_the_game() {
        let md = std::fs::read_to_string(format!("{}/Items.md", env!("CARGO_MANIFEST_DIR"))).expect("Items.md");
        let rows: Vec<Vec<String>> = md
            .lines()
            .skip_while(|l| !l.starts_with("| Item |"))
            .skip(2)
            .take_while(|l| l.starts_with('|'))
            .map(|l| l.trim_matches('|').split('|').map(|c| c.trim().to_string()).collect())
            .collect();
        assert_eq!(rows.len(), ALL.len(), "one row a kind of thing");
        let lying = [Kind::Rounds, Kind::Bandage, Kind::Medkit];
        for kind in ALL {
            let d = kind.def();
            let row = rows.iter().find(|r| r[0] == d.name).unwrap_or_else(|| panic!("{} isn't in Items.md", d.name));
            assert_eq!(row[1], format!("{}×{}", d.size.0, d.size.1), "{} size", d.name);
            assert_eq!(row[2], d.stack.to_string(), "{} stack", d.name);
            let rarity = d.rarity.name();
            assert_eq!(row[3].to_uppercase(), rarity, "{} rarity", d.name);
            assert_eq!(row[4], format!("${}", d.value), "{} value", d.name);
            assert!(!row[6].is_empty(), "{} has no flavor", d.name);
            if kind == Kind::Key {
                continue;
            }
            let mut found: Vec<&str> = [
                (Source::Crate, "crate"),
                (Source::Locker, "locker"),
                (Source::Car, "car"),
                (Source::Cage, "cage"),
                (Source::Fridge, "fridge"),
                (Source::Cabinet, "drawers"),
                (Source::Desk, "desk"),
                (Source::Wardrobe, "wardrobe"),
                (Source::Shelf, "shelf"),
                (Source::Register, "register"),
                (Source::Corpse, "zombies"),
            ]
                .iter()
                .filter(|(s, _)| s.holds(kind))
                .map(|(_, w)| *w)
                .collect();
            if lying.contains(&kind) {
                found.push("lying about");
            }
            assert_eq!(row[5], found.join(", "), "{} found in", d.name);
        }
    }
}
