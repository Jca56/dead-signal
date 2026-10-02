//! What a holdout's dead leave where they fall: now and then rounds (for
//! a gun one of the players has, in hand or put away: never for one
//! nobody carries), less often a bandage, and now and then a medkit, an
//! armor plate, or something to throw. It lies there half a minute
//! (`items::Ground`) for whoever looks at it and takes it.

use bevy_ecs::prelude::Resource;

use crate::loot::bag::Bag;
use crate::loot::{Dice, Kind, Stack};
use crate::zombie::kind::Kind as Dead;

/// How long anything set down in a holdout lies there, seconds.
pub const LIES_FOR: f64 = 30.0;
/// The chance one of the dead leaves each thing (a Shambler's: the rest
/// are as much likelier as they are to carry anything, `Traits::loot`).
pub const AMMO: f64 = 0.10;
pub const BANDAGE: f64 = 0.05;
pub const MEDKIT: f64 = 0.02;
pub const PLATE: f64 = 0.02;
pub const MOLOTOV: f64 = 0.015;
pub const PIPE_BOMB: f64 = 0.01;
/// The rounds there are, and how many of each are left at once (a
/// magazine or so).
pub const ROUNDS: [(Kind, u32); 9] = [
    (Kind::Rounds, 24),
    (Kind::Shells, 8),
    (Kind::RifleRounds, 8),
    (Kind::Rounds556, 30),
    (Kind::Rounds762, 50),
    (Kind::FlameFuel, 100),
    (Kind::Rounds45, 16),
    (Kind::Rounds44, 12),
    (Kind::RoundsAk, 30),
];
/// A Juggernaut leaves this many things, always.
const JUGGERNAUT: usize = 2;

/// The rounds the players' guns take, each kind once: what the dead leave
/// is for one of them. (Kept in step with their bags by the run.)
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct Wanted(pub Vec<Kind>);

impl Wanted {
    /// What the guns in `bags` take: in a slot, or put away.
    pub fn by<'a>(bags: impl Iterator<Item = &'a Bag>) -> Self {
        let mut kinds = Vec::new();
        for ammo in bags.flat_map(Bag::everything).filter_map(|stack| stack.kind.weapon().and_then(|w| w.spec().ammo)) {
            if !kinds.contains(&ammo) {
                kinds.push(ammo);
            }
        }
        Wanted(kinds)
    }
}

/// Something of what the dead leave, by `roll` (0–1) along the chances
/// (all of them `times` as likely): rounds (one of the kinds `wanted`:
/// none are, and it's nothing), a bandage, a medkit, a plate, something
/// to throw, or (past them all) nothing.
fn thing(roll: f64, times: f64, dice: &mut Dice, wanted: &Wanted) -> Option<Stack> {
    let mut edge = 0.0;
    for (chance, what) in [(AMMO, None), (BANDAGE, Some(Kind::Bandage)), (MEDKIT, Some(Kind::Medkit)), (PLATE, Some(Kind::ArmorPlate)), (MOLOTOV, Some(Kind::Molotov)), (PIPE_BOMB, Some(Kind::PipeBomb))] {
        edge += chance * times;
        if roll < edge {
            return match what {
                Some(kind) => Some(Stack::one(kind)),
                None => {
                    let fits: Vec<(Kind, u32)> = ROUNDS.into_iter().filter(|(kind, _)| wanted.0.contains(kind)).collect();
                    (!fits.is_empty()).then(|| fits[dice.next() as usize % fits.len()]).map(|(kind, n)| Stack::new(kind, n))
                }
            };
        }
    }
    None
}

