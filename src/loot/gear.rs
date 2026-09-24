//! What's worn: a helmet, something on the chest (a vest, a plate carrier,
//! a rig with pockets of its own, or both), a backpack (the backpack grid
//! is the pack worn), trousers (cargo pants: bigger pockets), a belt (a
//! bandolier: rounds only). Armor soaks blows before health does, as many
//! points as it has left (a worn piece's `loaded`, like a gun's rounds);
//! the heavier it all is, the slower the sprint, the quicker the breath
//! goes, and the louder the feet.

use super::Kind;

/// Where something's worn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Wear {
    Head,
    Chest,
    Back,
    Legs,
    Belt,
}

impl Wear {
    pub const ALL: [Wear; 5] = [Wear::Head, Wear::Chest, Wear::Back, Wear::Legs, Wear::Belt];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn name(self) -> &'static str {
        match self {
            Wear::Head => "HEAD",
            Wear::Chest => "CHEST",
            Wear::Back => "BACK",
            Wear::Legs => "LEGS",
            Wear::Belt => "BELT",
        }
    }

    /// Its name in a save file.
    pub fn save_key(self) -> &'static str {
        match self {
            Wear::Head => "head",
            Wear::Chest => "chest",
            Wear::Back => "back",
            Wear::Legs => "legs",
            Wear::Belt => "belt",
        }
    }
}

/// What a piece of gear is like.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Gear {
    pub wear: Wear,
    /// Armor points, whole.
    pub armor: u32,
    /// A grid of its own (a backpack's, a rig's, a belt's), cells across and
    /// down.
    pub grid: Option<(u8, u8)>,
    /// More pockets, across and down (cargo pants).
    pub pockets: (u8, u8),
    /// How heavy: 0 light, 1 medium, 2 heavy.
    pub weight: u8,
    /// Its grid takes rounds only.
    pub ammo_only: bool,
}

const fn gear(wear: Wear) -> Gear {
    Gear { wear, armor: 0, grid: None, pockets: (0, 0), weight: 0, ammo_only: false }
}

impl Kind {
    /// What it is to wear, if it's worn.
    pub fn gear(self) -> Option<Gear> {
        Some(match self {
            Kind::BikeHelmet => Gear { armor: 15, ..gear(Wear::Head) },
            Kind::MilitaryHelmet => Gear { armor: 40, weight: 1, ..gear(Wear::Head) },
            Kind::LightVest => Gear { armor: 40, weight: 1, ..gear(Wear::Chest) },
            Kind::PlateCarrier => Gear { armor: 80, weight: 2, ..gear(Wear::Chest) },
            Kind::ChestRig => Gear { grid: Some((4, 2)), ..gear(Wear::Chest) },
            Kind::ArmoredRig => Gear { armor: 50, grid: Some((3, 2)), weight: 1, ..gear(Wear::Chest) },
            Kind::Daypack => Gear { grid: Some((4, 3)), ..gear(Wear::Back) },
            Kind::Rucksack => Gear { grid: Some((6, 4)), ..gear(Wear::Back) },
            Kind::HikingPack => Gear { grid: Some((7, 5)), weight: 1, ..gear(Wear::Back) },
            Kind::MilitaryRuck => Gear { grid: Some((8, 6)), weight: 1, ..gear(Wear::Back) },
            Kind::CargoPants => Gear { pockets: (1, 1), ..gear(Wear::Legs) },
            Kind::Bandolier => Gear { grid: Some((4, 1)), ammo_only: true, ..gear(Wear::Belt) },
            _ => return None,
        })
    }

    /// Whether it's a gun's rounds (what a bandolier holds).
    pub fn is_ammo(self) -> bool {
        matches!(self, Kind::Rounds | Kind::Shells | Kind::RifleRounds | Kind::Rounds556)
    }
}

/// How much an armor plate puts back.
pub const PLATE: u32 = 40;

/// What all that's worn weighs together (each piece 0 to 2), and what that
/// costs: the sprint's speed and the breath's, shares of the usual, and
/// how far a sprinting footfall is heard, metres.
pub fn burden(weight: u32) -> (f64, f64, f64) {
    let w = f64::from(weight.min(6));
    (1.0 - 0.035 * w, 1.0 + 0.12 * w, 3.5 * w)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heavier_is_slower_and_louder() {
        assert_eq!(burden(0), (1.0, 1.0, 0.0), "light gear costs nothing");
        let (fast, breath, heard) = burden(4);
        assert!(fast < 0.9 && breath > 1.4 && heard > 10.0);
        assert_eq!(burden(9), burden(6), "no worse than six");
    }
}
