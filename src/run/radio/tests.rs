use super::*;
use crate::loot::bag::{Bag, Slot};
use crate::loot::{Kind, Stack};
use crate::radio::Radio;
use crate::radio::codes::Call;
use crate::weapon::{Trigger, Weapon};
use crate::world::Game;
use lntrn_ui::Key;
use lntrn_ui::testing::Harness;

const DT: f64 = 1.0 / 60.0;

/// A seat with a pistol and a knife and a radio, the pistol up in hand.
fn seat(combat: &mut Combat) -> Seat {
    let mut seat = Seat::default();
    seat.bag = Bag::empty();
    *seat.bag.slot_mut(Slot::Sidearm) = Some(Stack::gun(Kind::Pistol, 12));
    *seat.bag.slot_mut(Slot::Melee) = Some(Stack::one(Kind::Knife));
    seat.radio = Some(Radio::default());
    seat.radio.iter_mut().for_each(|r| r.signal.fill());
    seat.take_up(combat, Some(Slot::Sidearm));
    seat
}

/// A world with a floor and what a flare needs, the player on it.
fn game() -> Game {
    let mut game = Game::new();
    let mut solids = crate::collide::Solids::new();
    solids.add(&crate::collide::box_tris(lntrn_math::Vec3::new(-60.0, -1.0, -60.0), lntrn_math::Vec3::new(60.0, 0.0, 60.0)));
    game.world.insert_resource(crate::world::Solid(solids));
    game.spawn_player(0, 0.0, 0.0, 0.0);
    game
}

/// `frames` of the hands and the radio, with `key` pressed on the first
/// (the keys theirs) and the trigger pulled on it if `fire`.
fn run(h: &mut Harness, seat: &mut Seat, combat: &mut Combat, key: Option<Key>, fire: bool, frames: u32) {
    run_in(&mut game(), h, seat, combat, key, (fire, false), frames);
}

/// The same in `game`, the trigger pulled on the first frame and held
/// through them all as `trigger` says.
fn run_in(game: &mut Game, h: &mut Harness, seat: &mut Seat, combat: &mut Combat, key: Option<Key>, trigger: (bool, bool), frames: u32) {
    if let Some(key) = key {
        h.key(key);
    }
    for k in 0..frames {
        h.frame(|ui| {
            seat.input.update(ui, Some(Default::default()), true, Default::default(), DT);
            seat.input.set_dialing(seat.dialling());
            seat.radioing(ui, game, combat, true, Pulls { fire: trigger.0 && k == 0, hold: trigger.1, strike: false }, DT);
            seat.switch_hands(ui, combat, true);
            combat.arms[0].hands.update(Trigger::default(), DT);
        });
    }
}

#[test]
fn pulled_out_the_gun_goes_away_and_comes_back_when_it_s_put_away() {
    let mut h = Harness::new(800.0, 600.0);
    let mut combat = Combat::new();
    let mut seat = seat(&mut combat);
    run(&mut h, &mut seat, &mut combat, None, false, 60);
    assert!(!seat.radio_out() && seat.radio_shown().is_none());
    // Q: the pistol's put away, then the radio comes up; the pistol
    // stays away while it's out.
    run(&mut h, &mut seat, &mut combat, Some(Key::Char('q')), false, 2);
    assert!(seat.radio_out() && seat.radio_shown().is_none(), "the gun's not away yet");
    run(&mut h, &mut seat, &mut combat, None, false, 90);
    assert!(seat.radio_held() && combat.arms[0].hands.stowed() == Some(Some(Slot::Sidearm)));
    assert_eq!(seat.radio_shown().map(|s| (s.clip, s.stowed)), Some(("Idle", 0.0)));
    assert!(seat.dialling(), "the feet stand still for the dial");
    // A, D, A, D: 2X POINTS' code. Its bars are spent there and then,
    // it's keyed (the feet free again), and that done, the radio goes
    // away by itself and the pistol's back in hand.
    for key in ['a', 'd', 'a', 'd'] {
        run(&mut h, &mut seat, &mut combat, Some(Key::Char(key)), false, 2);
    }
    assert_eq!(seat.radio.map(|r| (r.calling(), r.signal.bars())), Some((Some(Call::DoublePoints), 2.0)), "three of its five bars spent");
    assert_eq!(seat.radio_shown().map(|s| s.clip), Some("Key"));
    assert!(!seat.dialling());
    run(&mut h, &mut seat, &mut combat, None, false, 90);
    let hands = &combat.arms[0].hands;
    assert!(!seat.radio_out() && hands.held == Some(Slot::Sidearm) && hands.weapon == Weapon::Pistol && !hands.busy());
    assert_eq!(hands.mag, 12);
    // Q, and Q again: out, and down it goes.
    run(&mut h, &mut seat, &mut combat, Some(Key::Char('q')), false, 90);
    assert!(seat.radio_held() && seat.radio.is_some_and(|r| r.dialing() && r.dial().len() == 0));
    run(&mut h, &mut seat, &mut combat, Some(Key::Char('q')), false, 90);
    assert!(!seat.radio_out() && combat.arms[0].hands.held == Some(Slot::Sidearm));
}

