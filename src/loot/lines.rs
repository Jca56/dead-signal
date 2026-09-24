//! What each kind of place (and each of the dead) holds, and how likely:
//! a line a kind of thing, its weight against the rest, and how many come
//! together. The tables' rules are `tables.rs`.

use super::Kind;
use super::tables::Line;

pub(super) const CRATE: &[Line] = &[
    (Kind::Rounds, 30, (8, 16)),
    (Kind::Bandage, 20, (1, 2)),
    (Kind::Beans, 20, (1, 1)),
    (Kind::Water, 15, (1, 1)),
    (Kind::Cash, 8, (1, 2)),
    (Kind::Pills, 5, (1, 1)),
    (Kind::Fuel, 4, (1, 1)),
    (Kind::Shells, 12, (4, 10)),
    (Kind::Molotov, 5, (1, 1)),
    (Kind::BikeHelmet, 3, (1, 1)),
    (Kind::Daypack, 3, (1, 1)),
];

pub(super) const LOCKER: &[Line] = &[
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
    (Kind::Knife, 5, (1, 1)),
    (Kind::Smg, 2, (1, 1)),
    (Kind::BikeHelmet, 3, (1, 1)),
    (Kind::LightVest, 4, (1, 1)),
    (Kind::ChestRig, 3, (1, 1)),
    (Kind::Daypack, 3, (1, 1)),
    (Kind::Rucksack, 2, (1, 1)),
    (Kind::CargoPants, 3, (1, 1)),
    (Kind::Bandolier, 2, (1, 1)),
];

// A map has a score of wrecks: their gold is rare.
pub(super) const CAR: &[Line] = &[
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
    (Kind::Machete, 3, (1, 1)),
    (Kind::Molotov, 5, (1, 1)),
    (Kind::BikeHelmet, 3, (1, 1)),
    (Kind::Daypack, 4, (1, 1)),
    (Kind::LightVest, 2, (1, 1)),
    (Kind::Rucksack, 2, (1, 1)),
];

pub(super) const CAGE: &[Line] = &[
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
    (Kind::Knife, 6, (1, 1)),
    (Kind::Smg, 4, (1, 1)),
    (Kind::AssaultRifle, 2, (1, 1)),
    (Kind::Rounds556, 10, (10, 20)),
    (Kind::PipeBomb, 5, (1, 2)),
    (Kind::MilitaryHelmet, 4, (1, 1)),
    (Kind::LightVest, 4, (1, 1)),
    (Kind::PlateCarrier, 2, (1, 1)),
    (Kind::ChestRig, 3, (1, 1)),
    (Kind::ArmoredRig, 2, (1, 1)),
    (Kind::Rucksack, 3, (1, 1)),
    (Kind::HikingPack, 2, (1, 1)),
    (Kind::MilitaryRuck, 1, (1, 1)),
    (Kind::ArmorPlate, 6, (1, 1)),
];

pub(super) const FRIDGE: &[Line] = &[
    (Kind::Water, 35, (1, 1)),
    (Kind::Beans, 25, (1, 1)),
    (Kind::Pills, 8, (1, 1)),
];

pub(super) const CABINET: &[Line] = &[
    (Kind::Bandage, 20, (1, 2)),
    (Kind::Rounds, 15, (6, 12)),
    (Kind::Pills, 12, (1, 1)),
    (Kind::Cash, 12, (1, 2)),
    (Kind::Beans, 10, (1, 1)),
    (Kind::Watch, 5, (1, 1)),
    (Kind::Molotov, 3, (1, 1)),
    (Kind::CargoPants, 3, (1, 1)),
];

// Houses are full of desks and wardrobes: their gold is rarer than a
// locker's.
pub(super) const DESK: &[Line] = &[
    (Kind::Cash, 50, (1, 3)),
    (Kind::Rounds, 30, (6, 12)),
    (Kind::Pills, 16, (1, 1)),
    (Kind::Watch, 20, (1, 1)),
    (Kind::Radio, 16, (1, 1)),
    (Kind::Chain, 1, (1, 1)),
    (Kind::Pistol, 4, (1, 1)),
];

pub(super) const WARDROBE: &[Line] = &[
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
    (Kind::BikeHelmet, 3, (1, 1)),
    (Kind::Daypack, 4, (1, 1)),
    (Kind::Rucksack, 3, (1, 1)),
    (Kind::HikingPack, 2, (1, 1)),
    (Kind::LightVest, 2, (1, 1)),
    (Kind::CargoPants, 5, (1, 1)),
];

pub(super) const SHELF: &[Line] = &[
    (Kind::Beans, 30, (1, 1)),
    (Kind::Water, 25, (1, 1)),
    (Kind::Bandage, 15, (1, 3)),
    (Kind::Pills, 8, (1, 1)),
    (Kind::Rounds, 8, (8, 16)),
    (Kind::Fuel, 3, (1, 1)),
    (Kind::Battery, 2, (1, 1)),
    (Kind::Shells, 6, (4, 10)),
    (Kind::BikeHelmet, 2, (1, 1)),
    (Kind::Daypack, 2, (1, 1)),
    (Kind::CargoPants, 2, (1, 1)),
];

pub(super) const REGISTER: &[Line] = &[
    (Kind::Cash, 60, (1, 5)),
    (Kind::Watch, 5, (1, 1)),
    (Kind::Rounds, 10, (8, 16)),
];

pub(super) const CORPSE: &[Line] = &[
    (Kind::Rounds, 35, (4, 8)),
    (Kind::Bandage, 20, (1, 1)),
    (Kind::Cash, 20, (1, 2)),
    (Kind::Pills, 10, (1, 1)),
    (Kind::Watch, 5, (1, 1)),
    (Kind::Ring, 2, (1, 1)),
    (Kind::Shells, 10, (2, 5)),
    (Kind::Knife, 3, (1, 1)),
];

