use super::*;
use lntrn_ui::testing::Harness;

#[test]
fn x_reloads_but_interacts_with_something_in_front() {
    let mut h = Harness::new(800.0, 600.0);
    h.frame(|ui| {
        let x = pad::frame_with(&[Button::West]);
        let mut i = Input::default();
        i.update(ui, Some(Keys::default()), true, x, 0.016);
        assert!(i.pressed(ui, Action::Reload) && !i.pressed(ui, Action::Interact));
        i.set_prompt(Prompt::Press);
        i.update(ui, Some(Keys::default()), true, x, 0.016);
        assert!(!i.pressed(ui, Action::Reload) && i.pressed(ui, Action::Interact));
        assert_eq!(i.name(Action::Interact), "X", "named for the pad just pressed");
    });
}

#[test]
fn a_mouse_button_counts_only_while_the_pointer_s_locked() {
    let mut h = Harness::new(800.0, 600.0);
    h.press();
    h.frame(|ui| {
        let mut i = Input::default();
        i.update(ui, Some(Keys::default()), false, PadFrame::default(), 0.016);
        assert!(!i.held(ui, Action::Fire), "unlocked, the mouse points");
        i.update(ui, Some(Keys::default()), true, PadFrame::default(), 0.016);
        assert!(i.held(ui, Action::Fire));
        assert_eq!(i.name(Action::Fire), "MOUSE LEFT");
    });
}

#[test]
fn where_x_is_held_to_use_a_tap_of_it_still_reloads() {
    let mut h = Harness::new(800.0, 600.0);
    h.frame(|ui| {
        let (held, rest) = (pad::frame_holding(&[Button::West]), PadFrame::default());
        let down = held.merge(pad::frame_with(&[Button::West]));
        // At a window to be boarded up, X tapped: a reload, as it's
        // let go; nothing's nailed.
        let mut i = Input::default();
        i.set_prompt(Prompt::Hold);
        i.update(ui, None, true, down, 0.016);
        assert!(!i.pressed(ui, Action::Reload) && !i.held(ui, Action::Interact), "not yet: it may be a hold");
        i.update(ui, None, true, held, 0.05);
        assert!(!i.held(ui, Action::Interact));
        i.update(ui, None, true, rest, 0.016);
        assert!(i.pressed(ui, Action::Reload) && !i.pressed(ui, Action::Interact));
        assert!(!i.pressed(ui, Action::Reload), "once");
        // Held: the boards are nailed while it's down, and no reload
        // as it's let go.
        i.update(ui, None, true, down, 0.016);
        i.update(ui, None, true, held, 0.25);
        assert!(i.held(ui, Action::Interact) && i.pressed(ui, Action::Interact) && !i.pressed(ui, Action::Reload));
        i.update(ui, None, true, held, 1.0);
        assert!(i.held(ui, Action::Interact));
        i.update(ui, None, true, rest, 0.016);
        assert!(!i.held(ui, Action::Interact) && !i.pressed(ui, Action::Reload));
        // The aim swung off the window before the tap was let go:
        // still a reload.
        i.update(ui, None, true, down, 0.016);
        i.set_prompt(Prompt::None);
        i.update(ui, None, true, rest, 0.016);
        assert!(i.pressed(ui, Action::Reload));
        // Nothing in front of them: a reload at once; held on up to a
        // window, the boards go on.
        i.update(ui, None, true, down, 0.016);
        assert!(i.pressed(ui, Action::Reload));
        i.set_prompt(Prompt::Hold);
        i.update(ui, None, true, held, 0.3);
        assert!(i.held(ui, Action::Interact) && !i.pressed(ui, Action::Reload));
        // Something a press uses (a gun on the wall): used at once.
        i.update(ui, None, true, rest, 0.016);
        i.set_prompt(Prompt::Press);
        i.update(ui, None, true, down, 0.016);
        assert!(i.pressed(ui, Action::Interact) && i.held(ui, Action::Interact) && !i.pressed(ui, Action::Reload));
    });
}

#[test]
fn y_tapped_is_the_other_gun_and_held_is_the_blade() {
    let mut h = Harness::new(800.0, 600.0);
    h.frame(|ui| {
        let (held, rest) = (pad::frame_holding(&[Button::North]), PadFrame::default());
        let down = held.merge(pad::frame_with(&[Button::North]));
        let mut i = Input::default();
        // Down: the other gun, at once; let go soon, that's all.
        i.update(ui, None, true, down, 0.016);
        assert_eq!(i.swap(), Some(Swap::Guns));
        assert!(i.swapping() && i.swap().is_none(), "down still: it may yet be the blade");
        i.update(ui, None, true, rest, 0.1);
        assert!(!i.swapping() && i.swap().is_none());
        // Held on: the blade, once.
        i.update(ui, None, true, down, 0.016);
        assert_eq!(i.swap(), Some(Swap::Guns));
        i.update(ui, None, true, held, 0.2);
        assert!(i.swapping() && i.swap().is_none());
        i.update(ui, None, true, held, 0.2);
        assert_eq!(i.swap(), Some(Swap::Blade));
        assert!(!i.swapping());
        i.update(ui, None, true, held, 0.5);
        assert!(i.swap().is_none(), "once a hold");
    });
}