#[test]
fn a_shot_or_another_weapon_puts_it_away() {
    let mut h = Harness::new(800.0, 600.0);
    let mut combat = Combat::new();
    let mut seat = seat(&mut combat);
    // The trigger: the radio down, the pistol back up.
    run(&mut h, &mut seat, &mut combat, Some(Key::Char('q')), false, 90);
    assert!(seat.radio_held());
    run(&mut h, &mut seat, &mut combat, None, true, 90);
    assert!(!seat.radio_out() && combat.arms[0].hands.held == Some(Slot::Sidearm) && !combat.arms[0].hands.busy());
    // The blade's key: the radio down, and it's the blade that comes up.
    run(&mut h, &mut seat, &mut combat, Some(Key::Char('q')), false, 90);
    run(&mut h, &mut seat, &mut combat, Some(Key::Char('3')), false, 90);
    assert!(!seat.radio_out() && combat.arms[0].hands.held == Some(Slot::Melee) && combat.arms[0].hands.weapon == Weapon::Knife);
    // The hands wanted for something else (not `free`): away at once.
    run(&mut h, &mut seat, &mut combat, Some(Key::Char('q')), false, 90);
    h.frame(|ui| seat.radioing(ui, &mut game(), &mut combat, false, Pulls::default(), DT));
    assert!(seat.radio_shown().is_some_and(|s| s.stowed >= 0.0) && !seat.radio_held());
    run(&mut h, &mut seat, &mut combat, None, false, 90);
    assert!(!seat.radio_out() && combat.arms[0].hands.held == Some(Slot::Melee));
}

#[test]
fn kills_charge_the_signal_and_a_code_there_s_not_the_signal_for_is_refused() {
    let mut h = Harness::new(800.0, 600.0);
    let mut combat = Combat::new();
    let mut seat = seat(&mut combat);
    seat.radio = Some(Radio::default());
    // Nothing killed yet: an ammo drop's code is refused, nothing keyed.
    run(&mut h, &mut seat, &mut combat, Some(Key::Char('q')), false, 90);
    for key in ['s', 's', 'w', 'd'] {
        run(&mut h, &mut seat, &mut combat, Some(Key::Char(key)), false, 2);
    }
    assert_eq!(seat.note.map(|(n, _)| n), Some("NOT ENOUGH SIGNAL"));
    assert!(seat.radio.is_some_and(|r| r.calling().is_none() && r.dial().len() == 0 && r.wrong().is_some()));
    assert_eq!(seat.radio_shown().map(|s| s.clip), Some("Idle"));
    // Ten kills, a bar: still not the two it costs. Twenty: now it goes
    // out, and the bars with it.
    seat.stats.gun_kills = 10;
    for key in ['s', 's', 'w', 'd'] {
        run(&mut h, &mut seat, &mut combat, Some(Key::Char(key)), false, 2);
    }
    assert!(seat.radio.is_some_and(|r| r.calling().is_none() && r.signal.bars() == 1.0));
    seat.stats.gun_kills = 20;
    for key in ['s', 's', 'w', 'd'] {
        run(&mut h, &mut seat, &mut combat, Some(Key::Char(key)), false, 2);
    }
    assert_eq!(seat.radio.map(|r| (r.calling(), r.flare(), r.signal.bars())), Some((None, Some(Call::AmmoDrop), 0.0)), "spent at once, and its flare's theirs");
    // Put away with the flare not thrown: it's kept, and the bars stay
    // spent.
    run(&mut h, &mut seat, &mut combat, Some(Key::Char('q')), false, 90);
    assert_eq!(seat.radio.map(|r| (r.out(), r.flare(), r.signal.bars())), Some((false, Some(Call::AmmoDrop), 0.0)));
}

