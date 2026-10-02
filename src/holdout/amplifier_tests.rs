//! The Amplifier: down in the bunker, with signs to it; what's in hand
//! put through it a tier at a time, each dearer than the last.

use super::tests::{built, world_of};
use super::*;
use crate::weapon::{Hands, Weapon, amp};

fn amplifier(h: &Holdout) -> usize {
    h.arena.buys.iter().position(|b| b.wares == Wares::Amplifier).expect("an Amplifier")
}

#[test]
fn the_amplifier_is_down_in_the_bunker_and_signed_from_the_yard() {
    let (_, h) = world_of(built());
    assert_eq!(h.arena.buys.iter().filter(|b| b.wares == Wares::Amplifier).count(), 1);
    let b = h.arena.buys[amplifier(&h)];
    assert_eq!(h.arena.zones[b.zone], "THE BUNKER");
    // Under the ground, and a sign on each of the three levels of the
    // way to it: out in the yard, in the lobby, at the foot of the stairs.
    assert!(b.at.y < 0.0, "at {:?}", b.at);
    assert_eq!(h.arena.signs.len(), 3);
    assert!(h.arena.signs.iter().any(|s| s.at.y < 0.0) && h.arena.signs.iter().filter(|s| s.at.y > 0.0).count() == 2);
}

#[test]
fn a_gun_put_through_comes_out_more_each_time_and_dearer() {
    let (mut world, mut h) = world_of(built());
    let amp_at = Aimed::Buy(amplifier(&h));
    let mut bag = Holdout::loadout();
    let sidearm = Some(Slot::Sidearm);
    // Nothing in hand: nothing to do.
    assert_eq!(h.prompt(amp_at, &bag, None), "THE AMPLIFIER  ·  NOTHING IN HAND");
    assert_eq!(h.press(&mut world, 0, amp_at, &mut bag, None), (None, Some("NOTHING IN HAND"), None));
    // The pistol: 5,000 for the first.
    assert_eq!(h.prompt(amp_at, &bag, sidearm), "AMPLIFY PISTOL [5000]");
    h.wallets[0].points = 4999;
    assert_eq!(h.press(&mut world, 0, amp_at, &mut bag, sidearm).1, Some("NOT ENOUGH POINTS"));
    assert_eq!(bag.slot(Slot::Sidearm).map(|g| g.tier), Some(0));
    h.wallets[0].points = 40_000;
    assert_eq!(h.press(&mut world, 0, amp_at, &mut bag, sidearm), (Some(Sfx::Amplify), Some("AMPLIFIED"), sidearm));
    let gun = bag.slot(Slot::Sidearm).unwrap();
    // Half as many rounds again in it, full; and as many carried as that
    // makes a full carry.
    assert_eq!((gun.tier, gun.loaded, gun.name(), h.wallets[0].points), (1, 18, "HOT MIC", 35_000));
    assert_eq!(bag.count(Kind::Rounds), 180);
    assert_eq!(h.prompt(amp_at, &bag, sidearm), "AMPLIFY HOT MIC [10000]");
    // Twice more, and it's had all there is.
    h.press(&mut world, 0, amp_at, &mut bag, sidearm);
    h.press(&mut world, 0, amp_at, &mut bag, sidearm);
    let gun = bag.slot(Slot::Sidearm).unwrap();
    assert_eq!((gun.tier, gun.loaded, gun.name(), h.wallets[0].points), (3, 30, "LAST BROADCAST", 5000));
    assert_eq!(h.prompt(amp_at, &bag, sidearm), "LAST BROADCAST  ·  FULLY AMPLIFIED");
    assert_eq!(h.press(&mut world, 0, amp_at, &mut bag, sidearm), (None, Some("FULLY AMPLIFIED"), None));
    assert_eq!(h.wallets[0].points, 5000);
    // In hand it holds what it holds now, and hits that much harder.
    let mut hands = Hands::default();
    hands.tier = gun.tier;
    hands.take_up(sidearm, Weapon::Pistol, gun.loaded);
    assert_eq!((hands.mag, hands.capacity(), hands.power()), (30, 30, amp::power(3)));
}

#[test]
fn an_amplified_gun_s_rounds_are_bought_and_handed_out_by_what_it_carries_now() {
    let (mut world, mut h) = world_of(built());
    let mut bag = Holdout::loadout();
    h.wallets[0].points = 50_000;
    let wall = h.arena.buys.iter().position(|b| b.wares == Wares::Weapon(Kind::Pistol)).expect("a pistol on a wall");
    h.amplify_free(0, &mut bag, Some(Slot::Sidearm));
    h.amplify_free(0, &mut bag, Some(Slot::Sidearm));
    let (ammo, most) = spare(Kind::Pistol, 2).unwrap();
    assert_eq!((most, bag.count(ammo)), (240, 240));
    // Some shot off: the wall sells it back up to the bigger carry, and a
    // hound round cleared fills it to the same.
    bag.remove(ammo, 100);
    h.press(&mut world, 0, Aimed::Buy(wall), &mut bag, None);
    assert_eq!(bag.count(ammo), 240);
    bag.remove(ammo, 60);
    Holdout::max_ammo(&mut bag);
    assert_eq!(bag.count(ammo), 240);
    // It costs nothing, the dev's way.
    assert_eq!(h.wallets[0].points, 50_000 - price(Kind::Pistol) / 2);
}
