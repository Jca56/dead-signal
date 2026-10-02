//! The bag screen worked with a pad, and laid out over a pane.

use lntrn_ui::testing::Harness;

use super::cursor::stepped;
use super::*;
use crate::input::steer::Steer;

const PACK_00: Cursor = Cursor { which: Which::Pack, x: 0, y: 0 };

fn at(which: Which, x: u8, y: u8) -> Cursor {
    Cursor { which, x, y }
}

#[test]
fn the_cursor_steps_cell_to_cell_past_a_thing_and_off_the_edge() {
    let mut h = Harness::new(1920.0, 1080.0);
    h.frame(|ui| {
        // A car battery (2×2) in the corner of a 4×3 pack, over 2×2 pockets.
        let mut bag = Bag::sized((4, 3), (2, 2));
        bag.pack.put(Stack::one(Kind::Battery), 0, 0, false);
        let screen = BagUi::default();
        let laid = screen.laid(ui, &bag, None);
        let places = screen.layout(ui, &bag, None, laid);
        let mut shelves = Shelves { bag: &mut bag, loot: None, sell: None, fit: Fit::default() };
        let mut step = |from, way, held| stepped(&mut shelves, &places, laid.cell, from, way, held);
        assert_eq!(step(PACK_00, (1, 0), false), at(Which::Pack, 2, 0), "past the rest of the battery");
        assert_eq!(step(at(Which::Pack, 1, 1), (0, 1), false), at(Which::Pack, 1, 2));
        assert_eq!(step(PACK_00, (1, 0), true), at(Which::Pack, 1, 0), "holding something, cell by cell");
        assert_eq!(step(at(Which::Pack, 3, 1), (1, 0), false), at(Which::Pack, 3, 1), "nothing that way: where it was");
        assert_eq!(step(at(Which::Pack, 1, 2), (0, 1), false), at(Which::Pockets, 1, 0), "off the pack's foot, the pockets under it");
        assert_eq!(step(at(Which::Pockets, 0, 0), (0, -1), false), at(Which::Pack, 0, 2));
        // Off its left, what's worn (the nearest box); left of that, the
        // weapons; nothing over the top.
        let head = at(Which::Worn(Wear::ALL[0]), 0, 0);
        assert_eq!(step(PACK_00, (-1, 0), false), head);
        assert_eq!(step(at(Which::Pack, 0, 2), (-1, 0), false), at(Which::Worn(Wear::ALL[1]), 0, 0));
        assert_eq!(step(head, (0, 1), false), at(Which::Worn(Wear::ALL[1]), 0, 0));
        assert_eq!(step(head, (-1, 0), false), at(Which::Slot(Slot::ALL[0]), 0, 0));
        assert_eq!(step(head, (0, -1), false), head);
        assert_eq!(step(head, (1, 0), false), PACK_00);
    });
    // Something searched: off the pack's right, into it.
    h.frame(|ui| {
        let mut bag = Bag::sized((4, 3), (2, 2));
        let mut crate_ = Grid::new(4, 3);
        let screen = BagUi::default();
        let laid = screen.laid(ui, &bag, Some((4, 3)));
        let places = screen.layout(ui, &bag, Some((4, 3)), laid);
        let mut shelves = Shelves { bag: &mut bag, loot: Some(("CRATE", &mut crate_)), sell: None, fit: Fit::default() };
        assert_eq!(stepped(&mut shelves, &places, laid.cell, at(Which::Pack, 3, 1), (1, 0), false), at(Which::Loot, 0, 1));
        let mut screen = BagUi::default();
        assert_eq!(screen.settle(&mut shelves, &places), at(Which::Loot, 0, 0), "it starts on what's searched");
    });
}

#[test]
fn it_all_fits_a_pane_and_the_whole_window_s_as_it_was() {
    let mut h = Harness::new(1920.0, 1080.0);
    h.frame(|ui| {
        let window = ui.clip();
        // A holdout's bag (an 8×8 pack, 4×2 pockets) over each kind of
        // pane, and over the whole window.
        let holdout = Bag::sized((8, 8), (4, 2));
        let top = Rect::new(window.min, Vec2::new(window.max.x, window.center().y));
        let left = Rect::new(window.min, Vec2::new(window.center().x, window.max.y));
        for (pane, least) in [(Some(top), 45.0), (Some(left), 55.0), (None, 79.0)] {
            let screen = BagUi { pane, ..BagUi::default() };
            let laid = screen.laid(ui, &holdout, None);
            let within = pane.unwrap_or(window);
            assert!(laid.cell >= least, "{pane:?}: cells {} px", laid.cell);
            for (which, r) in screen.layout(ui, &holdout, None, laid) {
                assert!(r.min.x >= within.min.x && r.max.x <= within.max.x && r.min.y >= within.min.y && r.max.y <= within.max.y - 60.0, "{which:?} at {r:?} in {within:?}");
            }
        }
        // The pockets go beside the pack where it's short of room down.
        assert!(BagUi { pane: Some(top), ..BagUi::default() }.laid(ui, &holdout, None).beside);
        assert!(!BagUi { pane: Some(left), ..BagUi::default() }.laid(ui, &holdout, None).beside);
        // With room for it as it always was, it's as it always was.
        let laid = BagUi::default().laid(ui, &Bag::empty(), Some((6, 4)));
        assert_eq!((laid.cell, laid.beside), (CELL * ui.m.scale, false));
    });
}

