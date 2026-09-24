use super::*;

#[test]
fn right_click_sends_things_across_and_keeps_what_wont_fit() {
    let mut bag = Bag::with_rounds();
    let mut crate_ = Grid::new(4, 3);
    crate_.place(Stack::new(Kind::Rounds, 12));
    crate_.place(Stack::one(Kind::Watch));
    let mut shelves = Shelves { bag: &mut bag, loot: Some(("CRATE", &mut crate_)), sell: None };
    // Rounds out of the crate top up the pocket's 24 to 30, the rest
    // take a place of their own.
    assert_eq!(quick_move(&mut shelves, Which::Loot, 0), 12);
    assert_eq!(shelves.bag.count(Kind::Rounds), 36);
    assert_eq!(shelves.bag.pockets.items[0].stack.count, 30);
    // And back: the pocket's rounds go into the crate.
    let i = shelves.bag.pockets.items.iter().position(|i| i.stack.kind == Kind::Rounds).unwrap();
    quick_move(&mut shelves, Which::Pockets, i);
    assert_eq!(shelves.loot.as_ref().unwrap().1.count(Kind::Rounds), 30);
    // Nothing searched: the pack and pockets trade.
    let mut bag = Bag::with_rounds();
    let mut shelves = Shelves { bag: &mut bag, loot: None, sell: None };
    quick_move(&mut shelves, Which::Pockets, 0);
    assert_eq!((shelves.bag.pack.count(Kind::Rounds), shelves.bag.pockets.count(Kind::Rounds)), (24, 0));
    // A full pack sends nothing, loses nothing.
    let mut bag = Bag::with_rounds();
    for _ in 0..6 {
        bag.pack.place(Stack::one(Kind::Battery));
    }
    let mut shelves = Shelves { bag: &mut bag, loot: None, sell: None };
    assert_eq!(quick_move(&mut shelves, Which::Pockets, 0), 0);
    assert_eq!(shelves.bag.pockets.count(Kind::Rounds), 24);
}

#[test]
fn a_stack_dropped_on_its_kind_tops_it_up_and_the_rest_goes_back() {
    // 28 rounds in the pack; 7 found in a crate, dragged onto them.
    let mut bag = Bag::with_rounds();
    bag.pockets.items.clear();
    bag.pack.put(Stack::new(Kind::Rounds, 28), 2, 1, false);
    let mut crate_ = Grid::new(4, 3);
    crate_.put(Stack::new(Kind::Rounds, 7), 1, 1, false);
    let mut shelves = Shelves { bag: &mut bag, loot: Some(("CRATE", &mut crate_)), sell: None };
    let item = shelves.grid(Which::Loot).unwrap().take(0);
    let held = Held { item, from: Which::Loot, was: item, grab: Vec2::ZERO };
    assert_eq!(landing(&shelves.bag.pack, item, (2, 1), (2, 1)), Landing::Merge(0, 2), "green: room for 2");
    assert_eq!(release(&mut shelves, held, Which::Pack, (2, 1), (2, 1)), 2);
    assert_eq!(shelves.bag.pack.items[0].stack.count, 30);
    let back = &shelves.loot.as_ref().unwrap().1.items[0];
    assert_eq!((back.stack.count, back.x, back.y), (5, 1, 1), "the 5 went back to the crate");
    // Onto a full stack: red, and nothing moves.
    let item = shelves.grid(Which::Loot).unwrap().take(0);
    assert_eq!(landing(&shelves.bag.pack, item, (2, 1), (2, 1)), Landing::Blocked);
    let held = Held { item, from: Which::Loot, was: item, grab: Vec2::ZERO };
    assert_eq!(release(&mut shelves, held, Which::Pack, (2, 1), (2, 1)), 0);
    assert_eq!(shelves.loot.as_ref().unwrap().1.count(Kind::Rounds), 5);
    // Onto something else: red.
    assert_eq!(landing(&shelves.bag.pack, Item { stack: Stack::one(Kind::Watch), ..item }, (2, 1), (2, 1)), Landing::Blocked);
}

