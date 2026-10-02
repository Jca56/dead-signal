//! What can be carried: every kind of thing, how much room it takes in a
//! grid, how many stack in one cell, how rare it is and what it's worth;
//! where things are kept (`grid.rs`, `bag.rs`) and what turns up where
//! (`tables.rs`).

pub mod bag;
pub mod gear;
mod lines;
pub mod grid;
pub mod tables;

use lntrn_math::Color;

use crate::weapon::{Weapon, amp};

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
    Shotgun,
    Shells,
    Rifle,
    RifleRounds,
    Knife,
    Machete,
    FireAxe,
    ArmoryKey,
    Smg,
    AssaultRifle,
    Rounds556,
    Molotov,
    PipeBomb,
    BikeHelmet,
    MilitaryHelmet,
    LightVest,
    PlateCarrier,
    ChestRig,
    ArmoredRig,
    Daypack,
    Rucksack,
    HikingPack,
    MilitaryRuck,
    CargoPants,
    Bandolier,
    ArmorPlate,
    PrecinctKey,
    Lmg,
    Rounds762,
    Flamethrower,
    FlameFuel,
    Pistol45,
    Rounds45,
    Magnum,
    Rounds44,
    MiniUzi,
    Ak47,
    RoundsAk,
    Bullpup,
    Rpk,
}

