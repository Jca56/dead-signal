//! A mystery drop's gun: which it is (luck's: the better the rarer, never
//! one whoever called for it carries, and now and then one that's been
//! through the Amplifier already), and its being taken up: in hand, with
//! a full carry of its rounds, what it takes the place of kept.

use super::Holdout;
use super::buy::spare;
use crate::loot::bag::{Bag, Slot};
use crate::loot::{Dice, Kind, Stack};

/// What can be in one, and how likely each is against the rest. (No
/// pistol: everyone's begun with one.)
pub const ODDS: [(Kind, u32); 6] = [(Kind::Rifle, 6), (Kind::Shotgun, 6), (Kind::Smg, 6), (Kind::AssaultRifle, 4), (Kind::Flamethrower, 2), (Kind::Lmg, 2)];
/// One in this many has been through the Amplifier, once.
pub const JACKPOT: u32 = 6;

/// Whether `kind` is carried in `bag`: in its slot, or put away.
fn carried(bag: &Bag, kind: Kind) -> bool {
    bag.count(kind) > 0 || Slot::ALL.iter().any(|&s| bag.slot(s).is_some_and(|g| g.kind == kind))
}

/// The gun in a mystery drop called for by whoever carries `bag`, by
/// `dice`'s luck: loaded, and amplified if it's that one in a few.
pub fn roll(dice: &mut Dice, bag: Option<&Bag>) -> Stack {
    let fresh: Vec<(Kind, u32)> = ODDS.iter().copied().filter(|&(kind, _)| !bag.is_some_and(|b| carried(b, kind))).collect();
    // (Carrying every one of them: any.)
    let pool = if fresh.is_empty() { ODDS.to_vec() } else { fresh };
    let mut pick = dice.next() % pool.iter().map(|(_, odds)| odds).sum::<u32>();
    let mut kind = pool[0].0;
    for &(k, odds) in &pool {
        if pick < odds {
            kind = k;
            break;
        }
        pick -= odds;
    }
    let mut gun = Stack::one(kind);
    gun.tier = u8::from(dice.next().is_multiple_of(JACKPOT));
    gun.loaded = gun.magazine().unwrap_or(0);
    gun
}