#[test]
fn a_drop_s_flare_is_aimed_with_the_trigger_held_and_thrown_as_it_s_let_go() {
    use crate::support::Flare;
    let mut h = Harness::new(800.0, 600.0);
    let mut combat = Combat::new();
    let mut seat = seat(&mut combat);
    let mut game = game();
    run_in(&mut game, &mut h, &mut seat, &mut combat, Some(Key::Char('q')), (false, false), 90);
    for key in ['s', 'w', 'd', 'a'] {
        run_in(&mut game, &mut h, &mut seat, &mut combat, Some(Key::Char(key)), (false, false), 2);
    }
    run_in(&mut game, &mut h, &mut seat, &mut combat, None, (false, false), 120);
    assert!(seat.radio.is_some_and(|r| r.marking() && r.flare() == Some(Call::MedicDrop)));
    assert!(seat.throw_arc().is_none());
    // The trigger held: the radio stays out, and the arc's shown.
    run_in(&mut game, &mut h, &mut seat, &mut combat, None, (true, true), 10);
    assert!(seat.radio_held() && seat.throw_arc().is_some_and(|(dots, lands)| !dots.is_empty() && lands.is_some()));
    assert_eq!(game.world.query::<&Flare>().iter(&game.world).count(), 0);
    // Let go: thrown with the left hand; and that done, the radio's
    // away by itself and the pistol's back.
    run_in(&mut game, &mut h, &mut seat, &mut combat, None, (false, false), 120);
    let flares: Vec<Flare> = game.world.query::<&Flare>().iter(&game.world).copied().collect();
    assert!(flares.len() == 1 && flares[0].call == Call::MedicDrop && flares[0].by == 0, "{flares:?}");
    assert!(seat.radio.is_some_and(|r| r.flare().is_none() && !r.out()) && combat.arms[0].hands.held == Some(Slot::Sidearm));
    assert!(seat.throw_arc().is_none());
}

#[test]
fn on_a_pad_view_held_pulls_it_out_and_tapped_puts_it_away_and_not_the_bag_up() {
    use crate::input::pad::{frame_holding, frame_with};
    use lntrn_sys::gamepad::Button;
    let mut h = Harness::new(800.0, 600.0);
    let mut combat = Combat::new();
    let mut seat = seat(&mut combat);
    let mut world = game();
    let mut frame = |seat: &mut Seat, combat: &mut Combat, pad, dt: f64| {
        let mut bag = false;
        h.frame(|ui| {
            seat.input.update(ui, None, true, pad, dt);
            seat.input.set_dialing(seat.dialling());
            seat.radioing(ui, &mut world, combat, true, Pulls::default(), dt);
            seat.switch_hands(ui, combat, true);
            combat.arms[0].hands.update(Trigger::default(), dt);
            bag = seat.input.pressed(ui, Action::Inventory);
        });
        bag
    };
    let (tap, held, rest) = (frame_with(&[Button::Select]), frame_holding(&[Button::Select]), Default::default());
    // Held: out it comes (and no bag as it's let go).
    frame(&mut seat, &mut combat, held.merge(tap), DT);
    assert!(!frame(&mut seat, &mut combat, held, 0.5) && seat.radio_out());
    assert!(!frame(&mut seat, &mut combat, rest, DT));
    for _ in 0..60 {
        frame(&mut seat, &mut combat, rest, DT);
    }
    assert!(seat.radio_held());
    // Tapped, the radio out: away it goes, and the bag stays shut.
    assert!(!frame(&mut seat, &mut combat, tap, DT), "the tap's the radio's");
    assert!(!seat.radio_held());
    for _ in 0..60 {
        frame(&mut seat, &mut combat, rest, DT);
    }
    assert!(!seat.radio_out() && combat.arms[0].hands.held == Some(Slot::Sidearm));
    // Tapped again, the radio away: the bag.
    assert!(frame(&mut seat, &mut combat, tap, DT));
}

