//! Armor in a holdout: on the walls, put on as it's bought, made whole
//! for half, and plates to mend it with.

use super::tests::{built, world_of};
use super::*;
use crate::loot::gear::Wear;

/// The wall buy of `wares`.
fn on_the_wall(h: &Holdout, wares: Wares) -> usize {
    h.arena.buys.iter().position(|b| b.wares == wares).unwrap_or_else(|| panic!("no {wares:?} on a wall"))
}

#[test]
fn armor_off_the_wall_is_worn_at_once_and_made_whole_again_for_half() {
    let (mut world, mut h) = world_of(built());
    let mut bag = Holdout::loadout();
    let vest = on_the_wall(&h, Wares::Gear(Kind::LightVest));
    assert_eq!(h.prompt(Aimed::Buy(vest), &bag), "BUY LIGHT VEST [750]");
    h.wallets[0].points = 700;
    assert_eq!(h.press(&mut world, 0, Aimed::Buy(vest), &mut bag).1, Some("NOT ENOUGH POINTS"));
    assert_eq!(bag.armor(), (0, 0));
    h.wallets[0].points = 2000;
    let (sound, said, took) = h.press(&mut world, 0, Aimed::Buy(vest), &mut bag);
    assert!(sound.is_some() && said.is_none() && took.is_none());
    assert_eq!((bag.armor(), h.wallets[0].points), ((40, 40), 1250));
    assert_eq!(bag.worn(Wear::Chest).map(|s| s.kind), Some(Kind::LightVest));
    // Whole, there's nothing to buy.
    assert_eq!(h.prompt(Aimed::Buy(vest), &bag), "LIGHT VEST · WORN");
    assert_eq!(h.press(&mut world, 0, Aimed::Buy(vest), &mut bag).1, Some("ARMOR WHOLE"));
    assert_eq!(h.wallets[0].points, 1250);
    // A blow soaked: half to make it whole.
    assert_eq!(bag.soak(25), 25);
    assert_eq!(h.prompt(Aimed::Buy(vest), &bag), "REPAIR LIGHT VEST [375]");
    h.press(&mut world, 0, Aimed::Buy(vest), &mut bag);
    assert_eq!((bag.armor(), h.wallets[0].points), ((40, 40), 875));
    // A helmet's worn with it; and the pack's as it was, its rounds in it.
    let helmet = on_the_wall(&h, Wares::Gear(Kind::BikeHelmet));
    h.press(&mut world, 0, Aimed::Buy(helmet), &mut bag);
    assert_eq!((bag.armor(), h.wallets[0].points), ((55, 55), 575));
    assert!(bag.count(Kind::Rounds) > 0 && bag.refit(Holdout::fit()).is_empty() && bag.count(Kind::Rounds) > 0);
    assert_eq!((bag.pack.w, bag.pack.h), PACK);
}

#[test]
fn better_armor_takes_the_place_of_less_and_less_is_not_sold_over_better() {
    let (mut world, mut h) = world_of(built());
    let mut bag = Holdout::loadout();
    h.wallets[0].points = 10_000;
    let (vest, carrier) = (on_the_wall(&h, Wares::Gear(Kind::LightVest)), on_the_wall(&h, Wares::Gear(Kind::PlateCarrier)));
    h.press(&mut world, 0, Aimed::Buy(vest), &mut bag);
    h.press(&mut world, 0, Aimed::Buy(carrier), &mut bag);
    assert_eq!((bag.armor(), h.wallets[0].points), ((80, 80), 10_000 - 750 - 2500));
    assert_eq!(h.prompt(Aimed::Buy(vest), &bag), "LIGHT VEST · WEARING BETTER");
    assert_eq!(h.press(&mut world, 0, Aimed::Buy(vest), &mut bag).1, Some("WEARING BETTER"));
    assert_eq!(bag.worn(Wear::Chest).map(|s| s.kind), Some(Kind::PlateCarrier));
    // Plates are carried, and mend it.
    let plate = on_the_wall(&h, Wares::Kit(Kind::ArmorPlate));
    h.press(&mut world, 0, Aimed::Buy(plate), &mut bag);
    h.press(&mut world, 0, Aimed::Buy(plate), &mut bag);
    assert_eq!(bag.count(Kind::ArmorPlate), 2);
    bag.soak(70);
    assert_eq!(bag.mend(crate::loot::gear::PLATE), 40);
    assert_eq!(bag.armor(), (50, 80));
}

#[test]
fn every_piece_of_armor_is_on_a_wall_with_nothing_of_its_own_to_carry_things_in() {
    let (_, h) = world_of(built());
    let sold: Vec<Kind> = h.arena.buys.iter().filter_map(|b| if let Wares::Gear(k) = b.wares { Some(k) } else { None }).collect();
    assert_eq!(sold.len(), 4);
    for kind in &sold {
        let g = kind.gear().expect("something to wear");
        // (Bought, it's put straight on: nothing laid out again to suit it.)
        assert!(g.armor > 0 && g.grid.is_none() && g.pockets == (0, 0), "{kind:?}");
        assert!(price(*kind) > 0 && price(*kind) != 1000, "{kind:?} has its own price");
    }
    assert_eq!(h.arena.buys.iter().filter(|b| b.wares == Wares::Kit(Kind::ArmorPlate)).count(), 3);
    // The cheap helmet's in the start room; the best of it deep in.
    let zone = |wares| h.arena.zones[h.arena.buys[on_the_wall(&h, wares)].zone];
    assert_eq!(zone(Wares::Gear(Kind::BikeHelmet)), "CONTROL ROOM");
    assert_eq!(zone(Wares::Gear(Kind::LightVest)), "THE YARD");
    assert_eq!(zone(Wares::Gear(Kind::MilitaryHelmet)), "BARRACKS");
    assert_eq!(zone(Wares::Gear(Kind::PlateCarrier)), "THE BUNKER");
}