#[test]
fn right_click_puts_a_weapon_in_its_empty_slot_and_takes_it_out() {
    let mut bag = Bag::empty();
    bag.pack.put(Stack::gun(Kind::Pistol, 6), 0, 0, false);
    let mut shelves = Shelves { bag: &mut bag, loot: None, sell: None };
    assert_eq!(quick_move(&mut shelves, Which::Pack, 0), 1);
    assert_eq!(shelves.bag.slot(Slot::Sidearm), Some(Stack::gun(Kind::Pistol, 6)), "in its slot, rounds and all");
    assert!(shelves.bag.pack.items.is_empty());
    // And out again, into the pack.
    assert_eq!(quick_move(&mut shelves, Which::Slot(Slot::Sidearm), 0), 1);
    assert_eq!((shelves.bag.slot(Slot::Sidearm), shelves.bag.pack.items[0].stack), (None, Stack::gun(Kind::Pistol, 6)));
    // With something searched, out of the slot goes into it.
    let mut crate_ = Grid::new(4, 3);
    bag.slots[Slot::Sidearm.index()] = Some(Stack::gun(Kind::Pistol, 2));
    let mut shelves = Shelves { bag: &mut bag, loot: Some(("CRATE", &mut crate_)), sell: None };
    assert_eq!(quick_move(&mut shelves, Which::Slot(Slot::Sidearm), 0), 1);
    assert_eq!(shelves.loot.as_ref().unwrap().1.items[0].stack, Stack::gun(Kind::Pistol, 2));
}

#[test]
fn a_weapon_dropped_on_a_full_slot_swaps_and_anything_else_goes_back() {
    let mut bag = Bag::empty();
    bag.slots[Slot::Sidearm.index()] = Some(Stack::gun(Kind::Pistol, 12));
    let mut crate_ = Grid::new(4, 3);
    crate_.put(Stack::gun(Kind::Pistol, 3), 1, 1, false);
    crate_.put(Stack::one(Kind::Watch), 3, 2, false);
    let mut shelves = Shelves { bag: &mut bag, loot: Some(("CRATE", &mut crate_)), sell: None };
    // The crate's pistol onto the sidearm: the full one goes to its place
    // in the crate.
    let item = shelves.take(Which::Loot, 0).unwrap();
    assert_eq!(slots::release(&mut shelves, Held { item, from: Which::Loot, was: item, grab: Vec2::ZERO }, Slot::Sidearm), 1);
    assert_eq!(shelves.bag.slot(Slot::Sidearm), Some(Stack::gun(Kind::Pistol, 3)));
    let back = shelves.loot.as_ref().unwrap().1.items.iter().find(|i| i.stack.kind == Kind::Pistol).copied().unwrap();
    assert_eq!((back.stack.loaded, back.x, back.y), (12, 1, 1));
    // A watch won't go in a slot; a pistol won't go in the primary.
    let i = shelves.loot.as_ref().unwrap().1.items.iter().position(|i| i.stack.kind == Kind::Watch).unwrap();
    let item = shelves.take(Which::Loot, i).unwrap();
    assert_eq!(slots::release(&mut shelves, Held { item, from: Which::Loot, was: item, grab: Vec2::ZERO }, Slot::Melee), 0);
    let item = shelves.take(Which::Slot(Slot::Sidearm), 0).unwrap();
    assert_eq!(slots::release(&mut shelves, Held { item, from: Which::Slot(Slot::Sidearm), was: item, grab: Vec2::ZERO }, Slot::Primary), 0);
    assert_eq!(shelves.bag.slot(Slot::Sidearm), Some(Stack::gun(Kind::Pistol, 3)), "back in its slot");
    assert_eq!(shelves.loot.as_ref().unwrap().1.count(Kind::Watch), 1, "back in the crate");
}