#[test]
fn a_strafing_run_s_strip_is_marked_where_they_look_and_the_trigger_sends_it() {
    use crate::support::strafe::Strafe;
    let mut h = Harness::new(800.0, 600.0);
    let mut combat = Combat::new();
    let mut seat = seat(&mut combat);
    let mut game = game();
    game.world.insert_resource(crate::zombie::Horde::default());
    run_in(&mut game, &mut h, &mut seat, &mut combat, Some(Key::Char('q')), (false, false), 90);
    assert!(seat.zone.is_none());
    for key in ['w', 'd', 'd'] {
        run_in(&mut game, &mut h, &mut seat, &mut combat, Some(Key::Char(key)), (false, false), 2);
    }
    // Called: three bars spent, and with the very next frame the
    // strip's marked on the ground ahead (the radio still being keyed),
    // running the way they face (north); and the feet are free.
    run_in(&mut game, &mut h, &mut seat, &mut combat, None, (false, false), 1);
    assert_eq!(seat.radio.map(|r| (r.placing(), r.signal.bars())), Some((Some(Call::StrafingRun), 2.0)));
    assert!(seat.zone.is_some() && !seat.dialling() && seat.radio_shown().is_some_and(|s| s.clip == "Key"));
    run_in(&mut game, &mut h, &mut seat, &mut combat, None, (false, false), 90);
    assert!(seat.radio_held(), "it waits to be told where");
    let Some(crate::support::Mark::Strip(strip)) = seat.zone else { panic!("no strip marked: {:?}", seat.zone) };
    assert!(strip.open && strip.dir.z < -0.99 && strip.mid.z < -5.0 && strip.mid.y.abs() < 1e-6, "{strip:?}");
    // The trigger: it's called in there, and the radio's away at once
    // (the pistol back after).
    run_in(&mut game, &mut h, &mut seat, &mut combat, None, (true, false), 2);
    let runs: Vec<Strafe> = game.world.query::<&Strafe>().iter(&game.world).copied().collect();
    assert!(runs.len() == 1 && runs[0].by == 0 && runs[0].strip == strip, "{runs:?}");
    assert!(seat.zone.is_none() && !seat.radio_held() && seat.radio.is_some_and(|r| r.strike().is_none()));
    run_in(&mut game, &mut h, &mut seat, &mut combat, None, (false, false), 90);
    assert!(!seat.radio_out() && combat.arms[0].hands.held == Some(Slot::Sidearm) && !combat.arms[0].hands.busy());
}

#[test]
fn a_precision_strike_s_circle_is_marked_where_they_look_and_refused_under_a_roof() {
    use crate::support::Mark;
    use crate::support::strike::Strike;
    let mut h = Harness::new(800.0, 600.0);
    let mut combat = Combat::new();
    let mut seat = seat(&mut combat);
    let mut game = game();
    game.world.insert_resource(crate::zombie::Horde::default());
    // A roof over everything north of z = -4: where they're looking.
    let roofed = {
        let mut solids = crate::collide::Solids::new();
        solids.add(&crate::collide::box_tris(lntrn_math::Vec3::new(-60.0, -1.0, -60.0), lntrn_math::Vec3::new(60.0, 0.0, 60.0)));
        solids.add(&crate::collide::box_tris(lntrn_math::Vec3::new(-30.0, 4.0, -40.0), lntrn_math::Vec3::new(30.0, 4.3, -4.0)));
        solids
    };
    let open = std::mem::replace(&mut game.world.resource_mut::<crate::world::Solid>().0, roofed);
    run_in(&mut game, &mut h, &mut seat, &mut combat, Some(Key::Char('q')), (false, false), 90);
    for key in ['d', 'd', 'w'] {
        run_in(&mut game, &mut h, &mut seat, &mut combat, Some(Key::Char(key)), (false, false), 2);
    }
    run_in(&mut game, &mut h, &mut seat, &mut combat, None, (false, false), 90);
    // Two bars spent; a spot's marked, but there's a roof over it: the
    // trigger's refused, and the strike's kept.
    assert_eq!(seat.radio.map(|r| (r.placing(), r.signal.bars())), Some((Some(Call::PrecisionStrike), 3.0)));
    assert!(matches!(seat.zone, Some(Mark::Spot(s)) if !s.open), "{:?}", seat.zone);
    run_in(&mut game, &mut h, &mut seat, &mut combat, None, (true, false), 2);
    assert_eq!(seat.note.map(|(n, _)| n), Some("NO OPEN SKY"));
    assert!(seat.radio.is_some_and(|r| r.strike().is_some()) && game.world.query::<&Strike>().iter(&game.world).count() == 0);
    // The roof gone: it's open, and the trigger sends it.
    game.world.resource_mut::<crate::world::Solid>().0 = open;
    run_in(&mut game, &mut h, &mut seat, &mut combat, None, (true, false), 2);
    assert_eq!(game.world.query::<&Strike>().iter(&game.world).count(), 1);
    assert!(seat.zone.is_none() && !seat.radio_held() && seat.radio.is_some_and(|r| r.strike().is_none()));
}

