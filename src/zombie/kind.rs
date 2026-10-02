//! The kinds of dead: the common Shambler and the specials, each its own
//! health, pace, sight, swipe (how quick, how far, how hard, and what it
//! leaves in you) and voice. The brain (`brain.rs`) is the same for all;
//! these say how it plays out.

use crate::sound::Sfx;
use crate::vitals::Affliction;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Shambler,
    /// Lean and fast, all claws: runs you down and opens you up, but a
    /// couple of good hits drop it.
    Ripper,
    /// Bloated with bile: keeps its distance and lobs it at you, and dead,
    /// swells and bursts over everything near.
    Spitter,
    /// Huge and plated in scrap: roars, then charges; turn it into a wall
    /// and it's dazed. Its front shrugs off shots; its back doesn't.
    Juggernaut,
    /// A dog long dead and still burning: quicker than anything on two
    /// legs, and a few shots drop it; its bite sets you alight, fire's
    /// nothing to it, and dead it goes up in flames where it falls.
    Hound,
}

/// Its swipe: seconds it takes, when in it the blow lands, the rest after,
/// how close it swipes from and how far the swipe reaches; what it takes
/// off, and what it leaves.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Swipe {
    pub time: f64,
    pub strike_at: f64,
    pub cooldown: f64,
    pub range: f64,
    pub reach: f64,
    pub damage: f64,
    pub leaves: Option<Affliction>,
}

/// How it plays.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Traits {
    pub hp: f64,
    /// Its walk, hunting run and lunge, m/s; and one in `fast_share`
    /// walks fast (`fast`).
    pub pace: (f64, f64, f64),
    pub fast: (f64, f64, f64),
    pub fast_share: f64,
    /// Times the usual sight.
    pub sight: f64,
    /// How fast it turns hunting, rad/s.
    pub turn: f64,
    /// Its lunge starts this far off.
    pub lunge: f64,
    pub swipe: Swipe,
    /// What it cries on seeing the player, and as it swipes; and what it
    /// mutters, going about.
    pub snarl: Sfx,
    pub voice: Sfx,
    /// Its drop: how likely (a share of a corpse's).
    pub loot: f64,
}

const SHAMBLER: Traits = Traits {
    hp: 150.0,
    pace: (1.6, 2.0, 3.8),
    fast: (3.0, 3.4, 4.5),
    fast_share: 0.25,
    sight: 1.0,
    turn: 3.0,
    lunge: 2.2,
    swipe: Swipe { time: 0.9, strike_at: 0.4, cooldown: 1.2, range: 1.4, reach: 1.8, damage: 20.0, leaves: None },
    snarl: Sfx::Snarl,
    voice: Sfx::Groan,
    loot: 1.0,
};

const RIPPER: Traits = Traits {
    hp: 70.0,
    pace: (7.5, 7.5, 9.5),
    fast: (7.5, 7.5, 9.5),
    fast_share: 0.0,
    sight: 1.4,
    turn: 6.0,
    lunge: 3.5,
    swipe: Swipe { time: 0.45, strike_at: 0.18, cooldown: 0.3, range: 1.5, reach: 1.9, damage: 7.0, leaves: Some(Affliction::Bleed) },
    snarl: Sfx::Shriek,
    voice: Sfx::Groan,
    loot: 1.75,
};

const SPITTER: Traits = Traits {
    hp: 110.0,
    pace: (1.5, 1.8, 2.2),
    fast: (1.5, 1.8, 2.2),
    fast_share: 0.0,
    sight: 1.3,
    turn: 2.5,
    lunge: 0.0,
    swipe: Swipe { time: 0.9, strike_at: 0.4, cooldown: 1.5, range: 1.4, reach: 1.7, damage: 10.0, leaves: None },
    snarl: Sfx::Retch,
    voice: Sfx::Groan,
    loot: 1.5,
};