impl Holdout {
    /// Player `seat` takes `gun` up (a mystery drop's) into `bag`: in its
    /// slot, full, with a full carry of its rounds; what it takes the
    /// place of kept, as with one off the wall. The slot it's in.
    pub fn take(&mut self, seat: usize, bag: &mut Bag, mut gun: Stack) -> Option<Slot> {
        let slot = Slot::of(gun.kind)?;
        // (The very gun they carry there: the more amplified of the two.)
        if let Some(old) = bag.slot(slot).filter(|old| old.kind == gun.kind) {
            gun.tier = gun.tier.max(old.tier);
            *bag.slot_mut(slot) = None;
        }
        gun.loaded = gun.magazine().unwrap_or(0);
        if let Some((ammo, most)) = spare(gun.kind, gun.tier) {
            let have = bag.count(ammo);
            if have < most {
                bag.add(Stack::new(ammo, most - have));
            }
        }
        self.arm(seat, bag, gun);
        Some(slot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::holdout::arena::Arena;
    use lntrn_math::Vec3;

    fn holdout() -> Holdout {
        let arena = Arena { reach: 20.0, bounds: (Vec3::splat(-20.0), Vec3::splat(20.0)), zones: vec!["YARD"], start: 0, spawn: Vec3::ZERO, windows: Vec::new(), doors: Vec::new(), buys: Vec::new(), lamps: Vec::new(), signs: Vec::new() };
        Holdout::new(arena, 1, 1)
    }

    #[test]
    fn the_better_guns_are_rarer_and_one_in_six_comes_amplified() {
        let mut dice = Dice(0xC0FFEE);
        let rolls: Vec<Stack> = (0..6000).map(|_| roll(&mut dice, None)).collect();
        let share = |kind| rolls.iter().filter(|g| g.kind == kind).count() as f64 / rolls.len() as f64;
        let total: u32 = ODDS.iter().map(|(_, odds)| odds).sum();
        for (kind, odds) in ODDS {
            let fair = f64::from(odds) / f64::from(total);
            assert!((share(kind) - fair).abs() < 0.03, "{kind:?}: {:.3} of them, for {fair:.3}", share(kind));
        }
        assert!(share(Kind::Lmg) < share(Kind::AssaultRifle) && share(Kind::AssaultRifle) < share(Kind::Smg));
        assert!(rolls.iter().all(|g| g.kind != Kind::Pistol && g.tier <= 1 && Some(g.loaded) == g.magazine()), "loaded, and never a pistol");
        let amplified = rolls.iter().filter(|g| g.tier == 1).count() as f64 / rolls.len() as f64;
        assert!((amplified - 1.0 / f64::from(JACKPOT)).abs() < 0.03, "{amplified:.3}");
        // An amplified one's full of what it holds now.
        let big = rolls.iter().find(|g| g.tier == 1 && g.kind == Kind::Smg).expect("an amplified SMG");
        assert!(big.loaded > Kind::Smg.weapon().unwrap().spec().mag);
    }

    #[test]
    fn never_one_that_s_carried_unless_they_all_are() {
        let mut dice = Dice(77);
        let mut bag = Holdout::loadout();
        *bag.slot_mut(Slot::Primary) = Some(Stack::gun(Kind::Smg, 30));
        bag.add(Stack::gun(Kind::Shotgun, 5));
        assert!(carried(&bag, Kind::Smg) && carried(&bag, Kind::Shotgun) && !carried(&bag, Kind::Lmg));
        for _ in 0..400 {
            let gun = roll(&mut dice, Some(&bag));
            assert!(!matches!(gun.kind, Kind::Smg | Kind::Shotgun), "{gun:?}");
        }
        // Every one of them carried: any of them, then.
        for (kind, _) in ODDS {
            bag.add(Stack::gun(kind, 0));
        }
        let got: Vec<Kind> = (0..200).map(|_| roll(&mut dice, Some(&bag)).kind).collect();
        assert!(ODDS.iter().all(|(kind, _)| got.contains(kind)), "{got:?}");
    }

    #[test]
    fn taken_it_s_in_hand_with_its_rounds_and_what_it_took_the_place_of_is_kept() {
        let mut h = holdout();
        let mut bag = Holdout::loadout();
        // Into an empty slot: there, full, with a full carry.
        let mut lmg = Stack::one(Kind::Lmg);
        lmg.tier = 1;
        assert_eq!(h.take(0, &mut bag, lmg), Some(Slot::Primary));
        let held = bag.slot(Slot::Primary).expect("the LMG");
        assert_eq!((held.kind, held.tier, Some(held.loaded)), (Kind::Lmg, 1, held.magazine()));
        let (ammo, most) = spare(Kind::Lmg, 1).unwrap();
        assert_eq!(bag.count(ammo), most);
        // Another long gun: in hand, the LMG put away in the pack.
        assert_eq!(h.take(0, &mut bag, Stack::one(Kind::Shotgun)), Some(Slot::Primary));
        assert_eq!(bag.slot(Slot::Primary).map(|g| g.kind), Some(Kind::Shotgun));
        assert!(bag.count(Kind::Lmg) == 1 && h.spilled.is_empty());
        assert_eq!(bag.count(Kind::Shells), spare(Kind::Shotgun, 0).unwrap().1);
        // The same gun again, plain, over an amplified one: the amplified
        // one's kept (not two of them), topped up.
        let mut shotgun = bag.slot(Slot::Primary).unwrap();
        (shotgun.tier, shotgun.loaded) = (2, 1);
        *bag.slot_mut(Slot::Primary) = Some(shotgun);
        h.take(0, &mut bag, Stack::one(Kind::Shotgun));
        let held = bag.slot(Slot::Primary).unwrap();
        assert_eq!((held.kind, held.tier, Some(held.loaded)), (Kind::Shotgun, 2, held.magazine()));
        assert_eq!(bag.count(Kind::Shotgun), 0);
        assert_eq!(bag.count(Kind::Shells), spare(Kind::Shotgun, 2).unwrap().1);
        // No room in the pack for what's put out of hand: at their feet.
        let full = bag.add(Stack::new(Kind::Beans, 10_000));
        assert!(full.count > 0, "the pack's full");
        h.take(0, &mut bag, Stack::one(Kind::Rifle));
        assert_eq!(h.spilled.iter().map(|(seat, s)| (*seat, s.kind)).collect::<Vec<_>>(), [(0, Kind::Shotgun)]);
    }
}