// A farmhouse's: the long gun that was kept there, likely as not.
pub(super) const GUN_CABINET: &[Line] = &[
    (Kind::Shotgun, 30, (1, 1)),
    (Kind::Shells, 40, (5, 12)),
    (Kind::Rounds, 20, (10, 20)),
    (Kind::Pistol, 10, (1, 1)),
    (Kind::Cash, 8, (1, 2)),
    (Kind::Rifle, 4, (1, 1)),
    (Kind::RifleRounds, 8, (4, 10)),
    (Kind::Bandolier, 6, (1, 1)),
];

// The hunter's: their rifle, and what it takes.
pub(super) const HUNTER_CABINET: &[Line] = &[
    (Kind::Rifle, 40, (1, 1)),
    (Kind::RifleRounds, 45, (6, 14)),
    (Kind::Shells, 15, (5, 10)),
    (Kind::Shotgun, 8, (1, 1)),
    (Kind::Cash, 6, (1, 3)),
    (Kind::FireAxe, 10, (1, 1)),
    (Kind::Bandolier, 6, (1, 1)),
];

// A barn's: what the farm cut with, and what it ran on.
pub(super) const TOOL_LOCKER: &[Line] = &[
    (Kind::Machete, 25, (1, 1)),
    (Kind::FireAxe, 20, (1, 1)),
    (Kind::Fuel, 20, (1, 1)),
    (Kind::Battery, 10, (1, 1)),
    (Kind::Shells, 8, (4, 10)),
    (Kind::Rounds, 8, (8, 16)),
    (Kind::Knife, 5, (1, 1)),
    (Kind::Molotov, 8, (1, 2)),
    (Kind::PipeBomb, 5, (1, 1)),
];

pub(super) const SUPPLY_CASE: &[Line] = &[
    (Kind::Medkit, 20, (1, 1)),
    (Kind::Bandage, 20, (1, 3)),
    (Kind::Pills, 15, (1, 1)),
    (Kind::Rounds, 15, (10, 20)),
    (Kind::RifleRounds, 12, (6, 12)),
    (Kind::Radio, 6, (1, 1)),
    (Kind::Cash, 6, (1, 3)),
    (Kind::Battery, 4, (1, 1)),
    (Kind::Knife, 4, (1, 1)),
    (Kind::Smg, 3, (1, 1)),
    (Kind::Rounds556, 18, (12, 24)),
    (Kind::PipeBomb, 5, (1, 1)),
    (Kind::MilitaryHelmet, 3, (1, 1)),
    (Kind::LightVest, 3, (1, 1)),
    (Kind::ChestRig, 4, (1, 1)),
    (Kind::ArmoredRig, 2, (1, 1)),
    (Kind::Rucksack, 2, (1, 1)),
    (Kind::ArmorPlate, 8, (1, 1)),
];

// The richest rounds on the map, and the guns to fire them.
pub(super) const AMMO_CAGE: &[Line] = &[
    (Kind::Rounds, 25, (20, 30)),
    (Kind::Shells, 20, (10, 20)),
    (Kind::RifleRounds, 20, (10, 20)),
    (Kind::Pistol, 8, (1, 1)),
    (Kind::Rifle, 7, (1, 1)),
    (Kind::Shotgun, 7, (1, 1)),
    (Kind::Medkit, 8, (1, 1)),
    (Kind::Knife, 6, (1, 1)),
    (Kind::Smg, 6, (1, 1)),
    (Kind::AssaultRifle, 7, (1, 1)),
    (Kind::Rounds556, 25, (20, 30)),
    (Kind::PipeBomb, 8, (1, 2)),
    (Kind::Molotov, 5, (1, 2)),
    (Kind::MilitaryHelmet, 4, (1, 1)),
    (Kind::PlateCarrier, 4, (1, 1)),
    (Kind::ArmoredRig, 3, (1, 1)),
    (Kind::MilitaryRuck, 2, (1, 1)),
    (Kind::Bandolier, 4, (1, 1)),
    (Kind::ArmorPlate, 8, (1, 2)),
];

pub(super) const SOLDIER: &[Line] = &[
    (Kind::Rounds, 25, (6, 12)),
    (Kind::RifleRounds, 12, (3, 8)),
    (Kind::Rounds556, 22, (10, 24)),
    (Kind::Bandage, 15, (1, 2)),
    (Kind::Shells, 8, (3, 6)),
    (Kind::Cash, 6, (1, 3)),
    (Kind::Medkit, 5, (1, 1)),
    (Kind::Knife, 5, (1, 1)),
    (Kind::ArmoryKey, 2, (1, 1)),
    (Kind::PipeBomb, 3, (1, 1)),
    (Kind::MilitaryHelmet, 3, (1, 1)),
    (Kind::CargoPants, 2, (1, 1)),
    (Kind::Bandolier, 2, (1, 1)),
    (Kind::ArmorPlate, 5, (1, 1)),
];

pub(super) const JUGGERNAUT: &[Line] = &[
    (Kind::GoldBar, 10, (1, 1)),
    (Kind::Rifle, 6, (1, 1)),
    (Kind::Shotgun, 6, (1, 1)),
    (Kind::RifleRounds, 12, (8, 15)),
    (Kind::Shells, 10, (6, 12)),
    (Kind::Medkit, 12, (1, 1)),
    (Kind::Cash, 12, (3, 6)),
    (Kind::ArmoryKey, 4, (1, 1)),
    (Kind::AssaultRifle, 6, (1, 1)),
    (Kind::Rounds556, 12, (15, 30)),
    (Kind::PlateCarrier, 5, (1, 1)),
    (Kind::ArmorPlate, 8, (1, 2)),
];