#[test]
fn no_radio_no_radio() {
    let mut h = Harness::new(800.0, 600.0);
    let mut combat = Combat::new();
    let mut seat = seat(&mut combat);
    seat.radio = None;
    run(&mut h, &mut seat, &mut combat, Some(Key::Char('q')), false, 60);
    assert!(!seat.radio_out() && combat.arms[0].hands.held == Some(Slot::Sidearm) && !combat.arms[0].hands.busy());
}

#[test]
fn a_mystery_drop_costs_points_too_and_without_them_it_s_refused() {
    let mut h = Harness::new(800.0, 600.0);
    let mut combat = Combat::new();
    let mut seat = seat(&mut combat);
    let mut game = game();
    // Up, right, down, down, down, with too few points: refused, and
    // nothing's spent of either.
    seat.points = Some(900);
    run_in(&mut game, &mut h, &mut seat, &mut combat, Some(Key::Char('q')), (false, false), 90);
    let dial = |game: &mut Game, h: &mut Harness, seat: &mut Seat, combat: &mut Combat| {
        for key in ['w', 'd', 's', 's', 's'] {
            run_in(game, h, seat, combat, Some(Key::Char(key)), (false, false), 2);
        }
    };
    dial(&mut game, &mut h, &mut seat, &mut combat);
    assert_eq!(seat.note.map(|(n, _)| n), Some("NOT ENOUGH POINTS"));
    assert_eq!(seat.radio.map(|r| (r.signal.bars(), r.flare(), r.dialing(), r.dial().len())), Some((5.0, None, true, 0)));
    assert_eq!(seat.points, Some(900));
    // With them: its flare's in hand, and the points and the bars gone.
    seat.points = Some(1000);
    dial(&mut game, &mut h, &mut seat, &mut combat);
    assert_eq!(seat.radio.map(|r| (r.signal.bars(), r.flare())), Some((3.0, Some(Call::MysteryDrop))));
    assert_eq!(seat.points, Some(50));
    // The points but not the signal: refused, and the points kept.
    let mut seat = self::seat(&mut combat);
    seat.radio = Some(Radio::default());
    seat.points = Some(5000);
    run_in(&mut game, &mut h, &mut seat, &mut combat, Some(Key::Char('q')), (false, false), 90);
    dial(&mut game, &mut h, &mut seat, &mut combat);
    assert_eq!((seat.note.map(|(n, _)| n), seat.points), (Some("NOT ENOUGH SIGNAL"), Some(5000)));
    // What costs no points asks for none (nor does anything, for the
    // dev's free hand).
    seat.radio.iter_mut().for_each(|r| r.signal.fill());
    seat.points = Some(0);
    for key in ['d', 'd', 'w'] {
        run_in(&mut game, &mut h, &mut seat, &mut combat, Some(Key::Char(key)), (false, false), 2);
    }
    assert_eq!(seat.radio.map(|r| (r.strike(), r.signal.bars())), Some((Some(Call::PrecisionStrike), 3.0)));
}
