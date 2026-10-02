use super::codes::Arrow::{Down as D, Left as L, Right as R, Up as U};
use super::*;

const DT: f64 = 1.0 / 60.0;

/// `radio` run on for `seconds`, the hands empty: everything heard.
fn run(radio: &mut Radio, seconds: f64) -> Vec<Cue> {
    (0..(seconds / DT).round() as u32).flat_map(|_| radio.update(true, DT)).collect()
}

/// A radio pulled out and up.
fn up() -> Radio {
    let mut r = Radio::default();
    r.pull();
    run(&mut r, RAISE + 0.1);
    r
}

#[test]
fn it_waits_for_the_gun_to_be_put_away_then_comes_up() {
    let mut r = Radio::default();
    assert!(!r.out() && r.shown().is_none() && r.card() == 0.0);
    r.pull();
    assert!(r.out() && r.shown().is_none(), "wanted, not yet in view");
    assert!(r.update(false, 1.0).is_empty() && r.shown().is_none(), "the gun's still in hand");
    assert_eq!(r.card(), 1.0, "its card's up to read meanwhile");
    assert_eq!(r.update(true, DT), [Cue::On]);
    assert_eq!(r.shown().map(|s| s.stowed), Some(1.0), "from out of view");
    run(&mut r, RAISE * 0.5);
    assert!(r.shown().is_some_and(|s| s.stowed > 0.3 && s.stowed < 0.7));
    run(&mut r, RAISE);
    assert_eq!(r.shown(), Some(Shown { clip: "Idle", t: None, stowed: 0.0 }));
}

#[test]
fn a_code_punched_in_keys_it_and_what_was_called_for_goes_out() {
    let mut r = Radio::default();
    assert_eq!(r.press(U), None, "not while it's away");
    let mut r = up();
    assert_eq!([U, R].map(|a| r.press(a)), [Some(Dialed::On); 2]);
    assert_eq!(r.press(R), Some(Dialed::Called(Call::StrafingRun)));
    assert_eq!((r.calling(), r.press(D)), (Some(Call::StrafingRun), None), "no more arrows till it's sent");
    r.update(true, DT);
    assert_eq!(r.shown().map(|s| s.clip), Some("Key"));
    assert_eq!(run(&mut r, KEY + 0.1), [Cue::Talk, Cue::Over(Call::StrafingRun)]);
    assert!(r.dialing() && r.calling().is_none() && r.dial().len() == 0, "ready for the next");
}

#[test]
fn arrows_are_taken_before_it_s_up_and_a_wrong_one_shows_a_moment() {
    let mut r = Radio::default();
    r.pull();
    // The gun still going away: the whole code's in already.
    assert_eq!([D, D, U].map(|a| r.press(a)), [Some(Dialed::On); 3]);
    r.update(false, DT);
    assert_eq!(r.press(R), Some(Dialed::Called(Call::AmmoDrop)));
    assert!(r.shown().is_none());
    // Up it comes, and it's keyed at once.
    let heard = run(&mut r, RAISE + KEY + 0.2);
    assert_eq!(heard, [Cue::On, Cue::Talk, Cue::Over(Call::AmmoDrop)]);
    // A wrong arrow: begun again, and it shows for a moment.
    assert_eq!([L, R].map(|a| r.press(a)), [Some(Dialed::On); 2]);
    assert_eq!(r.press(U), Some(Dialed::Wrong));
    assert!(r.wrong().is_some() && r.dial().len() == 1);
    run(&mut r, WRONG_FOR + 0.1);
    assert!(r.wrong().is_none());
}

#[test]
fn put_away_it_goes_down_from_where_it_is_and_a_code_half_in_is_forgotten() {
    let mut r = Radio::default();
    // Wanted and not yet up: just not wanted.
    r.pull();
    r.put_away();
    assert!(!r.out());
    // Half up: down from half way, in half the time.
    r.pull();
    run(&mut r, DT + RAISE * 0.5);
    let half = r.shown().map(|s| s.stowed).unwrap();
    r.press(D);
    r.put_away();
    assert!(r.out() && !r.holds() && (r.shown().unwrap().stowed - half).abs() < 0.05);
    assert!(r.dial().len() == 0 && r.card() < 1.0);
    run(&mut r, LOWER * 0.5 + 2.0 * DT);
    assert!(!r.out() && r.shown().is_none());
    // Up, and mid-word, before it's sent: down all the way, nothing called.
    let mut r = up();
    for a in [U, R, R] {
        r.press(a);
    }
    run(&mut r, 0.4);
    r.put_away();
    assert_eq!(r.shown().map(|s| s.stowed), Some(0.0));
    assert!(run(&mut r, LOWER + DT).is_empty(), "cut short: nothing more's heard");
    assert!(!r.out());
}

#[test]
fn its_signal_s_kept_through_being_pulled_out_put_away_and_dropped_and_a_refused_code_s_forgotten() {
    let mut r = Radio::default();
    r.signal.fill();
    r.pull();
    r.put_away();
    r.pull();
    run(&mut r, RAISE + 0.1);
    assert!(r.signal.has(signal::BARS));
    // A code there's no signal for, refused: not called in, shown as wrong.
    for a in [U, R, R] {
        r.press(a);
    }
    r.refuse();
    assert!(r.calling().is_none() && r.dial().len() == 0 && r.wrong().is_some() && r.dialing());
    assert!(run(&mut r, KEY + 0.1).is_empty(), "nothing's keyed");
    r.drop_it();
    assert!(!r.out() && r.signal.has(signal::BARS));
}

#[test]
fn a_flare_given_comes_up_in_the_left_hand_is_thrown_and_is_kept_if_the_radio_s_put_away_first() {
    let mut r = up();
    r.give_flare(Call::AmmoDrop);
    assert!(!r.dialing() && r.press(U).is_none(), "no dialling with a flare to throw");
    assert!(!r.throw(), "not till it's in hand");
    assert_eq!(r.update(true, DT), [Cue::Lit]);
    assert_eq!(r.shown().map(|s| s.clip), Some("FlareUp"));
    run(&mut r, FLARE_UP + 0.1);
    assert!(r.marking() && r.shown() == Some(Shown { clip: "Flare", t: None, stowed: 0.0 }));
    // Put away with it not thrown: it's kept, and up again with the radio.
    r.put_away();
    run(&mut r, LOWER + 0.1);
    assert!(!r.out() && r.flare() == Some(Call::AmmoDrop));
    r.drop_it();
    r.pull();
    assert_eq!(run(&mut r, RAISE + FLARE_UP + 0.2), [Cue::On, Cue::Lit]);
    // Thrown: it leaves the hand part way through, and the dial's free.
    assert!(r.marking() && r.throw() && !r.throw());
    assert_eq!(r.shown().map(|s| s.clip), Some("Throw"));
    assert_eq!(run(&mut r, THROW + 0.1), [Cue::Thrown(Call::AmmoDrop)]);
    assert!(r.flare().is_none() && r.dialing() && r.shown().map(|s| s.clip) == Some("Idle"));
}