#[test]
fn view_tapped_is_the_bag_and_held_is_the_map() {
    let mut h = Harness::new(800.0, 600.0);
    h.frame(|ui| {
        let (tap, held, rest) = (pad::frame_with(&[Button::Select]), pad::frame_holding(&[Button::Select]), PadFrame::default());
        let mut i = Input::default();
        // Down and up again between two frames: a tap, at once.
        i.update(ui, None, true, tap, 0.016);
        assert!(!i.pressed(ui, Action::Map) && i.pressed(ui, Action::Inventory));
        assert!(!i.pressed(ui, Action::Inventory), "taken once");
        // Held a little, then let go: a tap, as it's let go.
        i.update(ui, None, true, held.merge(tap), 0.016);
        assert!(!i.pressed(ui, Action::Inventory), "not while it's still down");
        i.update(ui, None, true, held, 0.1);
        i.update(ui, None, true, rest, 0.016);
        assert!(i.pressed(ui, Action::Inventory) && !i.pressed(ui, Action::Map));
        // Held on: the map, once, and no bag when it's let go.
        i.update(ui, None, true, held.merge(tap), 0.016);
        i.update(ui, None, true, held, 0.5);
        assert!(i.pressed(ui, Action::Map) && !i.pressed(ui, Action::Inventory));
        i.update(ui, None, true, held, 0.5);
        assert!(!i.pressed(ui, Action::Map), "once a hold");
        i.update(ui, None, true, rest, 0.016);
        assert!(!i.pressed(ui, Action::Inventory));
        // The map up, a tap puts it away (and doesn't put the bag up).
        i.set_mapped(true);
        i.update(ui, None, true, tap, 0.016);
        assert!(!i.pressed(ui, Action::Inventory) && i.pressed(ui, Action::Map));
        assert_eq!(i.name(Action::Inventory), "VIEW");
    });
}

#[test]
fn the_bag_up_a_pad_s_controls_arent_the_game_s() {
    let mut h = Harness::new(800.0, 600.0);
    h.frame(|ui| {
        let mut pad = pad::frame_with(&[Button::Left, Button::South, Button::Select]);
        pad.left = Vec2::new(1.0, 0.0);
        let mut i = Input::default();
        i.set_rummaging(true);
        i.update(ui, None, true, pad, 0.016);
        assert_eq!(i.walk(ui), Vec2::ZERO, "the stick's the cursor's");
        assert!(!i.pressed(ui, Action::Bandage) && !i.pressed(ui, Action::Jump) && i.swap().is_none());
        let steer = i.steer(0.016);
        assert_eq!((steer.step, steer.pick), ((1, 0), true), "they're the bag's");
        assert!(i.pad_pressed(Action::Inventory), "and View still puts it away");
        i.set_rummaging(false);
        i.update(ui, None, true, pad, 0.016);
        assert!(i.pressed(ui, Action::Bandage) && i.walk(ui).x > 0.9);
    });
}

#[test]
fn view_held_is_the_radio_where_that_s_what_s_asked_for_and_q_is_on_the_keys() {
    let mut h = Harness::new(800.0, 600.0);
    h.key(lntrn_ui::Key::Char('q'));
    h.frame(|ui| {
        let (tap, held) = (pad::frame_with(&[Button::Select]), pad::frame_holding(&[Button::Select]));
        let mut i = Input::default();
        // Tapped: the bag, not the radio.
        i.update(ui, None, true, tap, 0.016);
        assert!(!i.pressed(ui, Action::Radio) && i.pressed(ui, Action::Inventory));
        // Held on: the radio, once (and no bag as it's let go).
        i.update(ui, None, true, held.merge(tap), 0.016);
        i.update(ui, None, true, held, 0.5);
        assert!(i.pressed(ui, Action::Radio) && !i.pressed(ui, Action::Radio));
        i.update(ui, None, true, PadFrame::default(), 0.016);
        assert!(!i.pressed(ui, Action::Inventory) && !i.pressed(ui, Action::Radio));
        assert_eq!(i.name(Action::Radio), "VIEW");
        // On the keys it's Q.
        i.update(ui, Some(Keys::default()), true, PadFrame::default(), 0.016);
        assert!(i.pressed(ui, Action::Radio) && !i.pressed(ui, Action::Radio));
    });
}

#[test]
fn the_radio_out_the_keys_that_walk_and_the_d_pad_dial_and_only_the_stick_walks() {
    let mut h = Harness::new(800.0, 600.0);
    h.event(lntrn_ui::Event::Key { key: lntrn_ui::Key::Char('w'), pressed: true, repeat: false, mods: Default::default() });
    h.frame(|ui| {
        let mut i = Input::default();
        // Not dialling: W (down, and held) walks, and nothing's dialled.
        i.update(ui, Some(Keys::default()), true, PadFrame::default(), 0.016);
        assert!(i.dial(ui).is_none() && i.walk(ui).y > 0.9);
        // Dialling: W is up, once, and the feet stand still.
        i.set_dialing(true);
        assert_eq!((i.dial(ui), i.dial(ui)), (Some(Arrow::Up), None));
        assert_eq!(i.walk(ui), Vec2::ZERO);
        // On a pad the d-pad dials (no bandage's put on), the stick walks,
        // and the rest is the game's still.
        let mut pad = pad::frame_with(&[Button::Left, Button::South]);
        pad.left = Vec2::new(0.0, 1.0);
        i.update(ui, None, true, pad, 0.016);
        assert!(!i.pressed(ui, Action::Bandage) && i.pressed(ui, Action::Jump));
        assert_eq!((i.dial(ui), i.dial(ui)), (Some(Arrow::Left), None));
        assert!(i.walk(ui).y > 0.9);
        i.set_dialing(false);
        i.update(ui, None, true, pad, 0.016);
        assert!(i.pressed(ui, Action::Bandage) && i.dial(ui).is_none());
    });
}
