//! What a holdout's dead leave where they fall: now and then rounds (of
//! any kind at all: for a gun carried, or one yet to be bought, or a
//! friend's), less often a bandage, and now and then a medkit or an armor
//! plate. It lies there half a minute (`items::Ground`) for whoever looks
//! at it and takes it.

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
/// The rounds there are, and how many of each are left at once (a
/// magazine or so).
pub const ROUNDS: [(Kind, u32); 6] = [(Kind::Rounds, 24), (Kind::Shells, 8), (Kind::RifleRounds, 8), (Kind::Rounds556, 30), (Kind::Rounds762, 50), (Kind::FlameFuel, 100)];
/// A Juggernaut leaves this many things, always.
const JUGGERNAUT: usize = 2;

/// Something of what the dead leave, by `roll` (0–1) along the chances
/// (all of them `times` as likely): rounds, a bandage, a medkit, a plate,
/// or (past them all) nothing.
fn thing(roll: f64, times: f64, dice: &mut Dice) -> Option<Stack> {
    let mut edge = 0.0;
    for (chance, what) in [(AMMO, None), (BANDAGE, Some(Kind::Bandage)), (MEDKIT, Some(Kind::Medkit)), (PLATE, Some(Kind::ArmorPlate))] {
        edge += chance * times;
        if roll < edge {
            return Some(match what {
                Some(kind) => Stack::one(kind),
                None => {
                    let (kind, n) = ROUNDS[dice.next() as usize % ROUNDS.len()];
                    Stack::new(kind, n)
                }
            });
        }
    }
    None
}

/// What one of `kind` leaves, dead: mostly nothing. (A Hellhound, never
/// anything; a Juggernaut, always.)
pub fn left_by(kind: Dead, dice: &mut Dice) -> Vec<Stack> {
    if kind == Dead::Juggernaut {
        // (Something each time: the roll's along the chances alone.)
        let all = AMMO + BANDAGE + MEDKIT + PLATE;
        return (0..JUGGERNAUT).filter_map(|_| thing(dice.unit() * all * 0.999, 1.0, dice)).collect();
    }
    thing(dice.unit(), kind.traits().loot, dice).into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn about_one_in_five_leave_something_and_rounds_most_of_all() {
        let mut dice = Dice(0x1234_5677);
        let n = 20_000;
        let mut counts = std::collections::HashMap::new();
        let mut rounds = std::collections::HashSet::new();
        for _ in 0..n {
            for stack in left_by(Dead::Shambler, &mut dice) {
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
        // Rounds of every kind there is: any at all.
        assert_eq!(rounds.len(), ROUNDS.len());
        let every: std::collections::HashSet<Kind> = crate::weapon::Weapon::ALL.iter().filter_map(|w| w.spec().ammo).collect();
        assert_eq!(rounds, every, "every gun's rounds, and no others");
        // A hound never leaves a thing; a Juggernaut always two.
        assert!((0..500).all(|_| left_by(Dead::Hound, &mut dice).is_empty()));
        assert!((0..500).all(|_| left_by(Dead::Juggernaut, &mut dice).len() == JUGGERNAUT));
        // A Ripper's likelier to than a Shambler.
        let rippers = (0..n).filter(|_| !left_by(Dead::Ripper, &mut dice).is_empty()).count();
        assert!(rippers as f64 / f64::from(n) > 0.28, "{rippers}");
    }
}