/// What one of `kind` leaves, dead, the players' guns taking the rounds
/// `wanted`: mostly nothing. (A Hellhound, never anything; a Juggernaut,
/// always.)
pub fn left_by(kind: Dead, dice: &mut Dice, wanted: &Wanted) -> Vec<Stack> {
    if kind == Dead::Juggernaut {
        // (Something each time: the roll's along the chances alone, and
        // past the rounds if there's no gun to feed.)
        let all = AMMO + BANDAGE + MEDKIT + PLATE + MOLOTOV + PIPE_BOMB;
        let from = if wanted.0.is_empty() { AMMO } else { 0.0 };
        return (0..JUGGERNAUT).filter_map(|_| thing(from + dice.unit() * (all - from) * 0.999, 1.0, dice, wanted)).collect();
    }
    thing(dice.unit(), kind.traits().loot, dice, wanted).into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::holdout::Holdout;
    use crate::loot::bag::Slot;

    #[test]
    fn about_one_in_five_leave_something_and_rounds_most_of_all() {
        let mut dice = Dice(0x1234_5677);
        let n = 20_000;
        // Every gun there is, carried: rounds of every kind are wanted.
        let every = Wanted(ROUNDS.map(|(kind, _)| kind).to_vec());
        let mut counts = std::collections::HashMap::new();
        let mut rounds = std::collections::HashSet::new();
        for _ in 0..n {
            for stack in left_by(Dead::Shambler, &mut dice, &every) {
                let ammo = ROUNDS.contains(&(stack.kind, stack.count));
                *counts.entry(if ammo { None } else { Some(stack.kind) }).or_insert(0u32) += 1;
                if ammo {
                    rounds.insert(stack.kind);
                }
            }
        }
        let share = |what| f64::from(counts.get(&what).copied().unwrap_or(0)) / f64::from(n);
        assert!((share(None) - AMMO).abs() < 0.01, "rounds {:.3}", share(None));
        assert!((share(Some(Kind::Bandage)) - BANDAGE).abs() < 0.008, "bandages {:.3}", share(Some(Kind::Bandage)));
        assert!((share(Some(Kind::Medkit)) - MEDKIT).abs() < 0.006 && (share(Some(Kind::ArmorPlate)) - PLATE).abs() < 0.006);
        assert!((share(Some(Kind::Molotov)) - MOLOTOV).abs() < 0.005 && (share(Some(Kind::PipeBomb)) - PIPE_BOMB).abs() < 0.005);
        // There's a line of rounds for every gun there is, and no others.
        assert_eq!(rounds.len(), ROUNDS.len());
        let guns: std::collections::HashSet<Kind> = crate::weapon::Weapon::ALL.iter().filter_map(|w| w.spec().ammo).collect();
        assert_eq!(rounds, guns, "every gun's rounds, and no others");
        // A hound never leaves a thing; a Juggernaut always two.
        assert!((0..500).all(|_| left_by(Dead::Hound, &mut dice, &every).is_empty()));
        assert!((0..500).all(|_| left_by(Dead::Juggernaut, &mut dice, &every).len() == JUGGERNAUT));
        // A Ripper's likelier to than a Shambler.
        let rippers = (0..n).filter(|_| !left_by(Dead::Ripper, &mut dice, &every).is_empty()).count();
        assert!(rippers as f64 / f64::from(n) > 0.28, "{rippers}");
    }

    #[test]
    fn the_rounds_left_are_only_ever_for_guns_the_players_carry() {
        let mut dice = Dice(0xBEEF);
        // One with a holdout's pistol; the other an AK in hand and a
        // shotgun put away.
        let first = Holdout::loadout();
        let mut second = Holdout::loadout();
        *second.slot_mut(Slot::Primary) = Some(Stack::gun(Kind::Ak47, 30));
        second.add(Stack::gun(Kind::Shotgun, 5));
        let wanted = Wanted::by([&first, &second].into_iter());
        assert_eq!(wanted, Wanted(vec![Kind::Rounds, Kind::RoundsAk, Kind::Shells]));
        let mut left = std::collections::HashSet::new();
        for _ in 0..20_000 {
            left.extend(left_by(Dead::Shambler, &mut dice, &wanted).into_iter().filter(|s| ROUNDS.iter().any(|(kind, _)| *kind == s.kind)).map(|s| s.kind));
        }
        assert_eq!(left, [Kind::Rounds, Kind::RoundsAk, Kind::Shells].into_iter().collect());
        // No gun between them: no rounds at all, but the rest as ever (and
        // a Juggernaut's two things all the same).
        let none = Wanted::by(std::iter::empty());
        let all: Vec<Stack> = (0..20_000).flat_map(|_| left_by(Dead::Shambler, &mut dice, &none)).collect();
        assert!(!all.is_empty() && all.iter().all(|s| !ROUNDS.iter().any(|(kind, _)| *kind == s.kind)));
        assert!((0..300).all(|_| left_by(Dead::Juggernaut, &mut dice, &none).len() == JUGGERNAUT));
    }
}
