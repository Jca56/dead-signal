//! A weapon made more of by the Amplifier (a holdout's machine, down in
//! the bunker): up to three times, each a tier. A tier's a straight
//! raise of its numbers (how hard it hits, how many rounds it holds and
//! so how many are carried for it), a name of its own, and a glow.

use super::Weapon;

/// How many times a weapon can be put through.
pub const TIERS: u8 = 3;
/// What each tier makes of a weapon's damage, and of the rounds it holds.
const POWER: [f64; 4] = [1.0, 2.0, 3.0, 4.5];
const ROUNDS: [f64; 4] = [1.0, 1.5, 2.0, 2.5];
/// Each tier's glow (linear RGB): a valve's amber, violet, and the pale
/// blue of the static itself.
const GLOW: [[f32; 3]; 4] = [[0.0; 3], [1.0, 0.52, 0.10], [0.72, 0.28, 1.0], [0.40, 0.88, 1.0]];

fn at(tier: u8) -> usize {
    usize::from(tier.min(TIERS))
}

/// How many times as hard a weapon of `tier` hits.
pub fn power(tier: u8) -> f64 {
    POWER[at(tier)]
}

/// How many rounds `weapon` holds at `tier`.
pub fn capacity(weapon: Weapon, tier: u8) -> u32 {
    (f64::from(weapon.spec().mag) * ROUNDS[at(tier)]).round() as u32
}

/// A weapon of `tier`'s glow, if it has one.
pub fn glow(tier: u8) -> Option<[f32; 3]> {
    (tier > 0).then(|| GLOW[at(tier)])
}

/// What `weapon` is called at `tier`: its own name, then one for each
/// time it's been through.
pub fn name(weapon: Weapon, tier: u8) -> &'static str {
    let names: [&'static str; 3] = match weapon {
        Weapon::Fists => return weapon.spec().name,
        Weapon::Pistol => ["HOT MIC", "DEAD AIR", "LAST BROADCAST"],
        Weapon::Shotgun => ["LOUD AND CLEAR", "WIDE BAND", "WHITE NOISE"],
        Weapon::Rifle => ["LONG WAVE", "LINE OF SIGHT", "SKY WAVE"],
        Weapon::Smg => ["CHATTERBOX", "CROSS TALK", "BUSY SIGNAL"],
        Weapon::AssaultRifle => ["CARRIER WAVE", "HIGH GAIN", "FULL DUPLEX"],
        Weapon::Lmg => ["BROADBAND", "JAMMER", "WALL OF SOUND"],
        Weapon::Flamethrower => ["WARM FRONT", "HOT LINE", "SOLAR FLARE"],
        Weapon::Pistol45 => ["LOW END", "HEAVY ROTATION", "DROP THE BASS"],
        Weapon::Magnum => ["BIG VOICE", "SIX O'CLOCK NEWS", "SIGN OFF"],
        Weapon::MiniUzi => ["MOTORMOUTH", "PARTY LINE", "ALL CHANNELS"],
        Weapon::Ak47 => ["SHORTWAVE", "OVERDRIVE", "PIRATE RADIO"],
        Weapon::Bullpup => ["FINE TUNING", "CLEAR CHANNEL", "PERFECT PITCH"],
        Weapon::Rpk => ["HEAVY TRAFFIC", "FEEDBACK LOOP", "ROLLING THUNDER"],
        Weapon::Knife => ["SHARP NOTE", "STATIC SHOCK", "CUT SIGNAL"],
        Weapon::Machete => ["CLEAN CUT", "HARD CUT", "CUT TO BLACK"],
        Weapon::Axe => ["BREAKING NEWS", "HARD RESET", "FINAL CUT"],
    };
    match at(tier) {
        0 => weapon.spec().name,
        t => names[t - 1],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tier_is_more_and_has_a_name_of_its_own() {
        let mut names = std::collections::HashSet::new();
        for weapon in Weapon::ALL {
            for tier in 0..=TIERS {
                assert!(!name(weapon, tier).is_empty());
                if weapon != Weapon::Fists {
                    assert!(names.insert(name(weapon, tier)), "{} twice", name(weapon, tier));
                }
                if tier > 0 {
                    assert!(power(tier) > power(tier - 1));
                    assert!(capacity(weapon, tier) > capacity(weapon, tier - 1) || weapon.spec().mag == 0);
                    assert!(glow(tier).is_some());
                }
            }
            assert_eq!((name(weapon, 0), capacity(weapon, 0), glow(0)), (weapon.spec().name, weapon.spec().mag, None));
            // (Past the last tier there's no more.)
            assert_eq!(name(weapon, 9), name(weapon, TIERS));
        }
        assert_eq!((capacity(Weapon::Pistol, 1), capacity(Weapon::Pistol, 3), power(3)), (18, 30, 4.5));
    }
}
