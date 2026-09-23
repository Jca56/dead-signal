//! Perks: what levels buy. Eight of them, three ranks each, a point a
//! level to spend; each rank costs a point more than the last. What every
//! rank does is here, and nowhere else: the rest of the game asks.

/// One of the perks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Perk {
    Tough,
    Lungs,
    Medic,
    LightFeet,
    PackMule,
    DeepPockets,
    QuickHands,
    Brawler,
}

pub const ALL: [Perk; 8] = [Perk::Tough, Perk::Lungs, Perk::Medic, Perk::LightFeet, Perk::PackMule, Perk::DeepPockets, Perk::QuickHands, Perk::Brawler];

/// The most ranks a perk has.
pub const RANKS: u8 = 3;

/// The backpack's and the pockets' size at each rank of theirs.
const PACK: [(u8, u8); 4] = [(6, 4), (7, 4), (7, 5), (8, 5)];
const POCKETS: [(u8, u8); 4] = [(2, 2), (3, 2), (3, 3), (4, 3)];

impl Perk {
    pub fn name(self) -> &'static str {
        match self {
            Perk::Tough => "TOUGH",
            Perk::Lungs => "LUNGS",
            Perk::Medic => "FIELD MEDIC",
            Perk::LightFeet => "LIGHT FEET",
            Perk::PackMule => "PACK MULE",
            Perk::DeepPockets => "DEEP POCKETS",
            Perk::QuickHands => "QUICK HANDS",
            Perk::Brawler => "BRAWLER",
        }
    }

    /// Its name in a save file, never to change.
    pub fn key(self) -> &'static str {
        match self {
            Perk::Tough => "tough",
            Perk::Lungs => "lungs",
            Perk::Medic => "field_medic",
            Perk::LightFeet => "light_feet",
            Perk::PackMule => "pack_mule",
            Perk::DeepPockets => "deep_pockets",
            Perk::QuickHands => "quick_hands",
            Perk::Brawler => "brawler",
        }
    }

    /// What it does at `rank` (0: nothing yet), in a few words.
    pub fn does(self, rank: u8) -> String {
        let r = f64::from(rank);
        let pct = |per: f64| format!("{:.0}%", per * r * 100.0);
        match self {
            Perk::Tough => format!("{:.0} max health", Perks::tough(rank)),
            Perk::Lungs => format!("+{} stamina", pct(0.25)),
            Perk::Medic => format!("Kits {} faster, heal +{}", pct(0.25), pct(0.2)),
            Perk::LightFeet => format!("Seen {} less far", pct(0.15)),
            Perk::PackMule => {
                let (w, h) = PACK[usize::from(rank)];
                format!("Backpack {w}×{h}")
            }
            Perk::DeepPockets => {
                let (w, h) = POCKETS[usize::from(rank)];
                format!("Safe pockets {w}×{h}")
            }
            Perk::QuickHands => format!("Reload +{}, search +{}", pct(0.2), pct(0.25)),
            Perk::Brawler => format!("Melee +{}", pct(0.25)),
        }
    }
}

/// The ranks taken in every perk.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Perks([u8; 8]);

/// What the next rank of a perk at `rank` costs.
pub fn cost(rank: u8) -> u32 {
    u32::from(rank) + 1
}

/// Points to spend at `level`: one a level.
pub fn points(level: u32) -> u32 {
    level
}

impl Perks {
    fn at(p: Perk) -> usize {
        ALL.iter().position(|&q| q == p).expect("every perk is in ALL")
    }

    pub fn rank(&self, p: Perk) -> u8 {
        self.0[Self::at(p)]
    }

    pub fn set(&mut self, p: Perk, rank: u8) {
        self.0[Self::at(p)] = rank.min(RANKS);
    }

    /// Points spent on all of them.
    pub fn spent(&self) -> u32 {
        self.0.iter().map(|&r| (0..r).map(cost).sum::<u32>()).sum()
    }

    fn r(&self, p: Perk) -> f64 {
        f64::from(self.rank(p))
    }

    fn tough(rank: u8) -> f64 {
        100.0 + 15.0 * f64::from(rank)
    }

    // ---- what they do --------------------------------------------------------

    pub fn max_hp(&self) -> f64 {
        Self::tough(self.rank(Perk::Tough))
    }

    /// Stamina, and how fast it comes back: as a share of the usual.
    pub fn lungs(&self) -> f64 {
        1.0 + 0.25 * self.r(Perk::Lungs)
    }

    /// How long a kit takes (a share of the usual), and how much it heals.
    pub fn kit_time(&self) -> f64 {
        0.75f64.powf(self.r(Perk::Medic))
    }

    pub fn kit_heals(&self) -> f64 {
        1.0 + 0.2 * self.r(Perk::Medic)
    }

    /// How far the dead see the player, and hear them search: shares.
    pub fn seen_from(&self) -> f64 {
        1.0 - 0.15 * self.r(Perk::LightFeet)
    }

    pub fn search_heard(&self) -> f64 {
        1.0 - 0.2 * self.r(Perk::LightFeet)
    }

    pub fn pack(&self) -> (u8, u8) {
        PACK[usize::from(self.rank(Perk::PackMule))]
    }

    pub fn pockets(&self) -> (u8, u8) {
        POCKETS[usize::from(self.rank(Perk::DeepPockets))]
    }

    /// How fast a reload runs, and a search: multiples of the usual.
    pub fn reload_speed(&self) -> f64 {
        1.0 + 0.2 * self.r(Perk::QuickHands)
    }

    pub fn search_speed(&self) -> f64 {
        1.0 + 0.25 * self.r(Perk::QuickHands)
    }

    /// Melee damage and shove: a multiple of the usual.
    pub fn melee(&self) -> f64 {
        1.0 + 0.25 * self.r(Perk::Brawler)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranks_cost_more_as_they_go_and_do_what_they_say() {
        let mut p = Perks::default();
        assert_eq!(p.spent(), 0);
        p.set(Perk::Tough, 3);
        p.set(Perk::Brawler, 2);
        assert_eq!(p.spent(), (1 + 2 + 3) + (1 + 2));
        assert_eq!(p.max_hp(), 145.0);
        // Two blows of a rank-2 brawler kill (150 health, 50 a blow).
        assert!(50.0 * p.melee() * 2.0 >= 150.0);
        assert!(50.0 * Perks::default().melee() * 2.0 < 150.0);
        assert_eq!(Perks::default().pack(), crate::loot::bag::PACK);
        assert_eq!(Perks::default().pockets(), crate::loot::bag::POCKETS);
        p.set(Perk::PackMule, 9);
        assert_eq!(p.pack(), (8, 5), "ranks stop at three");
        for perk in ALL {
            for rank in 0..=RANKS {
                assert!(!perk.does(rank).is_empty());
            }
        }
    }
}