const JUGGERNAUT: Traits = Traits {
    hp: 900.0,
    // (Its lunge is its charge: it never lunges otherwise.)
    pace: (2.0, 2.4, 11.0),
    fast: (2.0, 2.4, 11.0),
    fast_share: 0.0,
    sight: 1.2,
    turn: 1.8,
    lunge: 0.0,
    swipe: Swipe { time: 1.3, strike_at: 0.65, cooldown: 1.2, range: 2.1, reach: 2.8, damage: 40.0, leaves: None },
    snarl: Sfx::Bellow,
    voice: Sfx::Groan,
    loot: 1.0,
};

const HOUND: Traits = Traits {
    hp: 60.0,
    // (Its lunge is its leap, the last few metres.)
    pace: (8.2, 8.2, 10.5),
    fast: (8.2, 8.2, 10.5),
    fast_share: 0.0,
    sight: 1.5,
    turn: 7.0,
    lunge: 4.0,
    swipe: Swipe { time: 0.5, strike_at: 0.2, cooldown: 0.6, range: 1.6, reach: 2.0, damage: 10.0, leaves: Some(Affliction::Burn) },
    snarl: Sfx::Bark,
    voice: Sfx::Growl,
    loot: 0.0,
};

impl Kind {
    pub const ALL: [Kind; 5] = [Kind::Shambler, Kind::Ripper, Kind::Spitter, Kind::Juggernaut, Kind::Hound];

    /// Its name, as it's shown.
    pub fn name(self) -> &'static str {
        match self {
            Kind::Shambler => "SHAMBLER",
            Kind::Ripper => "RIPPER",
            Kind::Spitter => "SPITTER",
            Kind::Juggernaut => "JUGGERNAUT",
            Kind::Hound => "HELLHOUND",
        }
    }

    /// Whether fire's nothing to it.
    pub fn fireproof(self) -> bool {
        self == Kind::Hound
    }

    pub fn traits(self) -> &'static Traits {
        match self {
            Kind::Shambler => &SHAMBLER,
            Kind::Ripper => &RIPPER,
            Kind::Spitter => &SPITTER,
            Kind::Juggernaut => &JUGGERNAUT,
            Kind::Hound => &HOUND,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ripper_runs_you_down_but_you_can_sprint_from_it() {
        let r = Kind::Ripper.traits();
        assert!(r.pace.1 > crate::player::WALK && r.pace.1 < crate::player::SPRINT);
        assert!(r.hp < Kind::Shambler.traits().hp / 2.0 + 1.0);
        // Quicker blows, each less, but they cut.
        let (s, rs) = (Kind::Shambler.traits().swipe, r.swipe);
        assert!(rs.time + rs.cooldown < s.time + s.cooldown && rs.damage < s.damage && rs.leaves == Some(Affliction::Bleed));
        for k in Kind::ALL {
            let w = k.traits().swipe;
            assert!(w.strike_at < w.time && w.range < w.reach, "{k:?}");
        }
    }

    #[test]
    fn every_kind_is_in_the_list_of_them_in_its_place() {
        // (Counted by kind, as a number: the dev's readout.)
        assert!(Kind::ALL.iter().enumerate().all(|(i, k)| *k as usize == i));
        let named: std::collections::HashSet<&str> = Kind::ALL.iter().map(|k| k.name()).collect();
        assert_eq!(named.len(), Kind::ALL.len());
    }

    #[test]
    fn a_hound_outruns_a_ripper_but_not_a_sprint_and_goes_down_easier() {
        let (h, r) = (Kind::Hound.traits(), Kind::Ripper.traits());
        assert!(h.pace.1 > r.pace.1 && h.pace.1 < crate::player::SPRINT && h.pace.2 > crate::player::SPRINT);
        assert!(h.hp < r.hp && h.swipe.leaves == Some(Affliction::Burn));
        assert!(Kind::Hound.fireproof() && !Kind::Ripper.fireproof());
    }
}
