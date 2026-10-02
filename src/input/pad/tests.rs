//! The pads' frames, a flick in the menus, the layout, and the rumble.

use super::*;

fn pad() -> PadState {
    PadState::default()
}

/// A pad at rest but for its sticks and its right trigger.
fn pushed(left: [f64; 2], right: [f64; 2], trigger: f64) -> PadState {
    let mut s = pad();
    s.left = left;
    s.right = right;
    s.right_trigger = trigger;
    s
}

#[test]
fn a_trigger_counts_past_its_pull_and_stays_till_let_go() {
    let mut pulled = [false; 2];
    let pull = |v: f64| pushed([0.0; 2], [0.0; 2], v);
    let rt = Control::RightTrigger;
    let f = frame(&pull(0.5), &[], &mut pulled, Labels::Xbox);
    assert!(!f.held(rt), "not far enough");
    let f = frame(&pull(0.6), &[], &mut pulled, Labels::Xbox);
    assert!(f.held(rt) && f.went_down(rt));
    let f = frame(&pull(0.45), &[], &mut pulled, Labels::Xbox);
    assert!(f.held(rt) && !f.went_down(rt), "eased off a little, still held, not pulled again");
    let f = frame(&pull(0.3), &[], &mut pulled, Labels::Xbox);
    assert!(!f.held(rt), "let go");
}

#[test]
fn a_tap_between_frames_still_counts_once_taken() {
    let mut f = frame(&pad(), &[Button::West], &mut [false; 2], Labels::Xbox);
    let x = Control::Button(Button::West);
    assert!(!f.held(x) && f.touched, "up again by the frame, but touched");
    assert!(f.take(x));
    assert!(!f.take(x), "taken once");
}

#[test]
fn a_resting_stick_is_nothing_and_two_pads_make_one() {
    let still = frame(&pushed([0.1, -0.05], [0.0; 2], 0.0), &[], &mut [false; 2], Labels::Xbox);
    assert_eq!(still.left, Vec2::ZERO);
    assert!(!still.touched);
    let looking = frame(&pushed([0.0; 2], [0.0, 1.0], 0.0), &[Button::South], &mut [false; 2], Labels::PlayStation);
    let both = still.merge(looking);
    assert!(both.right.y > 0.99 && both.went_down(Control::Button(Button::South)));
    assert_eq!(both.labels, Labels::PlayStation, "named as the one touched");
}

#[test]
fn a_walking_stick_let_go_short_of_its_middle_walks_nowhere() {
    // Let go from a push forward, it's come to rest a fifth of the way out.
    let rest = frame(&pushed([0.02, 0.2], [0.02, 0.2], 0.0), &[], &mut [false; 2], Labels::Xbox);
    assert_eq!(rest.left, Vec2::ZERO);
    assert!(!rest.touched);
    // (The look stick's finer; its curve makes next to nothing of as much.)
    let turn = crate::input::look::stick_turn(rest.right, &mut 0.0, 1.0, false, 1.0);
    assert!(rest.right.y > 0.0 && turn.length() < 0.02, "{turn:?} in a second");
    // And pushed, it still goes from a creep to all the way.
    let creep = frame(&pushed([0.0, 0.3], [0.0; 2], 0.0), &[], &mut [false; 2], Labels::Xbox);
    let full = frame(&pushed([0.0, 0.97], [0.0; 2], 0.0), &[], &mut [false; 2], Labels::Xbox);
    assert!(creep.left.y > 0.05 && creep.left.y < 0.12 && full.left.y == 1.0, "{creep:?}");
}

#[test]
fn a_flick_is_one_arrow_till_the_stick_comes_back() {
    let (key, armed) = flick(Vec2::new(0.0, 0.9), true);
    assert_eq!(key, Some(Key::ArrowUp));
    let (key, armed) = flick(Vec2::new(0.0, 0.9), armed);
    assert_eq!(key, None, "held up: once");
    let (_, armed) = flick(Vec2::new(0.0, 0.1), armed);
    assert_eq!(flick(Vec2::new(0.0, -0.8), armed).0, Some(Key::ArrowDown));
}

#[test]
fn every_action_the_pad_has_is_on_its_own_control_but_those_told_apart() {
    let b = PadBinds::default();
    // (X reloads or interacts, by what's in front of them; View is the bag
    // tapped and, held, the map, or the radio where there's no map.)
    let pairs = [(Action::Reload, Action::Interact), (Action::Inventory, Action::Map), (Action::Inventory, Action::Radio), (Action::Map, Action::Radio)];
    for a in Action::ALL {
        for o in Action::ALL {
            let shared = a != o && b.get(a).is_some() && b.get(a) == b.get(o);
            assert!(!shared || pairs.contains(&(a, o)) || pairs.contains(&(o, a)), "{a:?} and {o:?}");
        }
        assert!(b.get(a) != Some(b.next_weapon) && b.get(a) != Some(b.pause), "{a:?}");
    }
}

#[test]
fn a_harder_kick_thumps_harder_and_shakes_add_up_to_the_stronger() {
    let (smg, rifle) = (Rumble::shot(0.8), Rumble::shot(6.0));
    assert!(rifle.strong > smg.strong * 3.0 && rifle.seconds > smg.seconds);
    let mut r = Rumble::struck();
    r.add(Rumble::blow());
    assert_eq!(r, Rumble { strong: 0.75, weak: 0.55, seconds: 0.22 });
}
