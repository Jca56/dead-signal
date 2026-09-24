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
    /// What it cries on seeing the player, and as it swipes.
    pub snarl: Sfx,
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
    loot: 1.75,
};

impl Kind {
    pub fn traits(self) -> &'static Traits {
        match self {
            Kind::Shambler => &SHAMBLER,
            Kind::Ripper => &RIPPER,
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
        for k in [Kind::Shambler, Kind::Ripper] {
            let w = k.traits().swipe;
            assert!(w.strike_at < w.time && w.range < w.reach, "{k:?}");
        }
    }
}