/// A frame of `screen` with a pad at it asking `steer`.
fn pad_frame(h: &mut Harness, screen: &mut BagUi, shelves: &mut Shelves, steer: Steer) -> Moved {
    let mut moved = Moved::default();
    h.frame(|ui| moved = screen.frame(ui, shelves, &Icons::default(), Hands { mouse: false, pad: Some(steer) }));
    moved
}

#[test]
fn a_pad_picks_things_up_puts_them_down_and_sends_them_across() {
    let mut h = Harness::new(1920.0, 1080.0);
    let mut bag = Bag::sized((4, 3), (2, 2));
    bag.pack.put(Stack::new(Kind::Bandage, 2), 0, 0, false);
    bag.pack.put(Stack::one(Kind::Battery), 2, 0, false);
    bag.pack.put(Stack::one(Kind::BikeHelmet), 0, 1, false);
    // (A pack of a size of its own, as a holdout's is: no backpack worn.)
    let mut shelves = Shelves { bag: &mut bag, loot: None, sell: None, fit: Fit { pack: Some((4, 3)), ..Fit::default() } };
    let mut screen = BagUi::default();
    screen.opened(true);
    let (rest, pick, across, back) = (Steer::default(), Steer { pick: true, ..Steer::default() }, Steer { across: true, ..Steer::default() }, Steer { back: true, ..Steer::default() });
    let step = |x, y| Steer { step: (x, y), ..Steer::default() };
    // The bandages picked up, carried one cell right, and put down.
    pad_frame(&mut h, &mut screen, &mut shelves, rest);
    pad_frame(&mut h, &mut screen, &mut shelves, pick);
    assert!(screen.held.is_some() && shelves.bag.pack.count(Kind::Bandage) == 0);
    pad_frame(&mut h, &mut screen, &mut shelves, step(1, 0));
    pad_frame(&mut h, &mut screen, &mut shelves, pick);
    let bandages = shelves.bag.pack.items.iter().find(|i| i.stack.kind == Kind::Bandage).copied().unwrap();
    assert_eq!((screen.held.is_some(), bandages.x, bandages.y, bandages.stack.count), (false, 1, 0, 2));
    // Picked up again and put on the battery: it won't go, and it's kept
    // hold of; B puts it back where it came from.
    pad_frame(&mut h, &mut screen, &mut shelves, pick);
    pad_frame(&mut h, &mut screen, &mut shelves, step(1, 0));
    let moved = pad_frame(&mut h, &mut screen, &mut shelves, pick);
    assert_eq!((moved.said, screen.held.is_some()), (Some(WONT_GO), true));
    let moved = pad_frame(&mut h, &mut screen, &mut shelves, back);
    assert!(!moved.closed && screen.held.is_none());
    assert_eq!(shelves.bag.pack.at(1, 0).map(|i| shelves.bag.pack.items[i].stack.kind), Some(Kind::Bandage));
    // X on the helmet puts it on; X with something held throws it down.
    screen.cursor = Some(at(Which::Pack, 0, 1));
    pad_frame(&mut h, &mut screen, &mut shelves, across);
    assert_eq!(shelves.bag.worn(Wear::Head).map(|s| s.kind), Some(Kind::BikeHelmet));
    screen.cursor = Some(at(Which::Pack, 2, 0));
    pad_frame(&mut h, &mut screen, &mut shelves, pick);
    let moved = pad_frame(&mut h, &mut screen, &mut shelves, across);
    assert_eq!(moved.dropped.iter().map(|s| s.kind).collect::<Vec<_>>(), [Kind::Battery]);
    assert_eq!(shelves.bag.pack.count(Kind::Battery), 0);
    // B with nothing held puts the screen away.
    assert!(pad_frame(&mut h, &mut screen, &mut shelves, back).closed);
}

#[test]
fn the_mouse_taking_over_puts_back_what_the_pad_held() {
    let mut h = Harness::new(1920.0, 1080.0);
    let mut bag = Bag::sized((4, 3), (2, 2));
    bag.pack.put(Stack::one(Kind::Battery), 1, 1, false);
    let mut shelves = Shelves { bag: &mut bag, loot: None, sell: None, fit: Fit::default() };
    let mut screen = BagUi::default();
    screen.opened(true);
    let both = |steer| Hands { mouse: true, pad: Some(steer) };
    let mut frame = |h: &mut Harness, screen: &mut BagUi, hands| {
        h.frame(|ui| {
            screen.frame(ui, &mut shelves, &Icons::default(), hands);
        });
    };
    screen.cursor = Some(at(Which::Pack, 2, 2));
    frame(&mut h, &mut screen, both(Steer::default()));
    frame(&mut h, &mut screen, both(Steer { pick: true, ..Steer::default() }));
    assert_eq!((screen.held.is_some(), screen.cursor), (true, Some(at(Which::Pack, 1, 1))), "held by its top-left cell");
    // The mouse stirs (nowhere near anything): it's back in the pack, not
    // thrown on the ground.
    h.move_to(Vec2::new(5.0, 5.0));
    frame(&mut h, &mut screen, both(Steer::default()));
    assert!(screen.held.is_none() && !screen.by_pad);
    drop(frame);
    assert_eq!(shelves.bag.pack.items.iter().map(|i| (i.stack.kind, i.x, i.y)).collect::<Vec<_>>(), [(Kind::Battery, 1, 1)]);
}