pub const ALL: [Kind; 56] = [
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
    Kind::Shotgun,
    Kind::Shells,
    Kind::Rifle,
    Kind::RifleRounds,
    Kind::Knife,
    Kind::Machete,
    Kind::FireAxe,
    Kind::ArmoryKey,
    Kind::Smg,
    Kind::AssaultRifle,
    Kind::Rounds556,
    Kind::Molotov,
    Kind::PipeBomb,
    Kind::BikeHelmet,
    Kind::MilitaryHelmet,
    Kind::LightVest,
    Kind::PlateCarrier,
    Kind::ChestRig,
    Kind::ArmoredRig,
    Kind::Daypack,
    Kind::Rucksack,
    Kind::HikingPack,
    Kind::MilitaryRuck,
    Kind::CargoPants,
    Kind::Bandolier,
    Kind::ArmorPlate,
    Kind::PrecinctKey,
    Kind::Lmg,
    Kind::Rounds762,
    Kind::Flamethrower,
    Kind::FlameFuel,
    Kind::Pistol45,
    Kind::Rounds45,
    Kind::Magnum,
    Kind::Rounds44,
    Kind::MiniUzi,
    Kind::Ak47,
    Kind::RoundsAk,
    Kind::Bullpup,
    Kind::Rpk,
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
            Kind::Shotgun => "shotgun",
            Kind::Shells => "shells_12ga",
            Kind::Rifle => "hunting_rifle",
            Kind::RifleRounds => "rounds_308",
            Kind::Knife => "tactical_knife",
            Kind::Machete => "machete",
            Kind::FireAxe => "fire_axe",
            Kind::ArmoryKey => "armory_key",
            Kind::Smg => "smg",
            Kind::AssaultRifle => "assault_rifle",
            Kind::Rounds556 => "rounds_556",
            Kind::Molotov => "molotov",
            Kind::PipeBomb => "pipe_bomb",
            Kind::BikeHelmet => "bike_helmet",
            Kind::MilitaryHelmet => "military_helmet",
            Kind::LightVest => "light_vest",
            Kind::PlateCarrier => "plate_carrier",
            Kind::ChestRig => "chest_rig",
            Kind::ArmoredRig => "armored_rig",
            Kind::Daypack => "daypack",
            Kind::Rucksack => "rucksack",
            Kind::HikingPack => "hiking_pack",
            Kind::MilitaryRuck => "military_ruck",
            Kind::CargoPants => "cargo_pants",
            Kind::Bandolier => "bandolier",
            Kind::ArmorPlate => "armor_plate",
            Kind::PrecinctKey => "precinct_key",
            Kind::Lmg => "lmg",
            Kind::Rounds762 => "rounds_762",
            Kind::Flamethrower => "flamethrower",
            Kind::FlameFuel => "flame_fuel",
            Kind::Pistol45 => "pistol_45",
            Kind::Rounds45 => "rounds_45",
            Kind::Magnum => "magnum_44",
            Kind::Rounds44 => "rounds_44",
            Kind::MiniUzi => "mini_uzi",
            Kind::Ak47 => "ak47",
            Kind::RoundsAk => "rounds_762x39",
            Kind::Bullpup => "bullpup",
            Kind::Rpk => "rpk",
        }
    }

    pub fn from_key(key: &str) -> Option<Kind> {
        ALL.into_iter().find(|k| k.key() == key)
    }

    /// The weapon it is, if it's one.
    pub fn weapon(self) -> Option<Weapon> {
        match self {
            Kind::Pistol => Some(Weapon::Pistol),
            Kind::Shotgun => Some(Weapon::Shotgun),
            Kind::Rifle => Some(Weapon::Rifle),
            Kind::Knife => Some(Weapon::Knife),
            Kind::Machete => Some(Weapon::Machete),
            Kind::FireAxe => Some(Weapon::Axe),
            Kind::Smg => Some(Weapon::Smg),
            Kind::AssaultRifle => Some(Weapon::AssaultRifle),
            Kind::Lmg => Some(Weapon::Lmg),
            Kind::Flamethrower => Some(Weapon::Flamethrower),
            Kind::Pistol45 => Some(Weapon::Pistol45),
            Kind::Magnum => Some(Weapon::Magnum),
            Kind::MiniUzi => Some(Weapon::MiniUzi),
            Kind::Ak47 => Some(Weapon::Ak47),
            Kind::Bullpup => Some(Weapon::Bullpup),
            Kind::Rpk => Some(Weapon::Rpk),
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
            Kind::Cash => d("CASH", (1, 1), 50, Uncommon, 50, "ITEM_Cash"),
            Kind::Watch => d("WATCH", (1, 1), 1, Rare, 120, "ITEM_Watch"),
            Kind::Radio => d("RADIO", (1, 2), 1, Rare, 150, "ITEM_Radio"),
            Kind::Battery => d("CAR BATTERY", (2, 2), 1, Rare, 180, "ITEM_Battery"),
            Kind::Fuel => d("FUEL CAN", (2, 2), 1, Uncommon, 90, "ITEM_Fuel"),
            Kind::Ring => d("GOLD RING", (1, 1), 1, Epic, 300, "ITEM_Ring"),
            Kind::Chain => d("GOLD CHAIN", (1, 1), 1, Epic, 350, "ITEM_Chain"),
            Kind::Key => d("CAGE KEY", (1, 1), 1, Rare, 25, "ITEM_Key"),
            Kind::GoldBar => d("GOLD BAR", (2, 1), 1, Legendary, 1000, "ITEM_GoldBar"),
            Kind::Pistol => d("PISTOL", (2, 1), 1, Uncommon, 150, "ITEM_Pistol"),
            Kind::Shotgun => d("SHOTGUN", (4, 1), 1, Rare, 320, "ITEM_Shotgun"),
            Kind::Shells => d("12GA SHELLS", (1, 1), 20, Common, 4, "ITEM_Shells"),
            Kind::Rifle => d("HUNTING RIFLE", (5, 1), 1, Epic, 480, "ITEM_Rifle"),
            Kind::RifleRounds => d(".308 ROUNDS", (1, 1), 20, Uncommon, 8, "ITEM_RifleRounds"),
            Kind::Knife => d("TACTICAL KNIFE", (2, 1), 1, Uncommon, 90, "ITEM_Knife"),
            Kind::Machete => d("MACHETE", (3, 1), 1, Uncommon, 110, "ITEM_Machete"),
            Kind::FireAxe => d("FIRE AXE", (4, 1), 1, Rare, 160, "ITEM_Axe"),
            Kind::ArmoryKey => d("ARMORY KEY", (1, 1), 1, Rare, 40, "ITEM_ArmoryKey"),
            Kind::Smg => d("SMG", (3, 1), 1, Rare, 380, "ITEM_Smg"),
            Kind::AssaultRifle => d("ASSAULT RIFLE", (5, 1), 1, Epic, 650, "ITEM_AssaultRifle"),
            Kind::Rounds556 => d("5.56 ROUNDS", (1, 1), 30, Uncommon, 5, "ITEM_Rounds556"),
            Kind::Molotov => d("MOLOTOV", (2, 1), 2, Uncommon, 45, "ITEM_Molotov"),
            Kind::PipeBomb => d("PIPE BOMB", (2, 1), 2, Rare, 90, "ITEM_PipeBomb"),
            Kind::BikeHelmet => d("BIKE HELMET", (2, 2), 1, Common, 40, "ITEM_BikeHelmet"),
            Kind::MilitaryHelmet => d("MILITARY HELMET", (2, 2), 1, Rare, 180, "ITEM_MilitaryHelmet"),
            Kind::LightVest => d("LIGHT VEST", (3, 3), 1, Uncommon, 150, "ITEM_LightVest"),
            Kind::PlateCarrier => d("PLATE CARRIER", (3, 3), 1, Epic, 450, "ITEM_PlateCarrier"),
            Kind::ChestRig => d("CHEST RIG", (3, 2), 1, Uncommon, 120, "ITEM_ChestRig"),
            Kind::ArmoredRig => d("ARMORED RIG", (3, 3), 1, Rare, 320, "ITEM_ArmoredRig"),
            Kind::Daypack => d("DAYPACK", (3, 3), 1, Common, 60, "ITEM_Daypack"),
            Kind::Rucksack => d("RUCKSACK", (3, 4), 1, Uncommon, 140, "ITEM_Rucksack"),
            Kind::HikingPack => d("HIKING PACK", (4, 4), 1, Rare, 260, "ITEM_HikingPack"),
            Kind::MilitaryRuck => d("MILITARY RUCK", (4, 5), 1, Epic, 420, "ITEM_MilitaryRuck"),
            Kind::CargoPants => d("CARGO PANTS", (2, 2), 1, Uncommon, 90, "ITEM_CargoPants"),
            Kind::Bandolier => d("BANDOLIER", (2, 1), 1, Uncommon, 70, "ITEM_Bandolier"),
            Kind::ArmorPlate => d("ARMOR PLATE", (2, 2), 1, Uncommon, 70, "ITEM_ArmorPlate"),
            Kind::PrecinctKey => d("PRECINCT KEY", (1, 1), 1, Rare, 40, "ITEM_PrecinctKey"),
            Kind::Lmg => d("LMG", (5, 2), 1, Legendary, 1100, "ITEM_Lmg"),
            Kind::Rounds762 => d("7.62 BELT", (2, 1), 100, Rare, 6, "ITEM_Rounds762"),
            Kind::Flamethrower => d("FLAMETHROWER", (5, 2), 1, Legendary, 1200, "ITEM_Flamethrower"),
            Kind::FlameFuel => d("FLAME FUEL", (1, 2), 200, Rare, 2, "ITEM_FlameFuel"),
            Kind::Pistol45 => d(".45 PISTOL", (2, 1), 1, Rare, 260, "ITEM_Pistol45"),
            Kind::Rounds45 => d(".45 ROUNDS", (1, 1), 24, Uncommon, 4, "ITEM_Rounds45"),
            Kind::Magnum => d(".44 MAGNUM", (2, 1), 1, Epic, 520, "ITEM_Magnum"),
            Kind::Rounds44 => d(".44 ROUNDS", (1, 1), 18, Rare, 9, "ITEM_Rounds44"),
            Kind::MiniUzi => d("MINI UZI", (2, 2), 1, Rare, 420, "ITEM_Uzi"),
            Kind::Ak47 => d("AK-47", (5, 1), 1, Epic, 700, "ITEM_Ak"),
            Kind::RoundsAk => d("7.62 ROUNDS", (1, 1), 30, Uncommon, 6, "ITEM_RoundsAk"),
            Kind::Bullpup => d("BULLPUP", (4, 1), 1, Legendary, 900, "ITEM_Bullpup"),
            Kind::Rpk => d("RPK", (5, 2), 1, Legendary, 1000, "ITEM_Rpk"),
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
    /// How many times it's been through the Amplifier, a weapon
    /// (`weapon/amp.rs`: it hits harder and holds more).
    pub tier: u8,
}

impl Stack {
    pub fn new(kind: Kind, count: u32) -> Self {
        Self { kind, count, loaded: 0, tier: 0 }
    }

    pub fn one(kind: Kind) -> Self {
        Self::new(kind, 1)
    }

    /// A gun with `loaded` rounds in it (or armor with that many points
    /// left).
    pub fn gun(kind: Kind, loaded: u32) -> Self {
        Self { loaded, ..Self::one(kind) }
    }

    /// `count` of `kind` as new: a gun loaded, armor whole.
    pub fn fresh(kind: Kind, count: u32) -> Self {
        Self { loaded: Self::one(kind).most().unwrap_or(0), ..Self::new(kind, count) }
    }

    /// The most armor points it has, if it's armor.
    pub fn armor(self) -> Option<u32> {
        self.kind.gear().map(|g| g.armor).filter(|&a| a > 0)
    }

    /// The most it keeps in `loaded`: a gun's magazine, armor's points.
    pub fn most(self) -> Option<u32> {
        self.magazine().or(self.armor())
    }

    /// As many as `count`, but otherwise the same (a gun's rounds kept).
    pub fn with_count(self, count: u32) -> Self {
        Self { count, ..self }
    }

    /// How many of `ammo` this could take into its magazine now: a gun
    /// with room in it, and `ammo` what it takes (else none).
    pub fn room_for(self, ammo: Stack) -> u32 {
        match self.kind.weapon() {
            Some(w) if w.spec().ammo == Some(ammo.kind) => amp::capacity(w, self.tier).saturating_sub(self.loaded),
            _ => 0,
        }
    }

    /// How many rounds a gun holds (amplified as it is), if it's a gun.
    pub fn magazine(self) -> Option<u32> {
        self.kind.weapon().map(|w| amp::capacity(w, self.tier)).filter(|&m| m > 0)
    }

    /// What it's called: an amplified weapon by the name that gave it.
    pub fn name(self) -> &'static str {
        match self.kind.weapon() {
            Some(w) if self.tier > 0 => amp::name(w, self.tier),
            _ => self.kind.def().name,
        }
    }

    /// What it all is worth.
    pub fn value(self) -> u32 {
        self.kind.def().value * self.count
    }

    /// What it's called, with how many when more than one could be.
    pub fn label(self) -> String {
        let (def, name) = (self.kind.def(), self.name());
        if let Some(most) = self.most() {
            format!("{name}  {}/{}", self.loaded, most)
        } else if def.stack > 1 {
            format!("{name}  ×{}", self.count)
        } else {
            name.to_string()
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
mod tests;