#[test]
fn rounds_dropped_on_a_gun_load_it_and_the_rest_go_back() {
    let mut bag = Bag::empty();
    bag.pack.put(Stack::gun(Kind::Pistol, 5), 0, 0, false);
    bag.pack.put(Stack::new(Kind::Rounds, 30), 3, 0, false);
    bag.slots[Slot::Primary.index()] = Some(Stack::gun(Kind::Shotgun, 1));
    let mut shelves = Shelves { bag: &mut bag, loot: None, sell: None };
    // Thirty rounds onto the pistol: seven go in, twenty-three go back.
    let item = shelves.take(Which::Pack, 1).unwrap();
    assert_eq!(landing(&shelves.bag.pack, item, (0, 0), (0, 0)), Landing::Load(0, 7), "green over the pistol");
    assert_eq!(release(&mut shelves, Held { item, from: Which::Pack, was: item, grab: Vec2::ZERO }, Which::Pack, (0, 0), (0, 0)), 7);
    assert_eq!(shelves.bag.pack.items[0].stack, Stack::gun(Kind::Pistol, 12));
    assert_eq!(shelves.bag.pack.count(Kind::Rounds), 23);
    // The wrong rounds for a gun don't go in: 9mm won't load the shotgun
    // in its slot.
    let i = shelves.bag.pack.items.iter().position(|i| i.stack.kind == Kind::Rounds).unwrap();
    let item = shelves.take(Which::Pack, i).unwrap();
    assert_eq!(slots::release(&mut shelves, Held { item, from: Which::Pack, was: item, grab: Vec2::ZERO }, Slot::Primary), 0);
    assert_eq!(shelves.bag.slot(Slot::Primary), Some(Stack::gun(Kind::Shotgun, 1)));
    // Shells onto it do, as many as there's room for.
    shelves.bag.pack.place(Stack::new(Kind::Shells, 3));
    let i = shelves.bag.pack.items.iter().position(|i| i.stack.kind == Kind::Shells).unwrap();
    let item = shelves.take(Which::Pack, i).unwrap();
    assert!(slots::takes(item.stack, Slot::Primary, shelves.bag.slot(Slot::Primary)));
    assert_eq!(slots::release(&mut shelves, Held { item, from: Which::Pack, was: item, grab: Vec2::ZERO }, Slot::Primary), 3);
    assert_eq!(shelves.bag.slot(Slot::Primary), Some(Stack::gun(Kind::Shotgun, 4)));
    assert_eq!(shelves.bag.pack.count(Kind::Shells), 0);
    // A full gun has no room for more.
    let full = Stack::gun(Kind::Pistol, 12);
    assert_eq!(full.room_for(Stack::new(Kind::Rounds, 5)), 0);
}

#[test]
fn trading_right_click_sends_things_to_be_sold_and_back_to_the_stash() {
    let mut bag = Bag::with_rounds();
    bag.pack.place(Stack::one(Kind::Watch));
    let mut stash = Grid::new(4, 4);
    stash.place(Stack::one(Kind::Ring));
    let mut sell = Grid::new(SELL_W, SELL_H);
    let mut shelves = Shelves { bag: &mut bag, loot: Some(("STASH", &mut stash)), sell: Some(&mut sell) };
    // From the pack, and from the stash, into the box.
    let watch = shelves.bag.pack.items.iter().position(|i| i.stack.kind == Kind::Watch).unwrap();
    sell_move(&mut shelves, Which::Pack, watch);
    sell_move(&mut shelves, Which::Loot, 0);
    assert_eq!(shelves.bag.pack.count(Kind::Watch), 0);
    let box_ = shelves.sell.as_ref().unwrap();
    assert_eq!((box_.count(Kind::Watch), box_.count(Kind::Ring)), (1, 1));
    // Out of the box, back to the stash (not the bag).
    sell_move(&mut shelves, Which::Sell, 0);
    sell_move(&mut shelves, Which::Sell, 0);
    assert!(shelves.sell.as_ref().unwrap().items.is_empty());
    assert_eq!((stash.count(Kind::Watch), stash.count(Kind::Ring)), (1, 1));
}
